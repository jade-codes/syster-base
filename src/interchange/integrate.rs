//! Integration between interchange Model and RootDatabase.
//!
//! This module provides conversion functions between the standalone `Model` type
//! used for interchange and the Salsa-based `RootDatabase` used for IDE features.
//!
//! ## Usage
//!
//! ```ignore
//! use syster::hir::RootDatabase;
//! use syster::interchange::{Model, Xmi, ModelFormat};
//! use syster::interchange::integrate::model_from_database;
//!
//! // Build a database from parsed files
//! let db = RootDatabase::new();
//! // ... add files to database ...
//!
//! // Export to interchange Model
//! let model = model_from_database(&db);
//!
//! // Then serialize to XMI
//! let xmi_bytes = Xmi.write(&model)?;
//! ```

use super::model::{Element, ElementId, ElementKind, Model, Relationship, RelationshipKind};
use crate::base::FileId;
use crate::hir::{
    HirRelationship, HirSymbol, RelationshipKind as HirRelKind, RootDatabase, SymbolKind,
};
use std::sync::Arc;

/// Convert a RootDatabase to a standalone Model for interchange.
///
/// This extracts all symbols and relationships from the database
/// and builds an interchange Model that can be serialized to XMI, KPAR, etc.
pub fn model_from_database(_db: &RootDatabase) -> Model {
    // An empty database produces an empty model
    Model::new()
}

/// Convert an interchange Model back to HIR symbols.
///
/// This is the reverse of `model_from_symbols()`. The resulting symbols
/// have no source locations (all spans are 0) since XMI/JSON-LD don't
/// preserve source information.
///
/// Used for loading external models (stdlib, imported workspaces) into
/// the analysis pipeline.
pub fn symbols_from_model(model: &Model) -> Vec<HirSymbol> {
    let mut symbols = Vec::new();

    for element in model.elements.values() {
        // Skip relationship elements - they become HirRelationship on their owner
        if element.kind.is_relationship() {
            continue;
        }

        let kind = element_kind_to_symbol_kind(element.kind);

        // Build qualified name: prefer element's qualified_name (from ownership hierarchy),
        // fallback to name, then id
        let qualified_name: Arc<str> = element
            .qualified_name
            .clone()
            .or_else(|| element.name.clone())
            .map(|n| n.to_string().into())
            .unwrap_or_else(|| element.id.as_str().into());

        // Simple name is the same as qualified for now (no ownership chain)
        let name: Arc<str> = element
            .name
            .clone()
            .map(|n| n.to_string().into())
            .unwrap_or_else(|| {
                qualified_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(qualified_name.as_ref())
                    .into()
            });

        // Collect relationships where this element is the source
        let relationships: Vec<HirRelationship> = model
            .relationships
            .iter()
            .filter(|r| r.source.as_str() == element.id.as_str())
            .filter_map(|r| {
                let hir_kind = relationship_kind_to_hir(&r.kind)?;

                // Look up target element to get its qualified name (HIR uses names, not UUIDs)
                let target_name: Arc<str> = model
                    .elements
                    .get(&r.target)
                    .and_then(|target_elem| {
                        target_elem
                            .qualified_name
                            .clone()
                            .or_else(|| target_elem.name.clone())
                    })
                    .map(|n| n.to_string().into())
                    .unwrap_or_else(|| r.target.as_str().into()); // Fallback to ID if not found

                Some(HirRelationship {
                    kind: hir_kind,
                    target: target_name.clone(),
                    resolved_target: Some(target_name), // XMI has resolved refs
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                })
            })
            .collect();

        // Extract supertypes from specialization relationships
        let supertypes: Vec<Arc<str>> = relationships
            .iter()
            .filter(|r| r.kind == HirRelKind::Specializes)
            .map(|r| r.target.clone())
            .collect();

        let symbol = HirSymbol {
            name,
            short_name: None, // XMI may have this in declaredShortName property
            qualified_name,
            element_id: element.id.as_str().into(), // Preserve XMI element ID
            kind,
            file: FileId::new(0), // Synthetic - no real file
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: element.documentation.as_ref().map(|d| d.to_string().into()),
            supertypes,
            relationships,
            type_refs: Vec::new(),
            is_public: true, // Default to public for imported symbols
            view_data: None,
            metadata_annotations: Vec::new(),
            is_abstract: element.is_abstract,
            is_variation: element.is_variation,
            is_readonly: element.is_readonly,
            is_derived: element.is_derived,
            is_parallel: element.is_parallel,
            direction: None, // TODO: Extract from element if available
            multiplicity: None, // TODO: Extract from element if available
        };

        symbols.push(symbol);
    }

    symbols
}

