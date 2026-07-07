//! Parser Tests - Definition Kinds
//!
//! Phase 1: Parser/AST Layer
//! These tests verify that the parser correctly identifies definition kinds.
//!
//! Test data extracted from archived tests (tests_parser_sysml_ast.rs.archived,
//! tests_parser_kerml_ast.rs.archived).

use rstest::rstest;
use syster::parser::{AstNode, Definition, DefinitionKind, SourceFile, parse_kerml, parse_sysml};

/// Helper to parse SysML and get the first definition
fn parse_sysml_def(input: &str) -> Option<Definition> {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    file.members().find_map(|m| match m {
        syster::parser::NamespaceMember::Definition(d) => Some(d),
        _ => None,
    })
}

/// Helper to parse KerML and get the first definition
fn parse_kerml_def(input: &str) -> Option<Definition> {
    let parsed = parse_kerml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    file.members().find_map(|m| match m {
        syster::parser::NamespaceMember::Definition(d) => Some(d),
        _ => None,
    })
}

// ============================================================================
// SysML Definition Kinds
// ============================================================================

#[rstest]
#[case("part def MyPart;", DefinitionKind::Part, "MyPart")]
#[case("attribute def MyAttr;", DefinitionKind::Attribute, "MyAttr")]
#[case("port def MyPort;", DefinitionKind::Port, "MyPort")]
#[case("item def MyItem;", DefinitionKind::Item, "MyItem")]
#[case("action def MyAction;", DefinitionKind::Action, "MyAction")]
#[case("state def MyState;", DefinitionKind::State, "MyState")]
#[case(
    "constraint def MyConstraint;",
    DefinitionKind::Constraint,
    "MyConstraint"
)]
#[case("requirement def MyReq;", DefinitionKind::Requirement, "MyReq")]
#[case("case def MyCase;", DefinitionKind::Case, "MyCase")]
#[case("calc def MyCalc;", DefinitionKind::Calc, "MyCalc")]
#[case("connection def MyConn;", DefinitionKind::Connection, "MyConn")]
#[case("interface def MyInterface;", DefinitionKind::Interface, "MyInterface")]
#[case("allocation def MyAlloc;", DefinitionKind::Allocation, "MyAlloc")]
#[case("flow def MyFlow;", DefinitionKind::Flow, "MyFlow")]
#[case("view def MyView;", DefinitionKind::View, "MyView")]
#[case("viewpoint def MyViewpoint;", DefinitionKind::Viewpoint, "MyViewpoint")]
#[case("rendering def MyRendering;", DefinitionKind::Rendering, "MyRendering")]
#[case("metadata def MyMetadata;", DefinitionKind::Metadata, "MyMetadata")]
#[case(
    "occurrence def MyOccurrence;",
    DefinitionKind::Occurrence,
    "MyOccurrence"
)]
#[case("enum def MyEnum;", DefinitionKind::Enum, "MyEnum")]
#[case("analysis def MyAnalysis;", DefinitionKind::Analysis, "MyAnalysis")]
#[case(
    "verification def MyVerification;",
    DefinitionKind::Verification,
    "MyVerification"
)]
#[case("use case def MyUseCase;", DefinitionKind::UseCase, "MyUseCase")]
#[case("concern def MyConcern;", DefinitionKind::Concern, "MyConcern")]
fn test_sysml_definition_kind(
    #[case] input: &str,
    #[case] expected_kind: DefinitionKind,
    #[case] expected_name: &str,
) {
    let def = parse_sysml_def(input).expect("Should parse");
    assert_eq!(def.definition_kind(), Some(expected_kind));
    assert_eq!(
        def.name().and_then(|n| n.text()),
        Some(expected_name.to_string())
    );
}

// ============================================================================
// KerML Definition Kinds
// ============================================================================

