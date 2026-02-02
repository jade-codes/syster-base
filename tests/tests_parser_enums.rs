//! Parser Tests - Enums, Variants, and Individuals
//!
//! Phase 1: Parser/AST Layer
//! Tests for enums, variants, individuals, timeslice, snapshot.
//!
//! Test data from tests_parser_sysml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Enum Definitions
// ============================================================================

#[rstest]
#[case("enum def MyEnum;")]
#[case("enum def MyEnum {}")]
#[case("enum def Color { enum red; enum green; enum blue; }")]
fn test_enum_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Variant
// ============================================================================

#[rstest]
#[case("part def Test { variant myVariant; }")]
#[case("variation part def PartVariants;")]
#[case("variation part def VehicleChoices { variant sedan; variant suv; }")]
fn test_variants(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Individual
// ============================================================================

#[rstest]
#[case("individual def MyIndividual;")]
#[case("individual def MyIndividual {}")]
#[case("individual part def UniquePart;")]
#[case("package T { ref individual part uniquePart; }")]
fn test_individual(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Timeslice and Snapshot
// ============================================================================

#[rstest]
#[case("package T { snapshot part snap1; }")]
#[case("package T { timeslice part slice1; }")]
fn test_timeslice_snapshot(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Alias
// ============================================================================

#[rstest]
#[case("package Test { alias MyAlias for MyElement; }")]
#[case("package Test { private alias MyAlias for MyElement; }")]
fn test_alias(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Assign
// ============================================================================

#[rstest]
#[case("action def Test { assign x := value; }")]
fn test_assign(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}
