//! Decompile interchange Model to SysML text.
//!
//! This module converts a [`Model`] from XMI/JSON-LD into SysML text that
//! can be parsed by the normal parser. A companion metadata file preserves
//! element IDs and unmapped attributes for lossless round-tripping.
//!
//! ## Usage
//!
//! ```ignore
//! use syster::interchange::{Model, Xmi, ModelFormat};
//! use syster::interchange::decompile::decompile;
//!
//! let model = Xmi.read(&xmi_bytes)?;
//! let (sysml_text, metadata) = decompile(&model);
//! ```

use super::metadata::{ElementMeta, ImportMetadata, SourceInfo};
use super::model::{Element, ElementId, ElementKind, Model, RelationshipKind, Visibility};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;

/// Result of decompiling a model.
pub struct DecompileResult {
    /// Generated SysML text.
    pub text: String,
    /// Metadata preserving element IDs and unmapped info.
    pub metadata: ImportMetadata,
}

/// Decompile a model to SysML text and metadata.
pub fn decompile(model: &Model) -> DecompileResult {
    let mut ctx = DecompileContext::new(model);
    ctx.decompile_model();

    DecompileResult {
        text: ctx.output,
        metadata: ctx.metadata,
    }
}

/// Decompile with source information.
pub fn decompile_with_source(model: &Model, source: SourceInfo) -> DecompileResult {
    let mut ctx = DecompileContext::new(model);
    ctx.metadata.source = source;
    ctx.decompile_model();

    DecompileResult {
        text: ctx.output,
        metadata: ctx.metadata,
    }
}

/// Internal context for decompilation.
struct DecompileContext<'a> {
    model: &'a Model,
    output: String,
    metadata: ImportMetadata,
    indent_level: usize,
    /// Maps ElementId -> qualified name for metadata keys.
    qualified_names: HashMap<ElementId, String>,
    /// Tracks sibling order for each parent.
    sibling_order: HashMap<ElementId, u32>,
}

impl<'a> DecompileContext<'a> {
    fn new(model: &'a Model) -> Self {
        Self {
            model,
            output: String::new(),
            metadata: ImportMetadata::new(),
            indent_level: 0,
            qualified_names: HashMap::new(),
            sibling_order: HashMap::new(),
        }
    }

    fn decompile_model(&mut self) {
        // First pass: compute qualified names for all elements
        self.compute_qualified_names();

        // Decompile root elements
        for root_id in &self.model.roots {
            if let Some(element) = self.model.elements.get(root_id) {
                self.decompile_element(element);
            }
        }
    }

    fn compute_qualified_names(&mut self) {
        // Build parent -> children map for traversal
        for (id, _element) in &self.model.elements {
            let qn = self.compute_qualified_name(id.clone());
            self.qualified_names.insert(id.clone(), qn);
        }
    }

    fn compute_qualified_name(&self, id: ElementId) -> String {
        let Some(element) = self.model.elements.get(&id) else {
            return id.as_str().to_string();
        };

        // Use declared name, or fall back to element ID
        let name = element
            .name
            .as_deref()
            .unwrap_or_else(|| element.id.as_str());

        if let Some(owner_id) = &element.owner {
            if self.model.elements.contains_key(owner_id) {
                let owner_qn = self.compute_qualified_name(owner_id.clone());
                return format!("{}::{}", owner_qn, name);
            }
        }

        name.to_string()
    }

    fn decompile_element(&mut self, element: &Element) {
        // Relationship elements act as transparent containers - decompile their children
        if element.kind.is_relationship() {
            self.decompile_transparent_container(element);
            return;
        }

        // Record metadata
        self.record_metadata(element);

        // Generate SysML text based on element kind
        match element.kind {
            ElementKind::Package => self.decompile_package(element),
            ElementKind::LibraryPackage => self.decompile_library_package(element),
            ElementKind::PartDefinition => self.decompile_definition(element, "part def"),
            ElementKind::ItemDefinition => self.decompile_definition(element, "item def"),
            ElementKind::PortDefinition => self.decompile_definition(element, "port def"),
            ElementKind::AttributeDefinition => self.decompile_definition(element, "attribute def"),
            ElementKind::ActionDefinition => self.decompile_definition(element, "action def"),
            ElementKind::ConnectionDefinition => {
                self.decompile_definition(element, "connection def")
            }
            ElementKind::InterfaceDefinition => self.decompile_definition(element, "interface def"),
            ElementKind::AllocationDefinition => {
                self.decompile_definition(element, "allocation def")
            }
            ElementKind::RequirementDefinition => {
                self.decompile_definition(element, "requirement def")
            }
            ElementKind::ConstraintDefinition => {
                self.decompile_definition(element, "constraint def")
            }
            ElementKind::StateDefinition => self.decompile_definition(element, "state def"),
            ElementKind::CalculationDefinition => self.decompile_definition(element, "calc def"),
            ElementKind::UseCaseDefinition => self.decompile_definition(element, "use case def"),
            ElementKind::AnalysisCaseDefinition => {
                self.decompile_definition(element, "analysis def")
            }
            ElementKind::ViewDefinition => self.decompile_definition(element, "view def"),
            ElementKind::ViewpointDefinition => self.decompile_definition(element, "viewpoint def"),
            ElementKind::RenderingDefinition => self.decompile_definition(element, "rendering def"),
            ElementKind::EnumerationDefinition => self.decompile_definition(element, "enum def"),
            ElementKind::MetadataDefinition => self.decompile_definition(element, "metadata def"),
            ElementKind::ConcernDefinition => self.decompile_definition(element, "concern def"),

            ElementKind::PartUsage => self.decompile_usage(element, "part"),
            ElementKind::ItemUsage => self.decompile_usage(element, "item"),
            ElementKind::PortUsage => self.decompile_usage(element, "port"),
            ElementKind::AttributeUsage => self.decompile_usage(element, "attribute"),
            ElementKind::ActionUsage => self.decompile_usage(element, "action"),
            ElementKind::ConnectionUsage => self.decompile_usage(element, "connection"),
            ElementKind::InterfaceUsage => self.decompile_usage(element, "interface"),
            ElementKind::AllocationUsage => self.decompile_usage(element, "allocation"),
            ElementKind::RequirementUsage => self.decompile_usage(element, "requirement"),
            ElementKind::ConstraintUsage => self.decompile_usage(element, "constraint"),
            ElementKind::StateUsage => self.decompile_usage(element, "state"),
            ElementKind::CalculationUsage => self.decompile_usage(element, "calc"),
            ElementKind::ReferenceUsage => self.decompile_usage(element, "ref"),
            ElementKind::OccurrenceUsage => self.decompile_usage(element, "occurrence"),

            // KerML classifiers
            ElementKind::Class => self.decompile_definition(element, "class"),
            ElementKind::DataType => self.decompile_definition(element, "datatype"),
            ElementKind::Structure => self.decompile_definition(element, "struct"),
            ElementKind::Classifier => self.decompile_definition(element, "classifier"),

            // KerML features
            ElementKind::Feature => self.decompile_feature(element),

            // Multiplicity
            ElementKind::MultiplicityRange => self.decompile_multiplicity(element),

            // Documentation
            ElementKind::Comment => self.decompile_comment(element),
            ElementKind::Documentation => self.decompile_documentation(element),

            // For Other/unknown types, just decompile children (acts as transparent container)
            ElementKind::Other => self.decompile_transparent_container(element),

            // Skip other relationship types handled inline
            _ => {}
        }
    }

