//! Recompile SysML analysis results with original element IDs.
//!
//! This module restores original XMI element IDs when exporting a model
//! that was originally imported from XMI/JSON-LD. It uses either:
//!
//! - The companion metadata file created during import (`restore_element_ids`)
//! - The SymbolIndex directly (`restore_ids_from_symbols`)
//!
//! ## Usage
//!
//! ```ignore
//! use syster::interchange::{ImportMetadata, Model, recompile::restore_element_ids};
//!
//! // Option 1: From metadata file
//! let metadata: ImportMetadata = serde_json::from_str(&metadata_json)?;
//! let model_with_ids = restore_element_ids(model, &metadata);
//!
//! // Option 2: From SymbolIndex (element_ids already loaded into symbols)
//! let model_with_ids = restore_ids_from_symbols(model, analysis.symbol_index());
//! ```

use super::metadata::ImportMetadata;
use super::model::{ElementId, Model};
use crate::hir::SymbolIndex;
use std::collections::HashMap;

/// Restore original element IDs from metadata.
///
/// This takes a model (typically from analysis) and replaces generated IDs
/// with the original IDs from the import metadata. This ensures that
/// re-exported XMI preserves element IDs for version control stability.
pub fn restore_element_ids(mut model: Model, metadata: &ImportMetadata) -> Model {
    // Build a mapping from qualified name -> original element ID
    let qn_to_original_id: HashMap<&str, &str> = metadata
        .elements
        .iter()
        .filter_map(|(qn, meta)| meta.original_id.as_deref().map(|id| (qn.as_str(), id)))
        .collect();

    // Build a mapping from current ID -> original ID based on qualified names
    let id_mapping: HashMap<ElementId, ElementId> = model
        .elements
        .iter()
        .filter_map(|(current_id, _element)| {
            // Compute qualified name for this element
            let qn = compute_qualified_name(&model, current_id);

            // Look up original ID
            qn_to_original_id
                .get(qn.as_str())
                .map(|orig_id| (current_id.clone(), ElementId::new(*orig_id)))
        })
        .collect();

    // Apply the mapping
    apply_id_mapping(&mut model, &id_mapping);

    model
}

/// Restore element IDs from a SymbolIndex.
///
/// This reads element IDs directly from symbols, which is useful when
/// the metadata has already been loaded into the AnalysisHost via
/// `apply_metadata_to_host()`.
///
/// Use this instead of `restore_element_ids()` when you have an AnalysisHost
/// with metadata already applied.
pub fn restore_ids_from_symbols(mut model: Model, symbol_index: &SymbolIndex) -> Model {
    // Build a mapping from qualified name -> element ID from symbols
    let qn_to_id: HashMap<&str, &str> = symbol_index
        .all_symbols()
        .map(|sym| (sym.qualified_name.as_ref(), sym.element_id.as_ref()))
        .collect();

    // Build a mapping from current ID -> symbol's element ID
    let id_mapping: HashMap<ElementId, ElementId> = model
        .elements
        .iter()
        .filter_map(|(current_id, _element)| {
            // Compute qualified name for this element
            let qn = compute_qualified_name(&model, current_id);

            // Look up element ID from symbol
            qn_to_id
                .get(qn.as_str())
                .map(|sym_id| (current_id.clone(), ElementId::new(*sym_id)))
        })
        .collect();

    // Apply the mapping
    apply_id_mapping(&mut model, &id_mapping);

    model
}

/// Compute qualified name for an element by traversing ownership.
fn compute_qualified_name(model: &Model, id: &ElementId) -> String {
    let Some(element) = model.elements.get(id) else {
        return id.as_str().to_string();
    };

    let name = element.name.as_deref().unwrap_or_else(|| id.as_str());

    if let Some(owner_id) = &element.owner {
        let owner_qn = compute_qualified_name(model, owner_id);
        format!("{}::{}", owner_qn, name)
    } else {
        name.to_string()
    }
}

