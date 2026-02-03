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

/// XMI namespace URIs.
pub mod namespace {
    /// XMI 2.5.1 namespace.
    pub const XMI: &str = "http://www.omg.org/spec/XMI/20131001";
    /// KerML namespace.
    pub const KERML: &str = "http://www.omg.org/spec/KerML/20230201";
    /// SysML v2 namespace.
    pub const SYSML: &str = "http://www.omg.org/spec/SysML/20230201";
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
            }
        }

        pub fn read(&mut self, input: &[u8]) -> Result<Model, InterchangeError> {
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

            // Skip the XMI root element or namespace root
            if tag_name == "xmi:XMI"
                || tag_name == "XMI"
                || tag_name == "sysml:Namespace"
                || tag_name == "kerml:Namespace"
            {
                self.depth_stack.push(StackEntry::Root);
                return Ok(());
            }

            // Check if this is a containment wrapper (but NOT ownedRelationship or ownedRelatedElement - we want to parse those)
            if is_containment_tag(tag_name)
                && tag_name != "ownedRelationship"
                && tag_name != "ownedRelatedElement"
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
            let mut is_abstract = false;
            let mut is_variation = false;
            let mut is_derived = false;
            let mut is_readonly = false;
            let mut is_parallel = false;
            let mut is_standard = false;
            let mut is_composite = false;
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
                    "xmi:type" | "xsi:type" | "type" => xmi_type = Some(value),
                    "name" | "declaredName" => name = Some(value),
                    "qualifiedName" => qualified_name = Some(value),
                    "shortName" | "declaredShortName" => short_name = Some(value),
                    "elementId" => element_id = Some(value),
                    "isAbstract" => is_abstract = value == "true",
                    "isVariation" => is_variation = value == "true",
                    "isDerived" => is_derived = value == "true",
                    "isReadOnly" => is_readonly = value == "true",
                    "isParallel" => is_parallel = value == "true",
                    "isStandard" => is_standard = value == "true",
                    "isComposite" => is_composite = value == "true",
                    "body" => body = Some(value),
                    "href" => href = Some(value),
                    // Relationship source/target references
                    "source" | "relatedElement" | "subclassifier" | "typedFeature"
                    | "redefiningFeature" | "subsettingFeature" => source_ref = Some(value),
                    "target" | "superclassifier" | "redefinedFeature" | "subsettedFeature"
                    | "general" | "specific" => target_ref = Some(value),
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

                if let Some(n) = name {
                    element.name = Some(Arc::from(n.as_str()));
                }
                if let Some(qn) = qualified_name {
                    element.qualified_name = Some(Arc::from(qn.as_str()));
                }
                if let Some(sn) = short_name {
                    element.short_name = Some(Arc::from(sn.as_str()));
                }

                // Set boolean flags
                element.is_abstract = is_abstract;
                element.is_variation = is_variation;
                element.is_derived = is_derived;
                element.is_readonly = is_readonly;
                element.is_parallel = is_parallel;

                // Store isStandard in properties
                if is_standard {
                    element
                        .properties
                        .insert(Arc::from("isStandard"), PropertyValue::Boolean(true));
                }
                if is_composite {
                    element
                        .properties
                        .insert(Arc::from("isComposite"), PropertyValue::Boolean(true));
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

                // If this is a relationship kind, also create a Relationship
                if kind.is_relationship() {
                    if let (Some(src), Some(tgt)) = (
                        source_ref.or_else(|| self.parent_stack.last().cloned()),
                        target_ref,
                    ) {
                        let rel_kind = element_kind_to_relationship_kind(kind);
                        let relationship = Relationship::new(id.clone(), rel_kind, src, tgt);
                        self.relationships.push(relationship);
                    }
                }

                self.elements_by_id.insert(id.clone(), element);
                self.parent_stack.push(id.clone());
                self.depth_stack.push(StackEntry::Element(id));
            } else {
                // Element without ID - still track for depth
                self.depth_stack.push(StackEntry::Containment);
            }

            Ok(())
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

        fn build_model(&mut self) -> Result<Model, InterchangeError> {
            let mut model = Model::new();

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

    /// Convert ElementKind to RelationshipKind for relationship elements.
    fn element_kind_to_relationship_kind(kind: ElementKind) -> RelationshipKind {
        match kind {
            ElementKind::Specialization => RelationshipKind::Specialization,
            ElementKind::FeatureTyping => RelationshipKind::FeatureTyping,
            ElementKind::Subsetting => RelationshipKind::Subsetting,
            ElementKind::Redefinition => RelationshipKind::Redefinition,
            ElementKind::Import | ElementKind::NamespaceImport => RelationshipKind::NamespaceImport,
            ElementKind::MembershipImport => RelationshipKind::MembershipImport,
            ElementKind::Membership => RelationshipKind::Membership,
            ElementKind::OwningMembership => RelationshipKind::OwningMembership,
            ElementKind::FeatureMembership => RelationshipKind::FeatureMembership,
            ElementKind::Conjugation => RelationshipKind::Conjugation,
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

    /// XMI document writer.
    pub struct XmiWriter;

    impl XmiWriter {
        pub fn new() -> Self {
            Self
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let mut buffer = Cursor::new(Vec::new());
            let mut writer = Writer::new_with_indent(&mut buffer, b' ', 2);

            // Write XML declaration
            writer
                .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            // Write XMI root element
            let mut xmi_start = BytesStart::new("xmi:XMI");
            xmi_start.push_attribute(("xmlns:xmi", namespace::XMI));
            xmi_start.push_attribute(("xmlns:kerml", namespace::KERML));
            xmi_start.push_attribute(("xmlns:sysml", namespace::SYSML));

            writer
                .write_event(Event::Start(xmi_start))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            // Write root elements
            for root in model.iter_roots() {
                self.write_element(&mut writer, model, root)?;
            }

            // Close XMI root
            writer
                .write_event(Event::End(BytesEnd::new("xmi:XMI")))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            Ok(buffer.into_inner())
        }

        fn write_element<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            model: &Model,
            element: &Element,
        ) -> Result<(), InterchangeError> {
            let type_name = element.kind.xmi_type();

            let mut elem_start = BytesStart::new(type_name);
            elem_start.push_attribute(("xmi:id", element.id.as_str()));

            if let Some(ref name) = element.name {
                elem_start.push_attribute(("name", name.as_ref()));
            }
            if let Some(ref qualified_name) = element.qualified_name {
                elem_start.push_attribute(("qualifiedName", qualified_name.as_ref()));
            }
            if let Some(ref short_name) = element.short_name {
                elem_start.push_attribute(("shortName", short_name.as_ref()));
            }

            // Write boolean flags (only if true, per XMI convention)
            if element.is_abstract {
                elem_start.push_attribute(("isAbstract", "true"));
            }
            if element.is_variation {
                elem_start.push_attribute(("isVariation", "true"));
            }
            if element.is_derived {
                elem_start.push_attribute(("isDerived", "true"));
            }
            if element.is_readonly {
                elem_start.push_attribute(("isReadOnly", "true"));
            }
            if element.is_parallel {
                elem_start.push_attribute(("isParallel", "true"));
            }
            if let Some(super::super::model::PropertyValue::Boolean(true)) =
                element.properties.get("isStandard")
            {
                elem_start.push_attribute(("isStandard", "true"));
            }
            if let Some(super::super::model::PropertyValue::Boolean(true)) =
                element.properties.get("isComposite")
            {
                elem_start.push_attribute(("isComposite", "true"));
            }

            // Write documentation body if present
            if let Some(ref doc) = element.documentation {
                elem_start.push_attribute(("body", doc.as_ref()));
            }

            // Write href if present (for cross-file references)
            if let Some(super::super::model::PropertyValue::String(href)) =
                element.properties.get("href")
            {
                elem_start.push_attribute(("href", href.as_ref()));
            }

            // Write other stored attributes
            for (key, value) in &element.properties {
                // Skip ones we've already handled
                if key.as_ref() == "isStandard"
                    || key.as_ref() == "isComposite"
                    || key.as_ref() == "href"
                {
                    continue;
                }
                if let super::super::model::PropertyValue::String(s) = value {
                    elem_start.push_attribute((key.as_ref(), s.as_ref()));
                }
            }

            // Check if we have children
            let has_children = !element.owned_elements.is_empty();

            if has_children {
                writer
                    .write_event(Event::Start(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                // Write all owned children in order, choosing wrapper based on kind
                for child_id in &element.owned_elements {
                    if let Some(child) = model.get(child_id) {
                        // Use ownedRelationship for relationship kinds, ownedMember for others
                        let wrapper = if child.kind.is_relationship() {
                            "ownedRelationship"
                        } else {
                            "ownedMember"
                        };

                        writer
                            .write_event(Event::Start(BytesStart::new(wrapper)))
                            .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

                        self.write_element(writer, model, child)?;

                        writer
                            .write_event(Event::End(BytesEnd::new(wrapper)))
                            .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
                    }
                }

                writer
                    .write_event(Event::End(BytesEnd::new(type_name)))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            } else {
                // Self-closing element
                writer
                    .write_event(Event::Empty(elem_start))
                    .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;
            }

            Ok(())
        }

        #[allow(dead_code)]
        fn write_relationship<W: std::io::Write>(
            &self,
            writer: &mut Writer<W>,
            _model: &Model,
            rel: &Relationship,
        ) -> Result<(), InterchangeError> {
            // Start ownedRelationship wrapper
            writer
                .write_event(Event::Start(BytesStart::new("ownedRelationship")))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            // Write the relationship element
            let rel_type = relationship_kind_to_xmi_type(rel.kind);
            let mut rel_start = BytesStart::new(rel_type);
            rel_start.push_attribute(("xmi:id", rel.id.as_str()));

            // Add source and target based on relationship kind
            match rel.kind {
                RelationshipKind::Specialization => {
                    rel_start.push_attribute(("subclassifier", rel.source.as_str()));
                    rel_start.push_attribute(("superclassifier", rel.target.as_str()));
                }
                RelationshipKind::FeatureTyping => {
                    rel_start.push_attribute(("typedFeature", rel.source.as_str()));
                    rel_start.push_attribute(("type", rel.target.as_str()));
                }
                RelationshipKind::Redefinition => {
                    rel_start.push_attribute(("redefiningFeature", rel.source.as_str()));
                    rel_start.push_attribute(("redefinedFeature", rel.target.as_str()));
                }
                RelationshipKind::Subsetting => {
                    rel_start.push_attribute(("subsettingFeature", rel.source.as_str()));
                    rel_start.push_attribute(("subsettedFeature", rel.target.as_str()));
                }
                _ => {
                    rel_start.push_attribute(("source", rel.source.as_str()));
                    rel_start.push_attribute(("target", rel.target.as_str()));
                }
            }

            writer
                .write_event(Event::Empty(rel_start))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            // End ownedRelationship wrapper
            writer
                .write_event(Event::End(BytesEnd::new("ownedRelationship")))
                .map_err(|e| InterchangeError::xml(format!("Write error: {e}")))?;

            Ok(())
        }
    }

    /// Convert RelationshipKind to XMI type name.
    fn relationship_kind_to_xmi_type(kind: RelationshipKind) -> &'static str {
        match kind {
            RelationshipKind::Specialization => "kerml:Specialization",
            RelationshipKind::FeatureTyping => "kerml:FeatureTyping",
            RelationshipKind::Subsetting => "kerml:Subsetting",
            RelationshipKind::Redefinition => "kerml:Redefinition",
            RelationshipKind::NamespaceImport => "kerml:NamespaceImport",
            RelationshipKind::MembershipImport => "kerml:MembershipImport",
            RelationshipKind::Membership => "kerml:Membership",
            RelationshipKind::OwningMembership => "kerml:OwningMembership",
            RelationshipKind::FeatureMembership => "kerml:FeatureMembership",
            RelationshipKind::Conjugation => "kerml:Conjugation",
            RelationshipKind::Dependency => "kerml:Dependency",
            _ => "kerml:Relationship", // Fallback for other kinds
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

            assert!(output_str.contains("xmi:XMI"));
            assert!(output_str.contains("sysml:Package"));
            assert!(output_str.contains(r#"xmi:id="pkg1""#));
            assert!(output_str.contains(r#"name="TestPackage""#));
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
    }
}
