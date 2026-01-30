//! Expression parser tests adapted from Pest to Rowan
//!
//! This file adapts the original Pest-based expression tests to work with the new Rowan parser.
//! These tests focus on SysML expression parsing including:
//! - Primary expressions (member access, instantiation, invocation)
//! - Lambda/calculation bodies
//! - Constraint expressions
//! - Relational and equality operators
//! - Classification expressions (as, hastype, meta, @@)

#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]
#![allow(dead_code)]
#![allow(unused_imports)]

use rstest::rstest;
use syster::parser::rule_parser::{self, parse_rule as rowan_parse_rule, Rule};
use syster::parser::{parse_sysml, parse_kerml};

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse SysML source and assert it succeeds
fn assert_parses_sysml(input: &str, desc: &str) {
    let result = parse_sysml(input);
    assert!(
        result.ok(),
        "Failed to parse SysML ({}): {:?}\nInput: {}",
        desc,
        result.errors,
        input
    );
}

/// Parse a rule and assert it succeeds
fn assert_rule_parses(rule: Rule, input: &str, desc: &str) {
    let result = rowan_parse_rule(rule, input);
    assert!(
        result.is_ok(),
        "Failed to parse {} ({:?}):\nInput: {:?}\nErrors: {:?}",
        desc,
        rule,
        input,
        result.errors()
    );
}

/// Wrap expression in attribute context for testing
fn wrap_in_attribute(expr: &str) -> String {
    format!("package T {{ attribute x = {}; }}", expr)
}

/// Wrap expression in calc context for testing  
fn wrap_in_calc(body: &str) -> String {
    format!("package T {{ calc def Test {{ {} }} }}", body)
}

/// Wrap expression in constraint context for testing
fn wrap_in_constraint(expr: &str) -> String {
    format!("package T {{ constraint c {{ {} }} }}", expr)
}

// ============================================================================
// Primary Expression Tests
// ============================================================================

#[test]
fn test_chained_member_access() {
    let input = wrap_in_attribute("fn.samples.domainValue");
    assert_parses_sysml(&input, "chained member access");
}

#[test]
fn test_instantiation_expression_with_args() {
    let input = wrap_in_attribute("new SampledFunction(samples = values)");
    assert_parses_sysml(&input, "instantiation expression");
}

#[test]
fn test_instantiation_expression_positional() {
    let input = wrap_in_attribute("new SamplePair(x, y)");
    assert_parses_sysml(&input, "instantiation with positional args");
}

#[test]
fn test_arrow_invocation_with_block() {
    let input = wrap_in_attribute("list->select { in i; true }");
    assert_parses_sysml(&input, "arrow invocation with block");
}

#[test]
fn test_arrow_invocation_with_block_then_index() {
    let input = wrap_in_attribute("list->select { in i; true }#(1)");
    assert_parses_sysml(&input, "arrow invocation with block followed by indexing");
}

#[test]
fn test_typed_parameter_in_lambda() {
    let input = wrap_in_attribute("list->select { in i : Positive; domainValues#(i) <= value }");
    assert_parses_sysml(&input, "lambda with typed parameter");
}

#[test]
fn test_typed_parameter_in_lambda_then_index() {
    let input = wrap_in_attribute("list->select { in i : Positive; domainValues#(i) <= value }#(1)");
    assert_parses_sysml(&input, "lambda with typed parameter followed by indexing");
}

#[test]
fn test_full_sampled_functions_expression() {
    let input = wrap_in_attribute("(1..size(domainValues))->select { in i : Positive; domainValues#(i) <= value }#(1)");
    assert_parses_sysml(&input, "full SampledFunctions expression");
}

#[test]
fn test_nested_instantiation_in_collect() {
    let input = wrap_in_attribute("domainValues->collect { in x; new SamplePair(x, calculation(x)) }");
    assert_parses_sysml(&input, "collect with nested instantiation");
}

// ============================================================================
// Attribute Usage Tests
// ============================================================================

