//! Symbol extraction tests for the HIR layer.
//!
//! These tests verify that symbols are correctly extracted from SysML source code
//! with the proper kind, qualified name, and metadata.

use crate::helpers::hir_helpers::*;
use crate::helpers::source_fixtures::*;
use crate::helpers::symbol_assertions::*;
use syster::hir::{RelationshipKind, SymbolKind};

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
fn test_special_keyword_shorthand_usage_kinds_do_not_fall_back_to_reference_usage() {
    let source = r#"
        package Test {
            requirement def satisfiedReq;
            requirement def verifiedReq;
            constraint def assertedConstraint;
            constraint def assumedConstraint;
            constraint def requiredConstraint;
            state def shownState;

            part host {
                satisfy satisfiedReq;
                verify verifiedReq;
                exhibit shownState;
                assert assertedConstraint;
                assume assumedConstraint;
                require requiredConstraint;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let syms = analysis.symbol_index().all_symbols().collect::<Vec<_>>();

    let satisfy_requirement_usages = syms
        .iter()
        .filter(|s| {
            s.qualified_name.starts_with("Test::host::")
                && s.kind == SymbolKind::SatisfyRequirementUsage
        })
        .count();
    assert_eq!(
        satisfy_requirement_usages, 1,
        "host should contain satisfy shorthand as a SatisfyRequirementUsage symbol"
    );

    let verify_requirement_usages = syms
        .iter()
        .filter(|s| {
            s.qualified_name.starts_with("Test::host::") && s.kind == SymbolKind::RequirementUsage
        })
        .count();
    assert_eq!(
        verify_requirement_usages, 1,
        "host should contain verify shorthand as a RequirementUsage symbol"
    );

    let exhibit_state_usages = syms
        .iter()
        .filter(|s| {
            s.qualified_name.starts_with("Test::host::")
                && s.kind == SymbolKind::ExhibitStateUsage
        })
        .count();
    assert_eq!(
        exhibit_state_usages, 1,
        "host should contain exhibit shorthand as an ExhibitStateUsage symbol"
    );

    let assert_constraint_usages = syms
        .iter()
        .filter(|s| {
            s.qualified_name.starts_with("Test::host::")
                && s.kind == SymbolKind::AssertConstraintUsage
        })
        .count();
    assert_eq!(
        assert_constraint_usages, 1,
        "host should contain assert shorthand as an AssertConstraintUsage symbol"
    );

    let constraint_usages = syms
        .iter()
        .filter(|s| {
            s.qualified_name.starts_with("Test::host::") && s.kind == SymbolKind::ConstraintUsage
        })
        .count();
    assert_eq!(
        constraint_usages, 2,
        "host should contain assume/require as ConstraintUsage symbols"
    );
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
fn test_occurrence_def_extraction() {
    let (mut host, _) = analysis_from_sysml("occurrence def Lifetime;");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Lifetime");
    let sym = get_symbol(analysis.symbol_index(), "Lifetime");
    assert_symbol_kind(sym, SymbolKind::OccurrenceDefinition);
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
    let source = r#"
        package VerificationPkg {
            verification def TestVehicle;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "VerificationPkg::TestVehicle");
    let sym = get_symbol(analysis.symbol_index(), "VerificationPkg::TestVehicle");
    assert_symbol_kind(sym, SymbolKind::VerificationCaseDefinition);
}

#[test]
fn test_verification_case_def_uses_verification_case_implicit_supertype() {
    let source = r#"
        package VerificationPkg {
            verification def TestVehicle;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let sym = get_symbol(analysis.symbol_index(), "VerificationPkg::TestVehicle");
    assert_symbol_kind(sym, SymbolKind::VerificationCaseDefinition);
    assert!(
        sym.supertypes
            .iter()
            .any(|s| s.as_ref() == "VerificationCases::VerificationCase"),
        "verification defs should implicitly inherit VerificationCases::VerificationCase, got {:?}",
        sym.supertypes
    );
    assert!(
        !sym.supertypes
            .iter()
            .any(|s| s.as_ref() == "AnalysisCases::AnalysisCase"),
        "verification defs should no longer inherit AnalysisCases::AnalysisCase, got {:?}",
        sym.supertypes
    );
}

#[test]
fn test_case_usage_extraction_uses_distinct_case_usage_kinds() {
    let source = r#"
        package CasePkg {
            use case driveVehicle;
            analysis thermalStudy;
            verification safetyCheck;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "CasePkg::driveVehicle");
    assert_symbol_exists(analysis.symbol_index(), "CasePkg::thermalStudy");
    assert_symbol_exists(analysis.symbol_index(), "CasePkg::safetyCheck");

    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "CasePkg::driveVehicle"),
        SymbolKind::UseCaseUsage,
    );
    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "CasePkg::thermalStudy"),
        SymbolKind::AnalysisCaseUsage,
    );
    assert_symbol_kind(
        get_symbol(analysis.symbol_index(), "CasePkg::safetyCheck"),
        SymbolKind::VerificationCaseUsage,
    );
}

