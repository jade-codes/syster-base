//! Rowan Parser Rule-Based Tests
//!
//! This file contains tests migrated from the disabled Pest parser tests
//! (tests_parser_kerml_pest.rs.disabled) to use the Rowan parser's rule-based API.
//!
//! The `Rule` enum maps Pest grammar rules to appropriate Rowan parsing functions
//! by wrapping the input in the minimal required context.

use rstest::rstest;
use syster::parser::{Rule, parse_rule};

/// Helper to assert that a rule parses successfully
fn assert_rule_parses(rule: Rule, input: &str, desc: &str) {
    let result = parse_rule(rule, input);
    assert!(
        result.is_ok(),
        "Failed to parse {} as {:?}: {:?}\nInput: {}",
        desc,
        rule,
        result.errors(),
        input
    );
}

// =============================================================================
// Item Flow Tests (from test_parse_abstract_flow, etc.)
// =============================================================================

#[rstest]
#[case("flow myFlow;", "simple flow")]
#[case("flow myFlow from a to b;", "flow with endpoints")]
#[case("abstract flow flowTransfers: FlowTransfer[0..*] nonunique subsets transfers {}", "abstract flow with typing")]
fn test_item_flow(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::ItemFlow, input, desc);
}

// =============================================================================
// Connector Tests
// =============================================================================

#[rstest]
#[case("connector MyConnector;", "simple connector")]
#[case("connector MyConnector {}", "connector with body")]
#[case("connector a ::> a.x to b;", "named endpoint with ref subsetting")]
#[case("connector from self to occ;", "from keyword without declaration")]
#[case("connector x to y;", "simple binary without from")]
#[case("connector :HappensDuring from [1] shorterOccurrence references thisOccurrence to [1] longerOccurrence references thatOccurrence;", "connector with from/to endpoints")]
fn test_connector(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Connector, input, desc);
}

// =============================================================================
// Binding Connector Tests
// =============================================================================

#[rstest]
#[case("binding MyBinding;", "simple binding")]
#[case("binding MyBinding {}", "binding with body")]
#[case("binding [1] whileDecision.ifTest = [1] whileTest { }", "binding with multiplicity and endpoints")]
#[case("binding accept.receiver = triggerTarget;", "binding with feature chain")]
fn test_binding_connector(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::BindingConnector, input, desc);
}

// =============================================================================
// Succession Tests
// =============================================================================

#[rstest]
#[case("succession MySuccession;", "simple succession")]
#[case("succession MySuccession {}", "succession with body")]
#[case("succession a then b;", "succession with then")]
#[case("succession [1] ifTest then [0..1] thenClause { }", "succession with multiplicity")]
fn test_succession(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Succession, input, desc);
}

// =============================================================================
// Class Tests
// =============================================================================

#[rstest]
#[case("class MyClass;", "simple class")]
#[case("class MyClass {}", "class with body")]
#[case("abstract class MyClass;", "abstract class")]
#[case("class MyClass specializes Base {}", "class with specialization")]
#[case("abstract class MyClass specializes Base, Other {}", "abstract class with multiple specializations")]
#[case("abstract class Occurrence specializes Anything disjoint from DataValue { }", "class with disjoint")]
#[case("class all JohnLife[0..1] specializes John;", "class with all and multiplicity")]
#[case("class MyClass[1] :> Base { }", "class with multiplicity")]
fn test_class(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Class, input, desc);
}

// =============================================================================
// DataType Tests
// =============================================================================

#[rstest]
#[case("datatype MyData;", "simple datatype")]
#[case("datatype MyData {}", "datatype with body")]
#[case("abstract datatype ScalarValue specializes DataValue;", "abstract datatype")]
#[case("datatype Boolean specializes ScalarValue;", "datatype with specialization")]
fn test_datatype(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::DataType, input, desc);
}

// =============================================================================
// Feature Tests
// =============================================================================