#[test]
fn test_attribute_with_complex_initializer() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute index : Positive[0..1] = (1..size(domainValues))->select { in i : Positive; domainValues#(i) <= value }#(1);",
        "attribute with complex initializer"
    );
}

#[test]
fn test_attribute_without_type() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute index = (1..size(domainValues))->select { in i : Positive; domainValues#(i) <= value }#(1);",
        "attribute without type"
    );
}

#[test]
fn test_attribute_in_package_context() {
    let input = "package Test { attribute index = list->select { in i : Positive; vals#(i) <= v }#(1); }";
    assert_parses_sysml(input, "attribute in package");
}

#[test]
fn test_attribute_with_simple_type() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute index : Positive[0..1];",
        "attribute with simple type"
    );
}

#[test]
fn test_feature_value_with_lambda() {
    // Feature value with complex lambda expression - test in attribute context
    let input = "package T { attribute x = (1..size(domainValues))->select { in i : Positive; domainValues#(i) <= value }#(1); }";
    assert_parses_sysml(input, "feature_value with lambda");
}

#[test]
fn test_attribute_with_default_value_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x default foo { }",
        "attribute with default value and body"
    );
}

#[test]
fn test_attribute_with_redefinition_default_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute :>> x default foo { }",
        "attribute with redefinition, default, and body"
    );
}

#[test]
fn test_attribute_with_multiplicity_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x[1] { }",
        "attribute with multiplicity and body"
    );
}

#[test]
fn test_attribute_with_multiplicity_and_default() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x[1] default foo;",
        "attribute with multiplicity and default"
    );
}

#[test]
fn test_attribute_with_multiplicity_default_no_redef_with_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x[1] default foo { }",
        "attribute with multiplicity, default, and body (no redef)"
    );
}

#[test]
fn test_attribute_with_redef_multiplicity_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute :>> x[1] { }",
        "attribute with redef, multiplicity, and body"
    );
}

#[test]
fn test_attribute_with_redef_multiplicity_and_default() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute :>> x[1] default foo;",
        "attribute with redef, multiplicity, and default"
    );
}

#[test]
fn test_attribute_with_long_name_multiplicity_default_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute transformation[1] default nullTransformation { }",
        "attribute with long name, multiplicity, default, body"
    );
}

#[test]
fn test_attribute_y_multiplicity_default_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute y[1] default z { }",
        "attribute y[1] default z with body"
    );
}

#[test]
fn test_attribute_x_multiplicity_default_null_x_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x[1] default nullX { }",
        "attribute x[1] default nullX with body"
    );
}

#[test]
fn test_attribute_x_multiplicity_default_abc_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute x[1] default abcdef { }",
        "attribute x[1] default abcdef with body"
    );
}

#[test]
fn test_attribute_with_short_name() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute <x> myAttr;",
        "attribute with short name"
    );
}

#[test]
fn test_attribute_with_short_name_and_quoted_full_name() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute <isq> 'International System of Quantities';",
        "attribute with short name and quoted full name"
    );
}

#[test]
fn test_attribute_with_short_name_typed_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute <isq> 'International System of Quantities': SystemOfQuantities { }",
        "attribute with short name, type, and body"
    );
}

#[test]
fn test_attribute_with_empty_tuple_value() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute :>> dimensions = ();",
        "attribute with empty tuple value"
    );
}

#[test]
fn test_attribute_with_scientific_notation_value() {
    let input = "package T { attribute yocto { :>> conversionFactor = 1E-24; } }";
    assert_parses_sysml(input, "attribute with scientific notation value");
}

#[test]
fn test_attribute_with_multiplicity_default_and_body() {
    assert_rule_parses(
        Rule::AttributeUsage,
        "attribute :>> transformation[1] default nullTransformation { attribute :>> source; }",
        "attribute with multiplicity, default, and body"
    );
}