#[rstest]
#[case("classifier MyClassifier;", DefinitionKind::Classifier, "MyClassifier")]
#[case("class MyClass;", DefinitionKind::Class, "MyClass")]
#[case("struct MyStruct;", DefinitionKind::Struct, "MyStruct")]
#[case("datatype Real;", DefinitionKind::Datatype, "Real")]
#[case("behavior MyBehavior;", DefinitionKind::Behavior, "MyBehavior")]
#[case("function calculateArea;", DefinitionKind::Function, "calculateArea")]
#[case("predicate MyPredicate;", DefinitionKind::Predicate, "MyPredicate")]
#[case(
    "interaction MyInteraction;",
    DefinitionKind::Interaction,
    "MyInteraction"
)]
#[case("metaclass MyMetaclass;", DefinitionKind::Metaclass, "MyMetaclass")]
#[case("assoc MyAssoc;", DefinitionKind::Assoc, "MyAssoc")]
fn test_kerml_definition_kind(
    #[case] input: &str,
    #[case] expected_kind: DefinitionKind,
    #[case] expected_name: &str,
) {
    let def = parse_kerml_def(input).expect("Should parse");
    assert_eq!(def.definition_kind(), Some(expected_kind));
    assert_eq!(
        def.name().and_then(|n| n.text()),
        Some(expected_name.to_string())
    );
}

// ============================================================================
// Abstract Modifier
// ============================================================================

#[rstest]
#[case("abstract part def Vehicle;", true, "Vehicle")]
#[case("part def Vehicle;", false, "Vehicle")]
fn test_sysml_abstract_modifier(
    #[case] input: &str,
    #[case] expected_abstract: bool,
    #[case] expected_name: &str,
) {
    let def = parse_sysml_def(input).expect("Should parse");
    assert_eq!(def.is_abstract(), expected_abstract);
    assert_eq!(
        def.name().and_then(|n| n.text()),
        Some(expected_name.to_string())
    );
}

#[rstest]
#[case("abstract classifier Vehicle;", true)]
#[case("classifier Vehicle;", false)]
#[case("abstract class Occurrence;", true)]
#[case("abstract class Base;", true)]
#[case("class Base;", false)]
#[case("class Occurrence;", false)]
#[case("abstract datatype ScalarValue;", true)]
#[case("datatype Real;", false)]
fn test_kerml_abstract_modifier(#[case] input: &str, #[case] expected_abstract: bool) {
    let def = parse_kerml_def(input).expect("Should parse");
    assert_eq!(def.is_abstract(), expected_abstract);
}

// ============================================================================
// Variation Modifier
// ============================================================================

#[rstest]
#[case("variation part def VehicleChoices;", true)]
#[case("part def Vehicle;", false)]
fn test_variation_modifier(#[case] input: &str, #[case] expected_variation: bool) {
    let def = parse_sysml_def(input).expect("Should parse");
    assert_eq!(def.is_variation(), expected_variation);
}

/// Reserved keyword 'to' used as attribute name should parse without errors.
#[test]
fn test_attribute_named_to() {
    let parsed = parse_sysml("item def D { attribute to : String[0..1]; }");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);
}

/// 'actor def' should be parsed as a definition, not trigger a usage error.
#[test]
fn test_actor_def() {
    let parsed = parse_sysml("package P { actor def GapFourActor; }");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);
    let file = syster::parser::SourceFile::cast(parsed.syntax()).expect("cast");
    let pkg = file.members().find_map(|m| match m {
        syster::parser::NamespaceMember::Package(p) => Some(p),
        _ => None,
    }).expect("package");
    let def = pkg.body().expect("body").members().find_map(|m| match m {
        syster::parser::NamespaceMember::Definition(d) => Some(d),
        _ => None,
    }).expect("definition");
    assert_eq!(def.definition_kind(), Some(DefinitionKind::Actor));
    assert_eq!(def.name().and_then(|n| n.text()), Some("GapFourActor".to_string()));
}

/// Prefix metadata with a body block (#Tag { ... }) should not swallow the following member.
#[test]
fn test_prefix_metadata_with_body() {
    let parsed = parse_sysml(
        r#"package P {
            #AnyTag { :>> ref = "spec.adoc"; }
            use case def <'UC-LOST'> LostCase { action step_a; }
        }"#,
    );
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);
}

// ============================================================================
// Specific ACTION_DEFINITION / CALC_DEFINITION / CONSTRAINT_DEFINITION /
// REQUIREMENT_DEFINITION / ACTION_USAGE / CALC_USAGE / CONSTRAINT_USAGE /
// REQUIREMENT_USAGE node kinds.
//
// These were previously dead SyntaxKind variants: the parser always
// constructed the generic DEFINITION/USAGE node with boolean flags instead,
// so any code matching on the specific kinds would never see them. See
// docs/grammar-gaps.adoc.
// ============================================================================

