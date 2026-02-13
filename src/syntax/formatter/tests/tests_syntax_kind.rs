#![allow(clippy::unwrap_used)]

use crate::parser::{SyntaxKind, SysMLLanguage};
use crate::syntax::formatter::{FormatOptions, format_async};
use tokio_util::sync::CancellationToken;

// ============================================================================
// Tests for SysMLLanguage::kind_to_raw and kind_from_raw (#532, #533)
// ============================================================================
// These tests verify the trait implementation of rowan::Language for SysMLLanguage.
// We test both the conversion functions and their usage through the formatter API.

#[test]
fn test_kind_to_raw_via_formatter_simple_package() {
    // Tests that kind_to_raw correctly converts PackageKw, LBrace, RBrace, etc.
    let source = "package Test { }";
    let result = format_async(source, &FormatOptions::default(), &CancellationToken::new());
    assert!(result.is_some());
    assert!(result.unwrap().contains("package"));
}

#[test]
fn test_kind_to_raw_via_formatter_keywords() {
    // Tests that kind_to_raw handles various SysML keywords
    let source = "part def MyPart { }";
    let result = format_async(source, &FormatOptions::default(), &CancellationToken::new());
    assert!(result.is_some());
    let output = result.unwrap();
    assert!(output.contains("part"));
    assert!(output.contains("def"));
}

#[test]
fn test_kind_to_raw_via_formatter_punctuation() {
    // Tests that kind_to_raw handles punctuation tokens
    let source = "package A::B { }";
    let result = format_async(source, &FormatOptions::default(), &CancellationToken::new());
    assert!(result.is_some());
    assert!(result.unwrap().contains("::"));
}

#[test]
fn test_kind_to_raw_via_formatter_comments() {
    // Tests that kind_to_raw handles comment tokens
    let source = "// Comment\npackage Test { }";
    let result = format_async(source, &FormatOptions::default(), &CancellationToken::new());
    assert!(result.is_some());
    assert!(result.unwrap().contains("// Comment"));
}

#[test]
fn test_kind_to_raw_via_formatter_import() {
    // Tests that kind_to_raw handles import statements
    let source = "import Package::*;";
    let result = format_async(source, &FormatOptions::default(), &CancellationToken::new());
    assert!(result.is_some());
    assert!(result.unwrap().contains("import"));
}

#[test]
fn test_kind_to_raw_via_formatter_with_cancellation() {
    // Tests that the formatter (which uses kind_to_raw) respects cancellation
    let source = "package Test { }";
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = format_async(source, &FormatOptions::default(), &cancel);
    assert!(result.is_none());
}

// ============================================================================
// Direct tests for kind_to_raw and kind_from_raw (#532, #533)
// ============================================================================

/// Helper function to test round-trip conversion for multiple SyntaxKind variants
fn assert_roundtrip_conversion(kinds: &[SyntaxKind]) {
    for kind in kinds {
        let raw = <SysMLLanguage as rowan::Language>::kind_to_raw(*kind);
        let back = <SysMLLanguage as rowan::Language>::kind_from_raw(raw);
        assert_eq!(*kind, back, "Round-trip failed for {kind:?}");
    }
}

/// Test round-trip conversion for trivia tokens
#[test]
fn test_roundtrip_trivia_tokens() {
    assert_roundtrip_conversion(&[
        SyntaxKind::WHITESPACE,
        SyntaxKind::LINE_COMMENT,
        SyntaxKind::BLOCK_COMMENT,
    ]);
}

/// Test round-trip conversion for literal tokens
#[test]
fn test_roundtrip_literal_tokens() {
    assert_roundtrip_conversion(&[
        SyntaxKind::IDENT,
        SyntaxKind::INTEGER,
        SyntaxKind::STRING,
    ]);
}

