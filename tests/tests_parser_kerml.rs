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