    fn record_metadata(&mut self, element: &Element) {
        let qn = self
            .qualified_names
            .get(&element.id)
            .cloned()
            .unwrap_or_else(|| element.id.as_str().to_string());

        let mut meta = ElementMeta::with_id(element.id.as_str());

        // Track sibling order
        if let Some(owner_id) = &element.owner {
            let order = self.sibling_order.entry(owner_id.clone()).or_insert(0);
            meta = meta.with_order(*order);
            *order += 1;
        }

        // Store unmapped properties
        for (key, value) in &element.properties {
            // Convert PropertyValue to serde_json::Value
            let json_value = property_to_json(value);
            meta = meta.with_unmapped(key.as_ref(), json_value);
        }

        self.metadata.add_element(qn, meta);
    }

    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn write_line(&mut self, text: &str) {
        let indent = self.indent();
        let _ = writeln!(self.output, "{}{}", indent, text);
    }

    fn write_blank_line(&mut self) {
        let _ = writeln!(self.output);
    }

    fn decompile_package(&mut self, element: &Element) {
        self.write_visibility(element);

        if let Some(name) = &element.name {
            let short = self.format_short_name(element);
            self.write_line(&format!("package {}{} {{", short, name));
        } else {
            self.write_line("package {");
        }

        self.decompile_body(element);

        self.write_line("}");
        self.write_blank_line();
    }

    fn decompile_library_package(&mut self, element: &Element) {
        self.write_visibility(element);

        // Check for isStandard property
        let is_standard_key: Arc<str> = Arc::from("isStandard");
        let standard_kw = match element.properties.get(&is_standard_key) {
            Some(super::model::PropertyValue::Boolean(true)) => "standard ",
            Some(super::model::PropertyValue::String(s)) if s.as_ref() == "true" => "standard ",
            _ => "",
        };

        if let Some(name) = &element.name {
            let short = self.format_short_name(element);
            self.write_line(&format!(
                "{}library package {}{} {{",
                standard_kw, short, name
            ));
        } else {
            self.write_line(&format!("{}library package {{", standard_kw));
        }

        self.decompile_body(element);

        self.write_line("}");
        self.write_blank_line();
    }

    fn decompile_definition(&mut self, element: &Element, keyword: &str) {
        self.write_visibility(element);

        let abstract_kw = if element.is_abstract { "abstract " } else { "" };
        let short = self.format_short_name(element);
        let specializations = self.format_specializations(&element.id);

        if let Some(name) = &element.name {
            if element.owned_elements.is_empty() && element.documentation.is_none() {
                // Empty definition - use semicolon
                self.write_line(&format!(
                    "{}{} {}{}{};",
                    abstract_kw, keyword, short, name, specializations
                ));
            } else {
                self.write_line(&format!(
                    "{}{} {}{}{} {{",
                    abstract_kw, keyword, short, name, specializations
                ));
                self.decompile_body(element);
                self.write_line("}");
            }
        } else {
            self.write_line(&format!("{}{} {{{}", abstract_kw, keyword, specializations));
            self.decompile_body(element);
            self.write_line("}");
        }

        self.write_blank_line();
    }

    fn decompile_usage(&mut self, element: &Element, keyword: &str) {
        self.write_visibility(element);

        let typing = self.format_typing(&element.id);
        let subsetting = self.format_subsetting(&element.id);
        let redefinition = self.format_redefinition(&element.id);
        let value = self.format_feature_value(&element.id);
        let short = self.format_short_name(element);

        let relations = format!("{}{}{}", typing, subsetting, redefinition);

        // Filter children rendered inline (values and relationships like
        // FeatureTyping, Specialization, etc.) from the "has body" check.
        let has_body_children = element.owned_elements.iter().any(|child_id| {
            self.model
                .get(child_id)
                .map(|c| !c.kind.is_inline_rendered())
                .unwrap_or(false)
        });

        if let Some(name) = &element.name {
            if !has_body_children && element.documentation.is_none() {
                self.write_line(&format!(
                    "{} {}{}{}{};",
                    keyword, short, name, relations, value
                ));
            } else {
                self.write_line(&format!(
                    "{} {}{}{}{} {{",
                    keyword, short, name, relations, value
                ));
                self.decompile_body(element);
                self.write_line("}");
            }
        } else if !relations.is_empty() || !value.is_empty() {
            // Anonymous usage with typing/subsetting/value
            self.write_line(&format!("{}{}{};", keyword, relations, value));
        }
    }