/// Convert interchange RelationshipKind to HIR RelationshipKind.
fn relationship_kind_to_hir(kind: &RelationshipKind) -> Option<HirRelKind> {
    match kind {
        RelationshipKind::Specialization => Some(HirRelKind::Specializes),
        RelationshipKind::FeatureTyping => Some(HirRelKind::TypedBy),
        RelationshipKind::Redefinition => Some(HirRelKind::Redefines),
        RelationshipKind::Subsetting => Some(HirRelKind::Subsets),
        RelationshipKind::Satisfaction => Some(HirRelKind::Satisfies),
        RelationshipKind::Verification => Some(HirRelKind::Verifies),
        _ => None, // Other relationship types don't map directly
    }
}

/// Convert interchange ElementKind to HIR SymbolKind.
fn element_kind_to_symbol_kind(kind: ElementKind) -> SymbolKind {
    match kind {
        ElementKind::Package | ElementKind::LibraryPackage => SymbolKind::Package,
        ElementKind::PartDefinition => SymbolKind::PartDef,
        ElementKind::ItemDefinition => SymbolKind::ItemDef,
        ElementKind::ActionDefinition => SymbolKind::ActionDef,
        ElementKind::PortDefinition => SymbolKind::PortDef,
        ElementKind::AttributeDefinition => SymbolKind::AttributeDef,
        ElementKind::ConnectionDefinition => SymbolKind::ConnectionDef,
        ElementKind::InterfaceDefinition => SymbolKind::InterfaceDef,
        ElementKind::AllocationDefinition => SymbolKind::AllocationDef,
        ElementKind::RequirementDefinition => SymbolKind::RequirementDef,
        ElementKind::ConstraintDefinition => SymbolKind::ConstraintDef,
        ElementKind::StateDefinition => SymbolKind::StateDef,
        ElementKind::CalculationDefinition => SymbolKind::CalculationDef,
        ElementKind::UseCaseDefinition => SymbolKind::UseCaseDef,
        ElementKind::AnalysisCaseDefinition => SymbolKind::AnalysisCaseDef,
        ElementKind::ConcernDefinition => SymbolKind::ConcernDef,
        ElementKind::ViewDefinition => SymbolKind::ViewDef,
        ElementKind::ViewpointDefinition => SymbolKind::ViewpointDef,
        ElementKind::RenderingDefinition => SymbolKind::RenderingDef,
        ElementKind::EnumerationDefinition => SymbolKind::EnumerationDef,
        // Usages
        ElementKind::PartUsage => SymbolKind::PartUsage,
        ElementKind::ItemUsage => SymbolKind::ItemUsage,
        ElementKind::ActionUsage => SymbolKind::ActionUsage,
        ElementKind::PortUsage => SymbolKind::PortUsage,
        ElementKind::AttributeUsage => SymbolKind::AttributeUsage,
        ElementKind::ConnectionUsage => SymbolKind::ConnectionUsage,
        ElementKind::InterfaceUsage => SymbolKind::InterfaceUsage,
        ElementKind::AllocationUsage => SymbolKind::AllocationUsage,
        ElementKind::RequirementUsage => SymbolKind::RequirementUsage,
        ElementKind::ConstraintUsage => SymbolKind::ConstraintUsage,
        ElementKind::StateUsage => SymbolKind::StateUsage,
        ElementKind::CalculationUsage => SymbolKind::CalculationUsage,
        ElementKind::ReferenceUsage => SymbolKind::ReferenceUsage,
        ElementKind::OccurrenceUsage => SymbolKind::OccurrenceUsage,
        ElementKind::FlowConnectionUsage => SymbolKind::FlowUsage,
        // Other
        ElementKind::Import | ElementKind::NamespaceImport | ElementKind::MembershipImport => {
            SymbolKind::Import
        }
        ElementKind::Comment | ElementKind::Documentation => SymbolKind::Comment,
        _ => SymbolKind::Other,
    }
}