#[test]
fn test_attribute_with_qualified_type_and_as_cast() {
    let input = "package T { ref :>> baseType = causes as SysML::Usage; }";
    assert_parses_sysml(input, "attribute with qualified type in as expression");
}

#[test]
fn test_attribute_with_meta_expression() {
    let input = "package T { ref :>> baseType = multicausations meta SysML::Usage; }";
    assert_parses_sysml(input, "attribute with meta expression");
}

// ============================================================================
// Calculation Body Tests
// ============================================================================

#[test]
fn test_calculation_body_minimal() {
    let input = wrap_in_calc("vals#(i)");
    assert_parses_sysml(&input, "minimal calculation body");
}

#[test]
fn test_calculation_body_with_parameter_binding() {
    let input = wrap_in_calc("in i; vals#(i)");
    assert_parses_sysml(&input, "calculation body with parameter binding");
}

#[test]
fn test_calculation_body_with_typed_parameter() {
    let input = wrap_in_calc("in i : Positive; vals#(i)");
    assert_parses_sysml(&input, "calculation body with typed parameter");
}

#[test]
fn test_calculation_body_with_parameter_declaration() {
    let input = wrap_in_calc("in fn : SampledFunction;");
    assert_parses_sysml(&input, "calculation body with parameter declaration");
}

#[test]
fn test_calculation_body_with_param_and_return() {
    let input = wrap_in_calc("in fn : SampledFunction; return : Anything[0..*] = fn.samples.domainValue;");
    assert_parses_sysml(&input, "calc body with param and return");
}

#[test]
fn test_calculation_body_braces() {
    let input = wrap_in_calc("in fn : SampledFunction; return : Anything[0..*] = fn.samples.domainValue;");
    assert_parses_sysml(&input, "calculation_body with braces");
}

#[test]
fn test_calculation_body_with_simple_expression() {
    let input = wrap_in_calc("a == b");
    assert_parses_sysml(&input, "calculation body with simple expression");
}

#[test]
fn test_calculation_body_item_return() {
    let input = wrap_in_calc("return : Anything[0..*] = fn.samples.domainValue;");
    assert_parses_sysml(&input, "calculation_body_item with return");
}

#[test]
fn test_calculation_body_part_with_param_and_return() {
    let input = wrap_in_calc("in fn : SampledFunction; return : Anything[0..*] = fn.samples.domainValue;");
    assert_parses_sysml(&input, "calculation_body_part");
}

#[test]
fn test_calculation_body_item_without_semicolon() {
    let input = wrap_in_calc("in calc calculation { in x; }");
    assert_parses_sysml(&input, "calculation_body_item without trailing semicolon");
}

#[test]
fn test_calculation_body_item_attribute_with_semicolon() {
    let input = wrap_in_calc("in attribute domainValues [0..*];");
    assert_parses_sysml(&input, "attribute usage in calculation_body_item");
}

#[test]
fn test_calculation_body_mixed_items() {
    let input = wrap_in_calc("in calc calculation { in x; } in attribute domainValues [0..*]; return sampling = value;");
    assert_parses_sysml(&input, "calculation_body with mixed items");
}

#[test]
fn test_calculation_body_part_simple_expression() {
    let input = wrap_in_calc("a == b");
    assert_parses_sysml(&input, "simple expression as calculation_body_part");
}

#[test]
fn test_action_body_item_identifier() {
    // This should parse as a usage reference
    let input = "package T { action def A { a; } }";
    assert_parses_sysml(input, "action body item identifier");
}

#[test]
fn test_calculation_body_item_identifier() {
    // This should parse as a result expression
    let input = wrap_in_calc("a");
    assert_parses_sysml(&input, "calculation body item identifier");
}

// ============================================================================
// Calculation Definition Tests
// ============================================================================

#[test]
fn test_calculation_def_domain() {
    assert_rule_parses(
        Rule::CalcDef,
        r#"calc def Domain {
            in fn : SampledFunction;
            return : Anything[0..*] = fn.samples.domainValue;
        }"#,
        "Domain calc def"
    );
}