    fn decompile_feature(&mut self, element: &Element) {
        self.write_visibility(element);

        let abstract_kw = if element.is_abstract { "abstract " } else { "" };
        let typing = self.format_typing(&element.id);
        let subsetting = self.format_subsetting(&element.id);
        let redefinition = self.format_redefinition(&element.id);
        let chaining = self.format_chaining(&element.id);
        let multiplicity = self.format_inline_multiplicity(&element.id);
        let short = self.format_short_name(element);

        // Check for additional feature modifiers from properties
        let mut modifiers = Vec::new();
        let is_unique_key: Arc<str> = Arc::from("isUnique");
        if let Some(pv) = element.properties.get(&is_unique_key) {
            match pv {
                super::model::PropertyValue::Boolean(false) => modifiers.push("nonunique"),
                super::model::PropertyValue::String(s) if s.as_ref() == "false" => {
                    modifiers.push("nonunique")
                }
                _ => {}
            }
        }

        let mod_str = if modifiers.is_empty() {
            String::new()
        } else {
            format!(" {}", modifiers.join(" "))
        };

        let value = self.format_feature_value(&element.id);

        // Filter children rendered inline from the "has body" check
        let has_body_children = element.owned_elements.iter().any(|child_id| {
            self.model
                .get(child_id)
                .map(|c| !c.kind.is_inline_rendered())
                .unwrap_or(false)
        });

        // Build the full feature declaration
        // Format: [abstract] feature name : Type [mult] [modifiers] [subsets X] [chains Y] [redefines Z] [= value]
        if let Some(name) = &element.name {
            let decl = format!(
                "{}feature {}{}{}{}{}{}{}{}{}",
                abstract_kw,
                short,
                name,
                typing,
                multiplicity,
                mod_str,
                subsetting,
                chaining,
                redefinition,
                value,
            );

            if !has_body_children && element.documentation.is_none() {
                self.write_line(&format!("{};", decl));
            } else {
                self.write_line(&format!("{} {{", decl));
                self.decompile_body(element);
                self.write_line("}");
            }
        } else {
            // Anonymous feature
            let decl = format!(
                "{}feature{}{}{}{}{}{}",
                abstract_kw, typing, multiplicity, mod_str, subsetting, chaining, redefinition
            );
            self.write_line(&format!("{};", decl));
        }

        self.write_blank_line();
    }

    fn decompile_multiplicity(&mut self, element: &Element) {
        // Multiplicity ranges are typically decompiled inline, but if standalone:
        if let Some(name) = &element.name {
            let bounds = self.format_multiplicity_bounds(&element.id);

            // Check for documentation in children
            let has_doc = element.owned_elements.iter().any(|child_id| {
                self.model
                    .elements
                    .get(child_id)
                    .map(|c| {
                        c.kind == ElementKind::Documentation
                            || c.owned_elements.iter().any(|gc_id| {
                                self.model
                                    .elements
                                    .get(gc_id)
                                    .map(|gc| gc.kind == ElementKind::Documentation)
                                    .unwrap_or(false)
                            })
                    })
                    .unwrap_or(false)
            });

            if has_doc {
                self.write_line(&format!("multiplicity {} {} {{", name, bounds));
                self.indent_level += 1;

                // Decompile documentation
                for child_id in &element.owned_elements {
                    if let Some(child) = self.model.elements.get(child_id) {
                        if child.kind == ElementKind::Documentation {
                            self.decompile_documentation(child);
                        }
                        // Check through membership wrappers
                        for grandchild_id in &child.owned_elements {
                            if let Some(grandchild) = self.model.elements.get(grandchild_id) {
                                if grandchild.kind == ElementKind::Documentation {
                                    self.decompile_documentation(grandchild);
                                }
                            }
                        }
                    }
                }

                self.indent_level -= 1;
                self.write_line("}");
            } else {
                self.write_line(&format!("multiplicity {} {};", name, bounds));
            }
            self.write_blank_line();
        }
    }

    fn format_inline_multiplicity(&self, element_id: &ElementId) -> String {
        // Look for owned MultiplicityRange children
        if let Some(element) = self.model.elements.get(element_id) {
            for child_id in &element.owned_elements {
                if let Some(child) = self.model.elements.get(child_id) {
                    if child.kind == ElementKind::MultiplicityRange {
                        return self.format_multiplicity_bounds(&child.id);
                    }
                    // Check through membership containers
                    if child.kind.is_relationship() {
                        for grandchild_id in &child.owned_elements {
                            if let Some(grandchild) = self.model.elements.get(grandchild_id) {
                                if grandchild.kind == ElementKind::MultiplicityRange {
                                    return self.format_multiplicity_bounds(&grandchild.id);
                                }
                            }
                        }
                    }
                }
            }
        }
        String::new()
    }

    fn format_multiplicity_bounds(&self, mult_id: &ElementId) -> String {
        if let Some(mult_elem) = self.model.elements.get(mult_id) {
            let mut all_literals = Vec::new();

            // Look for LiteralInteger/LiteralInfinity children (may be wrapped in memberships)
            for child_id in &mult_elem.owned_elements {
                if let Some(child) = self.model.elements.get(child_id) {
                    // Go through membership wrappers and collect all literals
                    all_literals.extend(self.collect_literals(child));
                }
            }

            // First literal is lower bound, second is upper bound
            let lower = all_literals.first().cloned();
            let upper = all_literals.get(1).cloned();

            match (lower, upper) {
                (Some(l), Some(u)) => format!("[{}..{}]", l, u),
                (Some(l), None) => format!("[{}]", l),
                (None, Some(u)) => format!("[0..{}]", u),
                (None, None) => String::new(),
            }
        } else {
            String::new()
        }
    }