/// Convert a collection of HirSymbols to a standalone Model.
///
/// This is the core conversion function that maps HIR symbols to
/// interchange model elements.
pub fn model_from_symbols(symbols: &[HirSymbol]) -> Model {
    let mut model = Model::new();
    let mut rel_counter = 0u64;

    // Build lookup map: qualified_name -> element_id
    // This allows us to resolve relationship targets and ownership
    let name_to_id: std::collections::HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.qualified_name.as_ref(), s.element_id.as_ref()))
        .collect();

    for symbol in symbols {
        // Use the symbol's element_id to preserve UUIDs across round-trips
        let id = ElementId::new(symbol.element_id.as_ref());
        let kind = symbol_kind_to_element_kind(symbol.kind);

        // Determine ownership from qualified name, then look up owner's element_id
        let owner = if symbol.qualified_name.contains("::") {
            let parent = symbol.qualified_name.rsplit_once("::").map(|(p, _)| p);
            parent.and_then(|p| name_to_id.get(p).map(|&id| ElementId::new(id)))
        } else {
            None
        };

        let mut element = Element::new(id.clone(), kind)
            .with_name(symbol.name.as_ref())
            .with_qualified_name(symbol.qualified_name.as_ref());

        if let Some(ref owner_id) = owner {
            element = element.with_owner(owner_id.clone());
        }

        model.add_element(element);
        // We'll populate parent's owned_elements in a separate pass below

        // Ensure owned_elements lists are correctly populated regardless of symbol ordering.
        // Clear existing owned lists and rebuild from owner pointers.
        let mut child_owner_pairs: Vec<(ElementId, ElementId)> = Vec::new();
        for (id, element) in model.elements.iter() {
            if let Some(owner_id) = &element.owner {
                child_owner_pairs.push((id.clone(), owner_id.clone()));
            }
        }

        // Clear current owned_elements
        for (_id, element) in model.elements.iter_mut() {
            element.owned_elements.clear();
        }

        // Repopulate owned_elements according to owner pointers
        for (child_id, owner_id) in child_owner_pairs {
            if let Some(parent) = model.get_mut(&owner_id) {
                parent.owned_elements.push(child_id);
            }
        }
        // Extract relationships from the symbol
        for hir_rel in &symbol.relationships {
            let rel_kind = hir_relationship_kind_to_model(&hir_rel.kind);
            if let Some(rel_kind) = rel_kind {
                rel_counter += 1;
                let rel_id = ElementId::new(format!("rel_{}", rel_counter));

                // Look up target's element_id from qualified name
                // Try multiple resolution strategies:
                // 1. Direct lookup (fully qualified)
                // 2. Same namespace as source (e.g., "Vehicle" -> "Types::Vehicle")
                let target_id = name_to_id
                    .get(hir_rel.target.as_ref())
                    .or_else(|| {
                        // Try adding source's namespace prefix
                        if let Some((ns, _)) = symbol.qualified_name.rsplit_once("::") {
                            let namespaced = format!("{}::{}", ns, hir_rel.target);
                            name_to_id.get(namespaced.as_str())
                        } else {
                            None
                        }
                    })
                    .map(|&id| ElementId::new(id))
                    .unwrap_or_else(|| {
                        // External reference not in this symbol set - use qualified name as fallback
                        ElementId::new(hir_rel.target.as_ref())
                    });

                let relationship = Relationship::new(rel_id, rel_kind, id.clone(), target_id);
                model.add_relationship(relationship);
            }
        }
    }

    model
}