#[test]
fn test_calculation_def_with_expression_body() {
    assert_rule_parses(
        Rule::CalcDef,
        "calc def Test { a == b }",
        "calc def with expression body"
    );
}

// ============================================================================
// Parameter Tests
// ============================================================================

#[test]
fn test_parameter_membership() {
    assert_rule_parses(
        Rule::ParameterMembership,
        "in fn : SampledFunction;",
        "typed parameter membership"
    );
}

#[test]
fn test_return_parameter_membership() {
    assert_rule_parses(
        Rule::ReturnParameterMembership,
        "return : Anything[0..*] = fn.samples.domainValue;",
        "return parameter membership"
    );
}

#[test]
fn test_return_parameter_with_name() {
    assert_rule_parses(
        Rule::ReturnParameterMembership,
        "return sampling = new SampledFunction();",
        "return parameter with name"
    );
}

#[test]
fn test_return_parameter_with_name_and_type() {
    assert_rule_parses(
        Rule::ReturnParameterMembership,
        "return result: StateSpace = value;",
        "return parameter with name and type"
    );
}

#[test]
fn test_return_attribute_member() {
    let input = wrap_in_calc("return attribute result;");
    assert_parses_sysml(&input, "return attribute member");
}

#[test]
fn test_return_attribute_member_with_type() {
    let input = wrap_in_calc("return attribute result : ScalarValue[1];");
    assert_parses_sysml(&input, "return attribute member with type");
}

#[test]
fn test_return_attribute_with_body() {
    let input = r#"package T { calc def Test {
        return attribute result : ScalarValue[1] {
            doc
            /*
             * A comment
             */
        }
    } }"#;
    assert_parses_sysml(input, "return attribute with body");
}

#[test]
fn test_parameter_binding_simple() {
    let input = wrap_in_calc("in i;");
    assert_parses_sysml(&input, "simple parameter binding");
}

#[test]
fn test_parameter_binding_typed() {
    let input = wrap_in_calc("in fn : SampledFunction;");
    assert_parses_sysml(&input, "typed parameter binding");
}

#[test]
fn test_parameter_binding_without_direction() {
    // Parameter without direction needs proper context
    let input = "package T { calc def Test { in p : Point; } }";
    assert_parses_sysml(input, "parameter binding without direction");
}

// ============================================================================
// Expression Body Tests
// ============================================================================

#[test]
fn test_expression_body_with_parameter() {
    let input = wrap_in_calc("in i; vals#(i)");
    assert_parses_sysml(&input, "expression body with parameter");
}

#[test]
fn test_expression_body_with_doc() {
    let input = r#"package T {
        calc def Test {
            doc
            /*
             * Some documentation
             */
            in x; eval(x)
        }
    }"#;
    assert_parses_sysml(input, "expression body with doc");
}

#[test]
fn test_expression_body_with_ref_parameter() {
    let input = r#"package T {
        calc def Test {
            in ref a {
                doc
                /* The alternative */
            }
            a
        }
    }"#;
    assert_parses_sysml(input, "expression body with ref parameter");
}

#[test]
fn test_expression_body_with_typed_parameter_no_direction() {
    // Typed parameter without direction keyword needs 'in' prefix in SysML calc bodies
    let input = "package T { calc def Test { in p2 : Point; p1 != p2 } }";
    assert_parses_sysml(input, "expression body with typed parameter");
}

// ============================================================================
// Relational and Equality Operators
// ============================================================================

#[rstest]
#[case("a <= b", "<=")]
#[case("a >= b", ">=")]
#[case("a < b", "<")]
#[case("a > b", ">")]
fn test_relational_operators(#[case] expr: &str, #[case] operator: &str) {
    let input = wrap_in_constraint(expr);
    assert_parses_sysml(&input, &format!("{} operator", operator));
}

