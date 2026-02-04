//! YAML format support.
//!
//! YAML provides a human-readable format for SysML/KerML model interchange.
//! It uses the same structure as JSON-LD but in YAML syntax.
//!
//! ## YAML Structure
//!
//! ```yaml
//! - type: PartDefinition
//!   id: 550e8400-e29b-41d4-a716-446655440000
//!   name: Vehicle
//!   ownedMember:
//!     - id: 550e8400-e29b-41d4-a716-446655440001
//! ```

use super::model::Model;
use super::{FormatCapability, InterchangeError, ModelFormat};

/// YAML format handler.
#[derive(Debug, Clone, Copy, Default)]
pub struct Yaml;

impl ModelFormat for Yaml {
    fn name(&self) -> &'static str {
        "YAML"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn mime_type(&self) -> &'static str {
        "application/x-yaml"
    }

    fn capabilities(&self) -> FormatCapability {
        FormatCapability {
            read: true,
            write: true,
            streaming: false,
            lossless: true,
        }
    }

    fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            YamlReader::new().read(input)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = input;
            Err(InterchangeError::Unsupported(
                "YAML reading requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
        #[cfg(feature = "interchange")]
        {
            YamlWriter::new().write(model)
        }
        #[cfg(not(feature = "interchange"))]
        {
            let _ = model;
            Err(InterchangeError::Unsupported(
                "YAML writing requires the 'interchange' feature".to_string(),
            ))
        }
    }

    fn validate(&self, input: &[u8]) -> Result<(), InterchangeError> {
        let content = std::str::from_utf8(input)
            .map_err(|e| InterchangeError::yaml(format!("Invalid UTF-8: {e}")))?;

        // Quick check that it looks like YAML
        let trimmed = content.trim();
        // YAML can start with ---, a list item, or a key
        if trimmed.is_empty() {
            return Err(InterchangeError::yaml("Empty YAML content"));
        }

        Ok(())
    }
}

