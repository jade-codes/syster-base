//! Symbol extraction tests for the HIR layer.
//!
//! These tests verify that symbols are correctly extracted from SysML source code
//! with the proper kind, qualified name, and metadata.

use crate::helpers::hir_helpers::*;
use crate::helpers::source_fixtures::*;
use crate::helpers::symbol_assertions::*;
use syster::hir::SymbolKind;

// =============================================================================
// PACKAGE EXTRACTION
// =============================================================================

#[test]
fn test_package_symbol_extraction() {
    // Note: Packages with content create symbols; completely empty packages may not
    let (mut host, _) = analysis_from_sysml("package MyPackage { part def Inner; }");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "MyPackage");

    let sym = get_symbol(analysis.symbol_index(), "MyPackage");
    assert_symbol_kind(sym, SymbolKind::Package);
}

#[test]
fn test_nested_package_extraction() {
    let (mut host, _) = analysis_from_sysml(NESTED_PACKAGE);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicles");
    assert_symbol_exists(analysis.symbol_index(), "Vehicles::Vehicle");
    assert_symbol_exists(analysis.symbol_index(), "Vehicles::Car");

    let vehicles = get_symbol(analysis.symbol_index(), "Vehicles");
    assert_symbol_kind(vehicles, SymbolKind::Package);
}

#[test]
fn test_deeply_nested_packages() {
    let (mut host, _) = analysis_from_sysml(DEEPLY_NESTED_PACKAGES);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Level1");
    assert_symbol_exists(analysis.symbol_index(), "Level1::Level2");
    assert_symbol_exists(analysis.symbol_index(), "Level1::Level2::Level3");
    assert_symbol_exists(analysis.symbol_index(), "Level1::Level2::Level3::DeepPart");
}

#[test]
fn test_empty_package() {
    // Note: EMPTY_PACKAGE fixture is "package Empty {}", which should create a symbol
    let (mut host, _) = analysis_from_sysml("package Empty {}");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Empty");
    let sym = get_symbol(analysis.symbol_index(), "Empty");
    assert_symbol_kind(sym, SymbolKind::Package);
}

// =============================================================================
// PART DEFINITION EXTRACTION
// =============================================================================

#[test]
fn test_part_def_extraction() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_PART_DEF);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle");

    let sym = get_symbol(analysis.symbol_index(), "Vehicle");
    assert_symbol_kind(sym, SymbolKind::PartDefinition);
}

#[test]
fn test_multiple_part_defs() {
    let (mut host, _) = analysis_from_sysml(MULTIPLE_DEFINITIONS);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle");
    assert_symbol_exists(analysis.symbol_index(), "Car");
    assert_symbol_exists(analysis.symbol_index(), "Truck");

    // All should be PartDef
    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "Vehicle"),
        SymbolKind::PartDefinition,
    );
    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "Car"),
        SymbolKind::PartDefinition,
    );
    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "Truck"),
        SymbolKind::PartDefinition,
    );
}

#[test]
fn test_part_def_in_package_has_qualified_name() {
    let (mut host, _) = analysis_from_sysml(NESTED_PACKAGE);
    let analysis = host.analysis();

    let vehicle = get_symbol(analysis.symbol_index(), "Vehicles::Vehicle");
    assert_eq!(vehicle.qualified_name.as_ref(), "Vehicles::Vehicle");
    assert_eq!(vehicle.name.as_ref(), "Vehicle");
}

// =============================================================================
// OTHER DEFINITION KINDS
// =============================================================================

#[test]
fn test_port_def_extraction() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_PORT_DEF);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "DataPort");
    let sym = get_symbol(analysis.symbol_index(), "DataPort");
    assert_symbol_kind(sym, SymbolKind::PortDefinition);
}

#[test]
fn test_action_def_extraction() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_ACTION_DEF);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Move");
    let sym = get_symbol(analysis.symbol_index(), "Move");
    assert_symbol_kind(sym, SymbolKind::ActionDefinition);
}

#[test]
fn test_item_def_extraction() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_ITEM_DEF);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Payload");
    let sym = get_symbol(analysis.symbol_index(), "Payload");
    assert_symbol_kind(sym, SymbolKind::ItemDefinition);
}

#[test]
fn test_attribute_def_extraction() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_ATTRIBUTE_DEF);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Mass");
    let sym = get_symbol(analysis.symbol_index(), "Mass");
    assert_symbol_kind(sym, SymbolKind::AttributeDefinition);
}