#[rstest]
#[case("a == b", "==")]
#[case("a === b", "===")]
#[case("a != b", "!=")]
#[case("a !== b", "!==")]
fn test_equality_operators(#[case] expr: &str, #[case] operator: &str) {
    let input = wrap_in_constraint(expr);
    assert_parses_sysml(&input, &format!("{} operator", operator));
}

#[test]
fn test_equality_expression_with_member_access() {
    let input = wrap_in_constraint("stateSpace.order == order");
    assert_parses_sysml(&input, "equality expression with member access");
}

#[test]
fn test_complex_relational_expression() {
    let input = wrap_in_constraint("domainValues#(i) <= value");
    assert_parses_sysml(&input, "complex relational expression");
}

#[test]
fn test_conditional_expression_simple_equality() {
    let input = wrap_in_constraint("a == b");
    assert_parses_sysml(&input, "simple equality as conditional_expression");
}

#[test]
fn test_owned_expression_simple_equality() {
    let input = wrap_in_attribute("a == b");
    assert_parses_sysml(&input, "simple equality as owned_expression");
}

#[test]
fn test_result_expression_member_simple_equality() {
    let input = wrap_in_calc("a == b");
    assert_parses_sysml(&input, "simple equality as result_expression_member");
}

// ============================================================================
// Constraint Usage Tests  
// ============================================================================

#[test]
fn test_constraint_usage_with_expression() {
    assert_rule_parses(
        Rule::ConstraintUsage,
        "constraint { stateSpace.order == order }",
        "constraint usage with expression"
    );
}

#[test]
fn test_constraint_with_in_parameter() {
    let input = "package T { assert constraint c { in x = y; } }";
    assert_parses_sysml(input, "constraint with in parameter");
}

#[test]
fn test_constraint_with_simple_expression() {
    let input = "package T { assert constraint c { x implies y } }";
    assert_parses_sysml(input, "constraint with simple implies");
}

#[test]
fn test_constraint_with_function_on_right() {
    let input = "package T { assert constraint c { x implies f() } }";
    assert_parses_sysml(input, "constraint with function on right");
}

#[test]
fn test_constraint_with_actual_names() {
    let input = "package T { assert constraint c { x implies all(y) } }";
    assert_parses_sysml(input, "constraint with 'all' function");
}

#[test]
fn test_constraint_with_all_true_function() {
    let input = "package T { assert constraint c { x implies allTrue(y) } }";
    assert_parses_sysml(input, "constraint with 'allTrue' function");
}

#[test]
fn test_constraint_without_doc() {
    let input = r#"package T {
        assert constraint originalImpliesDerived {
            originalRequirement.result implies allTrue(derivedRequirements.result)
        }
    }"#;
    assert_parses_sysml(input, "constraint without doc");
}

#[test]
fn test_constraint_with_doc_and_expression() {
    let input = r#"package T {
        assert constraint originalImpliesDerived {
            doc 
            /* comment */
            originalRequirement.result implies allTrue(derivedRequirements.result)
        }
    }"#;
    assert_parses_sysml(input, "constraint with doc and expression");
}

// ============================================================================
// Classification Expression Tests
// ============================================================================

#[test]
fn test_as_expression_with_qualified_name() {
    let input = wrap_in_attribute("causes as SysML::Usage");
    assert_parses_sysml(&input, "as expression with qualified name");
}

#[test]
fn test_hastype_with_qualified_name() {
    let input = wrap_in_attribute("value hastype Domain::ItemType");
    assert_parses_sysml(&input, "hastype with qualified name");
}

#[test]
fn test_meta_expression_with_qualified_name() {
    let input = wrap_in_attribute("multicausations meta SysML::Usage");
    assert_parses_sysml(&input, "meta expression with qualified name");
}

#[test]
fn test_metadata_access_with_qualified_name() {
    let input = wrap_in_attribute("myMetadata @@ SysML::Metadata");
    assert_parses_sysml(&input, "@@ expression with qualified name");
}

