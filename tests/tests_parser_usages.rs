//! Parser Tests - Usage Kinds
//!
//! Phase 1: Parser/AST Layer
//! These tests verify that the parser correctly identifies usage kinds.
//!
//! Test data extracted from archived tests (tests_parser_sysml_ast.rs.archived).

use rstest::rstest;
use syster::parser::{
    AstNode, Direction, NamespaceMember, SourceFile, Usage, UsageKind, parse_sysml,
};

/// Helper to parse SysML and get the first usage from the first package
fn parse_usage(input: &str) -> Option<Usage> {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    for member in file.members() {
        if let NamespaceMember::Package(pkg) = member {
            if let Some(body) = pkg.body() {
                for m in body.members() {
                    if let NamespaceMember::Usage(u) = m {
                        return Some(u);
                    }
                }
            }
        }
    }
    None
}

/// Helper to parse SysML and get the first usage from a definition
fn parse_usage_in_def(input: &str) -> Option<Usage> {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    for member in file.members() {
        if let NamespaceMember::Definition(def) = member {
            if let Some(body) = def.body() {
                for m in body.members() {
                    if let NamespaceMember::Usage(u) = m {
                        return Some(u);
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// SysML Usage Kinds
// ============================================================================

#[rstest]
#[case("package Test { part myPart; }", UsageKind::Part, "myPart")]
#[case("package Test { attribute myAttr; }", UsageKind::Attribute, "myAttr")]
#[case("package Test { port myPort; }", UsageKind::Port, "myPort")]
#[case("package Test { item myItem; }", UsageKind::Item, "myItem")]
#[case("package Test { action myAction; }", UsageKind::Action, "myAction")]
#[case("package Test { state myState; }", UsageKind::State, "myState")]
#[case(
    "package Test { constraint myConstraint; }",
    UsageKind::Constraint,
    "myConstraint"
)]
#[case("package Test { requirement myReq; }", UsageKind::Requirement, "myReq")]
#[case("package Test { case myCase; }", UsageKind::Case, "myCase")]
#[case("package Test { calc myCalc; }", UsageKind::Calc, "myCalc")]
#[case("package Test { connection myConn; }", UsageKind::Connection, "myConn")]
#[case(
    "package Test { interface myInterface; }",
    UsageKind::Interface,
    "myInterface"
)]
#[case(
    "package Test { allocation myAlloc; }",
    UsageKind::Allocation,
    "myAlloc"
)]
#[case("package Test { flow myFlow; }", UsageKind::Flow, "myFlow")]
#[case(
    "package Test { occurrence myOccurrence; }",
    UsageKind::Occurrence,
    "myOccurrence"
)]
#[case("package Test { ref myRef; }", UsageKind::Ref, "myRef")]
fn test_usage_kind(
    #[case] input: &str,
    #[case] expected_kind: UsageKind,
    #[case] expected_name: &str,
) {
    let usage = parse_usage(input).expect("Should parse");
    assert_eq!(usage.usage_kind(), Some(expected_kind));
    assert_eq!(
        usage.name().and_then(|n| n.text()),
        Some(expected_name.to_string())
    );
}

// ============================================================================
// Modifiers
// ============================================================================

#[rstest]
#[case("package Test { ref part myPart; }", true)]
#[case("package Test { part myPart; }", false)]
fn test_ref_modifier(#[case] input: &str, #[case] expected_ref: bool) {
    let usage = parse_usage(input).expect("Should parse");
    assert_eq!(usage.is_ref(), expected_ref);
}

#[rstest]
#[case("package Test { readonly part myPart; }", true)]
#[case("package Test { part myPart; }", false)]
fn test_readonly_modifier(#[case] input: &str, #[case] expected_readonly: bool) {
    let usage = parse_usage(input).expect("Should parse");
    assert_eq!(usage.is_readonly(), expected_readonly);
}

#[rstest]
#[case("package Test { derived part myPart; }", true)]
#[case("package Test { part myPart; }", false)]
fn test_derived_modifier(#[case] input: &str, #[case] expected_derived: bool) {
    let usage = parse_usage(input).expect("Should parse");
    assert_eq!(usage.is_derived(), expected_derived);
}

// ============================================================================
// Direction
// ============================================================================

#[rstest]
#[case("action def Test { in item x; }", Direction::In)]
#[case("action def Test { out item x; }", Direction::Out)]
#[case("action def Test { inout item x; }", Direction::InOut)]
fn test_direction(#[case] input: &str, #[case] expected_direction: Direction) {
    let usage = parse_usage_in_def(input).expect("Should parse");
    assert_eq!(usage.direction(), Some(expected_direction));
}