// ============================================================================
// YAML READER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod reader {
    use super::*;
    use crate::interchange::model::{Element, ElementId, ElementKind, PropertyValue, Relationship, RelationshipKind};
    use serde_yaml::Value;
    use std::sync::Arc;

    /// YAML reader.
    pub struct YamlReader;

    impl YamlReader {
        pub fn new() -> Self {
            Self
        }

        pub fn read(&self, input: &[u8]) -> Result<Model, InterchangeError> {
            let value: Value = serde_yaml::from_slice(input)
                .map_err(|e| InterchangeError::yaml(format!("YAML parse error: {e}")))?;

            let mut model = Model::new();

            match value {
                Value::Mapping(map) => {
                    // Single element
                    let (element, relationships) = parse_element(&map)?;
                    if let Some(el) = element {
                        model.add_element(el);
                    }
                    model.relationships.extend(relationships);
                }
                Value::Sequence(seq) => {
                    // Array of elements
                    for item in seq {
                        if let Value::Mapping(map) = item {
                            let (element, relationships) = parse_element(&map)?;
                            if let Some(el) = element {
                                model.add_element(el);
                            }
                            model.relationships.extend(relationships);
                        }
                    }
                }
                _ => {
                    return Err(InterchangeError::yaml("Expected mapping or sequence"));
                }
            }

            // Build ownership relationships
            build_ownership(&mut model);

            Ok(model)
        }
    }

    /// Parse a YAML mapping into an Element and its relationships.
    fn parse_element(
        map: &serde_yaml::Mapping,
    ) -> Result<(Option<Element>, Vec<Relationship>), InterchangeError> {
        // Get type (required)
        let kind = if let Some(type_val) = map.get("type").or_else(|| map.get("@type")) {
            let type_str = type_val
                .as_str()
                .ok_or_else(|| InterchangeError::yaml("'type' must be a string"))?;
            ElementKind::from_xmi_type(type_str)
        } else {
            return Ok((None, Vec::new())); // Skip elements without type
        };

        // Get id (required)
        let id = if let Some(id_val) = map.get("id").or_else(|| map.get("@id")) {
            let id_str = id_val
                .as_str()
                .ok_or_else(|| InterchangeError::yaml("'id' must be a string"))?;
            ElementId::new(id_str)
        } else {
            ElementId::generate()
        };

        let mut element = Element::new(id.clone(), kind);
        let mut relationships = Vec::new();

        // Parse name
        if let Some(name_val) = map.get("name") {
            if let Some(name) = name_val.as_str() {
                element.name = Some(Arc::from(name));
            }
        }

        // Parse shortName
        if let Some(short_name_val) = map.get("shortName") {
            if let Some(short_name) = short_name_val.as_str() {
                element.short_name = Some(Arc::from(short_name));
            }
        }

        // Parse qualifiedName
        if let Some(qn_val) = map.get("qualifiedName") {
            if let Some(qn) = qn_val.as_str() {
                element.qualified_name = Some(Arc::from(qn));
            }
        }

        // Parse documentation
        if let Some(doc_val) = map.get("documentation") {
            if let Some(doc) = doc_val.as_str() {
                element.documentation = Some(Arc::from(doc));
            }
        }

        // Parse boolean flags
        if let Some(val) = map.get("isAbstract") {
            element.is_abstract = val.as_bool().unwrap_or(false);
        }
        if let Some(val) = map.get("isVariation") {
            element.is_variation = val.as_bool().unwrap_or(false);
        }
        if let Some(val) = map.get("isDerived") {
            element.is_derived = val.as_bool().unwrap_or(false);
        }
        if let Some(val) = map.get("isReadOnly") {
            element.is_readonly = val.as_bool().unwrap_or(false);
        }
        if let Some(val) = map.get("isParallel") {
            element.is_parallel = val.as_bool().unwrap_or(false);
        }

        // Parse owner reference
        if let Some(owner_val) = map.get("owner") {
            if let Some(owner_map) = owner_val.as_mapping() {
                if let Some(id_val) = owner_map.get("id").or_else(|| owner_map.get("@id")) {
                    if let Some(id_str) = id_val.as_str() {
                        element.owner = Some(ElementId::new(id_str));
                    }
                }
            } else if let Some(id_str) = owner_val.as_str() {
                element.owner = Some(ElementId::new(id_str));
            }
        }

        // Parse ownedMember references
        if let Some(members_val) = map.get("ownedMember") {
            if let Some(members_seq) = members_val.as_sequence() {
                for member in members_seq {
                    if let Some(member_map) = member.as_mapping() {
                        if let Some(id_val) = member_map.get("id").or_else(|| member_map.get("@id"))
                        {
                            if let Some(id_str) = id_val.as_str() {
                                element.owned_elements.push(ElementId::new(id_str));
                            }
                        }
                    } else if let Some(id_str) = member.as_str() {
                        element.owned_elements.push(ElementId::new(id_str));
                    }
                }
            }
        }

        // Parse relationships (specializes, typedBy, subsets, redefines, and more)
        parse_relationship_list(map, "specializes", RelationshipKind::Specialization, &id, &mut relationships);
        parse_relationship_list(map, "typedBy", RelationshipKind::FeatureTyping, &id, &mut relationships);
        parse_relationship_list(map, "subsets", RelationshipKind::Subsetting, &id, &mut relationships);
        parse_relationship_list(map, "redefines", RelationshipKind::Redefinition, &id, &mut relationships);
        parse_relationship_list(map, "conjugates", RelationshipKind::Conjugation, &id, &mut relationships);
        parse_relationship_list(map, "featureChaining", RelationshipKind::FeatureChaining, &id, &mut relationships);
        parse_relationship_list(map, "successionTo", RelationshipKind::Succession, &id, &mut relationships);
        parse_relationship_list(map, "connectsTo", RelationshipKind::Connection, &id, &mut relationships);
        parse_relationship_list(map, "flowsTo", RelationshipKind::FlowConnection, &id, &mut relationships);
        parse_relationship_list(map, "allocatedTo", RelationshipKind::Allocation, &id, &mut relationships);
        parse_relationship_list(map, "satisfies", RelationshipKind::Satisfaction, &id, &mut relationships);
        parse_relationship_list(map, "verifies", RelationshipKind::Verification, &id, &mut relationships);
        parse_relationship_list(map, "dependsOn", RelationshipKind::Dependency, &id, &mut relationships);
        // Note: "imports" handles both NamespaceImport and MembershipImport - we default to NamespaceImport
        parse_relationship_list(map, "imports", RelationshipKind::NamespaceImport, &id, &mut relationships);

        // Parse additional properties
        let reserved_keys = [
            "type",
            "@type",
            "id",
            "@id",
            "name",
            "shortName",
            "qualifiedName",
            "documentation",
            "isAbstract",
            "isVariation",
            "isDerived",
            "isReadOnly",
            "isParallel",
            "owner",
            "ownedMember",
            "specializes",
            "typedBy",
            "subsets",
            "redefines",
            "conjugates",
            "featureChaining",
            "successionTo",
            "connectsTo",
            "flowsTo",
            "allocatedTo",
            "satisfies",
            "verifies",
            "dependsOn",
            "imports",
        ];

        for (key, value) in map {
            if let Some(key_str) = key.as_str() {
                if !reserved_keys.contains(&key_str) {
                    if let Some(prop_value) = parse_property_value(value) {
                        element.properties.insert(Arc::from(key_str), prop_value);
                    }
                }
            }
        }

        Ok((Some(element), relationships))
    }
    
    /// Parse a relationship list field (e.g., specializes, typedBy).
    fn parse_relationship_list(
        map: &serde_yaml::Mapping,
        key: &str,
        kind: RelationshipKind,
        source_id: &ElementId,
        relationships: &mut Vec<Relationship>,
    ) {
        if let Some(val) = map.get(key) {
            if let Some(seq) = val.as_sequence() {
                for item in seq {
                    // Can be string (ID or qualified name) or mapping with id field
                    let target_id = if let Some(s) = item.as_str() {
                        s.to_string()
                    } else if let Some(m) = item.as_mapping() {
                        if let Some(id_val) = m.get("id").or_else(|| m.get("@id")) {
                            if let Some(id_str) = id_val.as_str() {
                                id_str.to_string()
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };
                    
                    // Generate a relationship ID
                    let rel_id = format!("{}-{}-{}", source_id.as_str(), kind.xmi_type().split(':').last().unwrap_or("rel"), target_id);
                    let mut rel = Relationship::new(rel_id, kind, source_id.clone(), target_id);
                    rel.owner = Some(source_id.clone());
                    relationships.push(rel);
                }
            } else if let Some(s) = val.as_str() {
                // Single value, not array
                let rel_id = format!("{}-{}-{}", source_id.as_str(), kind.xmi_type().split(':').last().unwrap_or("rel"), s);
                let mut rel = Relationship::new(rel_id, kind, source_id.clone(), s);
                rel.owner = Some(source_id.clone());
                relationships.push(rel);
            }
        }
    }

    /// Parse a YAML value into a PropertyValue.
    fn parse_property_value(value: &Value) -> Option<PropertyValue> {
        match value {
            Value::String(s) => Some(PropertyValue::String(Arc::from(s.as_str()))),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(PropertyValue::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Some(PropertyValue::Real(f))
                } else {
                    None
                }
            }
            Value::Bool(b) => Some(PropertyValue::Boolean(*b)),
            Value::Mapping(map) => {
                // Check if it's a reference
                if let Some(id_val) = map.get("id").or_else(|| map.get("@id")) {
                    if let Some(id_str) = id_val.as_str() {
                        return Some(PropertyValue::Reference(ElementId::new(id_str)));
                    }
                }
                None
            }
            Value::Sequence(seq) => {
                let items: Vec<PropertyValue> =
                    seq.iter().filter_map(parse_property_value).collect();
                if items.is_empty() {
                    None
                } else {
                    Some(PropertyValue::List(items))
                }
            }
            _ => None,
        }
    }

    /// Build ownership relationships from ownedMember references.
    fn build_ownership(model: &mut Model) {
        // Collect updates first to avoid borrow issues
        let mut updates = Vec::new();

        for element in model.iter_elements() {
            let owner_id = element.id.clone();
            for owned_id in &element.owned_elements {
                updates.push((owner_id.clone(), owned_id.clone()));
            }
        }

        // Apply updates
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
use reader::YamlReader;

// ============================================================================
// YAML WRITER (requires interchange feature)
// ============================================================================

#[cfg(feature = "interchange")]
mod writer {
    use super::*;
    use crate::interchange::model::{Element, PropertyValue, Relationship};
    use serde_yaml::{Mapping, Value};

    /// YAML writer.
    pub struct YamlWriter;

    impl YamlWriter {
        pub fn new() -> Self {
            Self
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let elements: Vec<Value> = model.iter_elements().map(|e| element_to_yaml(e, model)).collect();

            let output = if elements.len() == 1 && model.relationships.is_empty() {
                // Single element with no relationships - return mapping directly
                elements.into_iter().next().unwrap()
            } else {
                // Multiple elements or has relationships - return sequence
                Value::Sequence(elements)
            };

            serde_yaml::to_string(&output)
                .map(|s| s.into_bytes())
                .map_err(|e| InterchangeError::yaml(format!("YAML serialization error: {e}")))
        }
    }

    /// Convert an Element to YAML Value, including its outgoing relationships.
    fn element_to_yaml(element: &Element, model: &Model) -> Value {
        let mut map = Mapping::new();

        // type
        map.insert(
            Value::String("type".to_string()),
            Value::String(element.kind.jsonld_type().to_string()),
        );

        // id
        map.insert(
            Value::String("id".to_string()),
            Value::String(element.id.as_str().to_string()),
        );

        // name
        if let Some(ref name) = element.name {
            map.insert(
                Value::String("name".to_string()),
                Value::String(name.to_string()),
            );
        }

        // shortName
        if let Some(ref short_name) = element.short_name {
            map.insert(
                Value::String("shortName".to_string()),
                Value::String(short_name.to_string()),
            );
        }

        // qualifiedName
        if let Some(ref qn) = element.qualified_name {
            map.insert(
                Value::String("qualifiedName".to_string()),
                Value::String(qn.to_string()),
            );
        }

        // isAbstract (only if true)
        if element.is_abstract {
            map.insert(
                Value::String("isAbstract".to_string()),
                Value::Bool(true),
            );
        }

        // isVariation (only if true)
        if element.is_variation {
            map.insert(
                Value::String("isVariation".to_string()),
                Value::Bool(true),
            );
        }

        // isDerived (only if true)
        if element.is_derived {
            map.insert(
                Value::String("isDerived".to_string()),
                Value::Bool(true),
            );
        }

        // isReadOnly (only if true)
        if element.is_readonly {
            map.insert(
                Value::String("isReadOnly".to_string()),
                Value::Bool(true),
            );
        }

        // isParallel (only if true)
        if element.is_parallel {
            map.insert(
                Value::String("isParallel".to_string()),
                Value::Bool(true),
            );
        }

        // documentation
        if let Some(ref doc) = element.documentation {
            map.insert(
                Value::String("documentation".to_string()),
                Value::String(doc.to_string()),
            );
        }

        // Additional properties
        for (key, value) in &element.properties {
            let yaml_value = property_value_to_yaml(value);
            map.insert(Value::String(key.to_string()), yaml_value);
        }

        // owner (as reference)
        if let Some(ref owner_id) = element.owner {
            let mut owner_map = Mapping::new();
            owner_map.insert(
                Value::String("id".to_string()),
                Value::String(owner_id.as_str().to_string()),
            );
            map.insert(
                Value::String("owner".to_string()),
                Value::Mapping(owner_map),
            );
        }

        // ownedMember (as array of references)
        if !element.owned_elements.is_empty() {
            let members: Vec<Value> = element
                .owned_elements
                .iter()
                .map(|id| {
                    let mut m = Mapping::new();
                    m.insert(
                        Value::String("id".to_string()),
                        Value::String(id.as_str().to_string()),
                    );
                    Value::Mapping(m)
                })
                .collect();
            map.insert(
                Value::String("ownedMember".to_string()),
                Value::Sequence(members),
            );
        }

        // Relationships from this element (specialization, typing, etc.)
        let relationships: Vec<&Relationship> = model.relationships_from(&element.id).collect();
        if !relationships.is_empty() {
            use crate::interchange::model::RelationshipKind;
            
            // Helper to add relationship array if not empty
            fn add_rel_array(
                map: &mut Mapping, 
                key: &str, 
                rels: Vec<&&Relationship>, 
                model: &Model
            ) {
                if !rels.is_empty() {
                    let refs: Vec<Value> = rels.iter()
                        .map(|r| relationship_target_to_yaml(r, model))
                        .collect();
                    map.insert(
                        Value::String(key.to_string()),
                        Value::Sequence(refs),
                    );
                }
            }
            
            // Group relationships by kind for cleaner output
            add_rel_array(&mut map, "specializes", 
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Specialization)).collect(), model);
            add_rel_array(&mut map, "typedBy",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::FeatureTyping)).collect(), model);
            add_rel_array(&mut map, "subsets",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Subsetting)).collect(), model);
            add_rel_array(&mut map, "redefines",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Redefinition)).collect(), model);
            add_rel_array(&mut map, "conjugates",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Conjugation)).collect(), model);
            add_rel_array(&mut map, "featureChaining",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::FeatureChaining)).collect(), model);
            add_rel_array(&mut map, "successionTo",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Succession)).collect(), model);
            add_rel_array(&mut map, "connectsTo",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Connection)).collect(), model);
            add_rel_array(&mut map, "flowsTo",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::FlowConnection)).collect(), model);
            add_rel_array(&mut map, "allocatedTo",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Allocation)).collect(), model);
            add_rel_array(&mut map, "satisfies",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Satisfaction)).collect(), model);
            add_rel_array(&mut map, "verifies",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Verification)).collect(), model);
            add_rel_array(&mut map, "dependsOn",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::Dependency)).collect(), model);
            add_rel_array(&mut map, "imports",
                relationships.iter().filter(|r| matches!(r.kind, RelationshipKind::NamespaceImport | RelationshipKind::MembershipImport)).collect(), model);
        }

        Value::Mapping(map)
    }

    /// Convert a relationship target to YAML Value.
    /// We always use the target ID to ensure lossless roundtrip.
    fn relationship_target_to_yaml(rel: &Relationship, _model: &Model) -> Value {
        // Always use ID for lossless roundtrip
        Value::String(rel.target.as_str().to_string())
    }

    /// Convert a PropertyValue to YAML Value.
    fn property_value_to_yaml(value: &PropertyValue) -> Value {
        match value {
            PropertyValue::String(s) => Value::String(s.to_string()),
            PropertyValue::Integer(i) => Value::Number((*i).into()),
            PropertyValue::Real(f) => {
                Value::Number(serde_yaml::Number::from(*f))
            }
            PropertyValue::Boolean(b) => Value::Bool(*b),
            PropertyValue::Reference(id) => {
                let mut m = Mapping::new();
                m.insert(
                    Value::String("id".to_string()),
                    Value::String(id.as_str().to_string()),
                );
                Value::Mapping(m)
            }
            PropertyValue::List(items) => {
                Value::Sequence(items.iter().map(property_value_to_yaml).collect())
            }
        }
    }
}

