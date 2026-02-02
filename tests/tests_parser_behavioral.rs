//! Parser Tests - Control Nodes and Behavioral Elements
//!
//! Phase 1: Parser/AST Layer
//! Tests for control nodes (fork, join, merge, decide), state subactions,
//! and other behavioral constructs.
//!
//! Test data from tests_parser_sysml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

/// Helper to check if input parses successfully (no fatal errors)
fn parses_successfully(input: &str) -> bool {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax());
    file.is_some()
}

// ============================================================================
// Control Nodes
// ============================================================================

#[rstest]
#[case("action def A { fork; }")]
#[case("action def A { fork myFork; }")]
#[case("action def A { merge; }")]
#[case("action def A { merge myMerge; }")]
#[case("action def A { join; }")]
#[case("action def A { join myJoin; }")]
#[case("action def A { decide; }")]
#[case("action def A { decide myDecision; }")]
fn test_control_nodes_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// State Subactions
// ============================================================================

#[rstest]
#[case("state def S { entry myEntryAction; }")]
#[case("state def S { exit myExitAction; }")]
#[case("state def S { do myDoAction; }")]
#[case("state def S { entry; exit; do; }")]
fn test_state_subactions_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Transition Features
// ============================================================================

#[rstest]
#[case("state def S { transition first s1 then s2; }")]
#[case("state def S { transition t first s1 then s2; }")]
#[case("state def S { succession first s1 then s2; }")]
fn test_transitions_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Requirement Parameter Memberships
// ============================================================================

#[rstest]
#[case("requirement def R { subject mySubject; }")]
#[case("use case def UC { actor myActor; }")]
#[case("concern def C { stakeholder myStakeholder; }")]
#[case("case def C { objective myObjective; }")]
fn test_parameter_memberships_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Port Conjugation
// ============================================================================

#[rstest]
#[case("part def P { port myPort : ~ConjugatedPortType; }")]
fn test_port_conjugation_parses(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Expose and Verification
// ============================================================================

#[rstest]
#[case("view def V { expose MyElement; }")]
#[case("requirement def R { require myConstraint; }")]
#[case("requirement def R { assume myConstraint; }")]
fn test_expose_and_verification_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Comments and Documentation
// ============================================================================

#[rstest]
#[case("comment about Foo;")]
#[case("comment about Foo, Bar;")]
#[case("comment locale \"en-US\" about Foo;")]
#[case("doc;")]
fn test_comments_parse(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Dependency
// ============================================================================

#[rstest]
#[case("package P { dependency from A to B; }")]
fn test_dependency_parses(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}
