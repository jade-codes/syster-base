//! JSON-LD format support.
//!
//! JSON-LD (JSON Linked Data) is used by the OMG Systems Modeling API
//! for REST API responses. This module provides serialization compatible
//! with the API specification.
//!
//! ## JSON-LD Structure
//!
//! ```json
//! {
//!   "@context": "https://www.omg.org/spec/SysML/20230201/context",
//!   "@type": "PartDefinition",
//!   "@id": "550e8400-e29b-41d4-a716-446655440000",
//!   "name": "Vehicle",
//!   "ownedMember": [
//!     { "@id": "550e8400-e29b-41d4-a716-446655440001" }
//!   ]
//! }
//! ```

use super::model::Model;
use super::{FormatCapability, InterchangeError, ModelFormat};

/// JSON-LD context URIs.
pub mod context {
    /// SysML v2 JSON-LD context.
    pub const SYSML: &str = "https://www.omg.org/spec/SysML/20230201/context";
}

/// JSON-LD format handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonLd;

impl ModelFormat for JsonLd {
    fn name(&self) -> &'static str {
        "JSON-LD"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["jsonld", "json"]
    }

    fn mime_type(&self) -> &'static str {
        "application/ld+json"
    }

    fn capabilities(&self) -> FormatCapability {
        // JSON-LD is primarily for API output, read support is secondary
        FormatCapability {
            read: true,
            write: true,
            streaming: true, // Can stream JSON arrays
            lossless: true,
        }
    }

    fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            JsonLdReader::new().read(input)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = input;
            Err(InterchangeError::Unsupported(
                "JSON-LD reading requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            JsonLdWriter::new().write(model)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = model;
            Err(InterchangeError::Unsupported(
                "JSON-LD writing requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn validate(&self, input: &[u8]) -> Result<(), InterchangeError> {
        let content = std::str::from_utf8(input)
            .map_err(|e| InterchangeError::json(format!("Invalid UTF-8: {e}")))?;

        // Quick check for JSON structure
        let trimmed = content.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return Err(InterchangeError::json("Not valid JSON"));
        }

        Ok(())
    }
}

// ============================================================================
// JSON-LD READER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod reader {
    use super::*;
    use crate::interchange::model::{
        Element, ElementId, ElementKind, PropertyValue, Relationship, RelationshipKind,
    };
    use serde_json::Value;
    use std::sync::Arc;

    /// JSON-LD reader.
    pub struct JsonLdReader;

    impl JsonLdReader {
        pub fn new() -> Self {
            Self
        }

        pub fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
            let value: Value = serde_json::from_slice(input)
                .map_err(|e| InterchangeError::json(format!("Parse error: {e}")))?;

            let mut model = Model::new();

            match value {
                Value::Object(obj) => {
                    // Single element - could be element or relationship
                    if let Some(rel) = parse_relationship(&obj) {
                        model.relationships.push(rel);
                    } else if let Some(element) = parse_element(&obj)? {
                        model.add_element(element);
                    }
                }
                Value::Array(arr) => {
                    // Array of elements/relationships
                    for item in arr {
                        if let Value::Object(obj) = item {
                            // Try parsing as relationship first
                            if let Some(rel) = parse_relationship(&obj) {
                                model.relationships.push(rel);
                            } else if let Some(element) = parse_element(&obj)? {
                                model.add_element(element);
                            }
                        }
                    }
                }
                _ => {
                    return Err(InterchangeError::json("Expected object or array"));
                }
            }

            // Build ownership relationships
            build_ownership(&mut model);

            Ok(model)
        }
    }

    /// Parse a JSON object as a Relationship if it has source/target fields.
    fn parse_relationship(obj: &serde_json::Map<String, Value>) -> Option<Relationship> {
        // Must have @id, @type, source, and target
        let id = match obj.get("@id") {
            Some(Value::String(s)) => s.clone(),
            _ => return None,
        };

        let type_str = match obj.get("@type") {
            Some(Value::String(s)) => s.as_str(),
            _ => return None,
        };

        // Check if this is a relationship type
        let kind = match type_str {
            "Specialization" | "Subclassification" => RelationshipKind::Specialization,
            "FeatureTyping" => RelationshipKind::FeatureTyping,
            "Subsetting" => RelationshipKind::Subsetting,
            "Redefinition" => RelationshipKind::Redefinition,
            "Conjugation" => RelationshipKind::Conjugation,
            "Membership" => RelationshipKind::Membership,
            "OwningMembership" => RelationshipKind::OwningMembership,
            "FeatureMembership" => RelationshipKind::FeatureMembership,
            "NamespaceImport" => RelationshipKind::NamespaceImport,
            "MembershipImport" => RelationshipKind::MembershipImport,
            "Dependency" => RelationshipKind::Dependency,
            "SatisfyRequirementUsage" => RelationshipKind::Satisfaction,
            "RequirementVerificationMembership" => RelationshipKind::Verification,
            "AllocationUsage" => RelationshipKind::Allocation,
            "ConnectionUsage" => RelationshipKind::Connection,
            "FlowConnectionUsage" => RelationshipKind::FlowConnection,
            "Succession" => RelationshipKind::Succession,
            "FeatureChaining" => RelationshipKind::FeatureChaining,
            "Disjoining" => RelationshipKind::Disjoining,
            _ => return None, // Not a relationship type
        };

        // Get source
        let source = match obj.get("source") {
            Some(Value::Object(src_obj)) => match src_obj.get("@id") {
                Some(Value::String(s)) => s.clone(),
                _ => return None,
            },
            _ => return None,
        };

        // Get target
        let target = match obj.get("target") {
            Some(Value::Object(tgt_obj)) => match tgt_obj.get("@id") {
                Some(Value::String(s)) => s.clone(),
                _ => return None,
            },
            _ => return None,
        };

        let mut rel = Relationship::new(id, kind, source, target);

        // Get owner if present
        if let Some(Value::Object(owner_obj)) = obj.get("owner") {
            if let Some(Value::String(owner_id)) = owner_obj.get("@id") {
                rel.owner = Some(ElementId::new(owner_id.clone()));
            }
        }

        Some(rel)
    }

    /// Parse a JSON object into an Element.
    fn parse_element(
        obj: &serde_json::Map<String, Value>,
    ) -> Result<Option<Element>, InterchangeError> {
        // Get @id (required)
        let id = match obj.get("@id") {
            Some(Value::String(s)) => s.clone(),
            _ => return Ok(None), // Skip elements without @id
        };

        // Get @type
        let type_str = match obj.get("@type") {
            Some(Value::String(s)) => s.as_str(),
            _ => "Element",
        };
        let kind = ElementKind::from_xmi_type(type_str);

        let mut element = Element::new(id, kind);

        // Get name (also check declaredName for compatibility)
        if let Some(Value::String(name)) = obj.get("name").or_else(|| obj.get("declaredName")) {
            element.name = Some(Arc::from(name.as_str()));
        }

        // Get shortName (also check declaredShortName)
        if let Some(Value::String(short_name)) = obj
            .get("shortName")
            .or_else(|| obj.get("declaredShortName"))
        {
            element.short_name = Some(Arc::from(short_name.as_str()));
        }

        // Get isAbstract
        if let Some(Value::Bool(is_abstract)) = obj.get("isAbstract") {
            element.set_abstract(*is_abstract);
        }

        // Get isVariation
        if let Some(Value::Bool(is_variation)) = obj.get("isVariation") {
            element.set_variation(*is_variation);
        }

        // Get isDerived
        if let Some(Value::Bool(is_derived)) = obj.get("isDerived") {
            element.set_derived(*is_derived);
        }

        // Get isReadOnly
        if let Some(Value::Bool(is_readonly)) = obj.get("isReadOnly") {
            element.set_readonly(*is_readonly);
        }

        // Get isParallel
        if let Some(Value::Bool(is_parallel)) = obj.get("isParallel") {
            element.set_parallel(*is_parallel);
        }

        // Get documentation (body text)
        if let Some(Value::String(doc)) = obj.get("documentation").or_else(|| obj.get("body")) {
            element.documentation = Some(Arc::from(doc.as_str()));
        }

        // Get owner (as @id reference)
        if let Some(Value::Object(owner_obj)) = obj.get("owner") {
            if let Some(Value::String(owner_id)) = owner_obj.get("@id") {
                element.owner = Some(ElementId::new(owner_id.clone()));
            }
        }

        // Get ownedMember (array of @id references)
        if let Some(Value::Array(members)) = obj.get("ownedMember") {
            for member in members {
                if let Value::Object(member_obj) = member {
                    if let Some(Value::String(member_id)) = member_obj.get("@id") {
                        element
                            .owned_elements
                            .push(ElementId::new(member_id.clone()));
                    }
                }
            }
        }

        // Get additional properties (isStandard, isComposite, etc.)
        for (key, value) in obj {
            // Skip already-handled properties
            if matches!(
                key.as_str(),
                "@id"
                    | "@type"
                    | "@context"
                    | "name"
                    | "declaredName"
                    | "shortName"
                    | "declaredShortName"
                    | "isAbstract"
                    | "isVariation"
                    | "isDerived"
                    | "isReadOnly"
                    | "isParallel"
                    | "documentation"
                    | "body"
                    | "owner"
                    | "ownedMember"
                    | "ownedRelationship"
                    | "ownedRelatedElement"
            ) {
                continue;
            }
            // Store string/bool properties using PropertyValue
            let prop_key: Arc<str> = Arc::from(key.as_str());
            match value {
                Value::String(s) => {
                    element
                        .properties
                        .insert(prop_key, PropertyValue::from(s.as_str()));
                }
                Value::Bool(b) => {
                    element.properties.insert(prop_key, PropertyValue::from(*b));
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        element.properties.insert(prop_key, PropertyValue::from(i));
                    } else if let Some(f) = n.as_f64() {
                        element.properties.insert(prop_key, PropertyValue::from(f));
                    }
                }
                _ => {}
            }
        }

        Ok(Some(element))
    }

    /// Build ownership relationships from ownedMember arrays.
    fn build_ownership(model: &mut Model) {
        // Collect owner updates
        let mut updates: Vec<(ElementId, ElementId)> = Vec::new();

        for element in model.elements.values() {
            for owned_id in &element.owned_elements {
                updates.push((element.id.clone(), owned_id.clone()));
            }
        }

        // Apply owner to owned elements
        for (owner_id, owned_id) in updates {
            if let Some(owned) = model.elements.get_mut(&owned_id) {
                if owned.owner.is_none() {
                    owned.owner = Some(owner_id);
                }
            }
        }
    }
}

