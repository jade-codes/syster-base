//! Parser Tests - Specializations
//!
//! Phase 1: Parser/AST Layer
//! Tests for specialization kinds (:>, :>>, subsets, redefines, conjugates).
//!
//! Test data from tests_parser_kerml_ast.rs.archived.

use rstest::rstest;
use syster::parser::{
    AstNode, NamespaceMember, SourceFile, SpecializationKind, parse_kerml, parse_sysml,
};

/// Get first usage's first specialization kind
fn get_usage_spec_kind(input: &str) -> Option<SpecializationKind> {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    for member in file.members() {
        if let NamespaceMember::Package(pkg) = member {
            if let Some(body) = pkg.body() {
                for m in body.members() {
                    if let NamespaceMember::Usage(u) = m {
                        return u.specializations().next().and_then(|s| s.kind());
                    }
                }
            }
        }
    }
    None
}

/// Get definition specialization (via :> in definition header)
fn def_has_specialization(input: &str) -> bool {
    let parsed = parse_kerml(input);
    let file = SourceFile::cast(parsed.syntax()).expect("should parse");
    for member in file.members() {
        if let NamespaceMember::Definition(def) = member {
            if def.specializations().next().is_some() {
                return true;
            }
        }
    }
    false
}

// ============================================================================
// Feature Specialization Kinds
// ============================================================================

#[rstest]
#[case(
    "package Test { part wheelMass subsets mass; }",
    SpecializationKind::Subsets
)]
#[case(
    "package Test { part currentMass redefines mass; }",
    SpecializationKind::Redefines
)]
#[case("package Test { part x :> base; }", SpecializationKind::Specializes)]
#[case("package Test { part y :>> original; }", SpecializationKind::Redefines)]
fn test_feature_specialization_kind(
    #[case] input: &str,
    #[case] expected_kind: SpecializationKind,
) {
    assert_eq!(
        get_usage_spec_kind(input),
        Some(expected_kind),
        "Specialization kind mismatch for: {}",
        input
    );
}

// ============================================================================
// Definition Specializations
// ============================================================================

#[rstest]
#[case("classifier Car specializes Vehicle;", true)]
#[case("classifier SportsCar :> Car;", true)]
#[case("class Occurrence specializes Anything;", true)]
#[case("datatype ScalarValue specializes DataValue;", true)]
#[case("abstract class Base :> Root;", true)]
#[case("classifier Standalone;", false)]
fn test_definition_has_specialization(#[case] input: &str, #[case] expected: bool) {
    assert_eq!(
        def_has_specialization(input),
        expected,
        "Specialization presence mismatch for: {}",
        input
    );
}

// ============================================================================
// Conjugation (~ operator)
// ============================================================================

#[rstest]
#[case("part def P { port myPort : ~ConjugatedPortType; }")]
fn test_conjugation_parses(#[case] input: &str) {
    let parsed = parse_sysml(input);
    assert!(
        SourceFile::cast(parsed.syntax()).is_some(),
        "Failed to parse: {}",
        input
    );
}
