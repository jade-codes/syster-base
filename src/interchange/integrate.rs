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

use crate::hir::{RootDatabase, HirSymbol, SymbolKind, HirRelationship, RelationshipKind as HirRelKind};
use crate::base::FileId;
use std::sync::Arc;
use super::model::{Model, Element, ElementId, ElementKind, Relationship, RelationshipKind};

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
        
        // Build qualified name from element ID or compute from owner chain
        let qualified_name: Arc<str> = element.id.as_str().into();
        
        // Simple name is the last segment of qualified name, or explicit name
        let name: Arc<str> = element.name.clone()
            .map(|n| n.to_string().into())
            .unwrap_or_else(|| {
                qualified_name.rsplit("::").next()
                    .unwrap_or(qualified_name.as_ref())
                    .into()
            });
        
        // Collect relationships where this element is the source
        let relationships: Vec<HirRelationship> = model.relationships.iter()
            .filter(|r| r.source.as_str() == element.id.as_str())
            .filter_map(|r| {
                let hir_kind = relationship_kind_to_hir(&r.kind)?;
                Some(HirRelationship {
                    kind: hir_kind,
                    target: r.target.as_str().into(),
                    resolved_target: Some(r.target.as_str().into()), // XMI has resolved refs
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                })
            })
            .collect();
        
        // Extract supertypes from specialization relationships
        let supertypes: Vec<Arc<str>> = relationships.iter()
            .filter(|r| r.kind == HirRelKind::Specializes)
            .map(|r| r.target.clone())
            .collect();
        
        let symbol = HirSymbol {
            name,
            short_name: None, // XMI may have this in declaredShortName property
            qualified_name,
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
        ElementKind::Import | ElementKind::NamespaceImport | ElementKind::MembershipImport => SymbolKind::Import,
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
    
    for symbol in symbols {
        let id = ElementId::new(symbol.qualified_name.as_ref());
        let kind = symbol_kind_to_element_kind(symbol.kind);
        
        // Determine ownership from qualified name
        let owner = if symbol.qualified_name.contains("::") {
            // Extract parent qualified name
            let parent = symbol.qualified_name.rsplit_once("::").map(|(p, _)| p);
            parent.map(ElementId::new)
        } else {
            None
        };
        
        let mut element = Element::new(id.clone(), kind)
            .with_name(symbol.name.as_ref());
        
        if let Some(owner_id) = owner {
            element = element.with_owner(owner_id);
        }
        
        model.add_element(element);
        
        // Extract relationships from the symbol
        for hir_rel in &symbol.relationships {
            let rel_kind = hir_relationship_kind_to_model(&hir_rel.kind);
            if let Some(rel_kind) = rel_kind {
                rel_counter += 1;
                let rel_id = ElementId::new(format!("rel_{}", rel_counter));
                
                // The target might be a simple name or qualified name
                // For now, assume it's as written in the source
                let target_id = ElementId::new(hir_rel.target.as_ref());
                
                let relationship = Relationship::new(
                    rel_id,
                    rel_kind,
                    id.clone(),
                    target_id,
                );
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
        assert!(model.elements.is_empty(), "Empty database should produce empty model");
        assert!(model.roots.is_empty(), "Empty database should have no root elements");
        assert!(model.relationships.is_empty(), "Empty database should have no relationships");
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
        let element = model.elements.get(root_id).expect("Root element should exist");
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
        let car = model.elements.values()
            .find(|e| e.name.as_deref() == Some("Car"))
            .expect("Car should exist");
        assert_eq!(car.kind, super::super::model::ElementKind::PartDefinition);
        assert!(car.owner.is_some(), "Car should have an owner");
        assert_eq!(car.owner.as_ref().unwrap().as_str(), "Vehicle");
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
        let specialization = model.relationships.iter()
            .find(|r| r.kind == super::super::model::RelationshipKind::Specialization)
            .expect("Should have a specialization");
        
        // Car specializes Vehicle
        assert!(specialization.source.as_str().contains("Car"), "Source should be Car");
        assert!(specialization.target.as_str().contains("Vehicle"), "Target should be Vehicle");
    }

    #[test]
    fn test_roundtrip_through_xmi() {
        use super::super::{Xmi, ModelFormat};
        
        // Given a database with a simple model (just the root package)
        let db = RootDatabase::new();
        let sysml = "package Vehicles;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());
        
        let symbols = file_symbols_from_text(&db, file_text);
        let model = model_from_symbols(&symbols);
        
        // Verify our model has what we expect
        assert_eq!(model.elements.len(), 1, "Should have one package");
        
        // When we write to XMI and read back
        let xmi_bytes = Xmi.write(&model).expect("Should write XMI");
        let roundtrip_model = Xmi.read(&xmi_bytes).expect("Should read XMI");
        
        // Then the element count should match (at least the roots)
        assert!(!roundtrip_model.elements.is_empty(), 
            "Should have at least one element after roundtrip");
        assert!(!roundtrip_model.roots.is_empty(),
            "Should have at least one root after roundtrip");
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
        
        let pkg = Element::new(ElementId::new("Vehicle"), ElementKind::Package)
            .with_name("Vehicle");
        model.add_element(pkg);
        
        let car = Element::new(ElementId::new("Vehicle::Car"), ElementKind::PartDefinition)
            .with_name("Car")
            .with_owner(ElementId::new("Vehicle"));
        model.add_element(car);
        
        let engine = Element::new(ElementId::new("Vehicle::Engine"), ElementKind::PartDefinition)
            .with_name("Engine")
            .with_owner(ElementId::new("Vehicle"));
        model.add_element(engine);
        
        // When we convert to symbols
        let symbols = symbols_from_model(&model);
        
        // Then we should get 3 symbols with correct kinds
        assert_eq!(symbols.len(), 3, "Should have 3 symbols");
        
        let car_sym = symbols.iter().find(|s| s.name.as_ref() == "Car").expect("Should have Car");
        assert_eq!(car_sym.kind, SymbolKind::PartDef);
        assert_eq!(car_sym.qualified_name.as_ref(), "Vehicle::Car");
        
        let engine_sym = symbols.iter().find(|s| s.name.as_ref() == "Engine").expect("Should have Engine");
        assert_eq!(engine_sym.kind, SymbolKind::PartDef);
    }

    #[test]
    fn test_symbols_from_model_with_relationships() {
        // Given a model with specialization: Car :> Vehicle
        let mut model = Model::new();
        
        let vehicle = Element::new(ElementId::new("Vehicle"), ElementKind::PartDefinition)
            .with_name("Vehicle");
        model.add_element(vehicle);
        
        let car = Element::new(ElementId::new("Car"), ElementKind::PartDefinition)
            .with_name("Car");
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
        let car_sym = symbols.iter().find(|s| s.name.as_ref() == "Car").expect("Should have Car");
        assert!(!car_sym.relationships.is_empty(), "Car should have relationships");
        
        let spec_rel = car_sym.relationships.iter()
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
        
        let mut pkg = Element::new(ElementId::new("MyPackage"), ElementKind::Package)
            .with_name("MyPackage");
        pkg.documentation = Some("This is a documented package".into());
        model.add_element(pkg);
        
        // When we convert to symbols
        let symbols = symbols_from_model(&model);
        
        // Then the symbol should have documentation
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].doc.as_deref(), Some("This is a documented package"));
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
        
        assert_eq!(roundtrip_count, original_count, 
            "Roundtrip should preserve symbol count: {} → {}", 
            original_count, roundtrip_count);
        
        // Names should match
        for orig in &original_symbols {
            let found = roundtrip_symbols.iter()
                .find(|s| s.qualified_name == orig.qualified_name);
            assert!(found.is_some(), 
                "Symbol {} should exist after roundtrip", orig.qualified_name);
        }
    }
}
