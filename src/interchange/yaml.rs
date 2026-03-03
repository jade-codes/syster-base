//! YAML format support.
//!
//! YAML provides a human-readable format for SysML/KerML model interchange.
//! It uses the same structure as JSON-LD but in YAML syntax, ensuring lossless roundtrip.
//!
//! ## YAML Structure
//!
//! ```yaml
//! - "@type": Package
//!   "@id": 550e8400-e29b-41d4-a716-446655440000
//!   name: Vehicle
//! - "@type": Specialization
//!   "@id": rel-1
//!   source:
//!     "@id": elem-1
//!   target:
//!     "@id": elem-2
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

        let trimmed = content.trim();
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
    use crate::interchange::model::{Element, ElementId, ElementKind, PropertyValue};
    use serde_yaml::Value;
    use std::sync::Arc;

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
                    // Single item - element or relationship
                    if let Some((id, kind, source, target, owner)) = parse_relationship(&map) {
                        let rel_id = model.add_rel(id, kind, source, target, owner);
                        read_relationship_properties(&map, &rel_id, &mut model);
                    } else if let Some(element) = parse_element(&map)? {
                        model.add_element(element);
                    }
                }
                Value::Sequence(seq) => {
                    // Array of items
                    for item in seq {
                        if let Value::Mapping(map) = item {
                            // Try relationship first
                            if let Some((id, kind, source, target, owner)) =
                                parse_relationship(&map)
                            {
                                let rel_id = model.add_rel(id, kind, source, target, owner);
                                // Carry over any extra properties on the relationship
                                read_relationship_properties(&map, &rel_id, &mut model);
                            } else if let Some(element) = parse_element(&map)? {
                                model.add_element(element);
                            }
                        }
                    }
                }
                _ => {
                    return Err(InterchangeError::yaml("Expected mapping or sequence"));
                }
            }

            // Build ownership
            build_ownership(&mut model);

            Ok(model)
        }
    }

    /// Parse a YAML mapping as a Relationship if it has source/target fields.
    /// Returns (id, ElementKind, source, target, owner) tuple.
    fn parse_relationship(
        map: &serde_yaml::Mapping,
    ) -> Option<(String, ElementKind, String, String, Option<ElementId>)> {
        // Must have @id, @type, source, and target
        let id = get_string(map, "@id")?;
        let type_str = get_string(map, "@type")?;

        // Must have source and target to be a relationship
        let source = get_ref_id(map, "source")?;
        let target = get_ref_id(map, "target")?;

        // Parse the kind from the type string
        let kind = ElementKind::from_xmi_type(&type_str);

        // Get owner if present
        let owner = get_ref_id(map, "owner").map(ElementId::new);

        Some((id, kind, source, target, owner))
    }

    /// Read extra properties (name, importedNamespace, etc.) from a YAML
    /// relationship mapping and apply them to the already-created relationship
    /// element in the model.
    fn read_relationship_properties(
        map: &serde_yaml::Mapping,
        rel_id: &ElementId,
        model: &mut Model,
    ) {
        let reserved_keys = ["@type", "@id", "source", "target", "owner"];
        let mut name: Option<String> = None;
        let mut props: Vec<(Arc<str>, PropertyValue)> = Vec::new();

        for (key, value) in map {
            if let Some(key_str) = key.as_str() {
                if reserved_keys.contains(&key_str) {
                    continue;
                }
                if key_str == "name" {
                    if let Some(s) = value.as_str() {
                        name = Some(s.to_string());
                    }
                } else if let Some(prop_value) = parse_property_value(value) {
                    props.push((Arc::from(key_str), prop_value));
                }
            }
        }

        if name.is_some() || !props.is_empty() {
            if let Some(el) = model.get_mut(rel_id) {
                if let Some(n) = name {
                    el.name = Some(Arc::from(n.as_str()));
                }
                for (k, v) in props {
                    el.properties.insert(k, v);
                }
            }
        }
    }

    /// Parse a YAML mapping into an Element.
    fn parse_element(map: &serde_yaml::Mapping) -> Result<Option<Element>, InterchangeError> {
        // Get @type (required)
        let type_str = match get_string(map, "@type") {
            Some(s) => s,
            None => return Ok(None),
        };

        let kind = ElementKind::from_xmi_type(&type_str);

        // Get @id (required)
        let id = match get_string(map, "@id") {
            Some(s) => ElementId::new(s),
            None => ElementId::generate(),
        };

        let mut element = Element::new(id, kind);

        // Parse name
        if let Some(name) = get_string(map, "name") {
            element.name = Some(Arc::from(name.as_str()));
        }

        // Parse shortName
        if let Some(short_name) = get_string(map, "shortName") {
            element.short_name = Some(Arc::from(short_name.as_str()));
        }

        // Parse qualifiedName
        if let Some(qn) = get_string(map, "qualifiedName") {
            element.qualified_name = Some(Arc::from(qn.as_str()));
        }

        // Parse documentation
        if let Some(doc) = get_string(map, "documentation") {
            element.documentation = Some(Arc::from(doc.as_str()));
        }

        // Parse boolean flags (use setters to sync to properties)
        if let Some(val) = map.get("isAbstract") {
            element.set_abstract(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isVariation") {
            element.set_variation(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isDerived") {
            element.set_derived(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isReadOnly") {
            element.set_readonly(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isParallel") {
            element.set_parallel(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isIndividual") {
            element.set_individual(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isEnd") {
            element.set_end(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isDefault") {
            element.set_default(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isOrdered") {
            element.set_ordered(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isNonunique") {
            element.set_nonunique(val.as_bool().unwrap_or(false));
        }
        if let Some(val) = map.get("isPortion") {
            element.set_portion(val.as_bool().unwrap_or(false));
        }

        // Parse owner reference
        if let Some(owner_id) = get_ref_id(map, "owner") {
            element.owner = Some(ElementId::new(owner_id));
        }

        // Parse ownedMember references
        if let Some(members_val) = map.get("ownedMember") {
            if let Some(members_seq) = members_val.as_sequence() {
                for member in members_seq {
                    if let Some(member_map) = member.as_mapping() {
                        if let Some(id_str) = get_string(member_map, "@id") {
                            element.owned_elements.push(ElementId::new(id_str));
                        }
                    } else if let Some(id_str) = member.as_str() {
                        element.owned_elements.push(ElementId::new(id_str));
                    }
                }
            }
        }

        // Parse additional properties
        let reserved_keys = [
            "@type",
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
            "isIndividual",
            "isEnd",
            "isDefault",
            "isOrdered",
            "isNonunique",
            "isPortion",
            "owner",
            "ownedMember",
            "source",
            "target",
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

        Ok(Some(element))
    }

    /// Get a string value from a mapping.
    fn get_string(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
        map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Get a reference ID from a mapping (handles both `{@id: x}` and plain string).
    fn get_ref_id(map: &serde_yaml::Mapping, key: &str) -> Option<String> {
        map.get(key).and_then(|v| {
            if let Some(inner_map) = v.as_mapping() {
                get_string(inner_map, "@id")
            } else {
                v.as_str().map(|s| s.to_string())
            }
        })
    }

    /// Parse a YAML value into a PropertyValue.
    fn parse_property_value(value: &Value) -> Option<PropertyValue> {
        match value {
            Value::String(s) => Some(PropertyValue::String(Arc::from(s.as_str()))),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(PropertyValue::Integer(i))
                } else {
                    n.as_f64().map(PropertyValue::Real)
                }
            }
            Value::Bool(b) => Some(PropertyValue::Boolean(*b)),
            Value::Mapping(map) => {
                if let Some(id_str) = get_string(map, "@id") {
                    return Some(PropertyValue::Reference(ElementId::new(id_str)));
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
        let mut updates = Vec::new();
        for element in model.iter_elements() {
            let owner_id = element.id.clone();
            for owned_id in &element.owned_elements {
                updates.push((owner_id.clone(), owned_id.clone()));
            }
        }
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
    use crate::interchange::model::{Element, PropertyValue};
    use serde_yaml::{Mapping, Value};

    pub struct YamlWriter;

    impl YamlWriter {
        pub fn new() -> Self {
            Self
        }

        pub fn write(&self, model: &Model) -> Result<Vec<u8>, InterchangeError> {
            let mut all_items: Vec<Value> = Vec::new();

            // Add all elements (non-relationship)
            for element in model.iter_elements() {
                if element.relationship.is_none() {
                    all_items.push(element_to_yaml(element));
                }
            }

            // Add all relationship elements as separate objects
            for rel_element in model.iter_relationship_elements() {
                all_items.push(rel_element_to_yaml(rel_element));
            }

            let output = if all_items.len() == 1 {
                all_items.into_iter().next().unwrap()
            } else {
                Value::Sequence(all_items)
            };

            serde_yaml::to_string(&output)
                .map(|s| s.into_bytes())
                .map_err(|e| InterchangeError::yaml(format!("YAML serialization error: {e}")))
        }
    }

    /// Convert a relationship Element to YAML Value.
    fn rel_element_to_yaml(element: &Element) -> Value {
        let mut map = Mapping::new();

        // @type from ElementKind
        map.insert(
            Value::String("@type".to_string()),
            Value::String(element.kind.jsonld_type().to_string()),
        );
        map.insert(
            Value::String("@id".to_string()),
            Value::String(element.id.as_str().to_string()),
        );

        // name (if present)
        if let Some(ref name) = element.name {
            map.insert(
                Value::String("name".to_string()),
                Value::String(name.to_string()),
            );
        }

        if let Some(ref rd) = element.relationship {
            if let Some(src) = rd.source() {
                let mut source_map = Mapping::new();
                source_map.insert(
                    Value::String("@id".to_string()),
                    Value::String(src.as_str().to_string()),
                );
                map.insert(
                    Value::String("source".to_string()),
                    Value::Mapping(source_map),
                );
            }
            if let Some(tgt) = rd.target() {
                let mut target_map = Mapping::new();
                target_map.insert(
                    Value::String("@id".to_string()),
                    Value::String(tgt.as_str().to_string()),
                );
                map.insert(
                    Value::String("target".to_string()),
                    Value::Mapping(target_map),
                );
            }
        }

        // Properties (excluding internal _-prefixed keys)
        for (key, value) in &element.properties {
            if !key.starts_with('_') {
                let yaml_value = property_value_to_yaml(value);
                map.insert(Value::String(key.to_string()), yaml_value);
            }
        }

        // owner if present
        if let Some(ref owner_id) = element.owner {
            let mut owner_map = Mapping::new();
            owner_map.insert(
                Value::String("@id".to_string()),
                Value::String(owner_id.as_str().to_string()),
            );
            map.insert(
                Value::String("owner".to_string()),
                Value::Mapping(owner_map),
            );
        }

        Value::Mapping(map)
    }

    /// Convert an Element to YAML Value.
    fn element_to_yaml(element: &Element) -> Value {
        let mut map = Mapping::new();

        // @type - use jsonld_type for consistency
        map.insert(
            Value::String("@type".to_string()),
            Value::String(element.kind.jsonld_type().to_string()),
        );

        // @id
        map.insert(
            Value::String("@id".to_string()),
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

        // Boolean flags (only if true)
        if element.is_abstract {
            map.insert(Value::String("isAbstract".to_string()), Value::Bool(true));
        }
        if element.is_variation {
            map.insert(Value::String("isVariation".to_string()), Value::Bool(true));
        }
        if element.is_derived {
            map.insert(Value::String("isDerived".to_string()), Value::Bool(true));
        }
        if element.is_readonly {
            map.insert(Value::String("isReadOnly".to_string()), Value::Bool(true));
        }
        if element.is_parallel {
            map.insert(Value::String("isParallel".to_string()), Value::Bool(true));
        }
        if element.is_individual {
            map.insert(Value::String("isIndividual".to_string()), Value::Bool(true));
        }
        if element.is_end {
            map.insert(Value::String("isEnd".to_string()), Value::Bool(true));
        }
        if element.is_default {
            map.insert(Value::String("isDefault".to_string()), Value::Bool(true));
        }
        if element.is_ordered {
            map.insert(Value::String("isOrdered".to_string()), Value::Bool(true));
        }
        if element.is_nonunique {
            map.insert(Value::String("isNonunique".to_string()), Value::Bool(true));
        }
        if element.is_portion {
            map.insert(Value::String("isPortion".to_string()), Value::Bool(true));
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
                Value::String("@id".to_string()),
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
                        Value::String("@id".to_string()),
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

        Value::Mapping(map)
    }

    /// Convert a PropertyValue to YAML Value.
    fn property_value_to_yaml(value: &PropertyValue) -> Value {
        match value {
            PropertyValue::String(s) => Value::String(s.to_string()),
            PropertyValue::Integer(i) => Value::Number((*i).into()),
            PropertyValue::Real(f) => Value::Number(serde_yaml::Number::from(*f)),
            PropertyValue::Boolean(b) => Value::Bool(*b),
            PropertyValue::Reference(id) => {
                let mut m = Mapping::new();
                m.insert(
                    Value::String("@id".to_string()),
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
        assert!(yaml.validate(b"").is_err());
    }

    #[test]
    fn test_yaml_validate_valid() {
        let yaml = Yaml;
        assert!(yaml.validate(b"'@type': Package\nname: Test").is_ok());
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

            let bytes = yaml.write(&model).expect("write should succeed");
            let content = String::from_utf8(bytes.clone()).expect("should be valid UTF-8");
            assert!(content.contains("Package"));
            assert!(content.contains("TestPackage"));
            assert!(content.contains("test-id-123"));

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
