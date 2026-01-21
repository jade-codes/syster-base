//! Consolidated tests for syster-base
//!
//! This file consolidates all the individual test files into organized modules.
//! Run with: cargo test --test tests_suite

use pest::Parser;
use std::path::PathBuf;
use syster::parser::sysml::{Rule, SysMLParser};
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::{parse_file, parse_usage};

// ============================================================
// COMMON HELPER FUNCTIONS
// ============================================================

/// Create a workspace from SysML source
fn create_workspace(source: &str) -> Workspace<SyntaxFile> {
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");
    workspace
}

/// Assert that a symbol exists in the workspace
fn assert_symbol_exists(workspace: &Workspace<SyntaxFile>, qname: &str) {
    let exists = workspace
        .symbol_table()
        .iter_symbols()
        .any(|s| s.qualified_name() == qname);
    assert!(exists, "Symbol '{}' should exist", qname);
}

/// Assert that parsing succeeds for a given rule
fn assert_parses(rule: Rule, input: &str) {
    assert!(
        SysMLParser::parse(rule, input).is_ok(),
        "Should parse '{}' with rule {:?}",
        input,
        rule
    );
}

// ============================================================
// PARSER TESTS
// ============================================================

mod parser_tests {
    use super::*;

    #[test]
    fn test_assume_parser() {
        let source = r#"package Test {
    attribute def MassValue;
    attribute assumedCargoMass : MassValue;
    
    requirement def FuelEconomyRequirement {
        attribute requiredFuelEconomy;
    }
    
    requirement highwayFuelEconomyRequirement : FuelEconomyRequirement {
        assume constraint { assumedCargoMass <= 500 }
    }
}"#;
        assert!(
            SysMLParser::parse(Rule::file, source).is_ok(),
            "Should parse assume constraint"
        );
    }

    #[test]
    fn test_bind_parse() {
        let input = "bind engine.fuelCmdPort=fuelCmdPort;";
        assert_parses(Rule::binding_connector_as_usage, input);
    }

    #[test]
    fn test_bracket() {
        let input = r#"attribute :>> position3dVector = (0,0,0) [spatialCF];"#;
        assert_parses(Rule::attribute_usage, input);
    }

    #[test]
    fn test_conn() {
        let source = r#"#derivation connection {
    end #original ::> vehicleSpecification.vehicleMassRequirement;
}"#;
        assert_parses(Rule::connection_usage, source);
    }

    #[test]
    fn test_control_nodes() {
        assert_parses(Rule::fork_node, "fork fork1;");
        assert_parses(Rule::join_node, "join join1;");
        assert_parses(Rule::control_node, "fork fork1;");
        assert_parses(Rule::action_node, "fork fork1;");
    }

    #[test]
    fn test_derivation_parser() {
        let source = r#"package Test {
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;
        assert!(
            SysMLParser::parse(Rule::file, source).is_ok(),
            "Should parse derivation connection"
        );
    }

    #[test]
    fn test_filter_meta() {
        let source = r#"view def PartsTreeView {
    filter @SysML::PartUsage;
}"#;
        let pairs = SysMLParser::parse(Rule::view_definition, source.trim()).unwrap();
        let pair = pairs.into_iter().next().unwrap();
        let def = syster::syntax::sysml::ast::parse_definition(pair).unwrap();
        assert_eq!(def.name, Some("PartsTreeView".to_string()));
    }

    #[test]
    fn test_frame_concern() {
        assert_parses(
            Rule::framed_concern_member,
            "frame concern vs:VehicleSafety;",
        );
        // framed_concern_usage with 'frame' keyword requires going through framed_concern_member
        assert_parses(Rule::concern_usage, "concern vs:VehicleSafety;");
    }

    #[test]
    fn test_join_parse() {
        let input = "join join1;";
        assert_parses(Rule::join_node, input);

        let pair = SysMLParser::parse(Rule::join_node, input)
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert_eq!(usage.name, Some("join1".to_string()));
    }

    #[test]
    fn test_metadata_parser() {
        let source = r#"package ModelingMetadata {
    enum def StatusKind {
        enum open;
        enum closed;
    }
    
    metadata def StatusInfo {
        attribute status : StatusKind;
    }
}