#[test]
fn test_connection_def_extraction() {
    let (mut host, _) = analysis_from_sysml("connection def Link;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Link");
    let sym = get_symbol(analysis.symbol_index(), "Link");
    assert_symbol_kind(sym, SymbolKind::ConnectionDefinition);
}

#[test]
fn test_interface_def_extraction() {
    let (mut host, _) = analysis_from_sysml("interface def DataInterface;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "DataInterface");
    let sym = get_symbol(analysis.symbol_index(), "DataInterface");
    assert_symbol_kind(sym, SymbolKind::InterfaceDefinition);
}

#[test]
fn test_allocation_def_extraction() {
    let (mut host, _) = analysis_from_sysml("allocation def FunctionToComponent;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "FunctionToComponent");
    let sym = get_symbol(analysis.symbol_index(), "FunctionToComponent");
    assert_symbol_kind(sym, SymbolKind::AllocationDefinition);
}

#[test]
fn test_requirement_def_extraction() {
    let (mut host, _) = analysis_from_sysml("requirement def SafetyReq;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "SafetyReq");
    let sym = get_symbol(analysis.symbol_index(), "SafetyReq");
    assert_symbol_kind(sym, SymbolKind::RequirementDefinition);
}

#[test]
fn test_constraint_def_extraction() {
    let (mut host, _) = analysis_from_sysml("constraint def MassConstraint;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "MassConstraint");
    let sym = get_symbol(analysis.symbol_index(), "MassConstraint");
    assert_symbol_kind(sym, SymbolKind::ConstraintDefinition);
}

#[test]
fn test_state_def_extraction() {
    let (mut host, _) = analysis_from_sysml("state def OperatingState;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "OperatingState");
    let sym = get_symbol(analysis.symbol_index(), "OperatingState");
    assert_symbol_kind(sym, SymbolKind::StateDefinition);
}

#[test]
fn test_calc_def_extraction() {
    let (mut host, _) = analysis_from_sysml("calc def TotalMass;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "TotalMass");
    let sym = get_symbol(analysis.symbol_index(), "TotalMass");
    assert_symbol_kind(sym, SymbolKind::CalculationDefinition);
}

#[test]
fn test_case_def_extraction() {
    let (mut host, _) = analysis_from_sysml("case def DriveScenario;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "DriveScenario");
    let sym = get_symbol(analysis.symbol_index(), "DriveScenario");
    assert_symbol_kind(sym, SymbolKind::UseCaseDefinition);
}

#[test]
fn test_use_case_def_extraction() {
    let (mut host, _) = analysis_from_sysml("use case def StartVehicle;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "StartVehicle");
    let sym = get_symbol(analysis.symbol_index(), "StartVehicle");
    assert_symbol_kind(sym, SymbolKind::UseCaseDefinition);
}

#[test]
fn test_analysis_case_def_extraction() {
    let (mut host, _) = analysis_from_sysml("analysis def ThermalAnalysis;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "ThermalAnalysis");
    let sym = get_symbol(analysis.symbol_index(), "ThermalAnalysis");
    assert_symbol_kind(sym, SymbolKind::AnalysisCaseDefinition);
}

#[test]
fn test_view_def_extraction() {
    let (mut host, _) = analysis_from_sysml("view def SystemDiagram;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "SystemDiagram");
    let sym = get_symbol(analysis.symbol_index(), "SystemDiagram");
    assert_symbol_kind(sym, SymbolKind::ViewDefinition);
}

#[test]
fn test_viewpoint_def_extraction() {
    let (mut host, _) = analysis_from_sysml("viewpoint def ArchitectViewpoint;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "ArchitectViewpoint");
    let sym = get_symbol(analysis.symbol_index(), "ArchitectViewpoint");
    assert_symbol_kind(sym, SymbolKind::ViewpointDefinition);
}

#[test]
fn test_rendering_def_extraction() {
    let (mut host, _) = analysis_from_sysml("rendering def BoxRendering;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "BoxRendering");
    let sym = get_symbol(analysis.symbol_index(), "BoxRendering");
    assert_symbol_kind(sym, SymbolKind::RenderingDefinition);
}

#[test]
fn test_enumeration_def_extraction() {
    let (mut host, _) = analysis_from_sysml("enum def Color;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Color");
    let sym = get_symbol(analysis.symbol_index(), "Color");
    assert_symbol_kind(sym, SymbolKind::EnumerationDefinition);
}

#[test]
fn test_verification_case_def_extraction() {
    // VerificationCase maps to AnalysisCaseDef in SymbolKind
    let source = r#"
        package VerificationPkg {
            verification def TestVehicle;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "VerificationPkg::TestVehicle");
    let sym = get_symbol(analysis.symbol_index(), "VerificationPkg::TestVehicle");
    assert_symbol_kind(sym, SymbolKind::AnalysisCaseDefinition);
}

#[test]
fn test_metadata_def_extraction() {
    // Metadata definitions currently fall through to SymbolKind::Other
    // (no dedicated MetadataDef variant yet)
    let source = r#"
        package MetaPkg {
            metadata def Safety;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "MetaPkg::Safety");
    let sym = get_symbol(analysis.symbol_index(), "MetaPkg::Safety");
    // Currently maps to Other, could add MetadataDef variant in the future
    assert_symbol_kind(sym, SymbolKind::Other);
}

// =============================================================================
// USAGE EXTRACTION
// =============================================================================

#[test]
fn test_part_usage_extraction() {
    let source = r#"
        part def Vehicle {
            part engine;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle::engine");

    let engine = get_symbol(analysis.symbol_index(), "Vehicle::engine");
    assert_symbol_kind(engine, SymbolKind::PartUsage);
}

#[test]
fn test_typed_part_usage() {
    let (mut host, _) = analysis_from_sysml(TYPED_USAGE);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle");
    assert_symbol_exists(analysis.symbol_index(), "myCar");

    let my_car = get_symbol(analysis.symbol_index(), "myCar");
    assert_symbol_kind(my_car, SymbolKind::PartUsage);
    // Type reference should exist
    assert!(!my_car.type_refs.is_empty(), "myCar should have type refs");
}

#[test]
fn test_nested_usages_have_qualified_names() {
    let (mut host, _) = analysis_from_sysml(PART_WITH_USAGES);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle::engine");
    assert_symbol_exists(analysis.symbol_index(), "Vehicle::wheels");
    assert_symbol_exists(analysis.symbol_index(), "Vehicle::mass");

    let engine = get_symbol(analysis.symbol_index(), "Vehicle::engine");
    assert_eq!(engine.qualified_name.as_ref(), "Vehicle::engine");
}

#[test]
fn test_attribute_usage_extraction() {
    let source = r#"
        part def Container {
            attribute weight;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Container::weight");

    let weight = get_symbol(analysis.symbol_index(), "Container::weight");
    assert_symbol_kind(weight, SymbolKind::AttributeUsage);
}

#[test]
fn test_port_usage_extraction() {
    let source = r#"
        part def System {
            port dataIn;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "System::dataIn");

    let port = get_symbol(analysis.symbol_index(), "System::dataIn");
    assert_symbol_kind(port, SymbolKind::PortUsage);
}

#[test]
fn test_action_usage_extraction() {
    let source = r#"
        part def Controller {
            action process;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Controller::process");

    let action = get_symbol(analysis.symbol_index(), "Controller::process");
    assert_symbol_kind(action, SymbolKind::ActionUsage);
}

#[test]
fn test_item_usage_extraction() {
    let source = r#"
        part def Container {
            item payload;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Container::payload");

    let item = get_symbol(analysis.symbol_index(), "Container::payload");
    assert_symbol_kind(item, SymbolKind::ItemUsage);
}

#[test]
fn test_ref_usage_extraction() {
    let source = r#"
        part def System {
            ref target;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "System::target");

    let ref_usage = get_symbol(analysis.symbol_index(), "System::target");
    assert_symbol_kind(ref_usage, SymbolKind::ReferenceUsage);
}

// =============================================================================
// SPECIALIZATION EXTRACTION
// =============================================================================

#[test]
fn test_specialization_relationship() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_SPECIALIZATION);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Vehicle");
    assert_symbol_exists(analysis.symbol_index(), "Car");

    let car = get_symbol(analysis.symbol_index(), "Car");
    assert_specializes(car, "Vehicle");
}

#[test]
fn test_specialization_chain() {
    let (mut host, _) = analysis_from_sysml(SPECIALIZATION_CHAIN);
    let analysis = host.analysis();

    let car = get_symbol(analysis.symbol_index(), "Car");
    assert_specializes(car, "Vehicle");

    let sports_car = get_symbol(analysis.symbol_index(), "SportsCar");
    assert_specializes(sports_car, "Car");
}

// =============================================================================
// DUPLICATE DETECTION
// =============================================================================

#[test]
fn test_no_duplicate_symbols_in_package() {
    let (mut host, file_id) = analysis_from_sysml(NESTED_PACKAGE);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis
        .symbol_index()
        .symbols_in_file(file_id)
        .into_iter()
        .cloned()
        .collect();
    assert_no_duplicate_symbols(&symbols);
}

#[test]
fn test_same_name_different_namespaces_are_separate() {
    let source = r#"
        package Namespace1 {
            part def Shell;
        }
        package Namespace2 {
            part def Shell;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Both should exist with different qualified names
    assert_symbol_exists(analysis.symbol_index(), "Namespace1::Shell");
    assert_symbol_exists(analysis.symbol_index(), "Namespace2::Shell");

    // Should be two different symbols named "Shell"
    let shells = symbols_named(analysis.symbol_index(), "Shell");
    assert_eq!(
        shells.len(),
        2,
        "Should have two Shell symbols in different namespaces"
    );
}

#[test]
fn test_redefinition_does_not_create_duplicate() {
    let source = r#"
        package TestPkg {
            item def Shell {
                item edges;
            }
            item def Disc :> Shell {
                item :>> edges;
            }
        }
    "#;
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis
        .symbol_index()
        .symbols_in_file(file_id)
        .into_iter()
        .cloned()
        .collect();
    assert_no_duplicate_symbols(&symbols);

    // Shell should exist exactly once
    let shells = symbols_named(analysis.symbol_index(), "Shell");
    assert_eq!(shells.len(), 1, "Shell should be defined exactly once");
}

// =============================================================================
// VISIBILITY/PUBLIC EXTRACTION
// =============================================================================

#[test]
fn test_top_level_definitions_exist() {
    // Test that top-level definitions are extracted (visibility may vary)
    let (mut host, _) = analysis_from_sysml("part def PublicPart;");
    let analysis = host.analysis();

    let sym = get_symbol(analysis.symbol_index(), "PublicPart");
    assert_symbol_kind(sym, SymbolKind::PartDefinition);
}

// =============================================================================
// SPAN TRACKING
// =============================================================================

#[test]
fn test_symbol_has_span() {
    let (mut host, _) = analysis_from_sysml("part def Vehicle;");
    let analysis = host.analysis();

    let sym = get_symbol(analysis.symbol_index(), "Vehicle");
    // At minimum, the symbol should have position info
    // (exact values depend on implementation)
    assert_has_span(sym);
}

// =============================================================================
// ANONYMOUS USAGE TESTS
// =============================================================================

#[test]
fn test_anonymous_usage_no_name() {
    // Anonymous usages use `: Type` syntax without providing a name
    let source = r#"
        package TestPkg {
            part def Engine;
            part def Vehicle {
                : Engine;
            }
        }
    "#;
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // The named definitions should exist
    assert_symbol_exists(analysis.symbol_index(), "TestPkg::Engine");
    assert_symbol_exists(analysis.symbol_index(), "TestPkg::Vehicle");

    // Anonymous usages may or may not be tracked as symbols
    // (they have no name to reference by)
    let symbols: Vec<_> = analysis
        .symbol_index()
        .symbols_in_file(file_id)
        .into_iter()
        .collect();
    // Count should be at least 3 (TestPkg, Engine, Vehicle)
    assert!(
        symbols.len() >= 3,
        "Should have at least package + 2 definitions"
    );
}

#[test]
fn test_anonymous_attribute_usage() {
    let source = r#"
        package TestPkg {
            attribute def Color;
            part def Panel {
                attribute : Color;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "TestPkg::Color");
    assert_symbol_exists(analysis.symbol_index(), "TestPkg::Panel");
}

// =============================================================================
// GENERATED/STRESS TESTS
// =============================================================================

#[test]
fn test_many_part_definitions() {
    let source = package_with_n_parts(50);
    let (mut host, file_id) = analysis_from_sysml(&source);
    let analysis = host.analysis();

    // Should have 50 parts + 1 package = 51 symbols
    let symbol_count = analysis.symbol_index().symbols_in_file(file_id).len();
    assert!(
        symbol_count >= 50,
        "Expected at least 50 symbols, got {}",
        symbol_count
    );

    // Verify a few exist
    assert_symbol_exists(analysis.symbol_index(), "Generated::Part0");
    assert_symbol_exists(analysis.symbol_index(), "Generated::Part49");
}

#[test]
fn test_deeply_nested_packages_10_levels() {
    let source = nested_packages(10);
    let (mut host, _) = analysis_from_sysml(&source);
    let analysis = host.analysis();

    // Should be able to find the deepest symbol
    let deep_name =
        "Level1::Level2::Level3::Level4::Level5::Level6::Level7::Level8::Level9::Level10::DeepPart";
    assert_symbol_exists(analysis.symbol_index(), deep_name);
}
