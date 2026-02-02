//! Parser Tests - Feature Modifiers
//!
//! Phase 1: Parser/AST Layer
//! Tests for feature modifiers: ordered, nonunique, composite, portion, end, const.
//!
//! Test data from tests_parser_kerml_ast.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_kerml, parse_sysml};

fn parses_kerml(input: &str) -> bool {
    let parsed = parse_kerml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Ordered and Nonunique
// ============================================================================

#[rstest]
#[case("type MyType ordered {}")]
#[case("package Test { feature MyFeature ordered; }")]
#[case("package Test { feature MyFeature nonunique; }")]
#[case("package Test { feature MyFeature ordered nonunique; }")]
#[case("package Test { feature items[*] : ItemType ordered; }")]
#[case("package Test { feature dimensions: Positive[0..*] ordered nonunique { } }")]
fn test_ordered_nonunique(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Composite and Portion
// ============================================================================

#[rstest]
#[case("package Test { composite feature MyFeature; }")]
#[case("package Test { portion feature MyFeature; }")]
fn test_composite_portion(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// End Feature
// ============================================================================

#[rstest]
#[case("package Test { end feature MyFeature; }")]
#[case("package Test { end feature x : MyType; }")]
#[case("package Test { end feature y : BaseType[1]; }")]
fn test_end_feature(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Multiplicity
// ============================================================================

#[rstest]
#[case("package T { part x[1]; }")]
#[case("package T { part x[0..1]; }")]
#[case("package T { part x[0..*]; }")]
#[case("package T { part x[1..*]; }")]
#[case("package T { part x[*]; }")]
fn test_multiplicity_sysml(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

#[rstest]
#[case("package Test { feature x[1]; }")]
#[case("package Test { feature x[0..1]; }")]
#[case("package Test { feature x[0..*]; }")]
#[case("package Test { feature elements[0..*] :>> Collection::elements {} }")]
#[case("package Test { feature myFeature[1] :> BaseFeature; }")]
fn test_multiplicity_kerml(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Const (KerML)
// ============================================================================

#[rstest]
#[case("function Test { in abstract const feature MyFeature ordered; }")]
#[case("function Test { out composite derived feature MyFeature nonunique; }")]
fn test_const_modifier(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Feature Chains
// ============================================================================

#[rstest]
#[case("package Test { feature x chains y; }")]
#[case("package Test { feature x = a.b.c; }")]
fn test_feature_chains(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Invariant
// ============================================================================

#[rstest]
#[case("package Test { inv MyInvariant; }")]
#[case("package Test { inv not MyInvariant {} }")]
fn test_invariant(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}