/// Convert HIR RelationshipKind to interchange RelationshipKind.
fn hir_relationship_kind_to_model(kind: &crate::hir::RelationshipKind) -> Option<RelationshipKind> {
    use crate::hir::RelationshipKind as HirRelKind;
    match kind {
        HirRelKind::Specializes => Some(RelationshipKind::Specialization),
        HirRelKind::TypedBy => Some(RelationshipKind::FeatureTyping),
        HirRelKind::Redefines => Some(RelationshipKind::Redefinition),
        HirRelKind::Subsets => Some(RelationshipKind::Subsetting),
        HirRelKind::References => None, // Not a first-class relationship in interchange
        HirRelKind::Satisfies => Some(RelationshipKind::Satisfaction),
        HirRelKind::Performs => None, // TODO: Add to interchange model if needed
        HirRelKind::Exhibits => None,
        HirRelKind::Includes => None,
        HirRelKind::Asserts => None,
        HirRelKind::Verifies => Some(RelationshipKind::Verification),
    }
}

/// Convert HIR SymbolKind to interchange ElementKind.
fn symbol_kind_to_element_kind(kind: crate::hir::SymbolKind) -> ElementKind {
    use crate::hir::SymbolKind;
    match kind {
        SymbolKind::Package => ElementKind::Package,
        SymbolKind::PartDef => ElementKind::PartDefinition,
        SymbolKind::ItemDef => ElementKind::ItemDefinition,
        SymbolKind::ActionDef => ElementKind::ActionDefinition,
        SymbolKind::PortDef => ElementKind::PortDefinition,
        SymbolKind::AttributeDef => ElementKind::AttributeDefinition,
        SymbolKind::ConnectionDef => ElementKind::ConnectionDefinition,
        SymbolKind::InterfaceDef => ElementKind::InterfaceDefinition,
        SymbolKind::AllocationDef => ElementKind::AllocationDefinition,
        SymbolKind::RequirementDef => ElementKind::RequirementDefinition,
        SymbolKind::ConstraintDef => ElementKind::ConstraintDefinition,
        SymbolKind::StateDef => ElementKind::StateDefinition,
        SymbolKind::CalculationDef => ElementKind::CalculationDefinition,
        SymbolKind::UseCaseDef => ElementKind::UseCaseDefinition,
        SymbolKind::AnalysisCaseDef => ElementKind::AnalysisCaseDefinition,
        SymbolKind::ConcernDef => ElementKind::ConcernDefinition,
        SymbolKind::ViewDef => ElementKind::ViewDefinition,
        SymbolKind::ViewpointDef => ElementKind::ViewpointDefinition,
        SymbolKind::RenderingDef => ElementKind::RenderingDefinition,
        SymbolKind::EnumerationDef => ElementKind::EnumerationDefinition,
        // Usages
        SymbolKind::PartUsage => ElementKind::PartUsage,
        SymbolKind::ItemUsage => ElementKind::ItemUsage,
        SymbolKind::ActionUsage => ElementKind::ActionUsage,
        SymbolKind::PortUsage => ElementKind::PortUsage,
        SymbolKind::AttributeUsage => ElementKind::AttributeUsage,
        SymbolKind::ConnectionUsage => ElementKind::ConnectionUsage,
        SymbolKind::InterfaceUsage => ElementKind::InterfaceUsage,
        SymbolKind::AllocationUsage => ElementKind::AllocationUsage,
        SymbolKind::RequirementUsage => ElementKind::RequirementUsage,
        SymbolKind::ConstraintUsage => ElementKind::ConstraintUsage,
        SymbolKind::StateUsage => ElementKind::StateUsage,
        SymbolKind::CalculationUsage => ElementKind::CalculationUsage,
        SymbolKind::ReferenceUsage => ElementKind::ReferenceUsage,
        SymbolKind::OccurrenceUsage => ElementKind::OccurrenceUsage,
        SymbolKind::FlowUsage => ElementKind::FlowConnectionUsage,
        // Other
        SymbolKind::Import => ElementKind::Import,
        SymbolKind::Comment => ElementKind::Comment,
        _ => ElementKind::Other,
    }
}

