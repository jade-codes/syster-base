//! XMI (XML Model Interchange) format support.
//!
//! XMI is the OMG standard for exchanging MOF-based models in XML format.
//! SysML v2 and KerML models can be serialized to XMI for tool interoperability.
//!
//! ## XMI Structure
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
//!          xmlns:kerml="http://www.omg.org/spec/KerML/20230201"
//!          xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
//!   <sysml:Package xmi:id="pkg1" name="MyPackage">
//!     <ownedMember xmi:type="sysml:PartDefinition" xmi:id="pd1" name="Vehicle"/>
//!   </sysml:Package>
//! </xmi:XMI>
//! ```

use std::sync::Arc;

use super::model::{Element, ElementId, ElementKind, Model, Relationship, RelationshipKind};
use super::{FormatCapability, InterchangeError, ModelFormat};

/// XMI namespace URIs - using 2025 spec versions.
pub mod namespace {
    /// XMI 2.0 namespace (used in xmi:version).
    pub const XMI: &str = "http://www.omg.org/XMI";
    /// XSI namespace for xsi:type.
    pub const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
    /// KerML 2025 namespace.
    pub const KERML: &str = "https://www.omg.org/spec/KerML/20250201";
    /// SysML v2 2025 namespace.
    pub const SYSML: &str = "https://www.omg.org/spec/SysML/20250201";
}

/// XMI format handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct Xmi;