#[cfg(feature = "interchange")]
use writer::YamlWriter;

// Stub implementations when feature is disabled
#[cfg(not(feature = "interchange"))]
struct YamlReader;

#[cfg(not(feature = "interchange"))]
impl YamlReader {
    fn new() -> Self {
        Self
    }

    fn read(&self, _input: &[u8]) -> Result<Model, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "YAML reading requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(not(feature = "interchange"))]
struct YamlWriter;

#[cfg(not(feature = "interchange"))]
impl YamlWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, _model: &Model) -> Result<Vec<u8>, InterchangeError> {
        Err(InterchangeError::Unsupported(
            "YAML writing requires the 'interchange' feature".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_format_metadata() {
        let yaml = Yaml;
        assert_eq!(yaml.name(), "YAML");
        assert_eq!(yaml.extensions(), &["yaml", "yml"]);
        assert_eq!(yaml.mime_type(), "application/x-yaml");
        assert!(yaml.capabilities().read);
        assert!(yaml.capabilities().write);
    }

    #[test]
    fn test_yaml_validate_empty() {
        let yaml = Yaml;
        let input = b"";
        assert!(yaml.validate(input).is_err());
    }

    #[test]
    fn test_yaml_validate_valid() {
        let yaml = Yaml;
        let input = b"type: Package\nname: Test";
        assert!(yaml.validate(input).is_ok());
    }

    #[cfg(feature = "interchange")]
    mod interchange_tests {
        use super::*;
        use crate::interchange::model::{Element, ElementId, ElementKind};

        #[test]
        fn test_yaml_roundtrip_single_element() {
            let yaml = Yaml;

            let mut model = Model::new();
            let element = Element::new(ElementId::new("test-id-123"), ElementKind::Package)
                .with_name("TestPackage");
            model.add_element(element);

            // Write
            let bytes = yaml.write(&model).expect("write should succeed");
            let content = String::from_utf8(bytes.clone()).expect("should be valid UTF-8");
            assert!(content.contains("Package"));
            assert!(content.contains("TestPackage"));
            assert!(content.contains("test-id-123"));

            // Read back
            let model2 = yaml.read(&bytes).expect("read should succeed");
            assert_eq!(model2.elements.len(), 1);

            let elem = model2.elements.values().next().unwrap();
            assert_eq!(elem.name.as_ref().map(|s| s.as_ref()), Some("TestPackage"));
            assert_eq!(elem.kind, ElementKind::Package);
        }

        #[test]
        fn test_yaml_roundtrip_multiple_elements() {
            let yaml = Yaml;

            let mut model = Model::new();
            model.add_element(
                Element::new(ElementId::new("pkg-1"), ElementKind::Package).with_name("Package1"),
            );
            model.add_element(
                Element::new(ElementId::new("part-1"), ElementKind::PartDefinition)
                    .with_name("Part1"),
            );

            let bytes = yaml.write(&model).expect("write should succeed");
            let model2 = yaml.read(&bytes).expect("read should succeed");

            assert_eq!(model2.elements.len(), 2);
        }
    }
}