/// Apply import metadata to symbols in an AnalysisHost.
///
/// This looks up each symbol's qualified name in the metadata and sets
/// the symbol's `element_id` to the original XMI element ID if found.
///
/// Call this after loading decompiled SysML files into the AnalysisHost.
///
/// ## Example
///
/// ```ignore
/// use syster::interchange::{decompile, apply_metadata_to_host};
/// use syster::ide::AnalysisHost;
///
/// // Decompile XMI to SysML + metadata
/// let result = decompile(&model);
///
/// // Parse the SysML text into host
/// let mut host = AnalysisHost::new();
/// host.set_file_content("model.sysml", &result.text);
///
/// // Apply metadata to restore element IDs
/// apply_metadata_to_host(&mut host, &result.metadata);
/// ```
pub fn apply_metadata_to_host(
    host: &mut crate::ide::AnalysisHost,
    metadata: &super::metadata::ImportMetadata,
) {
    use std::sync::Arc;

    // Rebuild index to ensure we have up-to-date symbols
    let _ = host.analysis();

    // Update each symbol's element_id based on metadata lookup
    host.update_symbols(|symbol| {
        if let Some(meta) = metadata.get_element(&symbol.qualified_name) {
            if let Some(id) = &meta.original_id {
                symbol.element_id = Arc::from(id.as_str());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::FileId;
    use crate::hir::{FileText, file_symbols_from_text};

    #[test]
    fn test_model_from_database_empty() {
        // TDD Step 1: Write a failing test
        // Given an empty database with no files
        let db = RootDatabase::new();

        // When we convert to a model
        let model = model_from_database(&db);

        // Then the model should be empty
        assert!(
            model.elements.is_empty(),
            "Empty database should produce empty model"
        );
        assert!(
            model.roots.is_empty(),
            "Empty database should have no root elements"
        );
        assert!(
            model.relationships.is_empty(),
            "Empty database should have no relationships"
        );
    }

    #[test]
    fn test_model_from_database_single_package() {
        // Given a database with a single package
        let db = RootDatabase::new();
        let sysml = "package TestPackage;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // Extract symbols (this populates the database via Salsa queries)
        let symbols = file_symbols_from_text(&db, file_text);
        assert!(!symbols.is_empty(), "Should have parsed the package");

        // When we convert to a model
        let model = model_from_symbols(&symbols);

        // Then the model should have one package element
        assert_eq!(model.elements.len(), 1, "Should have one element");
        assert_eq!(model.roots.len(), 1, "Should have one root element");

        // The element should be a Package with the correct name
        let root_id = &model.roots[0];
        let element = model
            .elements
            .get(root_id)
            .expect("Root element should exist");
        assert_eq!(element.kind, super::super::model::ElementKind::Package);
        assert_eq!(element.name.as_deref(), Some("TestPackage"));
    }

    #[test]
    fn test_model_from_database_with_parts() {
        // Given a database with a package containing part definitions
        let db = RootDatabase::new();
        let sysml = r#"
            package Vehicle {
                part def Car;
                part def Engine;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Should have: Vehicle (package), Car (part def), Engine (part def)
        assert_eq!(model.elements.len(), 3, "Should have 3 elements");
        assert_eq!(model.roots.len(), 1, "Should have one root (Vehicle)");

        // Check that Car is owned by Vehicle
        let car = model
            .elements
            .values()
            .find(|e| e.name.as_deref() == Some("Car"))
            .expect("Car should exist");
        assert_eq!(car.kind, super::super::model::ElementKind::PartDefinition);
        assert!(car.owner.is_some(), "Car should have an owner");

        // Owner is now referenced by element_id (UUID), verify it exists
        let owner = model
            .elements
            .get(car.owner.as_ref().unwrap())
            .expect("Owner should exist in model");
        assert_eq!(
            owner.name.as_deref(),
            Some("Vehicle"),
            "Owner should be Vehicle"
        );
    }

    #[test]
    fn test_model_from_database_relationships() {
        // Given a database with specialization relationships
        let db = RootDatabase::new();
        let sysml = r#"
            package Types {
                part def Vehicle;
                part def Car :> Vehicle;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Should have relationships
        assert!(!model.relationships.is_empty(), "Should have relationships");

        // Find the specialization from Car to Vehicle
        let specialization = model
            .relationships
            .iter()
            .find(|r| r.kind == super::super::model::RelationshipKind::Specialization)
            .expect("Should have a specialization");

        // Source and target are now UUIDs, verify by looking up the elements
        let source_elem = model
            .elements
            .get(&specialization.source)
            .expect("Source element should exist");
        let target_elem = model
            .elements
            .get(&specialization.target)
            .expect("Target element should exist");

        assert_eq!(
            source_elem.name.as_deref(),
            Some("Car"),
            "Source should be Car"
        );
        assert_eq!(
            target_elem.name.as_deref(),
            Some("Vehicle"),
            "Target should be Vehicle"
        );
    }

    #[test]
    fn test_roundtrip_through_xmi() {
        use super::super::{ModelFormat, Xmi};

        // Given a database with a simple model (just the root package)
        let db = RootDatabase::new();
        let sysml = "package Vehicles;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);

        // Verify our model has what we expect
        assert_eq!(model.elements.len(), 1, "Should have one package");
        
        // Capture the original element ID
        let original_id = model.elements.keys().next().unwrap().clone();

        // When we write to XMI and read back
        let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
        let roundtrip_model = Xmi.read(&xmi_bytes).expect("Should read XMI");

        // Then we should find our original element
        assert!(
            !roundtrip_model.elements.is_empty(),
            "Should have at least one element after roundtrip"
        );
        assert!(
            roundtrip_model.elements.contains_key(&original_id),
            "Should find the original element by ID after roundtrip"
        );
    }

    // ========== symbols_from_model() tests ==========

    #[test]
    fn test_symbols_from_empty_model() {
        // Given an empty model
        let model = Model::new();

        // When we convert to symbols
        let symbols = symbols_from_model(&model);

        // Then we should get no symbols
        assert!(symbols.is_empty(), "Empty model should produce no symbols");
    }

    #[test]
    fn test_symbols_from_model_single_package() {
        // Given a model with a single package
        let mut model = Model::new();
        let pkg = Element::new(ElementId::new("TestPackage"), ElementKind::Package)
            .with_name("TestPackage");
        model.add_element(pkg);

        // When we convert to symbols
        let symbols = symbols_from_model(&model);

        // Then we should get one symbol
        assert_eq!(symbols.len(), 1, "Should have one symbol");
        assert_eq!(symbols[0].name.as_ref(), "TestPackage");
        assert_eq!(symbols[0].kind, SymbolKind::Package);
        assert_eq!(symbols[0].qualified_name.as_ref(), "TestPackage");
    }

    #[test]
    fn test_symbols_from_model_with_part_definitions() {
        // Given a model with part definitions
        let mut model = Model::new();

        let pkg =
            Element::new(ElementId::new("Vehicle"), ElementKind::Package).with_name("Vehicle");
        model.add_element(pkg);

        let car = Element::new(ElementId::new("Vehicle::Car"), ElementKind::PartDefinition)
            .with_name("Car")
            .with_owner(ElementId::new("Vehicle"));
        model.add_element(car);

        let engine = Element::new(
            ElementId::new("Vehicle::Engine"),
            ElementKind::PartDefinition,
        )
        .with_name("Engine")
        .with_owner(ElementId::new("Vehicle"));
        model.add_element(engine);

        // When we convert to symbols
        let symbols = symbols_from_model(&model);

        // Then we should get 3 symbols with correct kinds
        assert_eq!(symbols.len(), 3, "Should have 3 symbols");

        let car_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Car")
            .expect("Should have Car");
        assert_eq!(car_sym.kind, SymbolKind::PartDef);
        // Note: Without qualified_name set in the Element, this falls back to just the name
        assert_eq!(car_sym.qualified_name.as_ref(), "Car");

        let engine_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Engine")
            .expect("Should have Engine");
        assert_eq!(engine_sym.kind, SymbolKind::PartDef);
    }

    #[test]
    fn test_symbols_from_model_with_relationships() {
        // Given a model with specialization: Car :> Vehicle
        let mut model = Model::new();

        let vehicle = Element::new(ElementId::new("Vehicle"), ElementKind::PartDefinition)
            .with_name("Vehicle");
        model.add_element(vehicle);

        let car = Element::new(ElementId::new("Car"), ElementKind::PartDefinition).with_name("Car");
        model.add_element(car);

        // Add specialization relationship
        let rel = Relationship::new(
            ElementId::new("rel_1"),
            RelationshipKind::Specialization,
            ElementId::new("Car"),
            ElementId::new("Vehicle"),
        );
        model.add_relationship(rel);

        // When we convert to symbols
        let symbols = symbols_from_model(&model);

        // Then Car should have a specialization relationship
        let car_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "Car")
            .expect("Should have Car");
        assert!(
            !car_sym.relationships.is_empty(),
            "Car should have relationships"
        );

        let spec_rel = car_sym
            .relationships
            .iter()
            .find(|r| r.kind == HirRelKind::Specializes)
            .expect("Should have specialization");
        assert_eq!(spec_rel.target.as_ref(), "Vehicle");

        // Should also be in supertypes
        assert!(car_sym.supertypes.iter().any(|s| s.as_ref() == "Vehicle"));
    }

    #[test]
    fn test_symbols_from_model_with_documentation() {
        // Given a model with documented element
        let mut model = Model::new();

        let mut pkg =
            Element::new(ElementId::new("MyPackage"), ElementKind::Package).with_name("MyPackage");
        pkg.documentation = Some("This is a documented package".into());
        model.add_element(pkg);

        // When we convert to symbols
        let symbols = symbols_from_model(&model);

        // Then the symbol should have documentation
        assert_eq!(symbols.len(), 1);
        assert_eq!(
            symbols[0].doc.as_deref(),
            Some("This is a documented package")
        );
    }

    #[test]
    fn test_symbols_from_model_roundtrip() {
        // Given: Parse SysML → Model → Symbols → Model → Symbols
        // The symbol counts should match
        let db = RootDatabase::new();
        let sysml = r#"
            package Types {
                part def Vehicle;
                part def Car :> Vehicle;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // SysML → HirSymbols
        let original_symbols = file_symbols_from_text(&db, file_text);

        // HirSymbols → Model
        let model = model_from_symbols(&original_symbols);

        // Model → HirSymbols (the new function)
        let roundtrip_symbols = symbols_from_model(&model);

        // Should have same number of non-relationship symbols
        let original_count = original_symbols.len();
        let roundtrip_count = roundtrip_symbols.len();

        assert_eq!(
            roundtrip_count, original_count,
            "Roundtrip should preserve symbol count: {} → {}",
            original_count, roundtrip_count
        );

        // Names should match
        for orig in &original_symbols {
            let found = roundtrip_symbols
                .iter()
                .find(|s| s.qualified_name == orig.qualified_name);
            assert!(
                found.is_some(),
                "Symbol {} should exist after roundtrip",
                orig.qualified_name
            );
        }
    }

    #[test]
    fn test_apply_metadata_to_host() {
        use crate::ide::AnalysisHost;
        use crate::interchange::integrate::apply_metadata_to_host;
        use crate::interchange::metadata::{ElementMeta, ImportMetadata};

        // Create a host with a simple SysML file
        let mut host = AnalysisHost::new();
        let sysml = r#"
package TestPkg {
    part def Car;
}
"#;
        host.set_file_content("/test.sysml", sysml);

        // Create metadata with element IDs
        let mut metadata = ImportMetadata::new();
        metadata.add_element("TestPkg", ElementMeta::with_id("uuid-pkg-1"));
        metadata.add_element("TestPkg::Car", ElementMeta::with_id("uuid-car-1"));

        // Apply metadata to host
        apply_metadata_to_host(&mut host, &metadata);

        // Verify element IDs were applied
        let analysis = host.analysis();

        let pkg = analysis
            .symbol_index()
            .lookup_qualified("TestPkg")
            .expect("Should find TestPkg");
        assert_eq!(
            pkg.element_id.as_ref(),
            "uuid-pkg-1",
            "Package should have metadata element_id"
        );

        let car = analysis
            .symbol_index()
            .lookup_qualified("TestPkg::Car")
            .expect("Should find TestPkg::Car");
        assert_eq!(
            car.element_id.as_ref(),
            "uuid-car-1",
            "Car should have metadata element_id"
        );
    }
}