#[test]
fn test_type_reference_with_qualified_name() {
    let input = "package T { part x : Domain::Library::Type; }";
    assert_parses_sysml(input, "type_reference with qualified name");
}

#[test]
fn test_type_result_with_qualified_name() {
    let input = "package T { part x : SysML::Usage; }";
    assert_parses_sysml(input, "type_result with qualified name");
}

#[test]
fn test_metadata_reference_with_qualified_name() {
    let input = "package T { @MyPackage::MyMetadata part x; }";
    assert_parses_sysml(input, "metadata_reference with qualified name");
}

// ============================================================================
// Qualified Name Tests
// ============================================================================

#[test]
fn test_qualified_name_with_unicode_theta_simple() {
    assert_rule_parses(
        Rule::QualifiedReferenceChain,
        "isq.'Θ'",
        "qualified name with Unicode theta"
    );
}

#[test]
fn test_qualified_name_with_unicode_theta_in_expression() {
    let input = wrap_in_attribute("isq.'Θ'");
    assert_parses_sysml(&input, "qualified name with Unicode theta as expression");
}

#[test]
fn test_unrestricted_name_with_unicode_theta() {
    assert_rule_parses(
        Rule::UnrestrictedName,
        "'Θ'",
        "quoted name with Unicode theta"
    );
}

#[test]
fn test_qualified_name_with_regular_identifiers_in_attribute_body() {
    let input = "package T { attribute pf { :>> quantity = isq.theta; } }";
    assert_parses_sysml(input, "attribute with regular qualified name in body assignment");
}

#[test]
fn test_qualified_name_with_quoted_name_in_attribute_body() {
    let input = "package T { attribute pf { :>> quantity = isq.'z'; } }";
    assert_parses_sysml(input, "attribute with quoted name in body assignment");
}

#[test]
fn test_qualified_name_with_unicode_theta_as_owned_expression() {
    let input = wrap_in_attribute("isq.'Θ'");
    assert_parses_sysml(&input, "qualified name with Unicode theta as owned_expression");
}

#[test]
fn test_qualified_name_with_unicode_theta_assignment() {
    let input = "package T { attribute pf { :>> quantity = isq.'Θ'; } }";
    assert_parses_sysml(input, "attribute with Unicode theta in body assignment");
}

// ============================================================================
// Number Literal Tests
// ============================================================================

#[rstest]
#[case("1E-24", "scientific notation with negative exponent")]
#[case("1E24", "scientific notation with positive exponent")]
#[case("1e-24", "scientific notation lowercase e")]
#[case("3.14", "decimal number")]
#[case("42", "integer")]
#[case(".5", "decimal starting with dot")]
fn test_number_literals(#[case] num: &str, #[case] desc: &str) {
    let input = wrap_in_attribute(num);
    assert_parses_sysml(&input, desc);
}

// ============================================================================
// Function Call Tests
// ============================================================================

#[test]
fn test_function_call_simple() {
    let input = wrap_in_attribute("foo()");
    assert_parses_sysml(&input, "simple function call");
}

#[test]
fn test_function_call_with_argument() {
    let input = wrap_in_attribute("allTrue(x)");
    assert_parses_sysml(&input, "function call with argument");
}

#[test]
fn test_function_call_with_dotted_argument() {
    let input = wrap_in_attribute("allTrue(derivedRequirements.result)");
    assert_parses_sysml(&input, "function call with dotted argument");
}

#[test]
fn test_implies_expression_with_function_call() {
    let input = wrap_in_constraint("originalRequirement.result implies allTrue(derivedRequirements.result)");
    assert_parses_sysml(&input, "implies expression with function call");
}

#[test]
fn test_invocation_expression_direct() {
    assert_rule_parses(
        Rule::InvocationExpression,
        "allTrue(x)",
        "invocation expression directly"
    );
}

