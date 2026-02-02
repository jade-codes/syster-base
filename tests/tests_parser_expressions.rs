//! Parser Tests - Expressions
//!
//! Phase 1: Parser/AST Layer
//! Tests for expression parsing including operators, function calls,
//! arrow invocations, and complex expressions.
//!
//! Test data from tests_parser_expression_rowan.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

/// Helper to check if input parses successfully
fn parses_successfully(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// Attribute Expressions
// ============================================================================

#[rstest]
// Chained member access
#[case("package T { attribute x = fn.samples.domainValue; }")]
// Instantiation expressions
#[case("package T { attribute x = new SampledFunction(samples = values); }")]
#[case("package T { attribute x = new SamplePair(x, y); }")]
// Arrow invocations
#[case("package T { attribute x = list->select { in i; true }; }")]
#[case("package T { attribute x = list->select { in i; true }#(1); }")]
// Complex expressions
#[case(
    "package T { attribute x = (1..size(domainValues))->select { in i : Positive; domainValues#(i) <= value }#(1); }"
)]
#[case(
    "package T { attribute x = domainValues->collect { in x; new SamplePair(x, calculation(x)) }; }"
)]
fn test_attribute_expressions(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Constraint Expressions
// ============================================================================

#[rstest]
#[case("package T { constraint c { a <= b } }")]
#[case("package T { constraint c { a >= b } }")]
#[case("package T { constraint c { a < b } }")]
#[case("package T { constraint c { a > b } }")]
#[case("package T { constraint c { a == b } }")]
#[case("package T { constraint c { a === b } }")]
#[case("package T { constraint c { a != b } }")]
#[case("package T { constraint c { a !== b } }")]
#[case("package T { constraint c { stateSpace.order == order } }")]
fn test_constraint_expressions(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Classification Expressions
// ============================================================================

#[rstest]
#[case("package T { attribute x = causes as SysML::Usage; }")]
#[case("package T { attribute x = value hastype Domain::ItemType; }")]
#[case("package T { attribute x = multicausations meta SysML::Usage; }")]
#[case("package T { attribute x = myMetadata @@ SysML::Metadata; }")]
fn test_classification_expressions(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Number Literals
// ============================================================================

#[rstest]
#[case("package T { attribute x = 1E-24; }")]
#[case("package T { attribute x = 1E24; }")]
#[case("package T { attribute x = 1e-24; }")]
#[case("package T { attribute x = 3.14; }")]
#[case("package T { attribute x = 42; }")]
#[case("package T { attribute x = .5; }")]
fn test_number_literals(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Boolean and Logical Expressions
// ============================================================================

#[rstest]
#[case("package T { constraint c { x implies f() } }")]
#[case(
    "package T { constraint c { originalRequirement.result implies allTrue(derivedRequirements.result) } }"
)]
#[case("package T { constraint c { a and b } }")]
#[case("package T { constraint c { a or b } }")]
#[case("package T { constraint c { not a } }")]
#[case("package T { constraint c { a xor b } }")]
fn test_logical_expressions(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Attribute Patterns
// ============================================================================

#[rstest]
#[case("package T { attribute index : Positive[0..1]; }")]
#[case("package T { attribute x default foo { } }")]
#[case("package T { attribute :>> x default foo { } }")]
#[case("package T { attribute x[1] { } }")]
#[case("package T { attribute x[1] default foo; }")]
#[case("package T { attribute x[1] default foo { } }")]
#[case("package T { attribute :>> x[1] { } }")]
#[case("package T { attribute :>> x[1] default foo; }")]
#[case("package T { attribute transformation[1] default nullTransformation { } }")]
#[case("package T { attribute <x> myAttr; }")]
#[case("package T { attribute <isq> 'International System of Quantities'; }")]
#[case(
    "package T { attribute <isq> 'International System of Quantities': SystemOfQuantities { } }"
)]
#[case("package T { attribute :>> dimensions = (); }")]
fn test_attribute_patterns(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Calc Def Patterns
// ============================================================================

#[rstest]
#[case("calc def Test { a == b }")]
#[case("calc def Test { in p2 : Point; p1 != p2 }")]
fn test_calc_def_patterns(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Parameter Patterns
// ============================================================================

#[rstest]
#[case("calc def Test { in fn : SampledFunction; }")]
#[case("calc def Test { return : Anything[0..*] = fn.samples.domainValue; }")]
#[case("calc def Test { return sampling = new SampledFunction(); }")]
#[case("calc def Test { return result: StateSpace = value; }")]
fn test_parameter_patterns(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Flow and Interface Patterns
// ============================================================================

#[rstest]
#[case("package T { abstract interface interfaces: Interface[0..*] nonunique :> connections { } }")]
#[case("package T { abstract flow flows: Flow[0..*] nonunique :> messages, flowTransfers { } }")]
fn test_flow_interface_patterns(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}

// ============================================================================
// Feature Chain Patterns
// ============================================================================

#[rstest]
#[case(
    "package T { part def P { ref :>> outgoingTransfersFromSelf :> interfacingPorts.incomingTransfersToSelf { } } }"
)]
fn test_feature_chain_patterns(#[case] input: &str) {
    assert!(parses_successfully(input), "Failed to parse: {}", input);
}