/// Test round-trip conversion for punctuation tokens
#[test]
fn test_roundtrip_punctuation_tokens() {
    assert_roundtrip_conversion(&[
        SyntaxKind::L_BRACE,
        SyntaxKind::R_BRACE,
        SyntaxKind::L_BRACKET,
        SyntaxKind::R_BRACKET,
        SyntaxKind::L_PAREN,
        SyntaxKind::R_PAREN,
        SyntaxKind::SEMICOLON,
        SyntaxKind::COLON,
        SyntaxKind::COLON_COLON,
        SyntaxKind::DOT,
        SyntaxKind::COMMA,
        SyntaxKind::EQ,
        SyntaxKind::EQ_EQ,
        SyntaxKind::BANG_EQ,
        SyntaxKind::LT,
        SyntaxKind::GT,
        SyntaxKind::LT_EQ,
        SyntaxKind::GT_EQ,
        SyntaxKind::ARROW,
        SyntaxKind::AT,
        SyntaxKind::STAR,
        SyntaxKind::PLUS,
        SyntaxKind::MINUS,
        SyntaxKind::SLASH,
        SyntaxKind::PERCENT,
        SyntaxKind::CARET,
        SyntaxKind::TILDE,
        SyntaxKind::QUESTION,
        SyntaxKind::BANG,
        SyntaxKind::PIPE,
        SyntaxKind::AMP,
        SyntaxKind::HASH,
    ]);
}

/// Test round-trip conversion for common SysML keywords
#[test]
fn test_roundtrip_sysml_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::PACKAGE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::DEF_KW,
        SyntaxKind::IMPORT_KW,
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PORT_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::REQUIREMENT_KW,
        SyntaxKind::CONSTRAINT_KW,
        SyntaxKind::CONNECTION_KW,
        SyntaxKind::ALLOCATION_KW,
        SyntaxKind::INTERFACE_KW,
        SyntaxKind::FLOW_KW,
        SyntaxKind::USE_KW,
        SyntaxKind::VIEW_KW,
        SyntaxKind::VIEWPOINT_KW,
        SyntaxKind::RENDERING_KW,
        SyntaxKind::METADATA_KW,
        SyntaxKind::OCCURRENCE_KW,
        SyntaxKind::ANALYSIS_KW,
        SyntaxKind::VERIFICATION_KW,
        SyntaxKind::CONCERN_KW,
        SyntaxKind::ENUM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::CASE_KW,
        SyntaxKind::INDIVIDUAL_KW,
        SyntaxKind::END_KW,
    ]);
}

/// Test round-trip conversion for SysML modifier keywords
#[test]
fn test_roundtrip_sysml_modifier_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::REF_KW,
        SyntaxKind::CONST_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PROTECTED_KW,
        SyntaxKind::PUBLIC_KW,
    ]);
}

/// Test round-trip conversion for SysML relationship keywords
#[test]
fn test_roundtrip_sysml_relationship_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::SPECIALIZES_KW,
        SyntaxKind::SUBSETS_KW,
        SyntaxKind::REDEFINES_KW,
        SyntaxKind::TYPED_KW,
        SyntaxKind::REFERENCES_KW,
    ]);
}

/// Test round-trip conversion for SysML action and behavior keywords
#[test]
fn test_roundtrip_sysml_action_behavior_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::ASSERT_KW,
        SyntaxKind::ASSUME_KW,
        SyntaxKind::REQUIRE_KW,
        SyntaxKind::PERFORM_KW,
        SyntaxKind::EXHIBIT_KW,
        SyntaxKind::INCLUDE_KW,
        SyntaxKind::SATISFY_KW,
        SyntaxKind::ENTRY_KW,
        SyntaxKind::EXIT_KW,
        SyntaxKind::DO_KW,
        SyntaxKind::FORK_KW,
        SyntaxKind::JOIN_KW,
        SyntaxKind::MERGE_KW,
        SyntaxKind::DECIDE_KW,
        SyntaxKind::ACCEPT_KW,
        SyntaxKind::SEND_KW,
    ]);
}

/// Test round-trip conversion for SysML connection and reference keywords
#[test]
fn test_roundtrip_sysml_connection_reference_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::VIA_KW,
        SyntaxKind::TO_KW,
        SyntaxKind::FROM_KW,
        SyntaxKind::DEPENDENCY_KW,
        SyntaxKind::FILTER_KW,
        SyntaxKind::EXPOSE_KW,
        SyntaxKind::ALL_KW,
        SyntaxKind::FIRST_KW,
        SyntaxKind::HASTYPE_KW,
        SyntaxKind::ISTYPE_KW,
        SyntaxKind::AS_KW,
        SyntaxKind::META_KW,
    ]);
}