/// Apply ID mapping to all elements and relationships in the model.
fn apply_id_mapping(model: &mut Model, mapping: &HashMap<ElementId, ElementId>) {
    use indexmap::IndexMap;

    // Remap elements
    let old_elements = std::mem::take(&mut model.elements);
    let mut new_elements = IndexMap::with_capacity(old_elements.len());

    for (old_id, mut element) in old_elements {
        // Get new ID (or keep old if not in mapping)
        let new_id = mapping.get(&old_id).cloned().unwrap_or(old_id);

        // Update element's own ID
        element.id = new_id.clone();

        // Update owner reference
        if let Some(ref old_owner) = element.owner {
            if let Some(new_owner) = mapping.get(old_owner) {
                element.owner = Some(new_owner.clone());
            }
        }

        // Update owned element references
        element.owned_elements = element
            .owned_elements
            .iter()
            .map(|old_child| {
                mapping
                    .get(old_child)
                    .cloned()
                    .unwrap_or_else(|| old_child.clone())
            })
            .collect();

        new_elements.insert(new_id, element);
    }

    model.elements = new_elements;

    // Remap roots
    model.roots = model
        .roots
        .iter()
        .map(|old_id| {
            mapping
                .get(old_id)
                .cloned()
                .unwrap_or_else(|| old_id.clone())
        })
        .collect();

    // Remap relationships
    for rel in &mut model.relationships {
        if let Some(new_id) = mapping.get(&rel.id) {
            rel.id = new_id.clone();
        }
        if let Some(new_source) = mapping.get(&rel.source) {
            rel.source = new_source.clone();
        }
        if let Some(new_target) = mapping.get(&rel.target) {
            rel.target = new_target.clone();
        }
        if let Some(ref old_owner) = rel.owner {
            if let Some(new_owner) = mapping.get(old_owner) {
                rel.owner = Some(new_owner.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interchange::metadata::ElementMeta;
    use crate::interchange::model::{Element, ElementKind};

    #[test]
    fn test_restore_element_ids_empty_model() {
        let model = Model::new();
        let metadata = ImportMetadata::new();

        let result = restore_element_ids(model, &metadata);
        assert!(result.elements.is_empty());
    }

    #[test]
    fn test_restore_element_ids_single_element() {
        let mut model = Model::new();
        model.add_element(Element::new("new-id-1", ElementKind::Package).with_name("MyPackage"));

        let mut metadata = ImportMetadata::new();
        metadata.add_element("MyPackage", ElementMeta::with_id("original-uuid-123"));

        let result = restore_element_ids(model, &metadata);

        // Element should now have original ID
        assert!(
            result
                .elements
                .contains_key(&ElementId::new("original-uuid-123"))
        );
        assert!(!result.elements.contains_key(&ElementId::new("new-id-1")));

        let elem = result.get(&ElementId::new("original-uuid-123")).unwrap();
        assert_eq!(elem.name.as_deref(), Some("MyPackage"));
    }

    #[test]
    fn test_restore_element_ids_with_ownership() {
        let mut model = Model::new();

        // Parent package
        let mut pkg = Element::new("pkg-new", ElementKind::Package).with_name("MyPackage");
        pkg.owned_elements.push(ElementId::new("def-new"));

        // Child definition
        let def = Element::new("def-new", ElementKind::PartDefinition)
            .with_name("MyPart")
            .with_owner("pkg-new");

        model.add_element(pkg);
        model.add_element(def);

        // Metadata with original IDs
        let mut metadata = ImportMetadata::new();
        metadata.add_element("MyPackage", ElementMeta::with_id("pkg-original"));
        metadata.add_element("MyPackage::MyPart", ElementMeta::with_id("def-original"));

        let result = restore_element_ids(model, &metadata);

        // Check IDs restored
        assert!(
            result
                .elements
                .contains_key(&ElementId::new("pkg-original"))
        );
        assert!(
            result
                .elements
                .contains_key(&ElementId::new("def-original"))
        );

        // Check ownership references updated
        let pkg = result.get(&ElementId::new("pkg-original")).unwrap();
        assert_eq!(pkg.owned_elements.len(), 1);
        assert_eq!(pkg.owned_elements[0].as_str(), "def-original");

        let def = result.get(&ElementId::new("def-original")).unwrap();
        assert_eq!(
            def.owner.as_ref().map(|id| id.as_str()),
            Some("pkg-original")
        );
    }

    #[test]
    fn test_restore_preserves_unmapped_elements() {
        let mut model = Model::new();
        model.add_element(Element::new("existing", ElementKind::Package).with_name("Existing"));
        model.add_element(Element::new("new-only", ElementKind::Package).with_name("NewOnly"));

        // Only provide metadata for one element
        let mut metadata = ImportMetadata::new();
        metadata.add_element("Existing", ElementMeta::with_id("original-existing"));

        let result = restore_element_ids(model, &metadata);

        // One restored, one kept as-is
        assert!(
            result
                .elements
                .contains_key(&ElementId::new("original-existing"))
        );
        assert!(result.elements.contains_key(&ElementId::new("new-only")));
        assert_eq!(result.elements.len(), 2);
    }

    #[test]
    fn test_restore_updates_roots() {
        let mut model = Model::new();
        model.add_element(Element::new("root-new", ElementKind::Package).with_name("Root"));
        // add_element already adds to roots when no owner, so don't push again

        let mut metadata = ImportMetadata::new();
        metadata.add_element("Root", ElementMeta::with_id("root-original"));

        let result = restore_element_ids(model, &metadata);

        assert_eq!(result.roots.len(), 1);
        assert_eq!(result.roots[0].as_str(), "root-original");
    }

    #[test]
    fn test_restore_ids_from_symbols() {
        use crate::ide::AnalysisHost;
        use crate::interchange::integrate::apply_metadata_to_host;

        // Create a host with SysML content
        let mut host = AnalysisHost::new();
        let sysml = r#"
package TestPkg {
    part def Vehicle;
}
"#;
        host.set_file_content("/test.sysml", sysml);

        // Apply metadata to set element IDs on symbols
        let mut metadata = ImportMetadata::new();
        metadata.add_element("TestPkg", ElementMeta::with_id("uuid-pkg"));
        metadata.add_element("TestPkg::Vehicle", ElementMeta::with_id("uuid-vehicle"));
        apply_metadata_to_host(&mut host, &metadata);

        // Create a model that would come from exporting the analysis
        let mut model = Model::new();
        let mut pkg = Element::new("temp-pkg-id", ElementKind::Package).with_name("TestPkg");
        pkg.owned_elements.push(ElementId::from("temp-vehicle-id"));
        model.add_element(pkg);

        model.add_element(
            Element::new("temp-vehicle-id", ElementKind::PartDefinition)
                .with_name("Vehicle")
                .with_owner("temp-pkg-id"),
        );

        // Restore IDs from symbol index
        let analysis = host.analysis();
        let result = restore_ids_from_symbols(model, analysis.symbol_index());

        // Verify IDs were restored
        assert!(
            result.elements.contains_key(&ElementId::new("uuid-pkg")),
            "Package should have restored ID"
        );
        assert!(
            result
                .elements
                .contains_key(&ElementId::new("uuid-vehicle")),
            "Vehicle should have restored ID"
        );

        // Verify ownership references updated
        let pkg = result.get(&ElementId::new("uuid-pkg")).unwrap();
        assert_eq!(pkg.owned_elements[0].as_str(), "uuid-vehicle");

        let vehicle = result.get(&ElementId::new("uuid-vehicle")).unwrap();
        assert_eq!(
            vehicle.owner.as_ref().map(|id| id.as_str()),
            Some("uuid-pkg")
        );
    }
}