    fn collect_literals(&self, element: &Element) -> Vec<String> {
        let mut result = Vec::new();
        let value_key: Arc<str> = Arc::from("value");

        match element.kind {
            ElementKind::LiteralInteger => {
                // Check for value in properties (could be Integer or String)
                if let Some(pv) = element.properties.get(&value_key) {
                    match pv {
                        super::model::PropertyValue::Integer(v) => result.push(v.to_string()),
                        super::model::PropertyValue::String(s) => result.push(s.to_string()),
                        _ => result.push("0".to_string()),
                    }
                } else {
                    result.push("0".to_string());
                }
            }
            ElementKind::LiteralReal => {
                if let Some(pv) = element.properties.get(&value_key) {
                    match pv {
                        super::model::PropertyValue::Real(v) => result.push(v.to_string()),
                        super::model::PropertyValue::String(s) => result.push(s.to_string()),
                        _ => result.push("0.0".to_string()),
                    }
                } else {
                    result.push("0.0".to_string());
                }
            }
            ElementKind::LiteralInfinity => {
                result.push("*".to_string());
            }
            _ => {
                // Recurse through membership wrappers
                for child_id in &element.owned_elements {
                    if let Some(child) = self.model.elements.get(child_id) {
                        result.extend(self.collect_literals(child));
                    }
                }
            }
        }

        result
    }

    fn decompile_comment(&mut self, element: &Element) {
        if let Some(doc) = &element.documentation {
            // Multi-line comment
            if doc.contains('\n') {
                self.write_line(&format!("/* {} */", doc));
            } else {
                self.write_line(&format!("// {}", doc));
            }
        }
    }

    fn decompile_documentation(&mut self, element: &Element) {
        if let Some(doc) = &element.documentation {
            self.write_line(&format!("doc /* {} */", doc));
        }
    }

    /// Decompile children of a transparent container (e.g., Namespace wrapper).
    /// These elements don't generate SysML output themselves but their children do.
    fn decompile_transparent_container(&mut self, element: &Element) {
        for child_id in &element.owned_elements {
            if let Some(child) = self.model.elements.get(child_id) {
                self.decompile_element(child);
            }
        }
    }

    fn decompile_body(&mut self, element: &Element) {
        self.indent_level += 1;

        // Imports first
        self.decompile_imports(&element.id);

        // Documentation
        if let Some(doc) = &element.documentation {
            self.write_line(&format!("doc /* {} */", doc));
        }

        // Then children
        self.decompile_children_inner(element);

        self.indent_level -= 1;
    }

    fn decompile_children_inner(&mut self, element: &Element) {
        for child_id in &element.owned_elements {
            if let Some(child) = self.model.elements.get(child_id) {
                self.decompile_element(child);
            }
        }
    }

    fn write_visibility(&mut self, element: &Element) {
        // Only write non-public visibility
        match element.visibility {
            Visibility::Private => {
                let _ = write!(self.output, "{}private ", self.indent());
            }
            Visibility::Protected => {
                let _ = write!(self.output, "{}protected ", self.indent());
            }
            Visibility::Public => {}
        }
    }

    fn format_short_name(&self, element: &Element) -> String {
        if let Some(short) = &element.short_name {
            format!("<{}> ", short)
        } else {
            String::new()
        }
    }

    fn format_specializations(&self, element_id: &ElementId) -> String {
        let specializations: Vec<&str> = self
            .model
            .relationships
            .iter()
            .filter(|r| &r.source == element_id && r.kind == RelationshipKind::Specialization)
            .filter_map(|r| {
                self.model
                    .elements
                    .get(&r.target)
                    .and_then(|e| e.name.as_deref())
            })
            .collect();

        if specializations.is_empty() {
            String::new()
        } else {
            format!(" :> {}", specializations.join(", "))
        }
    }

