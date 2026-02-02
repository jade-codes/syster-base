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
#[case("abstract class Base;", true, "Base")]
#[case("class Base;", false, "Base")]
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