package Test {
    import ModelingMetadata::*;
    
    part myPart {
        @StatusInfo {
            status = StatusKind::closed;
        }
    }
}"#;
        assert!(
            SysMLParser::parse(Rule::root_namespace, source).is_ok(),
            "Should parse metadata"
        );
    }

    #[test]
    fn test_parse_perform() {
        // Short form
        assert_parses(Rule::perform_action_usage, "perform providePower;");
        // Long form
        assert_parses(Rule::perform_action_usage, "perform action providePower;");
    }

    #[test]
    fn test_refinement_parser() {
        let source = r#"package Test {
    part def Engine4Cyl;
    part engine4Cyl : Engine4Cyl;
    
    #refinement dependency engine4Cyl to Target::path::element;
}"#;
        assert!(
            SysMLParser::parse(Rule::file, source).is_ok(),
            "Should parse refinement dependency"
        );
    }

    #[test]
    fn test_return_parse() {
        assert_parses(Rule::return_parameter_member, "return : Real;");
        assert_parses(
            Rule::return_parameter_member,
            "return dpv :> distancePerVolume = 1/f;",
        );
        assert_parses(Rule::parameter_binding, "in bestFuelConsumption: Real;");
    }

    #[test]
    fn test_return_types() {
        let input = r#"calc def BestFuel {
        in mass: MassValue;
        return f_b : Real = bsfc * mass;
    }"#;
        assert_parses(Rule::calculation_definition, input);
    }

    #[test]
    fn test_succession_parse() {
        let input = "first driverGetInVehicle then join1;";
        assert_parses(Rule::succession_as_usage, input);
    }

    #[test]
    fn test_usecase_parse() {
        let source = r#"use case transportPassenger_1:TransportPassenger{
    action driverGetInVehicle subsets getInVehicle_a[1];
    action driveVehicleToDestination;
    action providePower;
    item def VehicleOnSignal;
    join join1;
    first start;
    then fork fork1;
    first join1 then trigger;
}"#;
        assert_parses(Rule::use_case_usage, source);
    }

    #[test]
    fn test_usecase_parsing() {
        // "first start;" is an initial_node_member
        assert_parses(Rule::initial_node_member, "first start;");
        // transition_target is "then X" where X is a connector end
        assert_parses(Rule::transition_target, "then fork1;");
        // action_usage with subsets
        assert_parses(
            Rule::action_usage,
            "action driverGetInVehicle subsets getInVehicle_a[1];",
        );
        // accept node declaration
        assert_parses(
            Rule::accept_node_declaration,
            "accept ignitionCmd:IgnitionCmd",
        );
        // fork and join are control nodes, not action_usage
        assert_parses(Rule::fork_node, "fork fork1;");
        assert_parses(Rule::join_node, "join join1;");
        // succession with first...then
        assert_parses(
            Rule::succession_as_usage,
            "first driverGetInVehicle then join1;",
        );
    }

    #[test]
    fn test_view_filter() {
        let code = r#"
package Test {
    view def PartsTreeView {
        filter @SysML::PartUsage;
    }
}
"#;
        assert!(
            SysMLParser::parse(Rule::file, code).is_ok(),
            "Should parse view filter"
        );
    }
}

// ============================================================
// AST TESTS
// ============================================================

mod ast_tests {
    use super::*;

    #[test]
    fn test_control_ast_fork() {
        let pair = SysMLParser::parse(Rule::fork_node, "fork fork1;")
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert_eq!(usage.name, Some("fork1".to_string()));
    }

    #[test]
    fn test_control_ast_join() {
        let pair = SysMLParser::parse(Rule::join_node, "join join1;")
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert_eq!(usage.name, Some("join1".to_string()));
    }

    #[test]
    fn test_control_ast_merge() {
        let pair = SysMLParser::parse(Rule::merge_node, "merge merge1;")
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert_eq!(usage.name, Some("merge1".to_string()));
    }

    #[test]
    fn test_control_ast_decide() {
        let pair = SysMLParser::parse(Rule::decision_node, "decide decide1;")
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert_eq!(usage.name, Some("decide1".to_string()));
    }

    #[test]
    fn test_derivation_ast() {
        let source = r#"package Test {
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;
        let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
        let file = parse_file(&mut pairs).expect("AST parse should succeed");
        assert!(!file.elements.is_empty(), "Should have AST elements");
    }

    #[test]
    fn test_succession_refs() {
        let input = "first driverGetInVehicle then join1;";
        let pair = SysMLParser::parse(Rule::succession_as_usage, input)
            .unwrap()
            .next()
            .unwrap();
        let usage = parse_usage(pair);
        assert!(
            usage.expression_refs.len() >= 2,
            "Should have at least 2 expression refs"
        );
    }
}

// ============================================================
// SEMANTIC TESTS
// ============================================================

mod semantic_tests {
    use super::*;

    #[test]
    fn test_accept_action() {
        let source = r#"package Test {
    use case transportPassenger_1{
        action trigger accept ignitionCmd:IgnitionCmd;
        first join1 then trigger;
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "Test");
        assert_symbol_exists(&workspace, "Test::transportPassenger_1");
    }

    #[test]
    fn test_assume_semantic() {
        let source = r#"package Test {
    attribute def MassValue;
    attribute assumedCargoMass : MassValue;
    
    requirement def FuelEconomyRequirement {
        attribute requiredFuelEconomy;
    }
    
    requirement highwayFuelEconomyRequirement : FuelEconomyRequirement {
        assume constraint { assumedCargoMass <= 500 }
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "Test");
        assert_symbol_exists(&workspace, "Test::MassValue");
        assert_symbol_exists(&workspace, "Test::assumedCargoMass");
        assert_symbol_exists(&workspace, "Test::FuelEconomyRequirement");
        assert_symbol_exists(&workspace, "Test::highwayFuelEconomyRequirement");
    }