/// Find the first descendant node of `kind` and assert its presence.
fn assert_has_kind(input: &str, kind: syster::parser::SyntaxKind, label: &str) {
    let parsed = parse_sysml(input);
    assert!(parsed.ok(), "Failed to parse without errors: {}\nerrors: {:?}", input, parsed.errors);
    let found = parsed.syntax().descendants().any(|n| n.kind() == kind);
    assert!(found, "expected a {:?} node for {}: {}", kind, label, input);
}

#[test]
fn test_specific_definition_kinds() {
    use syster::parser::SyntaxKind;
    assert_has_kind("action def A {}", SyntaxKind::ACTION_DEFINITION, "action def");
    assert_has_kind("calc def C {}", SyntaxKind::CALC_DEFINITION, "calc def");
    assert_has_kind(
        "constraint def C {}",
        SyntaxKind::CONSTRAINT_DEFINITION,
        "constraint def",
    );
    assert_has_kind(
        "requirement def R {}",
        SyntaxKind::REQUIREMENT_DEFINITION,
        "requirement def",
    );
    // Definitions without a dedicated kind must keep using the generic node.
    assert_has_kind("part def P {}", SyntaxKind::DEFINITION, "part def");
}

#[test]
fn test_specific_usage_kinds() {
    use syster::parser::SyntaxKind;
    assert_has_kind("part def P { action a; }", SyntaxKind::ACTION_USAGE, "action usage");
    assert_has_kind("part def P { calc c; }", SyntaxKind::CALC_USAGE, "calc usage");
    assert_has_kind(
        "part def P { constraint c; }",
        SyntaxKind::CONSTRAINT_USAGE,
        "constraint usage",
    );
    assert_has_kind(
        "package P { requirement r; }",
        SyntaxKind::REQUIREMENT_USAGE,
        "requirement usage",
    );
    // A defining `assert constraint c {...}` (via parse_requirement_constraint)
    // must also get CONSTRAINT_USAGE, not just the bare `constraint` form.
    assert_has_kind(
        "package P { assert constraint c { true } }",
        SyntaxKind::CONSTRAINT_USAGE,
        "assert constraint usage",
    );
    // Usages without a dedicated kind must keep using the generic node.
    assert_has_kind("part def P { port p; }", SyntaxKind::USAGE, "port usage");
}

#[test]
fn test_constraint_and_requirement_references_stay_generic_usage() {
    use syster::parser::SyntaxKind;
    // A bare reference (no 'constraint'/'requirement' keyword) is not a
    // defining usage, so it should not claim CONSTRAINT_USAGE/REQUIREMENT_USAGE.
    let parsed = parse_sysml("package P { assert c; }");
    assert!(parsed.ok(), "unexpected errors: {:?}", parsed.errors);
    assert!(
        !parsed
            .syntax()
            .descendants()
            .any(|n| n.kind() == SyntaxKind::CONSTRAINT_USAGE),
        "bare constraint reference should not be CONSTRAINT_USAGE"
    );

    let parsed = parse_sysml("part def P { satisfy r1; }");
    assert!(parsed.ok(), "unexpected errors: {:?}", parsed.errors);
    assert!(
        !parsed
            .syntax()
            .descendants()
            .any(|n| n.kind() == SyntaxKind::REQUIREMENT_USAGE),
        "bare satisfy shorthand should not be REQUIREMENT_USAGE"
    );
}

/// Regression: `UserDefinedKeyword = "#" MCQualifiedName` used to only consume
/// the first segment, so `#Foo::Bar` left `::Bar` dangling. See docs/grammar-gaps.adoc.
#[test]
fn test_prefix_metadata_qualified_name_sysml() {
    let parsed = parse_sysml("#Foo::Bar part def X;");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);

    // Single-segment form must still work unchanged.
    let parsed = parse_sysml("#Foo part def X;");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);
}

#[test]
fn test_prefix_metadata_qualified_name_kerml() {
    let parsed = parse_kerml("#Foo::Bar class X;");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);

    let parsed = parse_kerml("#Foo class X;");
    assert!(parsed.errors.is_empty(), "unexpected errors: {:?}", parsed.errors);
}