#[test]
fn test_include_reference_form_extracts_use_case_usage_and_includes_relationship() {
    let source = r#"
        package IncludePkg {
            use case included;

            use case host {
                include included;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let include_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("IncludePkg::host::<include:included")
        })
        .expect("anonymous include usage should exist");

    assert_symbol_kind(include_usage, SymbolKind::IncludeUseCaseUsage);
    assert_has_relationship(include_usage, RelationshipKind::Includes, "included");
}

#[test]
fn test_exhibit_reference_form_extracts_state_usage_and_exhibits_relationship() {
    let source = r#"
        package ExhibitPkg {
            state def shown;

            part host {
                exhibit shown;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let exhibit_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("ExhibitPkg::host::<exhibit:shown")
        })
        .expect("anonymous exhibit usage should exist");

    assert_symbol_kind(exhibit_usage, SymbolKind::ExhibitStateUsage);
    assert_has_relationship(exhibit_usage, RelationshipKind::Exhibits, "shown");
}

#[test]
fn test_assert_reference_form_extracts_constraint_usage_and_asserts_relationship() {
    let source = r#"
        package AssertPkg {
            constraint def checked;

            part host {
                assert checked;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let assert_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("AssertPkg::host::<assert:checked")
        })
        .expect("anonymous assert usage should exist");

    assert_symbol_kind(assert_usage, SymbolKind::AssertConstraintUsage);
    assert_has_relationship(assert_usage, RelationshipKind::Asserts, "checked");
}

#[test]
fn test_assume_and_require_reference_forms_extract_constraint_usage_and_relationships() {
    let source = r#"
        package ConstraintPkg {
            constraint def assumed;
            constraint def required;

            part host {
                assume assumed;
                require required;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let assume_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("ConstraintPkg::host::<assume:assumed")
        })
        .expect("anonymous assume usage should exist");
    assert_symbol_kind(assume_usage, SymbolKind::ConstraintUsage);
    assert_has_relationship(assume_usage, RelationshipKind::Assumes, "assumed");

    let require_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("ConstraintPkg::host::<require:required")
        })
        .expect("anonymous require usage should exist");
    assert_symbol_kind(require_usage, SymbolKind::ConstraintUsage);
    assert_has_relationship(require_usage, RelationshipKind::Requires, "required");
}

#[test]
fn test_satisfy_and_verify_reference_forms_extract_requirement_usage_and_relationships() {
    let source = r#"
        package RequirementPkg {
            part verifier;

            part checks {
                requirement required;
                requirement verified;
            }

            part host {
                satisfy checks.required by verifier;
                verify checks.verified;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let satisfy_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("RequirementPkg::host::<satisfy:checks.required")
        })
        .expect("anonymous satisfy usage should exist");
    assert_symbol_kind(satisfy_usage, SymbolKind::SatisfyRequirementUsage);
    assert_has_relationship(satisfy_usage, RelationshipKind::Satisfies, "checks.required");

    let verify_usage = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| {
            s.qualified_name
                .starts_with("RequirementPkg::host::<verify:checks.verified")
        })
        .expect("anonymous verify usage should exist");
    assert_symbol_kind(verify_usage, SymbolKind::RequirementUsage);
    assert_has_relationship(verify_usage, RelationshipKind::Verifies, "checks.verified");
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
            ref part target;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "System::target");

    let ref_usage = get_symbol(analysis.symbol_index(), "System::target");
    assert_symbol_kind(ref_usage, SymbolKind::PartUsage);
    assert_eq!(ref_usage.is_composite, Some(false));
}