    #[test]
    fn test_dependency_semantic() {
        let source = r#"package VehicleConfiguration_b {
    package PartsTree {
        part vehicle_b {
            part engine;
        }
    }
}

package Test {
    part def Engine4Cyl;
    part engine4Cyl : Engine4Cyl;
    
    #refinement dependency engine4Cyl to VehicleConfiguration_b::PartsTree::vehicle_b::engine;
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(
            &workspace,
            "VehicleConfiguration_b::PartsTree::vehicle_b::engine",
        );
        assert_symbol_exists(&workspace, "Test::engine4Cyl");
    }

    #[test]
    fn test_derivation_semantic() {
        let source = r#"package Test {
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "Test");
    }

    #[test]
    fn test_expression_patterns() {
        let source = r#"package Test {
    attribute def MassValue;
    attribute def PowerValue;
    attribute def CostValue;
    part def Engine;
    
    part baseEngine : Engine {
        attribute mass : MassValue;
        attribute peakHorsePower : PowerValue;
        attribute cost : CostValue;
    }
    
    part derivedEngine :> baseEngine {
        attribute mass redefines mass = 180;
        attribute cost redefines cost = mass * 10;
        attribute simpleAttr = mass;
        attribute chainedAttr = baseEngine.mass;
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "Test::baseEngine::mass");
        assert_symbol_exists(&workspace, "Test::derivedEngine");

        let ref_index = workspace.reference_index();
        assert!(
            !ref_index.targets().is_empty(),
            "Should have reference targets"
        );
    }

    #[test]
    fn test_framed_concern_extraction() {
        let source = r#"
viewpoint def SafetyViewpoint {
    frame concern vs:VehicleSafety;
}
"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "SafetyViewpoint");
    }

    #[test]
    fn test_metadata_semantic() {
        let source = r#"package ModelingMetadata {
    enum def StatusKind {
        enum open;
        enum closed;
    }
    
    metadata def StatusInfo {
        attribute status : StatusKind;
    }
}

package Test {
    import ModelingMetadata::*;
    
    part myPart {
        @StatusInfo {
            status = StatusKind::closed;
        }
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "ModelingMetadata::StatusKind");
        assert_symbol_exists(&workspace, "ModelingMetadata::StatusInfo");
        assert_symbol_exists(&workspace, "Test::myPart");
    }

    #[test]
    fn test_perform_complete() {
        let source = r#"
use case def TransportPassengerDef {
    action a {
        action driverGetInVehicle {
            action unlockDoor_in;
            action openDoor_in;
        }
    }
}

part def Test {
    use case transportPassenger : TransportPassengerDef;
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "TransportPassengerDef");
        assert_symbol_exists(
            &workspace,
            "TransportPassengerDef::a::driverGetInVehicle::unlockDoor_in",
        );
    }

    #[test]
    fn test_perform_extraction() {
        let source = r#"
part def Test {
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;
        let workspace = create_workspace(source);
        let refs = workspace
            .reference_index()
            .get_references_in_file("/test.sysml");
        assert!(
            !refs.is_empty(),
            "Should have references from perform statements"
        );
    }

    #[test]
    fn test_perform_hover() {
        let source = r#"
part def Test {
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;
        let workspace = create_workspace(source);
        let ref_index = workspace.reference_index();

        // Check that references exist in the file
        let refs = ref_index.get_references_in_file("/test.sysml");
        assert!(!refs.is_empty(), "Should have references");
    }

    #[test]
    fn test_refinement_semantic() {
        let source = r#"package VehicleConfiguration_b {
    package PartsTree {
        part vehicle_b {
            part engine;
        }
    }
}

package Test {
    part def Engine4Cyl;
    part engine4Cyl : Engine4Cyl;
    
    #refinement dependency engine4Cyl to VehicleConfiguration_b::PartsTree::vehicle_b::engine;
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "VehicleConfiguration_b");
        assert_symbol_exists(&workspace, "Test");
    }

    #[test]
    fn test_usecase_ast() {
        let source = r#"package Test {
    use case def TransportPassenger;
    
    use case transportPassenger_1:TransportPassenger{
        action driverGetInVehicle subsets getInVehicle_a[1];
        action driveVehicleToDestination;
        action providePower;
        item def VehicleOnSignal;
        join join1;
        first start;
        then fork fork1;
        first join1 then trigger;
    }
}"#;
        let workspace = create_workspace(source);
        assert_symbol_exists(&workspace, "Test::TransportPassenger");
        assert_symbol_exists(&workspace, "Test::transportPassenger_1");
        assert_symbol_exists(&workspace, "Test::transportPassenger_1::join1");
    }
}