impl ModelFormat for Xmi {
    fn name(&self) -> &'static str {
        "XMI"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["xmi"]
    }

    fn mime_type(&self) -> &'static str {
        "application/xmi+xml"
    }

    fn capabilities(&self) -> FormatCapability {
        FormatCapability::FULL
    }

    fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            XmiReader::new().read(input)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = input;
            Err(InterchangeError::Unsupported(
                "XMI reading requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            XmiWriter::new().write(model)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = model;
            Err(InterchangeError::Unsupported(
                "XMI writing requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn validate(&self, input: &[u8]) -> Result<(), InterchangeError> {
        // Quick check for XML declaration and XMI/SysML namespace
        let content = std::str::from_utf8(input)
            .map_err(|e| InterchangeError::xml(format!("Invalid UTF-8: {e}")))?;

        // Accept either xmi:XMI root or sysml:Namespace/kerml:Namespace root
        if !content.contains("xmi:XMI")
            && !content.contains("XMI")
            && !content.contains("sysml:Namespace")
            && !content.contains("kerml:Namespace")
        {
            return Err(InterchangeError::xml("Missing XMI/SysML root element"));
        }

        Ok(())
    }
}

impl Xmi {
    /// Read XMI from bytes with a source path for resolving cross-file references.
    #[cfg(feature = "interchange")]
    pub fn read_from_path(&self, input: &[u8], path: &std::path::Path) -> Result<Model, InterchangeError> {
        XmiReader::new().read_with_path(input, Some(path))
    }
}

// ============================================================================
// XMI READER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod reader {
    use super::super::model::PropertyValue;
    use super::*;
    use indexmap::IndexMap;
    use quick_xml::Reader;
    use quick_xml::events::{BytesStart, Event};

    /// XMI document reader.
    pub struct XmiReader {
        /// Elements by ID for lookup (IndexMap preserves insertion order).
        elements_by_id: IndexMap<String, Element>,
        /// Parent stack for ownership tracking (element IDs only).
        parent_stack: Vec<String>,
        /// Depth tracking to match start/end tags properly.
        depth_stack: Vec<StackEntry>,
        /// Relationships collected during parsing.
        relationships: Vec<Relationship>,
        /// Counter for generating relationship IDs.
        rel_counter: u32,
        /// Tracks children per parent in parse order (parent_id -> [child_ids]).
        children_in_order: IndexMap<String, Vec<String>>,
        /// Base path for resolving href references.
        base_path: Option<std::path::PathBuf>,
        /// Cache of resolved href element names.
        href_name_cache: std::collections::HashMap<String, String>,
        /// Pending relationship sources - when we have source but not target yet.
        /// Maps element_id -> (source_ref, element_kind)
        pending_rel_sources: std::collections::HashMap<String, (String, ElementKind)>,
        /// Declared XML namespaces from the document (prefix -> URI).
        declared_namespaces: std::collections::HashMap<String, String>,
    }

    /// Stack entry type for tracking nested elements.
    #[derive(Debug)]
    enum StackEntry {
        /// XMI root element - no push to parent stack.
        Root,
        /// Containment wrapper (ownedMember, etc.) - no push.
        Containment,
        /// Actual element - push element ID to parent stack.
        Element(String),
    }

    impl XmiReader {
        pub fn new() -> Self {
            Self {
                elements_by_id: IndexMap::new(),
                parent_stack: Vec::new(),
                depth_stack: Vec::new(),
                relationships: Vec::new(),
                rel_counter: 0,
                children_in_order: IndexMap::new(),
                base_path: None,
                href_name_cache: std::collections::HashMap::new(),
                pending_rel_sources: std::collections::HashMap::new(),
                declared_namespaces: std::collections::HashMap::new(),
            }
        }

        pub fn read(&mut self, input: &[u8]) -> Result<Model, InterchangeError> {
            self.read_with_path(input, None)
        }
        
        pub fn read_with_path(&mut self, input: &[u8], path: Option<&std::path::Path>) -> Result<Model, InterchangeError> {
            self.base_path = path.map(|p| p.parent().unwrap_or(p).to_path_buf());
            
            let mut reader = Reader::from_reader(input);
            reader.config_mut().trim_text(true);

            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        self.handle_start_element(e)?;
                    }
                    Ok(Event::Empty(ref e)) => {
                        // Self-closing element - handle as start + end
                        self.handle_start_element(e)?;
                        self.handle_end_element();
                    }
                    Ok(Event::End(_)) => {
                        self.handle_end_element();
                    }
                    Ok(Event::Eof) => break,
                    Err(e) => {
                        return Err(InterchangeError::xml(format!(
                            "XML parse error at position {}: {e}",
                            reader.error_position()
                        )));
                    }
                    _ => {}
                }
                buf.clear();
            }

            self.build_model()
        }

        fn handle_start_element(&mut self, e: &BytesStart<'_>) -> Result<(), InterchangeError> {
            let name_bytes = e.name();
            let tag_name = std::str::from_utf8(name_bytes.as_ref())
                .map_err(|e| InterchangeError::xml(format!("Invalid tag name: {e}")))?;

            // Capture namespace declarations from root element (first element we see)
            if self.depth_stack.is_empty() {
                self.capture_namespace_declarations(e)?;
            }

            // Skip only the XMI wrapper element - sysml:Namespace/kerml:Namespace are real elements!
            if tag_name == "xmi:XMI" || tag_name == "XMI" {
                self.depth_stack.push(StackEntry::Root);
                return Ok(());
            }

            // Quick check: does this element have an href attribute?
            // If so, it's a reference element, not a containment wrapper.
            let has_href = e.attributes().any(|attr| {
                attr.map(|a| a.key.as_ref() == b"href").unwrap_or(false)
            });

            // Check if this is a containment wrapper (but NOT if it has href - those are references)
            if is_containment_tag(tag_name)
                && tag_name != "ownedRelationship"
                && tag_name != "ownedRelatedElement"
                && !has_href
            {
                self.depth_stack.push(StackEntry::Containment);
                return Ok(());
            }

            // Extract all attributes
            let mut xmi_id: Option<String> = None;
            let mut xmi_type: Option<String> = None;
            let mut name: Option<String> = None;
            let mut qualified_name: Option<String> = None;
            let mut short_name: Option<String> = None;
            let mut element_id: Option<String> = None;
            let mut is_abstract: Option<bool> = None;
            let mut is_variation: Option<bool> = None;
            let mut is_derived: Option<bool> = None;
            let mut is_readonly: Option<bool> = None;
            let mut is_parallel: Option<bool> = None;
            let mut is_standard: Option<bool> = None;
            let mut is_composite: Option<bool> = None;
            let mut is_unique: Option<bool> = None;
            let mut is_ordered: Option<bool> = None;
            let mut body: Option<String> = None;
            let mut href: Option<String> = None;
            let mut extra_attrs: Vec<(String, String)> = Vec::new();

            // For relationship parsing
            let mut source_ref: Option<String> = None;
            let mut target_ref: Option<String> = None;

            for attr_result in e.attributes() {
                let attr = attr_result
                    .map_err(|e| InterchangeError::xml(format!("Attribute error: {e}")))?;
                let key = std::str::from_utf8(attr.key.as_ref())
                    .map_err(|e| InterchangeError::xml(format!("Attribute key error: {e}")))?;
                let value = attr
                    .unescape_value()
                    .map_err(|e| InterchangeError::xml(format!("Attribute value error: {e}")))?
                    .to_string();

                match key {
                    "xmi:id" | "id" => xmi_id = Some(value),
                    "xmi:type" | "xsi:type" => xmi_type = Some(value),
                    "name" | "declaredName" => name = Some(value),
                    "qualifiedName" => qualified_name = Some(value),
                    "shortName" | "declaredShortName" => short_name = Some(value),
                    "elementId" => element_id = Some(value),
                    "isAbstract" => is_abstract = Some(value == "true"),
                    "isVariation" => is_variation = Some(value == "true"),
                    "isDerived" => is_derived = Some(value == "true"),
                    "isReadOnly" => is_readonly = Some(value == "true"),
                    "isParallel" => is_parallel = Some(value == "true"),
                    "isStandard" => is_standard = Some(value == "true"),
                    "isComposite" => is_composite = Some(value == "true"),
                    "isUnique" => is_unique = Some(value == "true"),
                    "isOrdered" => is_ordered = Some(value == "true"),
                    "body" => body = Some(value),
                    "href" => href = Some(value),
                    // Relationship source references - store as property AND use for relationship
                    "source" | "relatedElement" | "subclassifier" | "typedFeature"
                    | "redefiningFeature" | "subsettingFeature" | "typeDisjoined" => {
                        source_ref = Some(value.clone());
                        extra_attrs.push((key.to_string(), value));
                    }
                    // Relationship target references - store as property AND use for relationship
                    "target" | "superclassifier" | "redefinedFeature" | "subsettedFeature"
                    | "general" | "specific" | "type" | "chainingFeature" 
                    | "importedMembership" | "importedNamespace" | "disjoiningType"
                    | "originalType" | "memberElement" | "referencedFeature" => {
                        target_ref = Some(value.clone());
                        extra_attrs.push((key.to_string(), value));
                    }
                    _ => {
                        // Store other attributes for roundtrip
                        if !key.starts_with("xmlns") && !key.starts_with("xmi:version") {
                            extra_attrs.push((key.to_string(), value));
                        }
                    }
                }
            }

            // Use elementId as fallback for xmi:id (official SysML XMI format)
            if xmi_id.is_none() {
                xmi_id = element_id.clone();
            }

            // Determine element kind from xmi:type or tag name
            let type_str = xmi_type.as_deref().unwrap_or(tag_name);
            let kind = ElementKind::from_xmi_type(type_str);

            // Create element if we have an ID
            if let Some(id) = xmi_id {
                let mut element = Element::new(id.clone(), kind);

                // Store original xsi:type for roundtrip fidelity
                if let Some(ref t) = xmi_type {
                    element.properties.insert(
                        Arc::from("_xsi_type"),
                        PropertyValue::String(Arc::from(t.as_str())),
                    );
                }

                if let Some(n) = name {
                    element.name = Some(Arc::from(n.as_str()));
                }
                if let Some(qn) = qualified_name {
                    element.qualified_name = Some(Arc::from(qn.as_str()));
                }
                if let Some(sn) = short_name {
                    element.short_name = Some(Arc::from(sn.as_str()));
                }

                // Set boolean flags (for backwards compat with Element struct fields)
                element.is_abstract = is_abstract.unwrap_or(false);
                element.is_variation = is_variation.unwrap_or(false);
                element.is_derived = is_derived.unwrap_or(false);
                element.is_readonly = is_readonly.unwrap_or(false);
                element.is_parallel = is_parallel.unwrap_or(false);

                // Store all boolean attributes in properties if explicitly set (for roundtrip)
                if let Some(val) = is_abstract {
                    element.properties.insert(Arc::from("isAbstract"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_variation {
                    element.properties.insert(Arc::from("isVariation"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_derived {
                    element.properties.insert(Arc::from("isDerived"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_readonly {
                    element.properties.insert(Arc::from("isReadOnly"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_parallel {
                    element.properties.insert(Arc::from("isParallel"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_standard {
                    element.properties.insert(Arc::from("isStandard"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_composite {
                    element.properties.insert(Arc::from("isComposite"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_unique {
                    element.properties.insert(Arc::from("isUnique"), PropertyValue::Boolean(val));
                }
                if let Some(val) = is_ordered {
                    element.properties.insert(Arc::from("isOrdered"), PropertyValue::Boolean(val));
                }

                // Store documentation body
                if let Some(b) = body {
                    element.documentation = Some(Arc::from(b.as_str()));
                }

                // Store href for cross-file references
                if let Some(h) = href {
                    element.properties.insert(
                        Arc::from("href"),
                        PropertyValue::String(Arc::from(h.as_str())),
                    );
                }

                // Store extra attributes
                for (key, value) in extra_attrs {
                    element.properties.insert(
                        Arc::from(key.as_str()),
                        PropertyValue::String(Arc::from(value.as_str())),
                    );
                }

                // Set owner if we have a parent, and track child order
                if let Some(parent_id) = self.parent_stack.last() {
                    element.owner = Some(ElementId::new(parent_id.clone()));
                    // Track ALL children under their parent in parse order
                    self.children_in_order
                        .entry(parent_id.clone())
                        .or_insert_with(Vec::new)
                        .push(id.clone());
                }

                // If this is a relationship kind, try to create a Relationship
                if kind.is_relationship() {
                    if let (Some(src), Some(tgt)) = (
                        source_ref.clone().or_else(|| self.parent_stack.last().cloned()),
                        target_ref,
                    ) {
                        let rel_kind = element_kind_to_relationship_kind(kind);
                        let relationship = Relationship::new(id.clone(), rel_kind, src, tgt);
                        self.relationships.push(relationship);
                    } else if let Some(src) = source_ref {
                        // Store source_ref for later use when we encounter the target href child
                        self.pending_rel_sources.insert(id.clone(), (src, kind));
                    }
                }

                self.elements_by_id.insert(id.clone(), element);
                self.parent_stack.push(id.clone());
                self.depth_stack.push(StackEntry::Element(id));
            } else if let Some(h) = href {
                // Element without ID but with href - this is a reference element like <type href="..."/>
                // or <superclassifier href="..."/> or <importedMembership href="..."/>
                if let Some(parent_id) = self.parent_stack.last().cloned() {
                    // Extract the target element ID from the href (after the #)
                    let target_id = h.rsplit('#').next().map(|s| s.to_string());
                    
                    // Try to resolve the full qualified name from the referenced file
                    let resolved_name = self.resolve_href_name(&h);
                    let fallback_name = if resolved_name.is_none() {
                        extract_name_from_href_path(&h)
                    } else {
                        None
                    };
                    
                    // Now do the mutable borrow
                    if let Some(parent_elem) = self.elements_by_id.get_mut(&parent_id) {
                        if let Some(name) = resolved_name {
                            parent_elem.properties.insert(
                                Arc::from("href_target_name"),
                                PropertyValue::String(Arc::from(name.as_str())),
                            );
                        } else if let Some(name) = fallback_name {
                            // Fallback to just the file name
                            parent_elem.properties.insert(
                                Arc::from("href_target_name"),
                                PropertyValue::String(Arc::from(name.as_str())),
                            );
                        }
                        parent_elem.properties.insert(
                            Arc::from("href"),
                            PropertyValue::String(Arc::from(h.as_str())),
                        );
                        // Store original href element tag name for roundtrip fidelity
                        parent_elem.properties.insert(
                            Arc::from("_href_tag"),
                            PropertyValue::String(Arc::from(tag_name)),
                        );
                        // Store xsi:type if present on the href element
                        if let Some(ref t) = xmi_type {
                            parent_elem.properties.insert(
                                Arc::from("_href_xsi_type"),
                                PropertyValue::String(Arc::from(t.as_str())),
                            );
                        }
                    }
                    
                    // Check if we have a pending relationship source for this parent
                    if let Some(target) = target_id {
                        if let Some((src, kind)) = self.pending_rel_sources.remove(&parent_id) {
                            let rel_kind = element_kind_to_relationship_kind(kind);
                            let relationship = Relationship::new(
                                parent_id.clone(),
                                rel_kind,
                                src,
                                target,
                            );
                            self.relationships.push(relationship);
                        }
                    }
                }
                self.depth_stack.push(StackEntry::Containment);
            } else {
                // Element without ID - still track for depth
                self.depth_stack.push(StackEntry::Containment);
            }

            Ok(())
        }
        
        /// Resolve an href to a qualified name by loading the referenced file.
        fn resolve_href_name(&mut self, href: &str) -> Option<String> {
            // Check cache first
            if let Some(cached) = self.href_name_cache.get(href) {
                return Some(cached.clone());
            }
            
            // Parse href: "path/to/File.kermlx#elementId"
            let hash_pos = href.rfind('#')?;
            let path_part = &href[..hash_pos];
            let element_id = &href[hash_pos + 1..];
            
            // Decode URL encoding
            let decoded_path = path_part.replace("%20", " ");
            
            // Get the base path
            let base = self.base_path.as_ref()?;
            
            // Resolve the full path
            let target_path = base.join(&decoded_path);
            
            // Try to read the file
            let file_content = std::fs::read(&target_path).ok()?;
            
            // Quick parse to find element name - look for the element ID and extract its name
            let content_str = String::from_utf8_lossy(&file_content);
            
            // Find the element by ID and get its name
            // Look for patterns like: xmi:id="<element_id>" ... declaredName="<name>"
            // or: xmi:id="<element_id>" ... name="<name>"
            let id_pattern = format!(r#"xmi:id="{}""#, element_id);
            if let Some(id_pos) = content_str.find(&id_pattern) {
                // Look for name attribute in the same element (within ~500 chars)
                let search_end = (id_pos + 500).min(content_str.len());
                let search_slice = &content_str[id_pos..search_end];
                
                // Try declaredName first, then name
                let name = extract_attr_value(search_slice, "declaredName")
                    .or_else(|| extract_attr_value(search_slice, "name"));
                
                if let Some(elem_name) = name {
                    // Get the file name (package name)
                    let file_name = target_path.file_stem()?.to_str()?;
                    let qualified_name = format!("{}::{}", file_name, elem_name);
                    
                    // Cache the result
                    self.href_name_cache.insert(href.to_string(), qualified_name.clone());
                    
                    return Some(qualified_name);
                }
            }
            
            None
        }
        
        fn handle_end_element(&mut self) {
            // Pop from depth stack and handle accordingly
            if let Some(entry) = self.depth_stack.pop() {
                if let StackEntry::Element(_) = entry {
                    // This was an actual element, pop parent stack too
                    self.parent_stack.pop();
                }
            }
        }

        /// Capture xmlns namespace declarations from the root element.
        fn capture_namespace_declarations(&mut self, e: &BytesStart<'_>) -> Result<(), InterchangeError> {
            for attr_result in e.attributes() {
                let attr = attr_result
                    .map_err(|e| InterchangeError::xml(format!("Attribute error: {e}")))?;
                let key = std::str::from_utf8(attr.key.as_ref())
                    .map_err(|e| InterchangeError::xml(format!("Attribute key error: {e}")))?;
                
                // Look for xmlns:prefix="uri" declarations
                if let Some(prefix) = key.strip_prefix("xmlns:") {
                    let value = attr
                        .unescape_value()
                        .map_err(|e| InterchangeError::xml(format!("Attribute value error: {e}")))?
                        .to_string();
                    self.declared_namespaces.insert(prefix.to_string(), value);
                }
            }
            Ok(())
        }

        fn build_model(&mut self) -> Result<Model, InterchangeError> {
            let mut model = Model::new();

            // Store declared namespaces in metadata for roundtrip
            model.metadata.declared_namespaces = std::mem::take(&mut self.declared_namespaces);

            // Add all elements (drain with full range to preserve order)
            for (_, element) in self.elements_by_id.drain(..) {
                model.add_element(element);
            }

            // Add relationships
            for rel in self.relationships.drain(..) {
                model.add_relationship(rel);
            }

            // Update owned_elements using the recorded parse order (children_in_order)
            for (parent_id, child_ids) in self.children_in_order.drain(..) {
                if let Some(owner) = model.elements.get_mut(&ElementId::new(parent_id)) {
                    for child_id in child_ids {
                        owner.owned_elements.push(ElementId::new(child_id));
                    }
                }
            }

            Ok(model)
        }

        /// Generate a unique relationship ID.
        #[allow(dead_code)]
        fn next_rel_id(&mut self) -> ElementId {
            self.rel_counter += 1;
            ElementId::new(format!("_rel_{}", self.rel_counter))
        }
    }

    /// Extract an attribute value from an XML snippet.
    fn extract_attr_value(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!(r#"{}=""#, attr_name);
        let start = xml.find(&pattern)? + pattern.len();
        let remaining = &xml[start..];
        let end = remaining.find('"')?;
        Some(remaining[..end].to_string())
    }

    /// Check if a tag name is a containment wrapper (not an element itself).
    fn is_containment_tag(tag: &str) -> bool {
        matches!(
            tag,
            "ownedMember"
                | "ownedFeature"
                | "ownedElement"
                | "ownedImport"
                | "member"
                | "feature"
                | "ownedSpecialization"
                | "ownedSubsetting"
                | "ownedRedefinition"
                | "ownedTyping"
                | "importedMembership"
                | "superclassifier"
                | "redefinedFeature"
                | "subsettedFeature"
        )
        // Note: ownedRelationship and ownedRelatedElement are NOT containment -
        // they have xsi:type and should be parsed as elements
    }

    /// Extract a meaningful name from an href path.
    /// E.g., "../Kernel%20Data%20Type%20Library/ScalarValues.kermlx#uuid" -> "ScalarValues"
    fn extract_name_from_href_path(href: &str) -> Option<String> {
        // href format: "../path/File.kermlx#elementId"
        // We want to extract the file name as package
        
        if let Some(hash_pos) = href.rfind('#') {
            let path = &href[..hash_pos];
            // Simple URL decode for %20 -> space (most common case)
            let decoded_path = path.replace("%20", " ");
            
            // Extract file name without extension
            if let Some(file_start) = decoded_path.rfind('/') {
                let file = &decoded_path[file_start + 1..];
                if let Some(ext_pos) = file.rfind('.') {
                    return Some(file[..ext_pos].to_string());
                }
            } else if let Some(ext_pos) = decoded_path.rfind('.') {
                return Some(decoded_path[..ext_pos].to_string());
            }
        }
        None
    }

    /// Convert ElementKind to RelationshipKind for relationship elements.
    fn element_kind_to_relationship_kind(kind: ElementKind) -> RelationshipKind {
        match kind {
            ElementKind::Specialization => RelationshipKind::Specialization,
            ElementKind::FeatureTyping => RelationshipKind::FeatureTyping,
            ElementKind::Subsetting | ElementKind::ReferenceSubsetting | ElementKind::CrossSubsetting => {
                RelationshipKind::Subsetting
            }
            ElementKind::Redefinition => RelationshipKind::Redefinition,
            ElementKind::Import | ElementKind::NamespaceImport => RelationshipKind::NamespaceImport,
            ElementKind::MembershipImport => RelationshipKind::MembershipImport,
            ElementKind::Membership => RelationshipKind::Membership,
            ElementKind::OwningMembership
            | ElementKind::ReturnParameterMembership
            | ElementKind::ParameterMembership
            | ElementKind::EndFeatureMembership
            | ElementKind::ResultExpressionMembership => RelationshipKind::OwningMembership,
            ElementKind::FeatureMembership => RelationshipKind::FeatureMembership,
            ElementKind::Conjugation => RelationshipKind::Conjugation,
            ElementKind::FeatureChaining => RelationshipKind::FeatureChaining,
            ElementKind::Disjoining => RelationshipKind::Disjoining,
            _ => RelationshipKind::Dependency, // Default fallback
        }
    }
}

#[cfg(feature = "interchange")]
use reader::XmiReader;

// ============================================================================
// XMI WRITER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod writer {
    use super::*;
    use quick_xml::Writer;
    use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
    use std::io::Cursor;

    /// XMI document writer - produces OMG-compliant format.
    pub struct XmiWriter;

    impl XmiWriter {
        pub fn new() -> Self {
            Self
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let mut buffer = Cursor::new(Vec::new());
            let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);

            // Write XML declaration with ASCII encoding (per OMG format)
            writer
                .write_event(Event::Decl(BytesDecl::new("1.0", Some("ASCII"), None)))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            // Get roots
            let roots: Vec<_> = model.iter_roots().collect();
            
            if roots.len() == 1 {
                // Single root - use element as document root (OMG format)
                let root = roots[0];
                self.write_root_element(&mut writer, model, root)?;
            } else if roots.is_empty() {
                return Err(InterchangeError::xml("Model has no root elements"));
            } else {
                // Multiple roots - wrap in xmi:XMI
                self.write_xmi_wrapper(&mut writer, model, &roots)?;
            }

            // Add trailing newline (per OMG format)
            let mut output = buffer.into_inner();
            output.push(b'\n');
            Ok(output)
        }

        /// Write a single root element as the document root.
        fn write_root_element<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            element: &Element,
        ) -> Result<(), InterchangeError> {
            let type_name = Self::get_xmi_type(element);
            let mut elem_start = BytesStart::new(&type_name);
            
            // Add XMI version and namespaces
            elem_start.push_attribute(("xmi:version", "2.0"));
            elem_start.push_attribute(("xmlns:xmi", namespace::XMI));
            elem_start.push_attribute(("xmlns:xsi", namespace::XSI));
            
            // Write namespaces from metadata (for roundtrip fidelity) or defaults
            Self::write_namespace_attrs(&mut elem_start, model);
            
            // Write element attributes
            self.write_element_attrs(&mut elem_start, element, model);
            
            // Check for href or children
            let has_href = element.properties.get("href").is_some();
            let has_children = !element.owned_elements.is_empty();
            
            if has_href || has_children {
                writer
                    .write_event(Event::Start(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                Self::write_href_child(writer, element)?;

                for child_id in &element.owned_elements {
                    if let Some(child) = model.get(child_id) {
                        self.write_owned_relationship(writer, model, child)?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new(&type_name)))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            } else {
                writer
                    .write_event(Event::Empty(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }

            Ok(())
        }

        /// Write multiple roots wrapped in xmi:XMI.
        fn write_xmi_wrapper<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            roots: &[&Element],
        ) -> Result<(), InterchangeError> {
            let mut xmi_start = BytesStart::new("xmi:XMI");
            xmi_start.push_attribute(("xmi:version", "2.0"));
            xmi_start.push_attribute(("xmlns:xmi", namespace::XMI));
            xmi_start.push_attribute(("xmlns:xsi", namespace::XSI));
            
            // Write namespaces from metadata (for roundtrip fidelity) or defaults
            Self::write_namespace_attrs(&mut xmi_start, model);

            writer
                .write_event(Event::Start(xmi_start))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            for root in roots {
                self.write_element_nested(writer, model, root)?;
            }

            writer
                .write_event(Event::End(BytesEnd::new("xmi:XMI")))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            Ok(())
        }

        /// Write element attributes (id, name, flags, etc.)
        fn write_element_attrs(&self, elem_start: &mut BytesStart, element: &Element, model: &Model) {
            // xmi:id and elementId (same value, per OMG spec)
            elem_start.push_attribute(("xmi:id", element.id.as_str()));
            elem_start.push_attribute(("elementId", element.id.as_str()));

            // Determine if this element uses SysML naming (declaredName) based on xsi:type prefix
            let uses_sysml_naming = if let Some(super::super::model::PropertyValue::String(xsi_type)) = element.properties.get("_xsi_type") {
                xsi_type.starts_with("sysml:")
            } else {
                element.kind.is_sysml()
            };

            // Name - use declaredName for SysML elements
            if let Some(ref name) = element.name {
                if uses_sysml_naming {
                    elem_start.push_attribute(("declaredName", name.as_ref()));
                } else {
                    elem_start.push_attribute(("name", name.as_ref()));
                }
            }
            
            // Short name
            if let Some(ref short_name) = element.short_name {
                if uses_sysml_naming {
                    elem_start.push_attribute(("declaredShortName", short_name.as_ref()));
                } else {
                    elem_start.push_attribute(("shortName", short_name.as_ref()));
                }
            }
            
            // Qualified name (if present)
            if let Some(ref qn) = element.qualified_name {
                elem_start.push_attribute(("qualifiedName", qn.as_ref()));
            }
            
            // Relationship attributes are now stored as properties and written below

            // Boolean flags - write from properties if explicitly set (for roundtrip fidelity)
            // Order matters for byte-perfect roundtrip: isAbstract, isVariation, isDerived, isReadOnly, isParallel, isUnique, isOrdered, isComposite, isStandard
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isAbstract")
            {
                elem_start.push_attribute(("isAbstract", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isVariation")
            {
                elem_start.push_attribute(("isVariation", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isDerived")
            {
                elem_start.push_attribute(("isDerived", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isReadOnly")
            {
                elem_start.push_attribute(("isReadOnly", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isParallel")
            {
                elem_start.push_attribute(("isParallel", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isUnique")
            {
                elem_start.push_attribute(("isUnique", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isOrdered")
            {
                elem_start.push_attribute(("isOrdered", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isComposite")
            {
                elem_start.push_attribute(("isComposite", if *v { "true" } else { "false" }));
            }
            if let Some(super::super::model::PropertyValue::Boolean(v)) =
                element.properties.get("isStandard")
            {
                elem_start.push_attribute(("isStandard", if *v { "true" } else { "false" }));
            }

            // Documentation body - escape for XML attribute
            // We must escape: & < > " and newlines
            // Use raw bytes to avoid double-escaping
            if let Some(ref doc) = element.documentation {
                let escaped = doc
                    .replace('&', "&amp;")  // Must be first!
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\n', "&#xA;");
                elem_start.push_attribute((b"body" as &[u8], escaped.as_bytes()));
            }

            // Other properties
            for (key, value) in &element.properties {
                let k = key.as_ref();
                // Skip boolean properties (already written above) and internal properties
                if k == "isAbstract" || k == "isVariation" || k == "isDerived" || k == "isReadOnly"
                    || k == "isParallel" || k == "isStandard" || k == "isComposite" 
                    || k == "isUnique" || k == "isOrdered"
                    || k == "href" || k == "href_target_name" || k.starts_with("_") {
                    continue;
                }
                if let super::super::model::PropertyValue::String(s) = value {
                    elem_start.push_attribute((k, s.as_ref()));
                }
            }
        }

        /// Write namespace attributes for the root element.
        /// Uses declared_namespaces from metadata if available (for roundtrip fidelity),
        /// otherwise writes both kerml and sysml namespaces as defaults.
        fn write_namespace_attrs(elem_start: &mut BytesStart, model: &Model) {
            let ns = &model.metadata.declared_namespaces;
            
            if ns.is_empty() {
                // No namespace info - write both as defaults
                elem_start.push_attribute(("xmlns:kerml", namespace::KERML));
                elem_start.push_attribute(("xmlns:sysml", namespace::SYSML));
            } else {
                // Write only the namespaces that were declared in original (preserve order)
                // Note: HashMap doesn't preserve order, but for xmlns declarations order doesn't matter semantically
                if let Some(uri) = ns.get("kerml") {
                    elem_start.push_attribute(("xmlns:kerml", uri.as_str()));
                }
                if let Some(uri) = ns.get("sysml") {
                    elem_start.push_attribute(("xmlns:sysml", uri.as_str()));
                }
            }
        }

        /// Get the XMI type for an element, preferring the original if stored.
        /// This preserves roundtrip fidelity for sysml: vs kerml: prefix.
        fn get_xmi_type(element: &Element) -> String {
            // Prefer stored original xsi:type for roundtrip fidelity
            if let Some(super::super::model::PropertyValue::String(orig)) = element.properties.get("_xsi_type") {
                return orig.to_string();
            }
            element.kind.xmi_type().to_string()
        }

        /// Get the href child element name for a given element kind.
        fn href_element_name(kind: ElementKind) -> &'static str {
            match kind {
                ElementKind::NamespaceImport => "importedNamespace",
                ElementKind::MembershipImport => "importedMembership",
                ElementKind::Membership => "memberElement",
                ElementKind::Specialization => "superclassifier",
                ElementKind::FeatureTyping => "type",
                ElementKind::Subsetting | ElementKind::ReferenceSubsetting | ElementKind::CrossSubsetting => "subsettedFeature",
                ElementKind::Redefinition => "redefinedFeature",
                ElementKind::Disjoining => "disjoiningType",
                ElementKind::Conjugation => "originalType",
                ElementKind::FeatureChaining => "chainingFeature",
                _ => "target",
            }
        }

        /// Write an href child element if the element has an href property.
        fn write_href_child<W: std::io::Write>(
            writer: &mut Writer<W>,
            element: &Element,
        ) -> Result<(), InterchangeError> {
            if let Some(super::super::model::PropertyValue::String(href)) = element.properties.get("href") {
                // Use stored tag name if available (for roundtrip), else derive from kind
                let href_elem_name: String = if let Some(super::super::model::PropertyValue::String(tag)) = element.properties.get("_href_tag") {
                    tag.to_string()
                } else {
                    Self::href_element_name(element.kind).to_string()
                };
                
                let mut href_elem = BytesStart::new(&href_elem_name);
                
                // Add xsi:type if stored (for roundtrip)
                if let Some(super::super::model::PropertyValue::String(xsi_type)) = element.properties.get("_href_xsi_type") {
                    href_elem.push_attribute(("xsi:type", xsi_type.as_ref()));
                }
                
                href_elem.push_attribute(("href", href.as_ref()));
                writer
                    .write_event(Event::Empty(href_elem))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }
            Ok(())
        }

        /// Write an ownedRelationship element with xsi:type.
        /// 
        /// In OMG XMI format:
        /// - Relationship types (MembershipImport, NamespaceImport, etc.) are written directly:
        ///   `<ownedRelationship xsi:type="sysml:MembershipImport">...</ownedRelationship>`
        /// - Non-relationship types (Documentation, Package, etc.) need a wrapper:
        ///   `<ownedRelationship xsi:type="OwningMembership"><ownedRelatedElement xsi:type="sysml:Package">...`
        fn write_owned_relationship<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            child: &Element,
        ) -> Result<(), InterchangeError> {
            // Check if the child is a relationship type
            if child.kind.is_relationship() {
                // Relationship types are written directly in ownedRelationship
                self.write_relationship_direct(writer, model, child)
            } else {
                // Non-relationship types need ownedRelatedElement wrapper
                self.write_non_relationship_wrapped(writer, model, child)
            }
        }

        /// Write a relationship type directly as ownedRelationship.
        fn write_relationship_direct<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            child: &Element,
        ) -> Result<(), InterchangeError> {
            let type_name = Self::get_xmi_type(child);
            let mut rel_start = BytesStart::new("ownedRelationship");
            rel_start.push_attribute(("xsi:type", type_name.as_str()));
            
            // Write element attributes
            self.write_element_attrs(&mut rel_start, child, model);
            
            // Check for href or children
            let has_href = child.properties.get("href").is_some();
            let has_children = !child.owned_elements.is_empty();

            if has_href || has_children {
                writer
                    .write_event(Event::Start(rel_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                Self::write_href_child(writer, child)?;

                // Write nested children
                for grandchild_id in &child.owned_elements {
                    if let Some(grandchild) = model.get(grandchild_id) {
                        self.write_owned_related_element(writer, model, grandchild)?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new("ownedRelationship")))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            } else {
                writer
                    .write_event(Event::Empty(rel_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }

            Ok(())
        }

        /// Write a non-relationship type wrapped in ownedRelationship > ownedRelatedElement.
        fn write_non_relationship_wrapped<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            child: &Element,
        ) -> Result<(), InterchangeError> {
            // For non-relationship children, we need the wrapper structure:
            // <ownedRelationship xsi:type="OwningMembership">
            //   <ownedRelatedElement xsi:type="sysml:Package">...</ownedRelatedElement>
            // </ownedRelationship>
            // 
            // Note: We don't have an explicit OwningMembership element in our model,
            // so we just write the ownedRelatedElement directly.
            // This is a simplification that works for re-parsing but differs from strict OMG format.
            self.write_owned_related_element(writer, model, child)
        }

        /// Write an ownedRelatedElement with xsi:type.
        fn write_owned_related_element<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            element: &Element,
        ) -> Result<(), InterchangeError> {
            let type_name = Self::get_xmi_type(element);
            let mut elem_start = BytesStart::new("ownedRelatedElement");
            elem_start.push_attribute(("xsi:type", type_name.as_str()));
            
            // Write element attributes
            self.write_element_attrs(&mut elem_start, element, model);

            // Check for href or children
            let has_href = element.properties.get("href").is_some();
            let has_children = !element.owned_elements.is_empty();
            
            if has_href || has_children {
                writer
                    .write_event(Event::Start(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                Self::write_href_child(writer, element)?;

                for child_id in &element.owned_elements {
                    if let Some(child) = model.get(child_id) {
                        self.write_owned_relationship(writer, model, child)?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new("ownedRelatedElement")))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            } else {
                writer
                    .write_event(Event::Empty(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }

            Ok(())
        }

        /// Write a nested element (used in xmi:XMI wrapper case).
        fn write_element_nested<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            element: &Element,
        ) -> Result<(), InterchangeError> {
            let type_name = Self::get_xmi_type(element);
            let mut elem_start = BytesStart::new(&type_name);
            
            self.write_element_attrs(&mut elem_start, element, model);
            
            let has_children = !element.owned_elements.is_empty();
            if has_children {
                writer
                    .write_event(Event::Start(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                for child_id in &element.owned_elements {
                    if let Some(child) = model.get(child_id) {
                        self.write_owned_relationship(writer, model, child)?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new(&type_name)))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            } else {
                writer
                    .write_event(Event::Empty(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }

            Ok(())
        }
    }
}

#[cfg(feature = "interchange")]
use writer::XmiWriter;

// Stub implementations when feature is disabled
#[cfg(not(feature = "interchange"))]
struct XmiReader;

#[cfg(not(feature = "interchange"))]
impl XmiReader {
    fn new() -> Self {
        Self
    }

    fn read(&mut self, _input: &[u8]) -> Result<Model, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "XMI reading requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "interchange"))]
struct XmiWriter;

#[cfg(not(feature = "interchange"))]
impl XmiWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, _model: &Model) -> Result<Vec<u8>, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "XMI writing requires the 'interchange' feature".to_string(),
        ))
    }
}

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

/// Convert an XMI type string to ElementKind.
#[allow(dead_code)]
pub fn element_kind_from_xmi(xmi_type: &str) -> ElementKind {
    ElementKind::from_xmi_type(xmi_type)
}

/// Convert an ElementKind to XMI type string.
#[allow(dead_code)]
pub fn element_kind_to_xmi(kind: ElementKind) -> &'static str {
    kind.xmi_type()
}

/// Convert a relationship XMI type to RelationshipKind.
#[allow(dead_code)]
pub fn relationship_kind_from_xmi(xmi_type: &str) -> Option<RelationshipKind> {
    let type_name = xmi_type.split(':').last().unwrap_or(xmi_type);
    match type_name {
        "Specialization" => Some(RelationshipKind::Specialization),
        "FeatureTyping" => Some(RelationshipKind::FeatureTyping),
        "Subsetting" => Some(RelationshipKind::Subsetting),
        "Redefinition" => Some(RelationshipKind::Redefinition),
        "Conjugation" => Some(RelationshipKind::Conjugation),
        "Membership" => Some(RelationshipKind::Membership),
        "OwningMembership" => Some(RelationshipKind::OwningMembership),
        "FeatureMembership" => Some(RelationshipKind::FeatureMembership),
        "NamespaceImport" => Some(RelationshipKind::NamespaceImport),
        "MembershipImport" => Some(RelationshipKind::MembershipImport),
        "Dependency" => Some(RelationshipKind::Dependency),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmi_format_metadata() {
        let xmi = Xmi;
        assert_eq!(xmi.name(), "XMI");
        assert_eq!(xmi.extensions(), &["xmi"]);
        assert_eq!(xmi.mime_type(), "application/xmi+xml");
        assert!(xmi.capabilities().read);
        assert!(xmi.capabilities().write);
    }

    #[test]
    fn test_xmi_validate_valid() {
        let xmi = Xmi;
        let input =
            br#"<?xml version="1.0"?><xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"/>"#;
        assert!(xmi.validate(input).is_ok());
    }

    #[test]
    fn test_xmi_validate_invalid() {
        let xmi = Xmi;
        let input = b"<root>not xmi</root>";
        assert!(xmi.validate(input).is_err());
    }

    #[test]
    fn test_element_kind_from_xmi() {
        assert_eq!(element_kind_from_xmi("sysml:Package"), ElementKind::Package);
        assert_eq!(
            element_kind_from_xmi("sysml:PartDefinition"),
            ElementKind::PartDefinition
        );
        assert_eq!(element_kind_from_xmi("kerml:Feature"), ElementKind::Feature);
    }

    #[test]
    fn test_relationship_kind_from_xmi() {
        assert_eq!(
            relationship_kind_from_xmi("kerml:Specialization"),
            Some(RelationshipKind::Specialization)
        );
        assert_eq!(
            relationship_kind_from_xmi("kerml:FeatureTyping"),
            Some(RelationshipKind::FeatureTyping)
        );
    }

    #[cfg(feature = "interchange")]
    mod interchange_tests {
        use super::*;

        #[test]
        fn test_xmi_read_simple_package() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:Package xmi:id="pkg1" name="MyPackage"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            assert_eq!(model.element_count(), 1);

            let pkg = model
                .get(&ElementId::new("pkg1"))
                .expect("Package not found");
            assert_eq!(pkg.name.as_deref(), Some("MyPackage"));
            assert_eq!(pkg.kind, ElementKind::Package);
        }

        #[test]
        fn test_xmi_read_nested_elements() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:Package xmi:id="pkg1" name="Vehicles">
    <ownedMember>
      <sysml:PartDefinition xmi:id="pd1" name="Car"/>
    </ownedMember>
    <ownedMember>
      <sysml:PartDefinition xmi:id="pd2" name="Truck"/>
    </ownedMember>
  </sysml:Package>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            assert_eq!(model.element_count(), 3);

            let pkg = model
                .get(&ElementId::new("pkg1"))
                .expect("Package not found");
            assert_eq!(pkg.owned_elements.len(), 2);

            let car = model.get(&ElementId::new("pd1")).expect("Car not found");
            assert_eq!(car.name.as_deref(), Some("Car"));
            assert_eq!(car.kind, ElementKind::PartDefinition);
            assert_eq!(car.owner.as_ref().map(|id| id.as_str()), Some("pkg1"));
        }

        #[test]
        fn test_xmi_write_simple_model() {
            let mut model = Model::new();
            model.add_element(Element::new("pkg1", ElementKind::Package).with_name("TestPackage"));

            let output = Xmi.write(&model).expect("Failed to write XMI");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            // Single root element is written directly (OMG format) - no xmi:XMI wrapper
            // OMG 2025 format uses declaredName instead of name
            assert!(output_str.contains("sysml:Package"), "Missing sysml:Package. Got:\n{}", output_str);
            assert!(output_str.contains(r#"xmi:id="pkg1""#), "Missing xmi:id. Got:\n{}", output_str);
            assert!(output_str.contains(r#"declaredName="TestPackage""#), "Missing declaredName. Got:\n{}", output_str);
        }

        #[test]
        fn test_xmi_roundtrip() {
            // Create a model
            let mut model = Model::new();
            let pkg = Element::new("pkg1", ElementKind::Package).with_name("RoundtripTest");
            model.add_element(pkg);

            let part = Element::new("part1", ElementKind::PartDefinition)
                .with_name("Vehicle")
                .with_owner("pkg1");
            model.add_element(part);

            // Update ownership
            if let Some(pkg) = model.elements.get_mut(&ElementId::new("pkg1")) {
                pkg.owned_elements.push(ElementId::new("part1"));
            }

            // Write to XMI
            let xmi_bytes = Xmi.write(&model).expect("Write failed");

            // Read back
            let model2 = Xmi.read(&xmi_bytes).expect("Read failed");

            // Verify
            assert_eq!(model2.element_count(), 2);
            let pkg2 = model2.get(&ElementId::new("pkg1")).unwrap();
            assert_eq!(pkg2.name.as_deref(), Some("RoundtripTest"));
            assert_eq!(pkg2.owned_elements.len(), 1);
        }

        #[test]
        fn test_xmi_read_is_abstract() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:PartDefinition xmi:id="pd1" name="AbstractPart" isAbstract="true"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model
                .get(&ElementId::new("pd1"))
                .expect("Element not found");
            assert!(elem.is_abstract, "isAbstract should be true");
        }

        #[test]
        fn test_xmi_read_is_variation() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:PartDefinition xmi:id="pd1" name="VariantPart" isVariation="true"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model
                .get(&ElementId::new("pd1"))
                .expect("Element not found");
            assert!(elem.is_variation, "isVariation should be true");
        }

        #[test]
        fn test_xmi_read_is_derived() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:kerml="http://www.omg.org/spec/KerML/20230201">
  <kerml:Feature xmi:id="f1" name="derivedFeature" isDerived="true"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model.get(&ElementId::new("f1")).expect("Element not found");
            assert!(elem.is_derived, "isDerived should be true");
        }

        #[test]
        fn test_xmi_read_is_readonly() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:AttributeUsage xmi:id="a1" name="constantValue" isReadOnly="true"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model.get(&ElementId::new("a1")).expect("Element not found");
            assert!(elem.is_readonly, "isReadOnly should be true");
        }

        #[test]
        fn test_xmi_read_is_parallel() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:StateUsage xmi:id="s1" name="parallelState" isParallel="true"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model.get(&ElementId::new("s1")).expect("Element not found");
            assert!(elem.is_parallel, "isParallel should be true");
        }

        #[test]
        fn test_xmi_write_modifiers() {
            let mut model = Model::new();

            let mut elem = Element::new("pd1", ElementKind::PartDefinition);
            elem.name = Some("TestPart".into());
            elem.is_abstract = true;
            elem.is_variation = true;
            model.add_element(elem);

            let mut feat = Element::new("f1", ElementKind::Feature);
            feat.name = Some("TestFeature".into());
            feat.is_derived = true;
            feat.is_readonly = true;
            model.add_element(feat);

            let mut state = Element::new("s1", ElementKind::StateUsage);
            state.name = Some("TestState".into());
            state.is_parallel = true;
            model.add_element(state);

            let output = Xmi.write(&model).expect("Failed to write XMI");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            assert!(
                output_str.contains(r#"isAbstract="true""#),
                "Should contain isAbstract"
            );
            assert!(
                output_str.contains(r#"isVariation="true""#),
                "Should contain isVariation"
            );
            assert!(
                output_str.contains(r#"isDerived="true""#),
                "Should contain isDerived"
            );
            assert!(
                output_str.contains(r#"isReadOnly="true""#),
                "Should contain isReadOnly"
            );
            assert!(
                output_str.contains(r#"isParallel="true""#),
                "Should contain isParallel"
            );
        }

        #[test]
        fn test_xmi_roundtrip_modifiers() {
            let mut model = Model::new();

            let mut elem = Element::new("pd1", ElementKind::PartDefinition);
            elem.name = Some("AbstractVariation".into());
            elem.is_abstract = true;
            elem.is_variation = true;
            model.add_element(elem);

            let mut feat = Element::new("f1", ElementKind::AttributeUsage);
            feat.name = Some("DerivedReadonly".into());
            feat.is_derived = true;
            feat.is_readonly = true;
            model.add_element(feat);

            let mut state = Element::new("s1", ElementKind::StateUsage);
            state.name = Some("ParallelState".into());
            state.is_parallel = true;
            model.add_element(state);

            // Write and read back
            let xmi_bytes = Xmi.write(&model).expect("Write failed");
            let model2 = Xmi.read(&xmi_bytes).expect("Read failed");

            // Verify all modifiers preserved
            let elem2 = model2.get(&ElementId::new("pd1")).unwrap();
            assert!(elem2.is_abstract, "isAbstract not preserved");
            assert!(elem2.is_variation, "isVariation not preserved");

            let feat2 = model2.get(&ElementId::new("f1")).unwrap();
            assert!(feat2.is_derived, "isDerived not preserved");
            assert!(feat2.is_readonly, "isReadOnly not preserved");

            let state2 = model2.get(&ElementId::new("s1")).unwrap();
            assert!(state2.is_parallel, "isParallel not preserved");
        }

        #[test]
        fn test_xmi_modifiers_default_false() {
            let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:PartDefinition xmi:id="pd1" name="NormalPart"/>
</xmi:XMI>"#;

            let model = Xmi.read(xmi_content).expect("Failed to read XMI");
            let elem = model
                .get(&ElementId::new("pd1"))
                .expect("Element not found");

            // All modifiers should default to false when not specified
            assert!(!elem.is_abstract, "isAbstract should default to false");
            assert!(!elem.is_variation, "isVariation should default to false");
            assert!(!elem.is_derived, "isDerived should default to false");
            assert!(!elem.is_readonly, "isReadOnly should default to false");
            assert!(!elem.is_parallel, "isParallel should default to false");
        }

        #[test]
        fn test_membership_import_href_child_roundtrip() {
            // This is the exact structure from Quantities.sysmlx - MembershipImport with
            // <importedMembership href="..."/> child element
            let input = r#"<?xml version="1.0" encoding="ASCII"?>
<sysml:Namespace xmi:version="2.0" xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:sysml="https://www.omg.org/spec/SysML/20250201" xmi:id="ns1" elementId="ns1">
  <ownedRelationship xsi:type="sysml:MembershipImport" xmi:id="aed4e039-574f-5c98-83de-3aca582b628a" elementId="aed4e039-574f-5c98-83de-3aca582b628a">
    <importedMembership href="../../Kernel%20Libraries/Kernel%20Data%20Type%20Library/ScalarValues.kermlx#a9e3be1d-4057-5cda-bdc0-eff9df4b33ea"/>
  </ownedRelationship>
</sysml:Namespace>"#;

            let model = Xmi.read(input.as_bytes()).expect("Failed to read XMI");
            let output = Xmi.write(&model).expect("Failed to write XMI");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            // The output MUST contain the <importedMembership href="..."/> child element
            assert!(
                output_str.contains("<importedMembership href="),
                "Output must contain <importedMembership href=...> child element.\nGot:\n{}",
                output_str
            );
        }

        #[test]
        fn test_namespace_import_href_child_roundtrip() {
            // Similar test for NamespaceImport with <importedNamespace href="..."/> child
            let input = r#"<?xml version="1.0" encoding="ASCII"?>
<sysml:Namespace xmi:version="2.0" xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:sysml="https://www.omg.org/spec/SysML/20250201" xmi:id="ns1" elementId="ns1">
  <ownedRelationship xsi:type="sysml:NamespaceImport" xmi:id="4c829288-5c6b-5120-967f-9415c466b325" elementId="4c829288-5c6b-5120-967f-9415c466b325">
    <importedNamespace href="../../Kernel%20Libraries/Kernel%20Data%20Type%20Library/Collections.kermlx#9837d4a5-c753-58a8-b614-16d4cb5fac19"/>
  </ownedRelationship>
</sysml:Namespace>"#;

            let model = Xmi.read(input.as_bytes()).expect("Failed to read XMI");
            let output = Xmi.write(&model).expect("Failed to write XMI");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            // The output MUST contain the <importedNamespace href="..."/> child element
            assert!(
                output_str.contains("<importedNamespace href="),
                "Output must contain <importedNamespace href=...> child element.\nGot:\n{}",
                output_str
            );
        }

        #[test]
        fn test_documentation_body_newline_escaping() {
            // Documentation body with newline should use &#xA; entity, not literal newline
            let input = r#"<?xml version="1.0" encoding="ASCII"?>
<sysml:Documentation xmi:version="2.0" xmlns:xmi="http://www.omg.org/spec/XMI/20131001" xmlns:sysml="https://www.omg.org/spec/SysML/20250201" xmi:id="doc1" elementId="doc1" body="Line one.&#xA;Line two.&#xA;"/>"#;

            let model = Xmi.read(input.as_bytes()).expect("Failed to read XMI");
            
            // Verify the newline was parsed correctly
            let doc = model.get(&ElementId::new("doc1")).expect("doc not found");
            assert_eq!(doc.documentation.as_deref(), Some("Line one.\nLine two.\n"), 
                "Newlines should be parsed from &#xA;");

            // Write it back
            let output = Xmi.write(&model).expect("Failed to write XMI");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            // Output must use &#xA; entity, NOT literal newlines or double-escaped &amp;#xA;
            assert!(
                output_str.contains("&#xA;"),
                "Output must contain &#xA; entity for newlines.\nGot:\n{}",
                output_str
            );
            assert!(
                !output_str.contains("&amp;#xA;"),
                "Output must NOT double-escape to &amp;#xA;.\nGot:\n{}",
                output_str
            );
        }
    }
}