#[test]
fn test_bare_ref_usage_defaults_to_reference_usage() {
    let source = r#"
        package sample {
            part def A {
                ref b;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ref_usage = get_symbol(analysis.symbol_index(), "sample::A::b");
    assert_symbol_kind(ref_usage, SymbolKind::ReferenceUsage);
    assert_eq!(ref_usage.is_composite, Some(false));
}

#[test]
fn test_definition_symbols_do_not_get_composite_semantics() {
    let source = r#"
        package sample {
            part def A {
                part def B;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let nested_def = get_symbol(analysis.symbol_index(), "sample::A::B");
    assert_symbol_kind(nested_def, SymbolKind::PartDefinition);
    assert_eq!(nested_def.is_composite, None);
}

#[test]
fn test_usage_modifier_and_composite_semantics_extraction() {
    let source = r#"
        package Root {
            composite part assembly;
            part def Vehicle {
                composite part wheel;
                part axle;
                ref part borrowed;
                port p {
                    part nested;
                }
                attribute values[*] nonunique;
            }
            action def Control {
                in port inputPort;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let assembly = get_symbol(analysis.symbol_index(), "Root::assembly");
    assert_symbol_kind(assembly, SymbolKind::PartUsage);
    assert_eq!(assembly.is_composite, Some(false));

    let wheel = get_symbol(analysis.symbol_index(), "Root::Vehicle::wheel");
    assert_symbol_kind(wheel, SymbolKind::PartUsage);
    assert_eq!(wheel.is_composite, Some(true));

    let axle = get_symbol(analysis.symbol_index(), "Root::Vehicle::axle");
    assert_symbol_kind(axle, SymbolKind::PartUsage);
    assert_eq!(axle.is_composite, Some(true));

    let borrowed = get_symbol(analysis.symbol_index(), "Root::Vehicle::borrowed");
    assert_symbol_kind(borrowed, SymbolKind::PartUsage);
    assert_eq!(borrowed.is_composite, Some(false));

    let nested = get_symbol(analysis.symbol_index(), "Root::Vehicle::p::nested");
    assert_symbol_kind(nested, SymbolKind::PartUsage);
    assert_eq!(nested.is_composite, Some(false));

    let values = get_symbol(analysis.symbol_index(), "Root::Vehicle::values");
    assert_symbol_kind(values, SymbolKind::AttributeUsage);
    assert!(values.is_nonunique);
    assert_eq!(values.is_composite, Some(false));

    let input_port = get_symbol(analysis.symbol_index(), "Root::Control::inputPort");
    assert_symbol_kind(input_port, SymbolKind::PortUsage);
    assert_eq!(input_port.direction, Some(syster::parser::Direction::In));
    assert_eq!(input_port.is_composite, Some(false));
}

#[test]
fn test_end_usage_does_not_become_composite() {
    let source = r#"
        interface def WaterDelivery {
            end port supplied;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let supplied = get_symbol(analysis.symbol_index(), "WaterDelivery::supplied");
    assert_symbol_kind(supplied, SymbolKind::PortUsage);
    assert!(supplied.is_end);
    assert_eq!(supplied.is_composite, Some(false));
}

#[test]
fn test_port_owned_by_part_is_not_composite() {
    let source = r#"
        part def Vehicle {
            port p;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let port = get_symbol(analysis.symbol_index(), "Vehicle::p");
    assert_symbol_kind(port, SymbolKind::PortUsage);
    assert_eq!(port.is_composite, Some(false));
}

#[test]
fn test_port_owned_by_port_is_composite() {
    let source = r#"
        port def Channel {
            port nested;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let port = get_symbol(analysis.symbol_index(), "Channel::nested");
    assert_symbol_kind(port, SymbolKind::PortUsage);
    assert_eq!(port.is_composite, Some(true));
}

#[test]
fn test_state_owned_by_part_is_composite() {
    let source = r#"
        part def Vehicle {
            state idle;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let state = get_symbol(analysis.symbol_index(), "Vehicle::idle");
    assert_symbol_kind(state, SymbolKind::StateUsage);
    assert_eq!(state.is_composite, Some(true));
}

#[test]
fn test_occurrence_and_part_owned_by_occurrence_are_composite() {
    let source = r#"
        occurrence def Lifetime {
            occurrence phase;
            part artifact;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let phase = get_symbol(analysis.symbol_index(), "Lifetime::phase");
    assert_symbol_kind(phase, SymbolKind::OccurrenceUsage);
    assert_eq!(phase.is_composite, Some(true));

    let artifact = get_symbol(analysis.symbol_index(), "Lifetime::artifact");
    assert_symbol_kind(artifact, SymbolKind::PartUsage);
    assert_eq!(artifact.is_composite, Some(true));
}

#[test]
fn test_calculation_owned_by_calculation_is_composite() {
    let source = r#"
        calc def Total {
            calc subtotal;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let calc = get_symbol(analysis.symbol_index(), "Total::subtotal");
    assert_symbol_kind(calc, SymbolKind::CalculationUsage);
    assert_eq!(calc.is_composite, Some(true));
}

#[test]
fn test_transition_owned_by_state_is_composite() {
    let source = r#"
        state def VehicleState {
            state off;
            state on;
            transition off_to_on first off then on;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let transition = get_symbol(analysis.symbol_index(), "VehicleState::off_to_on");
    assert_symbol_kind(transition, SymbolKind::TransitionUsage);
    assert_eq!(transition.is_composite, Some(true));
}

#[test]
fn test_perform_action_usage_is_not_composite() {
    let source = r#"
        part def Sys {
            perform Start;
        }
        action def Start;
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let performed = analysis
        .symbol_index()
        .all_symbols()
        .find(|symbol| {
            symbol
                .qualified_name
                .as_ref()
                .starts_with("Sys::<perform:Start")
        })
        .expect("expected perform action usage");
    assert_symbol_kind(performed, SymbolKind::PerformActionUsage);
    assert_eq!(performed.is_composite, Some(false));
}

#[test]
fn test_connection_usage_is_not_composite() {
    let source = r#"
        connection def C;
        part def Sys {
            connection link : C;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let link = get_symbol(analysis.symbol_index(), "Sys::link");
    assert_symbol_kind(link, SymbolKind::ConnectionUsage);
    assert_eq!(link.is_composite, Some(false));
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
// VIEW DEF BODY (issue #21 regression tests)
// =============================================================================

/// Regression: `ref action X::Y` in view def body — qualified member name must be preserved.
/// Previously `has_chain` in usage.rs only detected `.`, so `MyAction::a` was parsed as
/// name=`MyAction` (the qualifier), discarding `::a`. Fixed by extending `has_chain` to also
/// detect `COLON_COLON`.
#[test]
fn test_ref_action_qualified_name_in_view_body() {
    let source = r#"
action def MyAction {
    action a;
}
view def MyView {
    expose MyAction;
    ref action MyAction::a;
}
"#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let sym = get_symbol(analysis.symbol_index(), "MyView::a");
    assert_symbol_kind(sym, SymbolKind::ReferenceUsage);
}

/// Regression: `edge X::A to X::B` in view def body must be scoped to the view, not escape
/// to compilation-unit level, and must carry a SuccessionUsage kind.
/// Previously `edge` (a contextual IDENT keyword) was dispatched to `parse_shorthand_feature_member`,
/// which consumed only `edge` as the name and left `X::A to X::B` as garbage. Fixed by
/// adding `is_edge_member_start` lookahead in entry.rs and `parse_edge_succession` in connectors.rs.
#[test]
fn test_edge_succession_in_view_body() {
    let source = r#"
action def MyAction {
    action b;
    action c;
}
view def MyView {
    expose MyAction;
    edge MyAction::b to MyAction::c;
}
"#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let index = analysis.symbol_index();
    let succession = index
        .all_symbols()
        .find(|s| s.qualified_name.starts_with("MyView::") && s.kind == SymbolKind::SuccessionUsage);
    assert!(
        succession.is_some(),
        "Expected a SuccessionUsage scoped to MyView"
    );
}

/// Regression: `@MetadataDef {{ :>> attr = EnumType::member; }}` annotation preceding
/// `ref action X::Y` must appear in the HIR and not drop the annotated element.
/// The root cause was Bug 1 (ref action qualified name): the misparse left `::a;` tokens
/// that error recovery consumed along with the annotation body's closing brace, silently
/// dropping both elements from the view scope.
#[test]
fn test_annotation_with_qualified_enum_value_in_view_body() {
    let source = r#"
enum def NodeKind { branch; parallel; }
metadata def NodeMeta { attribute nodeKind : NodeKind; }
action def MyAction { action a; }
view def MyView {
    expose MyAction;
    @NodeMeta { :>> nodeKind = NodeKind::branch; }
    ref action MyAction::a;
}
"#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let index = analysis.symbol_index();

    // The annotation must appear as a child of MyView
    let annotation = index
        .all_symbols()
        .find(|s| s.qualified_name.starts_with("MyView::") && s.qualified_name.contains("NodeMeta"));
    assert!(
        annotation.is_some(),
        "Annotation @NodeMeta should appear as a child of MyView"
    );

    // The ref action must also appear, not be swallowed by annotation error recovery
    let sym = get_symbol(index, "MyView::a");
    assert_symbol_kind(sym, SymbolKind::ReferenceUsage);
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