#[test]
fn test_nested_function_call() {
    let input = wrap_in_attribute("allTrue(assumptions())");
    assert_parses_sysml(&input, "nested function call");
}

#[test]
fn test_outer_function_with_inner_call() {
    let input = wrap_in_attribute("allTrue(assumptions())");
    assert_parses_sysml(&input, "outer function with inner invocation");
}

#[test]
fn test_nested_invocation_expression() {
    let input = wrap_in_attribute("allTrue(assumptions())");
    assert_parses_sysml(&input, "nested invocation at classification_expression level");
}

#[test]
fn test_nested_invocation_equality() {
    let input = wrap_in_attribute("allTrue(assumptions())");
    assert_parses_sysml(&input, "nested invocation at equality_expression level");
}

#[test]
fn test_nested_invocation_classification() {
    let input = wrap_in_attribute("assumptions()");
    assert_parses_sysml(&input, "invocation at classification_expression level");
}

#[test]
fn test_nested_invocation_relational() {
    let input = wrap_in_attribute("assumptions()");
    assert_parses_sysml(&input, "invocation at relational_expression level");
}

#[test]
fn test_nested_invocation_base() {
    let input = wrap_in_attribute("assumptions()");
    assert_parses_sysml(&input, "invocation at base_expression level");
}

#[test]
fn test_invocation_after_constraint() {
    let input = wrap_in_attribute("assumptions()");
    assert_parses_sysml(&input, "invocation as owned_expression");
}

#[test]
fn test_argument_value_with_invocation() {
    let input = wrap_in_attribute("foo(assumptions())");
    assert_parses_sysml(&input, "invocation at argument_value level");
}

// ============================================================================
// Invocations with "as" prefix (identifier collision tests)
// ============================================================================

#[rstest]
#[case("allTrue(assumptions())", "assumptions starts with 'as'")]
#[case("allTrue(assertion())", "assertion starts with 'as'")]
#[case("allTrue(asdf())", "asdf starts with 'as'")]
#[case("foo(assumptions(), assertion(), asdf())", "multiple args starting with 'as'")]
fn test_invocations_starting_with_as_keyword(#[case] expr: &str, #[case] desc: &str) {
    let input = wrap_in_attribute(expr);
    assert_parses_sysml(&input, desc);
}

// ============================================================================
// "as" operator with proper word boundary
// ============================================================================

#[rstest]
#[case("x as Int", "simple cast")]
#[case("x as SysML::Usage", "cast to qualified name")]
#[case("foo() as MyType::SubType", "invocation result cast")]
#[case("causes as SysML::Usage", "exact case")]
fn test_as_operator_with_qualified_names(#[case] expr: &str, #[case] desc: &str) {
    let input = wrap_in_attribute(expr);
    assert_parses_sysml(&input, desc);
}

// ============================================================================
// "meta" operator with proper word boundary
// ============================================================================

#[rstest]
#[case("x meta Usage", "simple meta")]
#[case("x meta SysML::Usage", "meta with qualified name")]
#[case("multicausations meta SysML::Usage", "identifier starting with similar pattern")]
fn test_meta_operator_with_qualified_names(#[case] expr: &str, #[case] desc: &str) {
    let input = wrap_in_attribute(expr);
    assert_parses_sysml(&input, desc);
}

// ============================================================================
// Multiplicity Tests
// ============================================================================

#[test]
fn test_multiplicity_with_expression() {
    // Multiplicity with expression needs to be in a feature context
    let input = "package T { part x[nCauses]; }";
    assert_parses_sysml(input, "multiplicity with expression");
}

#[test]
fn test_multiplicity_with_range_expressions() {
    // Multiplicity with expression range needs to be in a feature context
    let input = "package T { part x[0..size(items)]; }";
    assert_parses_sysml(input, "multiplicity with expression range");
}

// ============================================================================
// Connector End Tests
// ============================================================================

