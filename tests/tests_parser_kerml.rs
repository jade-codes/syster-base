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

/// Test `doc` on separate line from block comment
/// From ImageMetadata.sysml:
/// ```
/// attribute type : String[0..1] {
///     doc
///     /* The MIME type ... */
/// }
/// ```
/// Issue: Parser expects block comment on same line as `doc` keyword
#[test]
fn test_doc_with_block_comment_on_next_line() {
    let input = r#"part def P {
        attribute type : String[0..1] {
            doc
            /*
             * The MIME type according to which the content should be interpreted.
             */
        }
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `type` keyword used as feature/attribute name
/// From ImageMetadata.sysml: `attribute type : String[0..1]`
/// `type` is a KerML keyword but commonly used as a feature name
#[test]
fn test_type_keyword_as_feature_name() {
    let input = r#"part def P {
        attribute type : String;
        feature type : Integer;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// Contextual Keywords as Names (TODO Item #1)
// =============================================================================

/// Test `frame` keyword used as parameter name
/// From SpatialFrames.kerml: `in frame : SpatialFrame[1] default defaultFrame;`
#[test]
fn test_frame_keyword_as_parameter_name() {
    let input = r#"function PositionOf {
        in point : Point[1];
        in time : Real[1];
        in frame : SpatialFrame[1];
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `frame` with default value - exact pattern from SpatialFrames.kerml
/// `in frame : SpatialFrame[1] default defaultFrame;`
#[test]
fn test_frame_keyword_with_default_value() {
    let input = r#"function PositionOf {
        in frame : SpatialFrame[1] default defaultFrame;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `entry` keyword used as step name
/// From StatePerformances.kerml: `step entry[1];`
#[test]
fn test_entry_keyword_as_step_name() {
    let input = r#"behavior StatePerformance {
        step entry[1];
        step do[1];
        step exit[1];
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `do` keyword used as step name
/// From StatePerformances.kerml: `step do[1] subsets middle;`
#[test]
fn test_do_keyword_as_step_name() {
    let input = r#"behavior B {
        step do[1] subsets middle;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `exit` keyword used as step name  
/// From StatePerformances.kerml: `step exit[1];`
#[test]
fn test_exit_keyword_as_step_name() {
    let input = r#"behavior B {
        step exit[1];
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `accept` keyword used as step name
/// From TransitionPerformances.kerml: `step accept action accepter`
#[test]
fn test_accept_keyword_as_step_name() {
    let input = r#"behavior TransitionPerformance {
        step accept;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `done` keyword used as member name
/// From ModelingMetadata.sysml: `done { doc /* ... */ }`
#[test]
fn test_done_keyword_as_member_name() {
    let input = r#"enum def StatusKind {
        enum done;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test `done` as standalone block with doc - exact pattern from ModelingMetadata.sysml
/// This tests the `done { doc /* */ }` pattern in an enum
#[test]
fn test_done_block_with_doc() {
    let input = r#"enum def StatusKind {
        done {
            doc /* Status is done */
        }
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// Subset with Feature Chain (TODO Item #2)
// =============================================================================

/// Test subset with feature chain - from Occurrences.kerml
/// `subset laterOccurrence.successors subsets earlierOccurrence.successors;`
#[test]
fn test_subset_with_feature_chain_standalone() {
    let input = r#"assoc HappensBefore {
        feature earlierOccurrence: Occurrence[1];
        feature laterOccurrence: Occurrence[1];
        subset laterOccurrence.successors subsets earlierOccurrence.successors;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// Comma-Separated Subsets (TODO Item #3)
// =============================================================================

/// Test multiple subset targets - from SysML.sysml
/// `subsets step, usage` pattern
#[test]
fn test_multiple_subsets_targets() {
    let input = r#"metadata def ActionDefinition {
        derived ref item x : Usage[0..*] subsets step, usage;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// Succession with `all` (Anonymous succession patterns)
// =============================================================================

/// Test `succession all` pattern - from StatePerformances.kerml
/// `private succession all [*] acceptable then [*] guard;`
#[test]
fn test_succession_all_anonymous() {
    let input = r#"behavior B {
        private succession all [*] acceptable then [*] guard;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test anonymous succession with multiplicity
/// `private succession [*] guard then [1] exit;`
#[test]
fn test_succession_anonymous_with_multiplicity() {
    let input = r#"behavior B {
        private succession [*] guard then [1] exit;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// Complex Feature Declarations (from SysML.sysml)
// =============================================================================

/// Test derived ref item with ordered, subsets, and usage subsets
/// From SysML.sysml: `derived ref item 'action' : ActionUsage[0..*] ordered subsets step, usage subsets Metadata::metadataItems;`
#[test]
fn test_derived_ref_item_with_usage_subsets() {
    let input = r#"metadata def ActionDefinition {
        derived ref item x : Usage[0..*] ordered subsets step, usage subsets Metadata::metadataItems;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Simpler version - just the comma-separated modifiers
#[test]
fn test_feature_with_comma_separated_modifiers() {
    let input = r#"package P {
        feature x : T subsets a, usage subsets b;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test metadata def with multiple specializations
/// From SysML.sysml: `metadata def ActionDefinition specializes Behavior, OccurrenceDefinition`
#[test]
fn test_metadata_def_with_multiple_specializations() {
    let input = r#"metadata def ActionDefinition specializes Behavior, OccurrenceDefinition {}"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test comma-separated specialization targets
#[test]
fn test_specializes_comma_separated() {
    let input = r#"class C specializes A, B, C {}"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

/// Test the exact pattern from SysML.sysml line 15-16
/// `subsets nestedReference, parameter subsets Metadata::metadataItems`
#[test]
fn test_subsets_with_additional_feature_subsets() {
    let input = r#"metadata def Test {
        derived ref item x : T[1..1] subsets nestedReference, parameter subsets Metadata::metadataItems;
    }"#;
    assert!(parses_kerml(input), "Failed to parse: {}", input);
}

// =============================================================================
// SysML vs KerML parsing tests
// =============================================================================

use syster::parser::parse_sysml;

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some() && parsed.errors.is_empty()
}

/// Test the exact pattern from SysML.sysml line 19-20 using SysML parser
#[test]
fn test_sysml_metadata_def_with_subsets_pattern() {
    let input = r#"metadata def ActionDefinition specializes Behavior, OccurrenceDefinition {
        derived ref item 'action' : ActionUsage[0..*] ordered subsets step, usage subsets Metadata::metadataItems;
    }"#;
    assert!(
        parses_sysml(input),
        "Failed to parse with SysML parser: {}",
        input
    );
}

/// Simpler test - just metadata def with specialization, no body items
#[test]
fn test_sysml_metadata_def_simple() {
    let input = r#"metadata def ActionDefinition specializes Behavior {}"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse with SysML parser: {}",
        input
    );
}

/// Test metadata def with multiple specializations
#[test]
fn test_sysml_metadata_def_multiple_specializes() {
    let input = r#"metadata def ActionDefinition specializes Behavior, OccurrenceDefinition {}"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse with SysML parser: {}",
        input
    );
}

/// Test metadata def with derived ref item in body
#[test]
fn test_sysml_metadata_def_with_derived_ref_item() {
    let input = r#"metadata def ActionDefinition specializes Behavior {
        derived ref item x : Usage[0..*];
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse with SysML parser: {}",
        input
    );
}

/// Test with ordered and subsets
#[test]
fn test_sysml_metadata_def_with_ordered_subsets() {
    let input = r#"metadata def ActionDefinition specializes Behavior {
        derived ref item x : Usage[0..*] ordered subsets step;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test subsets step (step is a keyword)
#[test]
fn test_sysml_subsets_step_keyword() {
    let input = r#"metadata def Test {
        derived ref item x : Usage subsets step;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test inv inside struct body (Clocks.kerml pattern)
#[test]
fn test_kerml_inv_inside_struct() {
    let input = r#"struct Clock {
        feature currentTime : NumericalValue[1];
        inv timeFlowConstraint {
            snapshots->forAll{in s : Clock; true}
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test inv inside function body (TimeOf function in Clocks.kerml)
#[test]
fn test_kerml_inv_inside_function() {
    let input = r#"function TimeOf {
        in o : Occurrence[1];
        return timeInstant : NumericalValue[1];
        inv startTimeConstraint {
            timeInstant == TimeOf(o.startShot, clock)
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test var feature (Clocks.kerml line 40 pattern)
#[test]
fn test_kerml_var_feature() {
    let input = r#"struct Clock {
        var feature currentTime : NumericalValue[1];
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test var feature with body (Clocks.kerml line 40 full pattern)
#[test]
fn test_kerml_var_feature_with_body() {
    let input = r#"struct Clock {
        var feature currentTime : NumericalValue[1] {
            doc /* comment */
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test Clock struct with var feature and inv (Clocks.kerml lines 29-58)
#[test]
fn test_kerml_clock_struct_full() {
    let input = r#"abstract struct Clock {
        private thisClock : Clock :>> self;
        
        var feature currentTime : NumericalValue[1] {
            doc /* comment */
        }
                        
        inv timeFlowConstraint {
            snapshots->forAll{in s : Clock; 
                TimeOf(s, thisClock) == s.currentTime
            }
        }       
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse: {}", input);
}

/// Test Clocks.kerml partial file to find where error starts
#[test]
fn test_kerml_clocks_partial() {
    let input = r#"standard library package Clocks {
    private import ScalarValues::NumericalValue;
    private import Occurrences::Occurrence;
    
    private struct UniversalClockLife[1] :> Clock, Life {
        doc /* comment */
    }
    
    feature universalClock : UniversalClockLife[1] {
        doc /* comment */
    }
    
    abstract struct Clock {
        private thisClock : Clock :>> self;
        
        var feature currentTime : NumericalValue[1] {
            doc /* comment */
        }
                        
        inv timeFlowConstraint {
            snapshots->forAll{in s : Clock; 
                TimeOf(s, thisClock) == s.currentTime
            }
        }       
    }
}"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse");
}

/// Test function parameter with default value (Clocks.kerml line 70)
#[test]
fn test_kerml_function_param_default() {
    let input = r#"function TimeOf {
        in clock : Clock[1] default localClock;
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse");
}

/// Test the actual Clocks.kerml file from stdlib
#[test]
fn test_kerml_clocks_file() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/sysml.library/Kernel Libraries/Kernel Semantic Library/Clocks.kerml"
    );
    let content = std::fs::read_to_string(path).expect("Failed to read Clocks.kerml");
    let parsed = parse_kerml(&content);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse Clocks.kerml");
}

/// Test comma-separated relationship modifiers (SysML.sysml line 20 pattern)
/// Pattern: `subsets step, usage subsets Metadata::metadataItems`
/// This is TWO separate relationship modifiers separated by comma
#[test]
fn test_sysml_comma_separated_modifiers() {
    let input = r#"metadata def ActionDefinition {
        derived ref item 'action' : ActionUsage[0..*] ordered subsets step, usage subsets Metadata::metadataItems;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse");
}

/// Test comma-separated redefines/subsets (SysML.sysml line 24 pattern)
/// Pattern: `redefines behavior, occurrenceDefinition subsets Metadata::metadataItems`
#[test]
fn test_sysml_comma_redefines_then_subsets() {
    let input = r#"metadata def ActionUsage {
        derived ref item actionDefinition : Behavior[0..*] ordered redefines behavior, occurrenceDefinition subsets Metadata::metadataItems;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse");
}

/// Minimal test for redefines A, B pattern
#[test]
fn test_sysml_redefines_comma_list() {
    // Just: redefines A, B (two items, same relationship)
    let input = r#"metadata def Test {
        item x redefines a, b;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse redefines comma list"
    );
}

/// Test redefines A, B subsets C (the full pattern)
#[test]
fn test_sysml_redefines_comma_then_subsets() {
    let input = r#"metadata def Test {
        item x redefines a, b subsets c;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse redefines a, b subsets c"
    );
}

/// Test with ordered before redefines
#[test]
fn test_sysml_ordered_redefines_comma() {
    let input = r#"metadata def Test {
        item x ordered redefines a, b subsets c;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse ordered redefines a, b subsets c"
    );
}

/// Test with full ref item pattern
#[test]
fn test_sysml_derived_ref_item_pattern() {
    let input = r#"metadata def Test {
        derived ref item x : Behavior[0..*] ordered redefines a, b subsets c;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse derived ref item pattern"
    );
}

/// Test exact line from SysML.sysml line 24
#[test]
fn test_sysml_exact_line_24() {
    // Exact pattern from SysML.sysml line 24
    let input = r#"metadata def Test {
        derived ref item actionDefinition : Behavior[0..*] ordered redefines behavior, occurrenceDefinition subsets Metadata::metadataItems;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse exact line 24 pattern"
    );
}

/// Test specializes with comma-separated types
#[test]
fn test_sysml_specializes_comma_list() {
    let input = r#"metadata def Test specializes A, B {
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse specializes comma list"
    );
}

/// Test simpler comma-separated relationship - just the comma part
#[test]
fn test_sysml_comma_after_subsets() {
    // Simpler test: `subsets A, name subsets B`
    let input = r#"metadata def Test {
        item x subsets step, usage subsets items;
    }"#;
    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse");
}

/// Test inv inside struct body (from Clocks.kerml)
#[test]
fn test_inv_inside_struct() {
    // Test var feature with doc block first
    let var_feature_input = r#"struct Clock {
                var feature currentTime : NumericalValue[1] {
                        doc
                        /*
                         * A scalar time reference that advances over the lifetime of the Clock. 
                         */
                }
        }"#;
    eprintln!("Testing var feature with doc block alone:");
    let parsed = parse_kerml(var_feature_input);
    if !parsed.errors.is_empty() {
        eprintln!("var feature errors: {:?}", parsed.errors);
    } else {
        eprintln!("var feature OK");
    }

    // Test var feature followed by inv
    let combined_input = r#"struct Clock {
                var feature currentTime : NumericalValue[1] {
                        doc
                        /*
                         * A scalar time reference that advances over the lifetime of the Clock. 
                         */
                }

                inv timeFlowConstraint {
                        snapshots->forAll{in s : Clock; 
                                TimeOf(s, thisClock) == s.currentTime
                        }
                }
        }"#;
    eprintln!("Testing var feature + inv combined:");
    let parsed = parse_kerml(combined_input);
    if !parsed.errors.is_empty() {
        eprintln!("combined errors: {:?}", parsed.errors);
    } else {
        eprintln!("combined OK");
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse var feature + inv combined"
    );

    // With private member using :>> self and var feature
    let input = r#"abstract struct Clock {
                private thisClock : Clock :>> self;

                var feature currentTime : NumericalValue[1] {
                        doc
                        /*
                         * A scalar time reference that advances over the lifetime of the Clock. 
                         */
                }

                inv timeFlowConstraint {
                        snapshots->forAll{in s : Clock; 
                                TimeOf(s, thisClock) == s.currentTime
                        }
                }
        }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Full input errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse inv inside struct"
    );
}

/// Test the actual SysML.sysml file from stdlib
#[test]
fn test_sysml_sysml_file() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/sysml.library/Systems Library/SysML.sysml"
    );
    let content = std::fs::read_to_string(path).expect("Failed to read SysML.sysml");
    let parsed = parse_sysml(&content);
    if !parsed.errors.is_empty() {
        eprintln!(
            "Errors ({} total): {:?}",
            parsed.errors.len(),
            &parsed.errors[..std::cmp::min(5, parsed.errors.len())]
        );
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse SysML.sysml with {} errors",
        parsed.errors.len()
    );
}

/// Test full Clocks.kerml content inline
#[test]
fn test_clocks_kerml_inline() {
    let input = r#"standard library package Clocks {
	doc
	/*
	 * This package models Clocks that provide an advancing numerical reference 
	 * usable for quantifying the time of an Occurrence.
	 */

	private import ScalarValues::NumericalValue;
	private import ScalarValues::Real;
	private import Occurrences::Occurrence;
	private import Occurrences::Life;
	private import ControlFunctions::forAll;
	
	private struct UniversalClockLife[1] :> Clock, Life {
	    doc
	    /*
	     * UniversalClockLife is the classifier of the singleton Life of the universalClock.
	     */
	}
	
	feature universalClock : UniversalClockLife[1] {
		doc
		/*
		 * universalClock is a single Clock that can be used as a default universal
		 * time reference.
		 */
	}
	
	abstract struct Clock {
		doc
		/*
		 * A Clock provides a numerical currentTime that advances montonically
		 * over its lifetime. Clock is an abstract base Structure that can be
		 * specialized for different kinds of time quantification (e.g., discrete
		 * time, continuous time, time with units, etc.).
		 */
		 
		private thisClock : Clock :>> self;
		
		var feature currentTime : NumericalValue[1] {
			doc
			/*
			 * A scalar time reference that advances over the lifetime of the Clock. 
			 */
		}
						
		inv timeFlowConstraint {
			doc
			/*
			 * The currentTime of a snapshot of a Clock is equal to
			 * the TimeOf the snapshot relative to that Clock.
			 */
			
			snapshots->forAll{in s : Clock; 
				TimeOf(s, thisClock) == s.currentTime
			}
		}		
	}
	
	abstract function TimeOf {
		doc
		/*
		 * TimeOf returns a numerical timeInstant for a given Occurrence relative to
		 * a given Clock. The timeInstant is the time of the start of the Occurrence,
		 * which is considered to be synchronized with the snapshot of the Clock 
		 * with a currentTime equal to the returned timeInstant.
		 */
		
		in o : Occurrence[1];
		in clock : Clock[1] default localClock;
		return timeInstant : NumericalValue[1];
		
		 inv startTimeConstraint {
		 	doc
			/*
			 * The TimeOf an Occurrence is equal to the time of its start snapshot.
			 */
			 
		 	timeInstant == TimeOf(o.startShot, clock)
		 }	 

		inv timeOrderingConstraint {
			doc
			/*
			 * If one Occurrence happens before another, then the TimeOf the end
			 * snapshot of the first Occurrence is no greater than the TimeOf the 
			 * second Occurrence.
			 */
			
			o.predecessors->forAll{in p : Occurrence; 
				TimeOf(p.endShot, clock) <= timeInstant
			}
		}
				
		inv timeContinuityConstraint {
			doc
			/*
			 * If one Occurrence happens immediately before another, then the TimeOf 
			 * the end snapshot of the first Occurrence equals the TimeOf the second
			 * Occurrence.
			 */
		 
			o.immediatePredecessors->forAll{in p : Occurrence; 
				TimeOf(p.endShot, clock) == timeInstant
			}
		}				
	}
	
	function DurationOf {
		doc
		/*
		 * DurationOf returns the duration of a given Occurrence relative to a
		 * given Clock, which is equal to the TimeOf the end snapshot of the
		 * Occurrence minus the TimeOf its start snapshot.
		 */
		
		in o : Occurrence[1]; 
		in clock : Clock[1] default localClock;
		return duration : NumericalValue =
			TimeOf(o.endShot, clock) - TimeOf(o.startShot, clock);
	}
	
	struct BasicClock :> Clock {
		doc
		/*
		 * A BasicClock is a Clock whose currentTime is a Real number.
		 */
		
		var feature :>> currentTime : Real;
	}
	
	function BasicTimeOf :> TimeOf {
		doc
		/*
		 * BasicTimeOf returns the TimeOf an Occurrence as a Real number relative
		 * to a BasicClock.
		 */

		in o : Occurrence[1];
		in clock : BasicClock[1];
		return : Real[1];
	}
	
	function BasicDurationOf :> DurationOf {
		doc
		/*
		 * BasicDurationOf returns the DurationOf an Occurrence as a Real number relative
		 * to a BasicClock.
		 */
		
		in o : Occurrence[1];
		in clock : BasicClock[1];
		return : Real[1];
	}

}"#;
    eprintln!("Input length: {} bytes", input.len());

    // Log context around first error position (1276)
    if input.len() > 1300 {
        eprintln!("--- Context around position 1276 ---");
        eprintln!("Bytes 1250-1300: {:?}", &input[1250..1300]);
        eprintln!("Text 1250-1300: {}", &input[1250..1300]);
        eprintln!("Bytes 1270-1285: {:?}", &input.as_bytes()[1270..1285]);
    }

    // Find all occurrences of "inv" in the input
    for (i, _) in input.match_indices("inv") {
        let start = i.saturating_sub(30);
        let end = (i + 50).min(input.len());
        eprintln!(
            "Found 'inv' at position {}: ...{}...",
            i,
            &input[start..end]
        );
    }

    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors ({} total):", parsed.errors.len());
        for err in &parsed.errors {
            let start_pos: usize = err.range.start().into();
            let end_pos: usize = err.range.end().into();
            let start = start_pos.saturating_sub(20);
            let end = (end_pos + 20).min(input.len());
            eprintln!("  {:?}", err);
            eprintln!("  Context: ...{}...", &input[start..end]);
        }
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse Clocks.kerml inline with {} errors",
        parsed.errors.len()
    );
}

// =============================================================================
// STDLIB PARSE FAILURES - Remaining issues to fix
// =============================================================================

/// Test feature chain subsetting (Occurrences.kerml pattern)
/// Pattern: `subset laterOccurrence.successors subsets earlierOccurrence.successors;`
/// This is a standalone subset relationship between two feature chains
#[test]
fn test_feature_chain_subsetting() {
    // Test simple subset relationship first
    let simple = r#"struct A { subset a subsets b; }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple subset errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse simple subset");

    // Test subset with feature chain (dot notation) - this is the core issue
    let with_chain = r#"struct A { subset a.b subsets c; }"#;
    let parsed = parse_kerml(with_chain);
    if !parsed.errors.is_empty() {
        eprintln!("Subset with chain errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse subset with feature chain"
    );

    // Full example from Occurrences.kerml
    let input = r#"assoc HappensBefore {
        feature earlierOccurrence: Occurrence[1] subsets that;
        feature laterOccurrence: Occurrence[1] subsets self;
        subset laterOccurrence.successors subsets earlierOccurrence.successors;
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Full example errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse feature chain subsetting"
    );
}

/// Test member feature (KerML.kerml pattern)
/// Pattern: `member feature 'private' : VisibilityKind[1];`
/// The 'member' keyword before 'feature'
#[test]
fn test_member_feature() {
    // Test basic member feature
    let simple = r#"struct A { member feature x; }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple member feature errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse simple member feature"
    );

    // Test member feature with unrestricted name
    let with_name = r#"struct A { member feature 'private'; }"#;
    let parsed = parse_kerml(with_name);
    if !parsed.errors.is_empty() {
        eprintln!("Member feature with name errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse member feature with unrestricted name"
    );

    // Full example from KerML.kerml
    let input = r#"datatype VisibilityKind {
        member feature 'private' : VisibilityKind[1];
        member feature 'protected' : VisibilityKind[1];
        member feature 'public' : VisibilityKind[1];
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Full member feature errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse member feature");
}

/// Test end with feature (TransitionPerformances.kerml pattern)
/// Pattern: `end guardedLink [0..1] feature constrainedHBLink: HappensBefore;`
/// The 'end' keyword followed by name, multiplicity, then 'feature'
#[test]
fn test_end_with_feature() {
    // First, test simple end feature
    let simple = r#"assoc A { end myEnd; }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple end errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse simple end");

    // Test end with multiplicity
    let with_mult = r#"assoc A { end myEnd [0..1]; }"#;
    let parsed = parse_kerml(with_mult);
    if !parsed.errors.is_empty() {
        eprintln!("End with mult errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse end with multiplicity"
    );

    // Test end with feature keyword
    let with_feature = r#"assoc A { end myEnd feature nested; }"#;
    let parsed = parse_kerml(with_feature);
    if !parsed.errors.is_empty() {
        eprintln!("End with feature errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse end with feature keyword"
    );

    // Test end with multiplicity and feature keyword
    let full = r#"assoc A { end myEnd [0..1] feature nested: Type; }"#;
    let parsed = parse_kerml(full);
    if !parsed.errors.is_empty() {
        eprintln!("Full pattern errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse end with mult and feature"
    );

    // Test end with type shorthand (e.g., `end bool x`)
    let end_with_type = r#"assoc A { end bool x; }"#;
    let parsed = parse_kerml(end_with_type);
    if !parsed.errors.is_empty() {
        eprintln!("End with type shorthand errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse end with type shorthand (end bool x)"
    );

    // Test original pattern from TransitionPerformances.kerml
    let input = r#"assoc struct TPCGuardConstraint {
        end guardedLink [0..1] feature constrainedHBLink: HappensBefore;
        end bool constrainedGuard;
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Original pattern errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse original TransitionPerformances pattern"
    );
}

/// Test binding of pattern (Transfers.kerml pattern)
/// Pattern: `private binding instant[instantNum] of [0..1] startShot = [0..1] endShot`
/// Binding with 'of' keyword and multiplicities
#[test]
fn test_binding_of_pattern() {
    // Minimal reproduction from Transfers.kerml line 59
    let input = r#"behavior Transfer {
        private binding instant[1] of [0..1] startShot = [0..1] endShot;
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse binding of pattern"
    );
}

/// Test anonymous feature redefines with default (StatePerformances.kerml pattern)
/// Pattern: `feature redefines isRunToCompletion default this.isRunToCompletion;`
/// Anonymous feature that just redefines another with a default value
#[test]
fn test_anonymous_feature_redefines_default() {
    let input = r#"behavior StatePerformance {
        feature redefines isRunToCompletion default this.isRunToCompletion;
        feature redefines runToCompletionScope default this.runToCompletionScope;
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse anonymous feature redefines with default"
    );
}

/// Test anonymous feature redefines (FeatureReferencingPerformances.kerml pattern)
/// Pattern: `feature redefines monitoredFeature;`
/// Anonymous feature that just redefines another
#[test]
fn test_anonymous_feature_redefines() {
    let input = r#"behavior B {
        feature monitoredFeature : Anything[*];
        feature beforeTimeSlice {
            feature redefines monitoredFeature;
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse anonymous feature redefines"
    );
}

/// Test naked if expression in function body (CollectionFunctions.kerml pattern)
/// Pattern: `private function index { in arr: Array[1]; if i <= 1? 1 else 2 }`
/// If expression as the body of a function (not assigned to a feature)
#[test]
fn test_naked_if_expression_in_function() {
    // Test simple if expression in function body
    let simple = r#"function f { if true? 1 else 2 }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple if errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse simple if in function"
    );

    // Test if with parameter
    let with_param = r#"function f { in x: Boolean; if x? 1 else 2 }"#;
    let parsed = parse_kerml(with_param);
    if !parsed.errors.is_empty() {
        eprintln!("If with param errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse if with parameter"
    );

    // Full example from CollectionFunctions.kerml
    let input = r#"function arrayElement {
        private function index { in arr: Array[1]; in i : Natural; in indexes : Positive[1..*];
            if i <= 1? indexes#(1) else arr.dimensions#(i)
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Full example errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse naked if expression in function"
    );
}

/// Test private function with if expression (CollectionFunctions.kerml pattern)
/// Pattern: `private function index { ... if cond? a else b }`
#[test]
fn test_private_function_with_if() {
    let input = r#"function arrayElement {
        private function index { in arr: Array[1]; in i : Natural; in indexes : Positive[1..*];
            if i <= 1? indexes#(1) else arr.dimensions#(i) * (index(arr, i-1, indexes) - 1) + indexes#(i)
        }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse private function with if expression"
    );
}

/// Test inv with allTrue (TransitionPerformances.kerml pattern)
/// Pattern: `private inv { allTrue(constrainedGuard()) }`
#[test]
fn test_inv_with_alltrue() {
    let input = r#"assoc struct TPCGuardConstraint {
        end bool constrainedGuard;
        private inv { allTrue(constrainedGuard()) }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse inv with allTrue");
}

/// Test inv with implies (StatePerformances.kerml pattern)
/// Pattern: `inv { isRunToCompletion implies ... }`
#[test]
fn test_inv_with_implies() {
    let input = r#"behavior StatePerformance {
        feature isRunToCompletion : Boolean;
        inv { isRunToCompletion implies true }
    }"#;
    let parsed = parse_kerml(input);
    if !parsed.errors.is_empty() {
        eprintln!("Errors: {:?}", parsed.errors);
    }
    assert!(parsed.errors.is_empty(), "Failed to parse inv with implies");
}

/// Test multiplicity after redefines (FeatureReferencingPerformances.kerml pattern)
/// Pattern: `feature x redefines y [*] nonunique;`
#[test]
fn test_multiplicity_after_redefines() {
    // Simple case
    let simple = r#"struct A { feature x redefines y [*]; }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse simple multiplicity after redefines"
    );

    // With nonunique
    let with_nonunique = r#"struct A { feature x redefines y [*] nonunique; }"#;
    let parsed = parse_kerml(with_nonunique);
    if !parsed.errors.is_empty() {
        eprintln!("With nonunique errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse multiplicity after redefines with nonunique"
    );
}

/// Test feature inside function body (StatePerformances.kerml pattern)
/// Pattern: `function f { in x; feature y: T = expr; return z; }`
#[test]
fn test_feature_in_function_body() {
    // Simple feature in function body
    let simple = r#"function f { feature x; }"#;
    let parsed = parse_kerml(simple);
    if !parsed.errors.is_empty() {
        eprintln!("Simple feature errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse simple feature in function body"
    );
    eprintln!("Simple feature OK");

    // Feature with typing - THIS IS FAILING
    let with_typing = r#"function f { feature x: Integer; }"#;
    let parsed = parse_kerml(with_typing);
    if !parsed.errors.is_empty() {
        eprintln!("Feature with typing errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse feature with typing in function body"
    );

    // Full pattern from StatePerformances.kerml
    let full = r#"function allSubstatePerformances {
        in p : Performance [1];
        feature substatePerformances: StatePerformance [*] = 1;
        return : StatePerformance [*] = 2;
    }"#;
    let parsed = parse_kerml(full);
    if !parsed.errors.is_empty() {
        eprintln!("Full function errors: {:?}", parsed.errors);
    }
    assert!(
        parsed.errors.is_empty(),
        "Failed to parse full function pattern"
    );
}

/// Test then private action (Actions.sysml pattern)
/// Pattern: `then private action whileLoop ...`
#[test]
fn test_then_private_action() {
    // Bottom-up triage: start simple, build up

    // 1. Basic assign
    let t1 = r#"action def A { assign x := 1; }"#;
    let p1 = parse_sysml(t1);
    eprintln!("T1 (basic assign): {:?}", p1.errors);
    assert!(p1.errors.is_empty(), "T1 failed");

    // 2. assign var (var is a keyword!)
    let t2 = r#"action def A { assign var := 1; }"#;
    let p2 = parse_sysml(t2);
    eprintln!("T2 (assign var): {:?}", p2.errors);
    assert!(p2.errors.is_empty(), "T2 failed - assign var");

    // 3. while with assign inside
    let t3 = r#"action def A { while true { assign x := 1; } }"#;
    let p3 = parse_sysml(t3);
    eprintln!("T3 (while with assign): {:?}", p3.errors);
    assert!(p3.errors.is_empty(), "T3 failed");

    // 4. while with assign var inside
    let t4 = r#"action def A { while true { assign var := 1; } }"#;
    let p4 = parse_sysml(t4);
    eprintln!("T4 (while with assign var): {:?}", p4.errors);
    assert!(p4.errors.is_empty(), "T4 failed - while with assign var");

    // Full pattern once basics pass
    let input = r#"action def ForLoop {
        private action initialization
            assign index := 1;
        then private action whileLoop
            while index <= size(seq) {
                assign var := seq#(index);
                then perform body;
                then assign index := index + 1;
            }
    }"#;

    let parsed = parse_sysml(input);
    if !parsed.errors.is_empty() {
        for err in &parsed.errors {
            let start: usize = err.range.start().into();
            let end: usize = err.range.end().into();
            let ctx_start = start.saturating_sub(20);
            let ctx_end = (end + 20).min(input.len());
            eprintln!("Error: {:?}", err);
            eprintln!("  At chars {}-{}: [{}]", start, end, &input[start..end]);
            eprintln!("  Context: [{}]", &input[ctx_start..ctx_end]);
        }
    }
    assert!(parsed.errors.is_empty(), "Failed to parse full pattern");
}