#[cfg(feature = "interchange")]
use reader::JsonLdReader;

// ============================================================================
// JSON-LD WRITER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod writer {
    use super::*;
    use crate::interchange::model::{Element, PropertyValue, Relationship, RelationshipKind};
    use serde_json::{Map, Value, json};

    /// JSON-LD writer.
    pub struct JsonLdWriter;

    impl JsonLdWriter {
        pub fn new() -> Self {
            Self
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let mut all_items: Vec<Value> = Vec::new();

            // Add all elements
            for element in model.iter_elements() {
                all_items.push(element_to_json(element));
            }

            // Add all relationships as separate objects
            for relationship in &model.relationships {
                all_items.push(relationship_to_json(relationship));
            }

            let output = if all_items.len() == 1 {
                // Single element - return object directly
                all_items.into_iter().next().unwrap()
            } else {
                // Multiple items - return array
                Value::Array(all_items)
            };

            serde_json::to_vec_pretty(&output)
                .map_err(|e| InterchangeError::json(format!("Serialization error: {e}")))
        }
    }

    /// Convert a Relationship to JSON-LD Value.
    fn relationship_to_json(rel: &Relationship) -> Value {
        let mut obj = Map::new();

        // @type based on relationship kind
        let rel_type = match rel.kind {
            RelationshipKind::Specialization => "Specialization",
            RelationshipKind::FeatureTyping => "FeatureTyping",
            RelationshipKind::Subsetting => "Subsetting",
            RelationshipKind::Redefinition => "Redefinition",
            RelationshipKind::Conjugation => "Conjugation",
            RelationshipKind::Membership => "Membership",
            RelationshipKind::OwningMembership => "OwningMembership",
            RelationshipKind::FeatureMembership => "FeatureMembership",
            RelationshipKind::NamespaceImport => "NamespaceImport",
            RelationshipKind::MembershipImport => "MembershipImport",
            RelationshipKind::Dependency => "Dependency",
            RelationshipKind::Satisfaction => "SatisfyRequirementUsage",
            RelationshipKind::Verification => "RequirementVerificationMembership",
            RelationshipKind::Allocation => "AllocationUsage",
            RelationshipKind::Connection => "ConnectionUsage",
            RelationshipKind::FlowConnection => "FlowConnectionUsage",
            RelationshipKind::Succession => "Succession",
            RelationshipKind::FeatureChaining => "FeatureChaining",
            RelationshipKind::Disjoining => "Disjoining",
        };

        obj.insert("@type".to_string(), json!(rel_type));
        obj.insert("@id".to_string(), json!(rel.id.as_str()));
        obj.insert("source".to_string(), json!({"@id": rel.source.as_str()}));
        obj.insert("target".to_string(), json!({"@id": rel.target.as_str()}));

        if let Some(ref owner_id) = rel.owner {
            obj.insert("owner".to_string(), json!({"@id": owner_id.as_str()}));
        }

        Value::Object(obj)
    }

    /// Convert an Element to JSON-LD Value.
    fn element_to_json(element: &Element) -> Value {
        let mut obj = Map::new();

        // @context (only for root elements)
        if element.owner.is_none() {
            obj.insert("@context".to_string(), json!(context::SYSML));
        }

        // @type
        obj.insert("@type".to_string(), json!(element.kind.jsonld_type()));

        // @id
        obj.insert("@id".to_string(), json!(element.id.as_str()));

        // name
        if let Some(ref name) = element.name {
            obj.insert("name".to_string(), json!(name.as_ref()));
        }

        // shortName
        if let Some(ref short_name) = element.short_name {
            obj.insert("shortName".to_string(), json!(short_name.as_ref()));
        }

        // isAbstract (only if true)
        if element.is_abstract {
            obj.insert("isAbstract".to_string(), json!(true));
        }

        // isVariation (only if true)
        if element.is_variation {
            obj.insert("isVariation".to_string(), json!(true));
        }

        // isDerived (only if true)
        if element.is_derived {
            obj.insert("isDerived".to_string(), json!(true));
        }

        // isReadOnly (only if true)
        if element.is_readonly {
            obj.insert("isReadOnly".to_string(), json!(true));
        }

        // isParallel (only if true)
        if element.is_parallel {
            obj.insert("isParallel".to_string(), json!(true));
        }

        // documentation
        if let Some(ref doc) = element.documentation {
            obj.insert("documentation".to_string(), json!(doc.as_ref()));
        }

        // Additional properties (isStandard, isComposite, etc.)
        for (key, value) in &element.properties {
            let json_value = property_value_to_json(value);
            obj.insert(key.to_string(), json_value);
        }

        // owner (as @id reference)
        if let Some(ref owner_id) = element.owner {
            obj.insert("owner".to_string(), json!({"@id": owner_id.as_str()}));
        }

        // ownedMember (as array of @id references)
        if !element.owned_elements.is_empty() {
            let members: Vec<Value> = element
                .owned_elements
                .iter()
                .map(|id| json!({"@id": id.as_str()}))
                .collect();
            obj.insert("ownedMember".to_string(), Value::Array(members));
        }

        Value::Object(obj)
    }

    /// Convert a PropertyValue to JSON Value.
    fn property_value_to_json(value: &PropertyValue) -> Value {
        use crate::interchange::model::PropertyValue;
        match value {
            PropertyValue::String(s) => json!(s.as_ref()),
            PropertyValue::Integer(i) => json!(*i),
            PropertyValue::Real(f) => json!(*f),
            PropertyValue::Boolean(b) => json!(*b),
            PropertyValue::Reference(id) => json!({"@id": id.as_str()}),
            PropertyValue::List(items) => {
                Value::Array(items.iter().map(property_value_to_json).collect())
            }
        }
    }
}