#[test]
fn test_connector_end_with_multiplicity_and_chain() {
    let input = "package T { succession first [nCauses] causes.startShot then effects; }";
    assert_parses_sysml(input, "connector end with multiplicity and feature chain");
}

#[test]
fn test_connector_end_with_multiplicity_and_identifier() {
    let input = "package T { succession first [1] endpoint then target; }";
    assert_parses_sysml(input, "connector end with multiplicity and identifier");
}

#[test]
fn test_connector_end_with_name_references() {
    let input = "package T { connection connect myEnd references source.port to target.port; }";
    assert_parses_sysml(input, "connector end with name and references");
}

#[test]
fn test_succession_with_multiplicity() {
    assert_rule_parses(
        Rule::SuccessionAsUsage,
        r#"succession causalOrdering first [nCauses] causes.startShot then [nEffects] effects {
            doc /* test */
        }"#,
        "succession with multiplicities"
    );
}

// ============================================================================
// Interface and Flow Usage Tests
// ============================================================================

#[test]
fn test_interface_usage_with_nonunique() {
    assert_rule_parses(
        Rule::InterfaceUsage,
        "abstract interface interfaces: Interface[0..*] nonunique :> connections { }",
        "interface usage with nonunique"
    );
}

#[test]
fn test_flow_usage_with_nonunique() {
    let input = "package T { abstract flow flows: Flow[0..*] nonunique :> messages, flowTransfers { } }";
    assert_parses_sysml(input, "flow usage with nonunique");
}

// ============================================================================
// Misc Expression Tests
// ============================================================================

#[test]
fn test_empty_tuple_expression() {
    let input = wrap_in_attribute("()");
    assert_parses_sysml(&input, "empty tuple expression");
}

#[test]
fn test_in_parameter_with_default_block() {
    let input = "package T { action def A { in whileTest default {true} { } } }";
    assert_parses_sysml(input, "in parameter with default block");
}

#[test]
fn test_ref_with_feature_chain_subsetting() {
    let input = "package T { part def P { ref :>> outgoingTransfersFromSelf :> interfacingPorts.incomingTransfersToSelf { } } }";
    assert_parses_sysml(input, "ref with feature chain subsetting");
}

#[test]
fn test_end_ref_with_name_only() {
    let input = "package T { connection def C { end ref source; } }";
    assert_parses_sysml(input, "end ref with name only");
}

// ============================================================================
// Invocation in calc body with constraints
// ============================================================================

#[test]
fn test_invocation_in_calc_body_with_constraints() {
    let input = r#"package T {
        calc def Test {
            constraint assumptions[0..*] :> constraintChecks, subperformances { }
            constraint constraints[0..*] :> constraintChecks, subperformances { }
            return result = allTrue(assumptions()) implies allTrue(constraints()) { }
        }
    }"#;
    assert_parses_sysml(input, "calc body with constraint declarations before return");
}

// ============================================================================
// Identifier Tests
// ============================================================================

#[rstest]
#[case("myVar")]
#[case("calculation1")]
#[case("result_value")]
#[case("InCamelCase")]
fn test_identifier_allows_valid_names(#[case] ident: &str) {
    assert_rule_parses(
        Rule::RegularName,
        ident,
        &format!("valid identifier '{}'", ident)
    );
}

#[test]
fn test_all_true_as_identifier() {
    assert_rule_parses(
        Rule::RegularName,
        "allTrue",
        "'allTrue' as identifier"
    );
}

// ============================================================================
// Return with Expressions
// ============================================================================

#[test]
fn test_return_with_equality_expression() {
    // Return with expression needs to be in calc context
    let input = "package T { calc def Test { return a == b; } }";
    assert_parses_sysml(input, "return with equality expression");
}

// ============================================================================
// Case Body Tests
// ============================================================================

#[test]
fn test_case_body_with_doc() {
    let input = r#"package T {
        case def Test {
            doc
            /*
             * A TradeStudy documentation
             */
        }
    }"#;
    assert_parses_sysml(input, "case body with doc");
}