    fn format_typing(&self, element_id: &ElementId) -> String {
        let mut types: Vec<String> = self
            .model
            .relationships
            .iter()
            .filter(|r| &r.source == element_id && r.kind == RelationshipKind::FeatureTyping)
            .filter_map(|r| self.get_element_ref_name(&r.target))
            .collect();

        // Also check for href-based typing (cross-file references)
        // These are stored on FeatureTyping elements that are children of this feature
        if let Some(element) = self.model.elements.get(element_id) {
            for child_id in &element.owned_elements {
                if let Some(child) = self.model.elements.get(child_id) {
                    // Check if it's a FeatureTyping with href
                    if child.kind == ElementKind::FeatureTyping {
                        let href_key: Arc<str> = Arc::from("href_target_name");
                        if let Some(super::model::PropertyValue::String(name)) =
                            child.properties.get(&href_key)
                        {
                            types.push(name.to_string());
                        }
                    }
                    // Also check nested in memberships
                    for grandchild_id in &child.owned_elements {
                        if let Some(grandchild) = self.model.elements.get(grandchild_id) {
                            if grandchild.kind == ElementKind::FeatureTyping {
                                let href_key: Arc<str> = Arc::from("href_target_name");
                                if let Some(super::model::PropertyValue::String(name)) =
                                    grandchild.properties.get(&href_key)
                                {
                                    types.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        if types.is_empty() {
            String::new()
        } else {
            format!(" : {}", types.join(", "))
        }
    }

    /// Get imports for an element (namespace imports).
    fn get_namespace_imports(&self, element_id: &ElementId) -> Vec<String> {
        self.model
            .relationships
            .iter()
            .filter(|r| {
                r.owner == Some(element_id.clone()) && r.kind == RelationshipKind::NamespaceImport
            })
            .filter_map(|r| {
                self.model
                    .elements
                    .get(&r.target)
                    .and_then(|e| e.qualified_name.as_deref().or(e.name.as_deref()))
                    .map(|n| format!("{}::*", n))
            })
            .collect()
    }

    /// Get imports for an element (membership imports).
    fn get_membership_imports(&self, element_id: &ElementId) -> Vec<String> {
        self.model
            .relationships
            .iter()
            .filter(|r| {
                r.owner == Some(element_id.clone()) && r.kind == RelationshipKind::MembershipImport
            })
            .filter_map(|r| {
                self.model
                    .elements
                    .get(&r.target)
                    .and_then(|e| e.qualified_name.as_deref().or(e.name.as_deref()))
                    .map(|n| n.to_string())
            })
            .collect()
    }

    /// Decompile imports for an element.
    fn decompile_imports(&mut self, element_id: &ElementId) {
        // Namespace imports (import X::*)
        for import_path in self.get_namespace_imports(element_id) {
            self.write_line(&format!("import {};", import_path));
        }

        // Membership imports (import X::Y)
        for import_path in self.get_membership_imports(element_id) {
            self.write_line(&format!("import {};", import_path));
        }
    }

    /// Format subsetting relationships (subsets).
    fn format_subsetting(&self, element_id: &ElementId) -> String {
        let subsets: Vec<String> = self
            .model
            .relationships
            .iter()
            .filter(|r| &r.source == element_id && r.kind == RelationshipKind::Subsetting)
            .filter_map(|r| self.get_element_ref_name(&r.target))
            .collect();

        if subsets.is_empty() {
            String::new()
        } else {
            format!(" subsets {}", subsets.join(", "))
        }
    }

    /// Format redefinition relationships (redefines).
    fn format_redefinition(&self, element_id: &ElementId) -> String {
        let redefines: Vec<String> = self
            .model
            .relationships
            .iter()
            .filter(|r| &r.source == element_id && r.kind == RelationshipKind::Redefinition)
            .filter_map(|r| self.get_qualified_element_ref(&r.target))
            .collect();

        if redefines.is_empty() {
            String::new()
        } else {
            format!(" redefines {}", redefines.join(", "))
        }
    }

    /// Format feature chaining relationships (chains).
    fn format_chaining(&self, element_id: &ElementId) -> String {
        let chains: Vec<String> = self
            .model
            .relationships
            .iter()
            .filter(|r| &r.source == element_id && r.kind == RelationshipKind::FeatureChaining)
            .filter_map(|r| self.get_chaining_ref(&r.target))
            .collect();

        if chains.is_empty() {
            String::new()
        } else {
            format!(" chains {}", chains.join("."))
        }
    }

    /// Format a feature value expression (e.g., ` = 42`, ` = "hello"`, ` = true`).
    /// Returns an empty string if the element has no FeatureValue child.
    fn format_feature_value(&self, element_id: &ElementId) -> String {
        if let Some(element) = self.model.get(element_id) {
            for child_id in &element.owned_elements {
                if let Some(child) = self.model.get(child_id) {
                    if child.kind == ElementKind::FeatureValue {
                        // Find the literal inside
                        for lit_id in &child.owned_elements {
                            if let Some(lit) = self.model.get(lit_id) {
                                let value_key: Arc<str> = Arc::from("value");
                                if let Some(pv) = lit.properties.get(&value_key) {
                                    return match (lit.kind, pv) {
                                        (
                                            ElementKind::LiteralString,
                                            super::model::PropertyValue::String(s),
                                        ) => {
                                            format!(" = \"{}\"", s)
                                        }
                                        (
                                            ElementKind::LiteralInteger,
                                            super::model::PropertyValue::Integer(v),
                                        ) => {
                                            format!(" = {}", v)
                                        }
                                        (
                                            ElementKind::LiteralBoolean,
                                            super::model::PropertyValue::Boolean(b),
                                        ) => {
                                            format!(" = {}", b)
                                        }
                                        (
                                            ElementKind::LiteralReal,
                                            super::model::PropertyValue::Real(v),
                                        ) => {
                                            format!(" = {}", v)
                                        }
                                        (ElementKind::NullExpression, _) => " = null".to_string(),
                                        (
                                            ElementKind::FeatureReferenceExpression,
                                            super::model::PropertyValue::String(s),
                                        ) => {
                                            format!(" = {}", s)
                                        }
                                        _ => String::new(),
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
        String::new()
    }

    /// Get reference for chaining - use simple names joined with dots.
    fn get_chaining_ref(&self, target_id: &ElementId) -> Option<String> {
        // For chaining, just use the simple element name
        // The chain path is built by joining multiple chaining relationships with dots
        self.model
            .elements
            .get(target_id)
            .and_then(|e| e.name.as_ref())
            .map(|n| n.to_string())
    }

    /// Get simple name or href-based name for an element reference.
    fn get_element_ref_name(&self, target_id: &ElementId) -> Option<String> {
        // First try to find the element in the model
        if let Some(element) = self.model.elements.get(target_id) {
            // Check for href (cross-file reference)
            let href_key: Arc<str> = Arc::from("href");
            if let Some(super::model::PropertyValue::String(href)) =
                element.properties.get(&href_key)
            {
                // Extract qualified name from href (format: "file.xmi#uuid" or just qualified name)
                return Some(self.extract_name_from_href(href));
            }
            // Use qualified name if available, else simple name
            if let Some(qn) = &element.qualified_name {
                return Some(qn.to_string());
            }
            return element.name.as_ref().map(|n| n.to_string());
        }
        None
    }

    /// Get qualified reference for redefines/chains (includes owner context).
    fn get_qualified_element_ref(&self, target_id: &ElementId) -> Option<String> {
        if let Some(element) = self.model.elements.get(target_id) {
            // Check for href first
            let href_key: Arc<str> = Arc::from("href");
            if let Some(super::model::PropertyValue::String(href)) =
                element.properties.get(&href_key)
            {
                return Some(self.extract_name_from_href(href));
            }

            // Build qualified name from owner chain
            let name = element.name.as_deref()?;

            // Walk up the ownership chain to find a named classifier/type
            let owner_name = self.find_named_owner(target_id);
            if let Some(owner) = owner_name {
                return Some(format!("{}::{}", owner, name));
            }

            return Some(name.to_string());
        }
        None
    }

    /// Walk up the ownership chain to find a named owner (classifier, package, etc.)
    fn find_named_owner(&self, element_id: &ElementId) -> Option<String> {
        let element = self.model.elements.get(element_id)?;
        let mut current_owner_id = element.owner.clone();

        while let Some(owner_id) = current_owner_id {
            if let Some(owner) = self.model.elements.get(&owner_id) {
                // If this owner has a name and is a "real" element (not a membership), use it
                if let Some(name) = &owner.name {
                    // Skip membership wrappers
                    if !owner.kind.is_relationship() {
                        return Some(name.to_string());
                    }
                }
                // Keep walking up
                current_owner_id = owner.owner.clone();
            } else {
                break;
            }
        }
        None
    }

    /// Extract a meaningful name from an href string.
    fn extract_name_from_href(&self, href: &str) -> String {
        // href format: "../../path/File.sysmlx#uuid" or "File.sysmlx#uuid"
        // Extract the file name and try to form a qualified name

        // First, try to find element by ID in the href
        if let Some(hash_pos) = href.rfind('#') {
            let id = &href[hash_pos + 1..];
            // Check if we have this element
            if let Some(element) = self.model.elements.get(&ElementId::new(id)) {
                if let Some(qn) = &element.qualified_name {
                    return qn.to_string();
                }
                if let Some(name) = &element.name {
                    return name.to_string();
                }
            }
        }

        // Try to extract from path (e.g., "ScalarValues.kermlx#uuid" -> ScalarValues)
        if let Some(hash_pos) = href.rfind('#') {
            let path = &href[..hash_pos];
            if let Some(file_start) = path.rfind('/') {
                let file = &path[file_start + 1..];
                if let Some(ext_pos) = file.rfind('.') {
                    return file[..ext_pos].to_string();
                }
            } else if let Some(ext_pos) = path.rfind('.') {
                return path[..ext_pos].to_string();
            }
        }

        // Fallback: return the href as-is (UUID)
        href.to_string()
    }
}

fn property_to_json(value: &super::model::PropertyValue) -> serde_json::Value {
    use super::model::PropertyValue;
    match value {
        PropertyValue::String(s) => serde_json::Value::String(s.to_string()),
        PropertyValue::Integer(i) => serde_json::Value::Number((*i).into()),
        PropertyValue::Real(f) => serde_json::json!(*f),
        PropertyValue::Boolean(b) => serde_json::Value::Bool(*b),
        PropertyValue::Reference(id) => serde_json::Value::String(format!("ref:{}", id.as_str())),
        PropertyValue::List(items) => {
            serde_json::Value::Array(items.iter().map(property_to_json).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Relationship;
    use super::*;

    #[test]
    fn test_decompile_empty_model() {
        let model = Model::new();
        let result = decompile(&model);

        assert!(result.text.is_empty());
        assert_eq!(result.metadata.version, ImportMetadata::CURRENT_VERSION);
    }

    #[test]
    fn test_decompile_simple_package() {
        let mut model = Model::new();

        let pkg = Element::new("pkg-1", ElementKind::Package).with_name("MyPackage");
        model.elements.insert(pkg.id.clone(), pkg);
        model.roots.push(ElementId::from("pkg-1"));

        let result = decompile(&model);

        assert!(result.text.contains("package MyPackage {"));
        assert!(result.text.contains("}"));

        // Metadata should have the element
        let meta = result.metadata.get_element("MyPackage").unwrap();
        assert_eq!(meta.original_id.as_deref(), Some("pkg-1"));
    }

    #[test]
    fn test_decompile_part_definition() {
        let mut model = Model::new();

        let def = Element::new("def-1", ElementKind::PartDefinition).with_name("Vehicle");
        model.elements.insert(def.id.clone(), def);
        model.roots.push(ElementId::from("def-1"));

        let result = decompile(&model);

        // Empty definition uses semicolon syntax
        assert!(result.text.contains("part def Vehicle;"));
    }

    #[test]
    fn test_decompile_abstract_definition() {
        let mut model = Model::new();

        let mut def = Element::new("def-1", ElementKind::PartDefinition).with_name("AbstractPart");
        def.is_abstract = true;
        model.elements.insert(def.id.clone(), def);
        model.roots.push(ElementId::from("def-1"));

        let result = decompile(&model);

        assert!(result.text.contains("abstract part def AbstractPart;"));
    }

    #[test]
    fn test_decompile_definition_with_short_name() {
        let mut model = Model::new();

        let def = Element::new("def-1", ElementKind::PartDefinition)
            .with_name("Vehicle")
            .with_short_name("V");
        model.elements.insert(def.id.clone(), def);
        model.roots.push(ElementId::from("def-1"));

        let result = decompile(&model);

        assert!(result.text.contains("part def <V> Vehicle;"));
    }

    #[test]
    fn test_decompile_nested_elements() {
        let mut model = Model::new();

        // Package with nested part def
        let pkg_id = ElementId::from("pkg-1");
        let def_id = ElementId::from("def-1");

        let mut pkg = Element::new(pkg_id.clone(), ElementKind::Package).with_name("MyPackage");
        pkg.owned_elements.push(def_id.clone());

        let def = Element::new(def_id.clone(), ElementKind::PartDefinition)
            .with_name("Part1")
            .with_owner(pkg_id.clone());

        model.elements.insert(pkg_id.clone(), pkg);
        model.elements.insert(def_id.clone(), def);
        model.roots.push(pkg_id);

        let result = decompile(&model);

        assert!(result.text.contains("package MyPackage {"));
        assert!(result.text.contains("part def Part1;"));

        // Check metadata has qualified name
        assert!(result.metadata.get_element("MyPackage::Part1").is_some());
    }

    #[test]
    fn test_decompile_with_specialization() {
        let mut model = Model::new();

        // Base definition
        let base = Element::new("base-1", ElementKind::PartDefinition).with_name("Base");
        model.elements.insert(base.id.clone(), base);
        model.roots.push(ElementId::from("base-1"));

        // Derived definition specializing Base
        let derived = Element::new("derived-1", ElementKind::PartDefinition).with_name("Derived");
        model.elements.insert(derived.id.clone(), derived);
        model.roots.push(ElementId::from("derived-1"));

        // Specialization relationship
        let rel = Relationship::new(
            "rel-1",
            RelationshipKind::Specialization,
            "derived-1",
            "base-1",
        );
        model.relationships.push(rel);

        let result = decompile(&model);

        assert!(result.text.contains("part def Derived :> Base;"));
    }

    #[test]
    fn test_decompile_part_usage_with_typing() {
        let mut model = Model::new();

        // Package
        let pkg_id = ElementId::from("pkg-1");
        let def_id = ElementId::from("def-1");
        let usage_id = ElementId::from("usage-1");

        // Type definition
        let def = Element::new(def_id.clone(), ElementKind::PartDefinition).with_name("Engine");

        // Usage with typing
        let usage = Element::new(usage_id.clone(), ElementKind::PartUsage)
            .with_name("engine")
            .with_owner(pkg_id.clone());

        let mut pkg = Element::new(pkg_id.clone(), ElementKind::Package).with_name("Car");
        pkg.owned_elements.push(def_id.clone());
        pkg.owned_elements.push(usage_id.clone());

        model.elements.insert(pkg_id.clone(), pkg);
        model.elements.insert(def_id.clone(), def);
        model.elements.insert(usage_id.clone(), usage);
        model.roots.push(pkg_id);

        // Typing relationship
        let rel = Relationship::new("rel-1", RelationshipKind::FeatureTyping, "usage-1", "def-1");
        model.relationships.push(rel);

        let result = decompile(&model);

        assert!(result.text.contains("part engine : Engine;"));
    }

    #[test]
    fn test_decompile_with_documentation() {
        let mut model = Model::new();

        let mut def = Element::new("def-1", ElementKind::PartDefinition).with_name("Documented");
        def.documentation = Some("This is a documented element.".into());
        // Add a child so it uses braces (body) syntax
        def.owned_elements.push(ElementId::from("dummy"));

        // Add a dummy child element
        let child =
            Element::new("dummy", ElementKind::Comment).with_owner(ElementId::from("def-1"));
        model.elements.insert(child.id.clone(), child);

        model.elements.insert(def.id.clone(), def);
        model.roots.push(ElementId::from("def-1"));

        let result = decompile(&model);

        assert!(
            result
                .text
                .contains("doc /* This is a documented element. */")
        );
    }

    #[test]
    fn test_decompile_library_package() {
        let mut model = Model::new();

        let pkg = Element::new("pkg-1", ElementKind::LibraryPackage).with_name("MyLibrary");
        model.elements.insert(pkg.id.clone(), pkg);
        model.roots.push(ElementId::from("pkg-1"));

        let result = decompile(&model);

        assert!(result.text.contains("library package MyLibrary {"));
    }

    #[test]
    fn test_metadata_preserves_source_info() {
        let model = Model::new();
        let source = SourceInfo::from_path("/path/to/model.xmi").with_format("xmi");

        let result = decompile_with_source(&model, source);

        assert_eq!(
            result.metadata.source.path.as_deref(),
            Some("/path/to/model.xmi")
        );
        assert_eq!(result.metadata.source.format.as_deref(), Some("xmi"));
    }

    #[test]
    fn test_decompile_private_visibility() {
        let mut model = Model::new();

        let mut def = Element::new("def-1", ElementKind::PartDefinition).with_name("PrivatePart");
        def.visibility = Visibility::Private;
        model.elements.insert(def.id.clone(), def);
        model.roots.push(ElementId::from("def-1"));

        let result = decompile(&model);

        assert!(result.text.contains("private part def PrivatePart;"));
    }

    #[test]
    fn test_decompile_namespace_import() {
        let mut model = Model::new();

        // Target package to import
        let target_pkg = Element::new("target-1", ElementKind::Package).with_name("TargetPackage");
        model.elements.insert(target_pkg.id.clone(), target_pkg);
        model.roots.push(ElementId::from("target-1"));

        // Package with import
        let mut pkg = Element::new("pkg-1", ElementKind::Package).with_name("MyPackage");
        pkg.owned_elements.push(ElementId::from("dummy")); // Need a child for body

        // Add a dummy child
        let child = Element::new("dummy", ElementKind::PartDefinition)
            .with_name("Dummy")
            .with_owner(ElementId::from("pkg-1"));
        model.elements.insert(child.id.clone(), child);

        model.elements.insert(pkg.id.clone(), pkg);
        model.roots.push(ElementId::from("pkg-1"));

        // Namespace import relationship (owned by pkg)
        let mut rel = Relationship::new(
            "import-1",
            RelationshipKind::NamespaceImport,
            "pkg-1",
            "target-1",
        );
        rel.owner = Some(ElementId::from("pkg-1"));
        model.relationships.push(rel);

        let result = decompile(&model);

        assert!(result.text.contains("import TargetPackage::*;"));
    }

    #[test]
    fn test_decompile_with_subsetting() {
        let mut model = Model::new();

        // Base feature
        let base = Element::new("base-1", ElementKind::PartUsage).with_name("basePart");
        model.elements.insert(base.id.clone(), base);
        model.roots.push(ElementId::from("base-1"));

        // Derived feature with subsetting
        let derived = Element::new("derived-1", ElementKind::PartUsage).with_name("derivedPart");
        model.elements.insert(derived.id.clone(), derived);
        model.roots.push(ElementId::from("derived-1"));

        // Subsetting relationship
        let rel = Relationship::new("rel-1", RelationshipKind::Subsetting, "derived-1", "base-1");
        model.relationships.push(rel);

        let result = decompile(&model);

        assert!(result.text.contains("part derivedPart subsets basePart;"));
    }

    #[test]
    fn test_decompile_with_redefinition() {
        let mut model = Model::new();

        // Original feature
        let original = Element::new("orig-1", ElementKind::PartUsage).with_name("originalPart");
        model.elements.insert(original.id.clone(), original);
        model.roots.push(ElementId::from("orig-1"));

        // Redefining feature
        let redefining = Element::new("redef-1", ElementKind::PartUsage).with_name("redefPart");
        model.elements.insert(redefining.id.clone(), redefining);
        model.roots.push(ElementId::from("redef-1"));

        // Redefinition relationship
        let rel = Relationship::new("rel-1", RelationshipKind::Redefinition, "redef-1", "orig-1");
        model.relationships.push(rel);

        let result = decompile(&model);

        assert!(
            result
                .text
                .contains("part redefPart redefines originalPart;")
        );
    }

    #[test]
    fn test_decompile_usage_with_typing_and_subsetting() {
        let mut model = Model::new();

        // Type
        let type_def = Element::new("type-1", ElementKind::PartDefinition).with_name("Engine");
        model.elements.insert(type_def.id.clone(), type_def);
        model.roots.push(ElementId::from("type-1"));

        // Base feature
        let base = Element::new("base-1", ElementKind::PartUsage).with_name("basePart");
        model.elements.insert(base.id.clone(), base);
        model.roots.push(ElementId::from("base-1"));

        // Feature with both typing and subsetting
        let feature = Element::new("feat-1", ElementKind::PartUsage).with_name("myEngine");
        model.elements.insert(feature.id.clone(), feature);
        model.roots.push(ElementId::from("feat-1"));

        // Typing relationship
        model.relationships.push(Relationship::new(
            "rel-1",
            RelationshipKind::FeatureTyping,
            "feat-1",
            "type-1",
        ));

        // Subsetting relationship
        model.relationships.push(Relationship::new(
            "rel-2",
            RelationshipKind::Subsetting,
            "feat-1",
            "base-1",
        ));

        let result = decompile(&model);

        assert!(
            result
                .text
                .contains("part myEngine : Engine subsets basePart;")
        );
    }

    #[test]
    fn test_decompile_xmi_roundtrip() {
        use crate::interchange::{ModelFormat, Xmi};
        use crate::syntax::parser::parse_content;
        use std::path::Path;

        // XMI representing a simple model
        let xmi_content = br#"<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmlns:xmi="http://www.omg.org/spec/XMI/20131001"
         xmlns:sysml="http://www.omg.org/spec/SysML/20230201">
  <sysml:Package xmi:id="pkg1" name="VehicleModel">
    <ownedMember>
      <sysml:PartDefinition xmi:id="pd1" name="Vehicle"/>
    </ownedMember>
    <ownedMember>
      <sysml:PartDefinition xmi:id="pd2" name="Car"/>
    </ownedMember>
  </sysml:Package>
</xmi:XMI>"#;

        // Step 1: Load XMI
        let model = Xmi.read(xmi_content).expect("Failed to read XMI");
        assert_eq!(model.element_count(), 3);

        // Step 2: Decompile to SysML text
        let result = decompile(&model);

        // Verify generated text contains expected elements
        assert!(
            result.text.contains("package VehicleModel"),
            "Missing package: {}",
            result.text
        );
        assert!(
            result.text.contains("part def Vehicle"),
            "Missing Vehicle: {}",
            result.text
        );
        assert!(
            result.text.contains("part def Car"),
            "Missing Car: {}",
            result.text
        );

        // Step 3: Parse the generated SysML
        let parse_result = parse_content(&result.text, Path::new("generated.sysml"));
        assert!(
            parse_result.is_ok(),
            "Parse failed: {:?}",
            parse_result.err()
        );

        // Step 4: Verify metadata
        assert!(result.metadata.get_element("VehicleModel").is_some());
        assert!(
            result
                .metadata
                .get_element("VehicleModel::Vehicle")
                .is_some()
        );
        assert!(result.metadata.get_element("VehicleModel::Car").is_some());

        // Verify element IDs are preserved
        let pkg_meta = result.metadata.get_element("VehicleModel").unwrap();
        assert_eq!(pkg_meta.original_id.as_deref(), Some("pkg1"));
    }

    #[test]
    fn test_metadata_keyed_by_qualified_name() {
        let mut model = Model::new();

        // Create a package with children properly linked
        let mut pkg = Element::new("pkg-1", ElementKind::Package).with_name("TestPackage");
        pkg.owned_elements.push(ElementId::from("def-1"));
        model.add_element(pkg);

        // Add nested definition (owned_elements must include children)
        let mut def = Element::new("def-1", ElementKind::PartDefinition)
            .with_name("OuterDef")
            .with_owner("pkg-1");
        def.owned_elements.push(ElementId::from("usage-1"));
        model.add_element(def);

        // Add usage inside definition
        model.add_element(
            Element::new("usage-1", ElementKind::PartUsage)
                .with_name("innerPart")
                .with_owner("def-1"),
        );

        let result = decompile(&model);

        // Verify metadata is keyed by qualified name
        assert!(
            result.metadata.get_element("TestPackage").is_some(),
            "Should have metadata for TestPackage"
        );
        assert!(
            result
                .metadata
                .get_element("TestPackage::OuterDef")
                .is_some(),
            "Should have metadata for TestPackage::OuterDef, got keys: {:?}",
            result.metadata.elements.keys().collect::<Vec<_>>()
        );
        assert!(
            result
                .metadata
                .get_element("TestPackage::OuterDef::innerPart")
                .is_some(),
            "Should have metadata for TestPackage::OuterDef::innerPart"
        );

        // Verify element IDs are preserved
        assert_eq!(
            result
                .metadata
                .get_element("TestPackage")
                .unwrap()
                .original_id
                .as_deref(),
            Some("pkg-1")
        );
        assert_eq!(
            result
                .metadata
                .get_element("TestPackage::OuterDef")
                .unwrap()
                .original_id
                .as_deref(),
            Some("def-1")
        );
        assert_eq!(
            result
                .metadata
                .get_element("TestPackage::OuterDef::innerPart")
                .unwrap()
                .original_id
                .as_deref(),
            Some("usage-1")
        );
    }
}
