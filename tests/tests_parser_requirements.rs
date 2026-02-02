//! Parser Tests - Requirements and Verification
//!
//! Phase 1: Parser/AST Layer
//! Tests for requirements, verification, analysis, and related constructs.
//!
//! Test data from tests_parser_sysml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Requirement Definitions
// ============================================================================

#[rstest]
#[case("requirement def MyReq;")]
#[case("requirement def MyReq {}")]
#[case("requirement def MyReq { doc /* text */ }")]
#[case("requirement def MyReq { subject mySubject; }")]
fn test_requirement_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Requirement Usages
// ============================================================================

#[rstest]
#[case("package P { requirement myReq; }")]
#[case("package P { requirement myReq { assume myConstraint; } }")]
#[case("package P { requirement myReq { require myConstraint; } }")]
fn test_requirement_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Constraint Definitions
// ============================================================================

#[rstest]
#[case("constraint def MyConstraint;")]
#[case("constraint def MyConstraint {}")]
#[case("constraint def MyConstraint { x > 0 }")]
fn test_constraint_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Constraint Usages
// ============================================================================

#[rstest]
#[case("package P { constraint myConstraint; }")]
#[case("package P { constraint myConstraint { a == b } }")]
#[case("package P { assert constraint c { true } }")]
fn test_constraint_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Verification Definitions
// ============================================================================

#[rstest]
#[case("verification def MyVerification;")]
#[case("verification def MyVerification {}")]
fn test_verification_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Analysis Definitions
// ============================================================================

#[rstest]
#[case("analysis def MyAnalysis;")]
#[case("analysis def MyAnalysis {}")]
fn test_analysis_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Case Definitions
// ============================================================================

#[rstest]
#[case("case def MyCase;")]
#[case("case def MyCase {}")]
#[case("case def MyCase { objective obj; }")]
fn test_case_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Use Case Definitions
// ============================================================================

#[rstest]
#[case("use case def MyUseCase;")]
#[case("use case def MyUseCase {}")]
#[case("use case def MyUseCase { actor myActor; }")]
fn test_use_case_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Concern and Stakeholder
// ============================================================================

#[rstest]
#[case("concern def MyConcern;")]
#[case("concern def MyConcern {}")]
#[case("concern def MyConcern { stakeholder myStakeholder; }")]
fn test_concern_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Satisfy and Verify
// ============================================================================

#[rstest]
#[case("part def P { satisfy myReq; }")]
#[case("requirement def R { verify myVerification; }")]
fn test_satisfy_verify(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Subject, Actor, Objective
// ============================================================================

#[rstest]
#[case("requirement def R { subject mySubject; }")]
#[case("use case def UC { actor myActor; }")]
#[case("case def C { objective myObjective; }")]
#[case("concern def Co { stakeholder myStakeholder; }")]
fn test_special_memberships(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}