#[rstest]
#[case("feature MyFeature;", "simple feature")]
#[case("feature MyFeature {}", "feature with body")]
#[case("in feature MyFeature;", "feature with direction")]
#[case("out feature MyFeature;", "feature with out direction")]
#[case("inout feature MyFeature;", "feature with inout direction")]
#[case("abstract feature MyFeature;", "abstract feature")]
#[case("composite feature MyFeature;", "composite feature")]
#[case("const feature MyFeature;", "const feature")]
#[case("derived feature MyFeature;", "derived feature")]
#[case("feature MyFeature ordered;", "feature with ordered")]
#[case("feature MyFeature nonunique;", "feature with nonunique")]
#[case("feature MyFeature ordered nonunique;", "feature with both ordering props")]
#[case("feature dimensions: Positive[0..*] ordered nonunique { }", "feature with modifiers after typing")]
#[case("feature elements[0..*] :>> Collection::elements {}", "feature with multiplicity and relationships")]
#[case("abstract feature dataValues: DataValue[0..*] nonunique subsets things { }", "feature with multiplicity props before subsetting")]
#[case("derived var feature annotatedElement : Element[1..*] ordered redefines annotatedElement;", "feature with var modifier")]
#[case("end feature thisThing: Anything redefines source subsets sameThing crosses sameThing.self;", "end feature with crosses")]
fn test_feature(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Feature, input, desc);
}

// =============================================================================
// Function Tests
// =============================================================================

#[rstest]
#[case("function MyFunction;", "simple function")]
#[case("function MyFunction {}", "function with body")]
#[case("function '==' { }", "function with operator name")]
#[case("function '!=' { }", "function with not-equal name")]
#[case("function '+' { }", "function with plus name")]
#[case("abstract function '-' { }", "abstract function with minus name")]
#[case("function 'not' specializes ScalarFunctions::'not' { }", "function specializes quoted")]
fn test_function(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Function, input, desc);
}

// =============================================================================
// Expression Tests
// =============================================================================

#[rstest]
#[case("null", "null expression")]
#[case("true", "true literal")]
#[case("false", "false literal")]
#[case("42", "integer literal")]
#[case("3.14", "decimal literal")]
#[case("myFeature", "simple reference")]
#[case("a.b", "feature chain")]
#[case("a.b.c", "nested feature chain")]
#[case("x == y", "equality")]
#[case("x != y", "not equal")]
#[case("x < y", "less than")]
#[case("x <= y", "less than or equal")]
#[case("x > y", "greater than")]
#[case("x >= y", "greater than or equal")]
#[case("x + y", "addition")]
#[case("x - y", "subtraction")]
#[case("x * y", "multiplication")]
#[case("x / y", "division")]
#[case("x and y", "logical and")]
#[case("x or y", "logical or")]
#[case("x xor y", "logical xor")]
#[case("x ?? 0", "null coalescing")]
#[case("if true ? 1 else 0", "conditional expression")]
#[case("size(dimensions)", "invocation")]
#[case("foo()", "empty invocation")]
#[case("max(a, b)", "invocation with args")]
#[case("NumericalFunctions::sum0(x, y)", "qualified invocation")]
#[case("subp istype StatePerformance", "istype operator")]
#[case("w == null or isZeroVector(w) implies u == w", "implies operator")]
#[case("col->reduce '+' ?? zero", "collection operators")]
fn test_operator_expression(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::OperatorExpression, input, desc);
}

// =============================================================================
// Parameter Tests
// =============================================================================

#[rstest]
#[case("in x: Anything[0..1];", "in parameter")]
#[case("in y: Boolean[1];", "in bool parameter")]
#[case("out result: Natural[1];", "out parameter")]
#[case("inout value: Complex[0..*];", "inout parameter")]
#[case("in x: Anything[0..*] nonunique;", "parameter with nonunique")]
#[case("in x: Anything[0..*] ordered;", "parameter with ordered")]
#[case("in indexes: Positive[n] ordered nonunique;", "parameter with identifier multiplicity")]
#[case("in x: Integer[1] default 0;", "parameter with default")]
#[case("in redefines ifTest;", "parameter with only redefines")]
#[case("in bool redefines onOccurrence { }", "parameter with bool type")]
fn test_parameter_membership(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::ParameterMembership, input, desc);
}

#[rstest]
#[case("return : Boolean[1];", "simple return")]
#[case("return result: Natural[1];", "named return")]
#[case("return : Complex[1] = x + y;", "return with expression")]
#[case("return : Integer[1] default sum0(collection, 0);", "return with default")]
#[case("return : NumericalVectorValue[1] { }", "return with body")]
#[case("return resultValues : Anything [*] nonunique redefines result redefines values;", "return with multiple redefines")]
fn test_return_parameter_membership(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::ReturnParameterMembership, input, desc);
}

