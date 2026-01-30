//! Migrated tests from the old Pest parser tests.
//! 
//! These tests verify that the rowan parser correctly handles the same 
//! SysML constructs that were tested with the Pest parser.

use super::ast::{AstNode, Definition, SourceFile, SpecializationKind, Usage};
use super::parser::{parse, parse_sysml};

// ============================================================================
// Helper functions
// ============================================================================

/// Parse SysML content and extract the first definition
fn parse_first_definition(source: &str) -> Definition {
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Parse failed: {:?}", parse.errors);
    
    let root = SourceFile::cast(parse.syntax()).expect("should have SOURCE_FILE");
    root.members()
        .find_map(|m| match m {
            super::ast::NamespaceMember::Definition(d) => Some(d),
            _ => None,
        })
        .expect("should have a definition")
}

/// Parse SysML content and extract the first usage
fn parse_first_usage(source: &str) -> Usage {
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Parse failed: {:?}", parse.errors);
    
    let root = SourceFile::cast(parse.syntax()).expect("should have SOURCE_FILE");
    root.members()
        .find_map(|m| match m {
            super::ast::NamespaceMember::Usage(u) => Some(u),
            _ => None,
        })
        .expect("should have a usage")
}

// ============================================================================
// Definition tests (migrated from tests_sysml_parsing.rs)
// ============================================================================

#[test]
fn test_parse_definition_with_specialization() {
    let source = "part def Car :> Vehicle;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("Car".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 1, "Expected 1 specialization");
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("Vehicle".to_string())
    );
}

#[test]
fn test_parse_definition_with_multiple_specializations() {
    let source = "part def SportsCar :> Car, Vehicle;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("SportsCar".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 2, "Expected 2 specializations");
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("Car".to_string())
    );
    assert_eq!(
        specializations[1].target().map(|t| t.text()),
        Some("Vehicle".to_string())
    );
}

#[test]
fn test_parse_anonymous_definition() {
    let source = "part def;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), None);
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 0);
}

#[test]
fn test_parse_action_definition_with_specialization() {
    let source = "action def Drive :> Action;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("Drive".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 1);
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("Action".to_string())
    );
}

#[test]
fn test_parse_requirement_definition_with_specialization() {
    let source = "requirement def SafetyReq :> BaseRequirement;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("SafetyReq".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 1);
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("BaseRequirement".to_string())
    );
}

#[test]
fn test_parse_item_definition_with_specialization() {
    let source = "item def Fuel :> Material;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("Fuel".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 1);
}

#[test]
fn test_parse_attribute_definition_with_specialization() {
    let source = "attribute def Speed :> Measurement;";
    let def = parse_first_definition(source);
    
    assert_eq!(def.name().and_then(|n| n.text()), Some("Speed".to_string()));
    
    let specializations: Vec<_> = def.specializations().collect();
    assert_eq!(specializations.len(), 1);
}

// ============================================================================
// Usage tests (migrated from tests_sysml_parsing.rs)
// ============================================================================

#[test]
fn test_parse_usage_with_typed_by() {
    let source = "part vehicle : Vehicle;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("vehicle".to_string()));
    assert_eq!(
        usage.typing().and_then(|t| t.target()).map(|t| t.text()),
        Some("Vehicle".to_string())
    );
}

#[test]
fn test_parse_usage_with_subsets() {
    let source = "part vehicle2 :> vehicle1;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("vehicle2".to_string()));
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 1, "Expected 1 subset");
    
    // In rowan, :> can be subsets or specializes depending on context
    // For usages, :> typically means subsets
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("vehicle1".to_string())
    );
}

#[test]
fn test_parse_usage_with_redefines() {
    let source = "part vehicle2 :>> vehicle1;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("vehicle2".to_string()));
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 1, "Expected 1 redefinition");
    
    // Check that it's a redefinition (not just specialization)
    assert_eq!(specializations[0].kind(), Some(SpecializationKind::Redefines));
    assert_eq!(
        specializations[0].target().map(|t| t.text()),
        Some("vehicle1".to_string())
    );
}