#[cfg(feature = "interchange")]
use writer::JsonLdWriter;

// Stub implementations when feature is disabled
#[cfg(not(feature = "interchange"))]
struct JsonLdReader;

#[cfg(not(feature = "interchange"))]
impl JsonLdReader {
    fn new() -> Self {
        Self
    }

    fn read(&self, _input: &[u8]) -> Result<Model, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "JSON-LD reading requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "interchange"))]
struct JsonLdWriter;

#[cfg(not(feature = "interchange"))]
impl JsonLdWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, _model: &Model) -> Result<Vec<u8>, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "JSON-LD writing requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonld_format_metadata() {
        let jsonld = JsonLd;
        assert_eq!(jsonld.name(), "JSON-LD");
        assert_eq!(jsonld.extensions(), &["jsonld", "json"]);
        assert_eq!(jsonld.mime_type(), "application/ld+json");
        assert!(jsonld.capabilities().read);
        assert!(jsonld.capabilities().write);
        assert!(jsonld.capabilities().streaming);
    }

    #[test]
    fn test_jsonld_validate_object() {
        let jsonld = JsonLd;
        let input = br#"{"@context": "...", "@type": "Package"}"#;
        assert!(jsonld.validate(input).is_ok());
    }

    #[test]
    fn test_jsonld_validate_array() {
        let jsonld = JsonLd;
        let input = br#"[{"@id": "1"}, {"@id": "2"}]"#;
        assert!(jsonld.validate(input).is_ok());
    }

    #[test]
    fn test_jsonld_validate_invalid() {
        let jsonld = JsonLd;
        let input = b"not json";
        assert!(jsonld.validate(input).is_err());
    }

    #[cfg(feature = "interchange")]
    mod interchange_tests {
        use super::*;
        use crate::interchange::model::{Element, ElementId, ElementKind, PropertyValue};
        use std::sync::Arc;

        #[test]
        fn test_jsonld_write_single_element() {
            let mut model = Model::new();
            model.add_element(Element::new("pkg1", ElementKind::Package).with_name("TestPackage"));

            let json_bytes = JsonLd.write(&model).expect("Write failed");
            let json_str = String::from_utf8(json_bytes).expect("Invalid UTF-8");

            assert!(json_str.contains("@context"));
            assert!(json_str.contains("@type"));
            assert!(json_str.contains("Package"));
            assert!(json_str.contains("pkg1"));
            assert!(json_str.contains("TestPackage"));
        }

        #[test]
        fn test_jsonld_write_multiple_elements() {
            let mut model = Model::new();
            model.add_element(Element::new("pkg1", ElementKind::Package).with_name("Package1"));
            model.add_element(Element::new("pkg2", ElementKind::Package).with_name("Package2"));

            let json_bytes = JsonLd.write(&model).expect("Write failed");
            let json_str = String::from_utf8(json_bytes).expect("Invalid UTF-8");

            // Multiple elements should be an array
            assert!(json_str.trim().starts_with('['));
            assert!(json_str.contains("Package1"));
            assert!(json_str.contains("Package2"));
        }

        #[test]
        fn test_jsonld_read_single_element() {
            let json = br#"{
                "@type": "Package",
                "@id": "pkg1",
                "name": "TestPackage"
            }"#;

            let model = JsonLd.read(json).expect("Read failed");
            assert_eq!(model.element_count(), 1);

            let pkg = model
                .get(&ElementId::new("pkg1"))
                .expect("Package not found");
            assert_eq!(pkg.name.as_deref(), Some("TestPackage"));
            assert_eq!(pkg.kind, ElementKind::Package);
        }

        #[test]
        fn test_jsonld_read_array() {
            let json = br#"[
                {"@type": "Package", "@id": "pkg1", "name": "First"},
                {"@type": "Package", "@id": "pkg2", "name": "Second"}
            ]"#;

            let model = JsonLd.read(json).expect("Read failed");
            assert_eq!(model.element_count(), 2);
        }

        #[test]
        fn test_jsonld_read_with_attributes() {
            let json = br#"{
                "@type": "Class",
                "@id": "cls1",
                "name": "AbstractClass",
                "shortName": "AC",
                "isAbstract": true,
                "documentation": "This is a doc comment",
                "isStandard": true,
                "customNumber": 42,
                "customString": "hello"
            }"#;

            let model = JsonLd.read(json).expect("Read failed");
            let cls = model.get(&ElementId::new("cls1")).expect("Class not found");

            assert_eq!(cls.name.as_deref(), Some("AbstractClass"));
            assert_eq!(cls.short_name.as_deref(), Some("AC"));
            assert!(cls.is_abstract);
            assert_eq!(cls.documentation.as_deref(), Some("This is a doc comment"));
            assert_eq!(
                cls.properties.get(&Arc::from("isStandard")),
                Some(&PropertyValue::Boolean(true))
            );
            assert_eq!(
                cls.properties.get(&Arc::from("customNumber")),
                Some(&PropertyValue::Integer(42))
            );
            assert_eq!(
                cls.properties.get(&Arc::from("customString")),
                Some(&PropertyValue::String(Arc::from("hello")))
            );
        }

        #[test]
        fn test_jsonld_write_with_attributes() {
            let mut model = Model::new();
            let mut cls = Element::new("cls1", ElementKind::Class);
            cls.name = Some(Arc::from("AbstractClass"));
            cls.short_name = Some(Arc::from("AC"));
            cls.set_abstract(true);
            cls.documentation = Some(Arc::from("This is documented"));
            cls.properties
                .insert(Arc::from("isStandard"), PropertyValue::Boolean(true));
            cls.properties
                .insert(Arc::from("count"), PropertyValue::Integer(99));
            model.add_element(cls);

            let json_bytes = JsonLd.write(&model).expect("Write failed");
            let json_str = String::from_utf8(json_bytes).expect("Invalid UTF-8");

            assert!(json_str.contains("\"isAbstract\": true"));
            assert!(json_str.contains("\"documentation\": \"This is documented\""));
            assert!(json_str.contains("\"isStandard\": true"));
            assert!(json_str.contains("\"count\": 99"));
            assert!(json_str.contains("\"shortName\": \"AC\""));
        }

        #[test]
        fn test_jsonld_roundtrip() {
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

            // Write to JSON-LD
            let json_bytes = JsonLd.write(&model).expect("Write failed");

            // Read back
            let model2 = JsonLd.read(&json_bytes).expect("Read failed");

            // Verify
            assert_eq!(model2.element_count(), 2);
            let pkg2 = model2.get(&ElementId::new("pkg1")).unwrap();
            assert_eq!(pkg2.name.as_deref(), Some("RoundtripTest"));
        }

        #[test]
        fn test_jsonld_roundtrip_with_all_attributes() {
            let mut model = Model::new();

            // Create element with all attributes
            let mut cls = Element::new("cls1", ElementKind::Class);
            cls.name = Some(Arc::from("TestClass"));
            cls.short_name = Some(Arc::from("TC"));
            cls.set_abstract(true);
            cls.documentation = Some(Arc::from("A documented class"));
            cls.properties
                .insert(Arc::from("isStandard"), PropertyValue::Boolean(true));
            cls.properties
                .insert(Arc::from("priority"), PropertyValue::Integer(5));
            cls.properties
                .insert(Arc::from("ratio"), PropertyValue::Real(2.5));
            cls.properties
                .insert(Arc::from("label"), PropertyValue::String(Arc::from("test")));
            model.add_element(cls);

            // Roundtrip
            let json_bytes = JsonLd.write(&model).expect("Write failed");
            let model2 = JsonLd.read(&json_bytes).expect("Read failed");

            // Verify all attributes preserved
            let cls2 = model2
                .get(&ElementId::new("cls1"))
                .expect("Class not found");
            assert_eq!(cls2.name.as_deref(), Some("TestClass"));
            assert_eq!(cls2.short_name.as_deref(), Some("TC"));
            assert!(cls2.is_abstract, "isAbstract not preserved");
            assert_eq!(cls2.documentation.as_deref(), Some("A documented class"));
            assert_eq!(
                cls2.properties.get(&Arc::from("isStandard")),
                Some(&PropertyValue::Boolean(true)),
                "isStandard property not preserved"
            );
            assert_eq!(
                cls2.properties.get(&Arc::from("priority")),
                Some(&PropertyValue::Integer(5)),
                "priority property not preserved"
            );
            assert_eq!(
                cls2.properties.get(&Arc::from("ratio")),
                Some(&PropertyValue::Real(2.5)),
                "ratio property not preserved"
            );
            assert_eq!(
                cls2.properties.get(&Arc::from("label")),
                Some(&PropertyValue::String(Arc::from("test"))),
                "label property not preserved"
            );
        }

        #[test]
        fn test_jsonld_read_is_variation() {
            let json = r#"{
                "@context": "https://www.omg.org/spec/SysML/20230201/sysml-context.jsonld",
                "@type": "PartDefinition",
                "@id": "pd1",
                "name": "VariantPart",
                "isVariation": true
            }"#;

            let model = JsonLd
                .read(json.as_bytes())
                .expect("Failed to read JSON-LD");
            let elem = model
                .get(&ElementId::new("pd1"))
                .expect("Element not found");
            assert!(elem.is_variation, "isVariation should be true");
        }

        #[test]
        fn test_jsonld_read_is_derived() {
            let json = r#"{
                "@context": "https://www.omg.org/spec/SysML/20230201/sysml-context.jsonld",
                "@type": "Feature",
                "@id": "f1",
                "name": "derivedFeature",
                "isDerived": true
            }"#;

            let model = JsonLd
                .read(json.as_bytes())
                .expect("Failed to read JSON-LD");
            let elem = model.get(&ElementId::new("f1")).expect("Element not found");
            assert!(elem.is_derived, "isDerived should be true");
        }

        #[test]
        fn test_jsonld_read_is_readonly() {
            let json = r#"{
                "@context": "https://www.omg.org/spec/SysML/20230201/sysml-context.jsonld",
                "@type": "AttributeUsage",
                "@id": "a1",
                "name": "constantValue",
                "isReadOnly": true
            }"#;

            let model = JsonLd
                .read(json.as_bytes())
                .expect("Failed to read JSON-LD");
            let elem = model.get(&ElementId::new("a1")).expect("Element not found");
            assert!(elem.is_readonly, "isReadOnly should be true");
        }

        #[test]
        fn test_jsonld_read_is_parallel() {
            let json = r#"{
                "@context": "https://www.omg.org/spec/SysML/20230201/sysml-context.jsonld",
                "@type": "StateUsage",
                "@id": "s1",
                "name": "parallelState",
                "isParallel": true
            }"#;

            let model = JsonLd
                .read(json.as_bytes())
                .expect("Failed to read JSON-LD");
            let elem = model.get(&ElementId::new("s1")).expect("Element not found");
            assert!(elem.is_parallel, "isParallel should be true");
        }

        #[test]
        fn test_jsonld_write_modifiers() {
            let mut model = Model::new();

            let mut elem = Element::new("pd1", ElementKind::PartDefinition);
            elem.name = Some("TestPart".into());
            elem.set_abstract(true);
            elem.set_variation(true);
            model.add_element(elem);

            let mut feat = Element::new("f1", ElementKind::Feature);
            feat.name = Some("TestFeature".into());
            feat.set_derived(true);
            feat.set_readonly(true);
            model.add_element(feat);

            let mut state = Element::new("s1", ElementKind::StateUsage);
            state.name = Some("TestState".into());
            state.set_parallel(true);
            model.add_element(state);

            let output = JsonLd.write(&model).expect("Failed to write JSON-LD");
            let output_str = String::from_utf8(output).expect("Invalid UTF-8");

            assert!(
                output_str.contains(r#""isAbstract": true"#),
                "Should contain isAbstract"
            );
            assert!(
                output_str.contains(r#""isVariation": true"#),
                "Should contain isVariation"
            );
            assert!(
                output_str.contains(r#""isDerived": true"#),
                "Should contain isDerived"
            );
            assert!(
                output_str.contains(r#""isReadOnly": true"#),
                "Should contain isReadOnly"
            );
            assert!(
                output_str.contains(r#""isParallel": true"#),
                "Should contain isParallel"
            );
        }

        #[test]
        fn test_jsonld_roundtrip_modifiers() {
            let mut model = Model::new();

            let mut elem = Element::new("pd1", ElementKind::PartDefinition);
            elem.name = Some("AbstractVariation".into());
            elem.set_abstract(true);
            elem.set_variation(true);
            model.add_element(elem);

            let mut feat = Element::new("f1", ElementKind::AttributeUsage);
            feat.name = Some("DerivedReadonly".into());
            feat.set_derived(true);
            feat.set_readonly(true);
            model.add_element(feat);

            let mut state = Element::new("s1", ElementKind::StateUsage);
            state.name = Some("ParallelState".into());
            state.set_parallel(true);
            model.add_element(state);

            // Write and read back
            let json_bytes = JsonLd.write(&model).expect("Write failed");
            let model2 = JsonLd.read(&json_bytes).expect("Read failed");

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
        fn test_jsonld_modifiers_default_false() {
            let json = r#"{
                "@context": "https://www.omg.org/spec/SysML/20230201/sysml-context.jsonld",
                "@type": "PartDefinition",
                "@id": "pd1",
                "name": "NormalPart"
            }"#;

            let model = JsonLd
                .read(json.as_bytes())
                .expect("Failed to read JSON-LD");
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
