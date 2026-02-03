//! Parser Tests - KerML Constructs
//!
//! Phase 1: Parser/AST Layer  
//! Tests for KerML-specific constructs.
//!
//! Test data from tests_parser_kerml_ast.rs.archived, tests_parser_kerml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_kerml};

fn parses_kerml(input: &str) -> bool {
    let parsed = parse_kerml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// KerML Type Definitions
// ============================================================================

#[rstest]
#[case("type MyType;")]
#[case("abstract type MyType {}")]
#[case("type MyType ordered {}")]
#[case("type MyType specializes Base;")]
fn test_type_definitions(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Classifiers with Bodies
// ============================================================================

#[rstest]
#[case("class MyClass;")]
#[case("class MyClass {}")]
#[case("abstract class MyClass;")]
#[case("class MyClass specializes Base {}")]
#[case("struct MyStruct;")]
#[case("struct MyStruct {}")]
#[case("abstract struct MyStruct specializes Base {}")]
#[case("assoc MyAssoc;")]
#[case("assoc MyAssoc {}")]
#[case("abstract assoc Link specializes Anything {}")]
fn test_classifier_bodies(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Behaviors and Functions
// ============================================================================

#[rstest]
#[case("behavior MyBehavior;")]
#[case("behavior MyBehavior {}")]
#[case("abstract behavior Performance specializes Occurrence {}")]
#[case("function MyFunction;")]
#[case("function MyFunction {}")]
#[case("predicate MyPredicate;")]
#[case("predicate MyPredicate {}")]
#[case("interaction MyInteraction;")]
#[case("interaction MyInteraction {}")]
fn test_behavior_definitions(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Features with Typing
// ============================================================================

#[rstest]
#[case("package Test { feature mass : Real; }")]
#[case("package Test { feature x : Integer; }")]
#[case("package Test { feature value : Boolean; }")]
#[case("package Test { in feature x; }")]
#[case("package Test { out feature y; }")]
#[case("package Test { inout feature z; }")]
fn test_features_with_typing(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Connectors
// ============================================================================

#[rstest]
#[case("package Test { connector c from a to b; }")]
#[case("package Test { connector myConn: Type from a to b; }")]
#[case("package Test { binding b of x = y; }")]
fn test_connectors(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Steps
// ============================================================================

#[rstest]
#[case("behavior B { step myStep; }")]
#[case("behavior B { step s : Action; }")]
fn test_steps(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Expressions
// ============================================================================

#[rstest]
#[case("function F { expr myExpr; }")]
#[case("function F { inv myInvariant; }")]
fn test_kerml_expressions(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Visibility
// ============================================================================

#[rstest]
#[case("package P { public import MyLib; }")]
#[case("package P { private import all Base; }")]
#[case("package P { protected import MyLib; }")]
#[case("class C { private feature x; }")]
#[case("class C { protected feature y; }")]
#[case("class C { public feature z; }")]
fn test_visibility(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Dependencies
// ============================================================================

#[rstest]
#[case("package P { dependency from Source to Target; }")]
#[case("package P { dependency MyDep from Source to Target; }")]
fn test_dependencies(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Comments
// ============================================================================

#[rstest]
#[case("package Test { comment /* This is a comment */ }")]
#[case("package Test { comment myComment /* text */ }")]
#[case("package Test { doc /* Documentation */ }")]
fn test_kerml_comments(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Nested Definitions
// ============================================================================

#[rstest]
#[case("classifier Vehicle { feature mass : Real; }")]
#[case("class Car { feature engine : Engine; }")]
#[case("struct Point { feature x : Real; feature y : Real; }")]
fn test_nested_features(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Library Packages
// ============================================================================

#[rstest]
#[case("library package MyLib;")]
#[case("standard library package ScalarValues;")]
#[case("standard library package Base;")]
fn test_library_packages(#[case] input: &str) {
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// STDLIB SYNTAX COVERAGE TESTS
// ============================================================================
// These tests cover advanced KerML/SysML v2 syntax patterns found in the
// standard library (sysml.library). They document parser gaps that need
// to be addressed for full stdlib support.
//
// Reference: SysML v2 Specification (OMG Document Number: formal/2023-06-01)
// ============================================================================

/// Test `this` keyword as a feature name
/// From Occurrences.kerml: `feature this : Occurrence[1] default self { ... }`
/// Per SysML v2 Spec §7.3.4.3: "The keyword this denotes a reference to the
/// context object of the current feature."
#[test]
fn test_this_keyword_as_feature_name() {
    let input = r#"classifier C { feature this : C[1]; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `this` with default value
/// From Occurrences.kerml: `feature this : Occurrence[1] default self { ... }`
#[test]
fn test_this_with_default_value() {
    let input = r#"classifier C { feature this : C[1] default self; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `chains` keyword in feature chaining
/// From Base.kerml: `feature self: Anything[1] subsets things chains things.that`
/// Per SysML v2 Spec §7.3.4.5: "Feature chaining provides a shorthand for
/// expressing navigational paths through features."
#[test]
fn test_chains_keyword() {
    let input = r#"classifier C { feature self: C[1] subsets things chains things.that; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test feature path expressions with dot notation
/// From Base.kerml: `chains things.that`
#[test]
fn test_feature_path_expression() {
    let input = r#"classifier C { feature x subsets a.b.c; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `metaclass` definition
/// From KerML.kerml: `metaclass AnnotatingElement specializes Element { ... }`
/// Per SysML v2 Spec §7.2.3.2: "A Metaclass is a Classifier whose instances
/// are themselves Classifiers."
#[test]
fn test_metaclass_definition() {
    let input = r#"metaclass MyMeta specializes Element;"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `metaclass` with body
/// From KerML.kerml
#[test]
fn test_metaclass_with_body() {
    let input = r#"metaclass Comment specializes AnnotatingElement {
        feature body : String[1..1];
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `var` feature modifier
/// From KerML.kerml: `var feature annotatedElement : Element[1..*] ordered`
/// Per SysML v2 Spec §7.3.3.6: "The keyword var indicates that a Feature
/// is a variable, meaning its value can be changed during execution."
#[test]
fn test_var_feature_modifier() {
    let input = r#"classifier C { var feature x : Integer[1]; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `derived var` combined modifiers
/// From KerML.kerml: `derived var feature annotatedElement`
#[test]
fn test_derived_var_modifier() {
    let input = r#"classifier C { derived var feature x : Integer[1]; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `all` keyword for universal quantification
/// From Occurrences.kerml: `feature all x : T[*]`
/// Per SysML v2 Spec §7.3.3.4: "The keyword all indicates that a Feature
/// includes all instances of its type."
#[test]
fn test_all_keyword_in_feature() {
    let input = r#"classifier C { feature all instances : C[*]; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test filter expression with bracket notation
/// From FeatureReferencingPerformances.kerml: `expr[1]`
/// Per SysML v2 Spec §7.4.2.5: "Filter expressions select elements based
/// on conditions."
#[test]

fn test_filter_expression_in_redefines() {
    let input = r#"classifier C { feature x : T redefines y[1]; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `= value` assignment in feature declaration
/// From Parts.sysml: `ref part this : Part :>> Action::this = that as Part`
/// Per SysML v2 Spec §7.3.3.8: "A FeatureValue specifies the value of a
/// Feature in the context of its owning Type."
#[test]
fn test_feature_value_with_cast() {
    let input = r#"classifier C { feature x : T = y as T; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test combined redefines with assignment
/// From Parts.sysml: `:>> Action::this = that as Part`
#[test]
fn test_redefines_with_assignment() {
    let input = r#"classifier C { feature x :>> parent::y = initialValue; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test anonymous connector with from/to clauses
/// From Occurrences.kerml: `connector :HappensDuring from [1] self to [1] this;`
#[test]
fn test_anonymous_connector_with_multiplicity() {
    let input = r#"classifier C { connector :Link from [1] a to [1] b; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// ============================================================================
// STDLIB PARSE FAILURES - Issues discovered from CLI analyze tests
// ============================================================================

/// Test scientific notation with negative exponent
/// From SIPrefixes.sysml: `conversionFactor = 1E-6`
/// Issue: The `-` in `1E-6` is being parsed as a separate operator
#[test]
fn test_scientific_notation_negative_exponent() {
    let input = r#"package P { attribute x = 1E-6; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test scientific notation with positive exponent (should work)
/// From SIPrefixes.sysml: `conversionFactor = 1E6`
#[test]
fn test_scientific_notation_positive_exponent() {
    let input = r#"package P { attribute x = 1E6; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `done` as enum variant name (contextual keyword)
/// From ModelingMetadata.sysml: `done { doc /* Status is done. */ }`
/// Issue: `done` is lexed as DONE_KW keyword instead of identifier
#[test]
fn test_done_as_enum_variant_name() {
    let input = r#"enum Status { done; }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test SI unit with inline attribute assignments
/// From SI.sysml: complex unit definitions with multiple inline assignments
/// Issue: Parser fails on complex inline assignment patterns
#[test]
fn test_si_unit_inline_assignments() {
    let input = r#"
        package P {
            attribute def UnitPrefix {
                attribute longName : String;
                attribute symbol : String;
                attribute conversionFactor : Real;
            }
            attribute micro: UnitPrefix { 
                :>> longName = "micro"; 
                :>> symbol = "μ"; 
                :>> conversionFactor = 1E-6; 
            }
        }
    "#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test metadata with `about` clause
/// From ImageMetadata.sysml: `metadata def ImageItem about Item`
/// Issue: Parser may not handle `about` clause in metadata definitions
#[test]
fn test_metadata_about_clause() {
    let input = r#"metadata def ImageItem about Item;"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}