// =============================================================================
// Invariant Tests
// =============================================================================

#[rstest]
#[case("inv MyInvariant;", "simple invariant")]
#[case("inv not MyInvariant {}", "negated invariant")]
#[case("inv zeroAddition { w == null or isZeroVector(w) implies u == w }", "invariant with implies")]
fn test_invariant(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Invariant, input, desc);
}

// =============================================================================
// Step Tests  
// =============================================================================

#[rstest]
#[case("step MyStep;", "simple step")]
#[case("step MyStep {}", "step with body")]
#[case("abstract step enactedPerformances: Performance[0..*] subsets involvingPerformances, timeEnclosedOccurrences { }", "step with multiple subsets")]
fn test_step(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Step, input, desc);
}

// =============================================================================
// Import Tests
// =============================================================================

#[rstest]
#[case("import MyPackage;", "simple import")]
#[case("public import MyLib;", "public import")]
#[case("private import all Base;", "private import all")]
#[case("import MyPackage::*;", "namespace import")]
#[case("import MyPackage::**;", "recursive import")]
#[case("import MyPackage {}", "import with body")]
fn test_import(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Import, input, desc);
}

// =============================================================================
// Package Tests
// =============================================================================

#[rstest]
#[case("package Foo;", "simple package")]
#[case("package Foo { }", "package with body")]
#[case("library package LibPkg;", "library package")]
#[case("standard library package StdLib;", "standard library package")]
fn test_package(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Package, input, desc);
}

// =============================================================================
// Comment/Documentation Tests
// =============================================================================

#[rstest]
#[case("comment /* simple comment */", "simple comment")]
#[case("comment myComment /* comment text */", "named comment")]
#[case("comment about Foo /* about Foo */", "comment with about")]
#[case("comment about Bar, Baz /* about multiple */", "comment about multiple")]
#[case("comment about StructuredSurface, StructuredCurve, StructuredPoint /* comment body */", "comment with multiple about targets")]
fn test_comment_annotation(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::CommentAnnotation, input, desc);
}

#[rstest]
#[case("doc /* documentation */", "simple doc")]
#[case("doc MyDoc /* doc text */", "named doc")]
fn test_documentation(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::Documentation, input, desc);
}

// =============================================================================
// SysML Definition Tests
// =============================================================================

#[rstest]
#[case("part def Vehicle;", "simple part def")]
#[case("part def Vehicle { }", "part def with body")]
#[case("abstract part def Vehicle;", "abstract part def")]
fn test_part_def(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::PartDef, input, desc);
}

#[rstest]
#[case("action def Drive;", "simple action def")]
#[case("action def Drive { }", "action def with body")]
fn test_action_def(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::ActionDef, input, desc);
}

// =============================================================================
// SysML Usage Tests
// =============================================================================

#[rstest]
#[case("part engine;", "simple part")]
#[case("part engine : Engine;", "typed part")]
#[case("part engine : Engine[1];", "typed part with multiplicity")]
fn test_part_usage(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::PartUsage, input, desc);
}

#[rstest]
#[case("action drive;", "simple action")]
#[case("action drive : Drive;", "typed action")]
fn test_action_usage(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::ActionUsage, input, desc);
}

// =============================================================================
// SysML Action Body Elements
// =============================================================================

#[rstest]
#[case("perform startEngine;", "simple perform")]
#[case("perform action startEngine;", "perform with action keyword")]
fn test_perform_action_usage(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::PerformActionUsage, input, desc);
}

#[rstest]
#[case("send message;", "simple send")]
#[case("send message to receiver;", "send with receiver")]
fn test_send_action_usage(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::SendActionUsage, input, desc);
}

#[rstest]
#[case("accept message;", "simple accept")]
#[case("accept message via port;", "accept with via")]
fn test_accept_action_usage(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::AcceptActionUsage, input, desc);
}

// =============================================================================
// Qualified Reference Tests
// =============================================================================

#[rstest]
#[case("Foo", "simple name")]
#[case("Foo::Bar", "two-part qualified")]
#[case("Foo::Bar::Baz", "three-part qualified")]
#[case("ScalarFunctions::'not'", "qualified with quoted")]
#[case("Base::'=='", "qualified with operator")]
fn test_qualified_reference_chain(#[case] input: &str, #[case] desc: &str) {
    assert_rule_parses(Rule::QualifiedReferenceChain, input, desc);
}