/// Test round-trip conversion for KerML keywords
#[test]
fn test_roundtrip_kerml_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::STRUCT_KW,
        SyntaxKind::CLASS_KW,
        SyntaxKind::DATATYPE_KW,
        SyntaxKind::ASSOC_KW,
        SyntaxKind::BEHAVIOR_KW,
        SyntaxKind::FUNCTION_KW,
        SyntaxKind::TYPE_KW,
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
        SyntaxKind::BINDING_KW,
        SyntaxKind::SUCCESSION_KW,
        SyntaxKind::CONNECTOR_KW,
        SyntaxKind::INV_KW,
        SyntaxKind::NONUNIQUE_KW,
        SyntaxKind::ORDERED_KW,
        SyntaxKind::IDENT,
    ]);
}

/// Test round-trip conversion for composite node kinds
#[test]
fn test_roundtrip_composite_nodes() {
    assert_roundtrip_conversion(&[
        SyntaxKind::SOURCE_FILE,
        SyntaxKind::PACKAGE,
        SyntaxKind::DEFINITION,
        SyntaxKind::USAGE,
        SyntaxKind::IMPORT,
        SyntaxKind::ALIAS_MEMBER,
        SyntaxKind::COMMENT_ELEMENT,
        SyntaxKind::NAME,
        SyntaxKind::NAMESPACE_BODY,
        SyntaxKind::RELATIONSHIP,
    ]);
}

/// Test round-trip conversion for special tokens
#[test]
fn test_roundtrip_special_tokens() {
    assert_roundtrip_conversion(&[SyntaxKind::ERROR, SyntaxKind::TOMBSTONE]);
}

/// Test that kind_to_raw produces unique raw values for different kinds
#[test]
fn test_kind_to_raw_uniqueness() {
    let kinds = [
        SyntaxKind::WHITESPACE,
        SyntaxKind::PACKAGE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::DEF_KW,
        SyntaxKind::L_BRACE,
        SyntaxKind::R_BRACE,
        SyntaxKind::IDENT,
        SyntaxKind::SOURCE_FILE,
        SyntaxKind::ERROR,
        SyntaxKind::TOMBSTONE,
    ];

    let mut raw_values = std::collections::HashSet::new();
    for kind in kinds {
        let raw = <SysMLLanguage as rowan::Language>::kind_to_raw(kind);
        assert!(
            raw_values.insert(raw.0),
            "Duplicate raw value {} for {:?}",
            raw.0,
            kind
        );
    }
}

/// Test boundary values - first and last enum variants
#[test]
fn test_roundtrip_boundary_values() {
    assert_roundtrip_conversion(&[
        SyntaxKind::WHITESPACE, // First variant (0)
        SyntaxKind::TOMBSTONE,        // Last variant
    ]);
}

/// Test that raw values preserve the numeric representation
#[test]
fn test_kind_to_raw_numeric_value() {
    // Test that the numeric value is preserved correctly
    let kind = SyntaxKind::WHITESPACE;
    let raw = <SysMLLanguage as rowan::Language>::kind_to_raw(kind);
    assert_eq!(
        raw.0, kind as u16,
        "Raw value should match enum discriminant"
    );

    let kind = SyntaxKind::PACKAGE_KW;
    let raw = <SysMLLanguage as rowan::Language>::kind_to_raw(kind);
    assert_eq!(
        raw.0, kind as u16,
        "Raw value should match enum discriminant"
    );
}

/// Test round-trip with boolean and control flow keywords
#[test]
fn test_roundtrip_boolean_control_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::TRUE_KW,
        SyntaxKind::FALSE_KW,
        SyntaxKind::NULL_KW,
        SyntaxKind::AND_KW,
        SyntaxKind::OR_KW,
        SyntaxKind::NOT_KW,
        SyntaxKind::XOR_KW,
        SyntaxKind::IMPLIES_KW,
        SyntaxKind::IF_KW,
        SyntaxKind::ELSE_KW,
        SyntaxKind::THEN_KW,
        SyntaxKind::LOOP_KW,
        SyntaxKind::WHILE_KW,
        SyntaxKind::UNTIL_KW,
        SyntaxKind::FOR_KW,
    ]);
}

/// Test round-trip for documentation and metadata keywords
#[test]
fn test_roundtrip_documentation_keywords() {
    assert_roundtrip_conversion(&[
        SyntaxKind::DOC_KW,
        SyntaxKind::COMMENT_KW,
        SyntaxKind::ABOUT_KW,
        SyntaxKind::REP_KW,
        SyntaxKind::LANGUAGE_KW,
        SyntaxKind::ALIAS_KW,
        SyntaxKind::IDENT,
        SyntaxKind::LIBRARY_KW,
        SyntaxKind::STANDARD_KW,
    ]);
}
