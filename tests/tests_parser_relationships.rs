//! Parser Tests - KerML Relationships and Namespaces
//!
//! Phase 1: Parser/AST Layer
//! Tests for KerML relationships (redefinition, specialization, subset)
//! and namespace declarations.
//!
//! Test data from tests_parser_kerml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_kerml};

fn parses_kerml(input: &str) -> bool {
    let parsed = parse_kerml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Namespace Declarations
// ============================================================================

#[rstest]
#[case("namespace MyNamespace;")]
#[case("namespace MyNamespace {}")]
#[case("namespace <short> named {}")]
fn test_namespace(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Redefinition Relationships
// ============================================================================

#[rstest]
#[case("package P { redefinition b.f redefines b.a; }")]
#[case("package P { redefinition a :>> b; }")]
fn test_redefinition(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Specialization Relationships
// ============================================================================

#[rstest]
#[case("package P { specialization id subtype A specializes B { } }")]
#[case("package P { specialization Super subclassifier A specializes B; }")]
fn test_specialization_relationship(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Subset Relationships
// ============================================================================

#[rstest]
#[case("package P { subset laterOccurrence.successors subsets earlierOccurrence.successors; }")]
fn test_subset_relationship(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Disjoining
// ============================================================================

#[rstest]
#[case("package P { disjoining disjoint A from B; }")]
fn test_disjoining(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Conjugation
// ============================================================================

#[rstest]
#[case("package P { conjugation conj conjugate A conjugates B; }")]
fn test_conjugation(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Featuring
// ============================================================================

#[rstest]
#[case("package P { featuring feat of A by B; }")]
fn test_featuring(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Inverting
// ============================================================================

#[rstest]
#[case("package P { inverting inv inverse A inverses B; }")]
fn test_inverting(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}