#[test]
fn test_parse_usage_with_multiple_subsets() {
    let source = "part myPart :> part1, part2, part3;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("myPart".to_string()));
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 3, "Expected 3 subsets");
    assert_eq!(specializations[0].target().map(|t| t.text()), Some("part1".to_string()));
    assert_eq!(specializations[1].target().map(|t| t.text()), Some("part2".to_string()));
    assert_eq!(specializations[2].target().map(|t| t.text()), Some("part3".to_string()));
}

#[test]
fn test_parse_usage_with_typed_and_subsets() {
    let source = "part vehicle : VehicleDef :> basePart;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("vehicle".to_string()));
    assert_eq!(
        usage.typing().and_then(|t| t.target()).map(|t| t.text()),
        Some("VehicleDef".to_string())
    );
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 1);
    assert_eq!(specializations[0].target().map(|t| t.text()), Some("basePart".to_string()));
}

#[test]
fn test_parse_usage_with_multiple_redefines() {
    let source = "part newPart :>> oldPart1, oldPart2;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("newPart".to_string()));
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 2);
    
    // First should be a redefinition (has :>> token)
    assert_eq!(specializations[0].kind(), Some(SpecializationKind::Redefines));
    
    // Note: subsequent items in comma-separated list may not have their own kind token
    // This is a limitation of the current parser structure
    
    assert_eq!(specializations[0].target().map(|t| t.text()), Some("oldPart1".to_string()));
    assert_eq!(specializations[1].target().map(|t| t.text()), Some("oldPart2".to_string()));
}

#[test]
fn test_parse_usage_complex_relationships() {
    let source = "part enginePart : Engine :> vehiclePart :>> oldEngine;";
    let usage = parse_first_usage(source);
    
    assert_eq!(usage.name().and_then(|n| n.text()), Some("enginePart".to_string()));
    assert_eq!(
        usage.typing().and_then(|t| t.target()).map(|t| t.text()),
        Some("Engine".to_string())
    );
    
    let specializations: Vec<_> = usage.specializations().collect();
    assert_eq!(specializations.len(), 2, "Expected 2 relationships (subset + redefine)");
    
    // First should be subset (vehiclePart)
    assert_eq!(specializations[0].target().map(|t| t.text()), Some("vehiclePart".to_string()));
    
    // Second should be redefine (oldEngine)
    assert_eq!(specializations[1].kind(), Some(SpecializationKind::Redefines));
    assert_eq!(specializations[1].target().map(|t| t.text()), Some("oldEngine".to_string()));
}

// ============================================================================
// Model-level tests (migrated from tests_sysml_parsing.rs)
// ============================================================================

#[test]
fn test_parse_model_with_satisfy_relationship() {
    let source = "requirement def SafetyReq; case def SafetyCase { satisfy SafetyReq; }";
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Failed to parse model with satisfy: {:?}", parse.errors);
}

#[test]
fn test_parse_model_with_satisfy_requirement_keyword() {
    let source = "requirement def SafetyReq; case def SafetyCase { satisfy requirement SafetyReq; }";
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Failed to parse model with satisfy requirement: {:?}", parse.errors);
}

#[test]
fn test_parse_model_with_perform_relationship() {
    let source = "action def Move; part def Robot { perform Move; }";
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Failed to parse model with perform: {:?}", parse.errors);
}

#[test]
fn test_parse_model_with_exhibit_relationship() {
    let source = "state def Moving; part def Vehicle { exhibit Moving; }";
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Failed to parse model with exhibit: {:?}", parse.errors);
}

#[test]
fn test_parse_model_with_include_relationship() {
    let source = "use case def Login; use case def ManageAccount { include Login; }";
    let parse = parse_sysml(source);
    assert!(parse.ok(), "Failed to parse model with include: {:?}", parse.errors);
}
