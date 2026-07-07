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

// Regression: fork/join/merge/decide only parsed an optional name, no typing
// (`: TypeName`), specializations (`:>`/`:>>`), multiplicity, or default
// value, even though they all extend ActionUsage and share its full
// declaration grammar. See docs/grammar-gaps.adoc.
#[rstest]
#[case("action def A { fork f : MyForkKind; }")]
#[case("action def A { decide d : DecideKind { } }")]
#[case("action def A { join j :> otherJoin; }")]
#[case("action def A { merge m[1] : MergeKind; }")]
#[case("action def A { merge m :>> otherMerge default foo; }")]
fn test_control_nodes_declaration_tail(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "Failed to parse without errors: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
}

// Regression: fork/join/merge/decide can be preceded by a ControlNodePrefix
// (ref, individual, snapshot, timeslice, etc.), per grammar. Since these
// control keywords aren't SysML usage/definition keywords, the top-level
// dispatcher used to route any of these prefixes straight into
// parse_definition_or_usage(), which chokes on the control keyword that
// follows and produces a real syntax error. See docs/grammar-gaps.adoc.
#[rstest]
#[case("action def A { individual fork f { } }")]
#[case("action def A { ref fork f { } }")]
#[case("action def A { snapshot fork f { } }")]
#[case("action def A { timeslice join j { } }")]
#[case("action def A { ref individual decide d : DecideKind; }")]
fn test_control_node_prefix(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "Failed to parse without errors: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
}

// ============================================================================
// Parallel state marker
// Regression: `state s parallel { ... }` used to lex `parallel` as a plain
// IDENT, so it was mis-parsed as the name of a second, unrelated usage
// instead of a marker on the state. See docs/grammar-gaps.adoc.
// ============================================================================

#[rstest]
#[case("part def P { state s parallel { entry; } }")]
#[case("part def P { action def A { exhibit state s parallel { entry; } } }")]
#[case("state def S parallel { entry; }")]
fn test_parallel_state_marker(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "Failed to parse without errors: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
    // `parallel` must be recognized as the PARALLEL_KW marker token (proving
    // it wasn't swallowed as the NAME of a second, unrelated usage).
    let has_parallel_kw = parsed
        .syntax()
        .descendants_with_tokens()
        .any(|n| n.kind() == syster::parser::SyntaxKind::PARALLEL_KW);
    assert!(
        has_parallel_kw,
        "expected a PARALLEL_KW token for: {}",
        input
    );
}

// `parallel` must still work as a plain identifier outside the StateUsage
// marker position (it is only a contextual keyword there).
#[rstest]
#[case("enum def NodeKind { branch; parallel; }")]
#[case("part def P { attribute parallel : Boolean; }")]
fn test_parallel_as_identifier(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "`parallel` should be valid as an identifier: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
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

// Regression: entry/do bodies referencing a nested action via a dotted
// qualified name (e.g. `entry Off.entry;`) used to hit a syntax error on the
// '.', because `parse_state_subaction` only ever parsed a single bare name.
// See docs/grammar-gaps.adoc.
#[rstest]
#[case("state def S { entry Off.entry; }")]
#[case("state def S { do Off.doThing; }")]
#[case("state def S { entry On::entry; }")]
// The plain single-name forms (declaration and reference) must keep working.
#[case("state def S { do myAction : ActionType { } }")]
#[case("state def S { exit myExitAction; }")]
fn test_state_subaction_qualified_name_parses(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "Failed to parse without errors: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
}

// Regression: a call-style effect in a transition's `do` clause (e.g.
// `do action1()`) used to leave the trailing '(' ')' unconsumed, corrupting
// the rest of the transition (the `then` target was dropped). See
// docs/grammar-gaps.adoc.
#[rstest]
#[case("state def S { transition t first s1 do action1() then s2; }")]
#[case("state def S { transition t first s1 do action1(1, 2) then s2; }")]
#[case("state def S { transition t first s1 accept p do action1() then s2; }")]
fn test_transition_do_effect_call_parses(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        parsed.ok(),
        "Failed to parse without errors: {}\nerrors: {:?}",
        input,
        parsed.errors
    );
    // The `then` target must survive -- this is exactly what the bug dropped.
    let has_then_kw = parsed
        .syntax()
        .descendants_with_tokens()
        .any(|n| n.kind() == syster::parser::SyntaxKind::THEN_KW);
    assert!(has_then_kw, "expected THEN_KW to survive for: {}", input);
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
// Causality/timing element (MontiCore SysMLCausality, not in official KEBNF)
// ============================================================================

#[rstest]
#[case("part def P { timing; }")]
#[case("part def P { timing instant; }")]
#[case("part def P { timing delayed; }")]
#[case("package P { part def Q { timing instant; } }")]
fn test_timing_parses(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

#[test]
fn test_timing_produces_dedicated_node() {
    use syster::parser::SyntaxKind;

    for input in [
        "part def P { timing; }",
        "part def P { timing instant; }",
        "part def P { timing delayed; }",
    ] {
        let parsed = parse_sysml(input);
        assert!(parsed.ok(), "Failed to parse {}: {:?}", input, parsed.errors);
        assert!(
            parsed
                .syntax()
                .descendants()
                .any(|n| n.kind() == SyntaxKind::CAUSALITY),
            "expected a CAUSALITY node for: {}",
            input
        );
    }
}

// ============================================================================
// Dependency
// ============================================================================

#[rstest]
#[case("package P { dependency from A to B; }")]
fn test_dependency_parses(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// `state` as a plain identifier (issue #18)
// KerML spec §8.2.2.6: `state` is a contextual keyword, not reserved.
// ============================================================================

#[rstest]
#[case("port def P { out item state : T; }")]
#[case("item def WorldModelState; port def WorldModelStatePort { out item state : WorldModelState; }")]
#[case("package TestState { item def WorldModelState; port def WorldModelStatePort { out item state : WorldModelState; } }")]
#[case("part def P { attribute state : Boolean; }")]
fn test_state_as_identifier_in_feature_decl(#[case] input: &str) {
    assert!(parses_successfully(input), "`state` should be valid as an identifier: {}", input);
}
