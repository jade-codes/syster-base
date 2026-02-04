//! Tests for error display and kind_to_name coverage

use syster::parser::{SyntaxKind, kind_to_name};

#[test]
fn test_all_keywords_have_specific_names() {
    // Keywords that should have specific names, not "keyword" fallback
    let keywords_with_expected_names = [
        (SyntaxKind::PART_KW, "'part'"),
        (SyntaxKind::ACTION_KW, "'action'"),
        (SyntaxKind::STATE_KW, "'state'"),
        (SyntaxKind::PACKAGE_KW, "'package'"),
        (SyntaxKind::IMPORT_KW, "'import'"),
        (SyntaxKind::DEF_KW, "'def'"),
        (SyntaxKind::REQUIREMENT_KW, "'requirement'"),
        (SyntaxKind::CONSTRAINT_KW, "'constraint'"),
        (SyntaxKind::ATTRIBUTE_KW, "'attribute'"),
        (SyntaxKind::PORT_KW, "'port'"),
        (SyntaxKind::ITEM_KW, "'item'"),
        (SyntaxKind::FLOW_KW, "'flow'"),
        (SyntaxKind::CONNECTION_KW, "'connection'"),
        (SyntaxKind::INTERFACE_KW, "'interface'"),
        (SyntaxKind::IF_KW, "'if'"),
        (SyntaxKind::ELSE_KW, "'else'"),
        (SyntaxKind::THEN_KW, "'then'"),
        (SyntaxKind::WHILE_KW, "'while'"),
        (SyntaxKind::FOR_KW, "'for'"),
        (SyntaxKind::ACCEPT_KW, "'accept'"),
        (SyntaxKind::SEND_KW, "'send'"),
        (SyntaxKind::ENTRY_KW, "'entry'"),
        (SyntaxKind::EXIT_KW, "'exit'"),
        (SyntaxKind::TRANSITION_KW, "'transition'"),
        (SyntaxKind::CLASS_KW, "'class'"),
        (SyntaxKind::STRUCT_KW, "'struct'"),
        (SyntaxKind::FUNCTION_KW, "'function'"),
        (SyntaxKind::TRUE_KW, "'true'"),
        (SyntaxKind::FALSE_KW, "'false'"),
        (SyntaxKind::NULL_KW, "'null'"),
    ];

    for (kind, expected) in keywords_with_expected_names {
        let actual = kind_to_name(kind);
        assert_eq!(
            actual, expected,
            "kind_to_name({:?}) returned '{}', expected '{}'",
            kind, actual, expected
        );
    }
}

#[test]
fn test_no_keyword_returns_generic_keyword() {
    // Test a sampling of keywords to ensure none return "keyword"
    let sample_keywords = [
        SyntaxKind::PACKAGE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::REQUIREMENT_KW,
        SyntaxKind::IMPORT_KW,
        SyntaxKind::CLASS_KW,
        SyntaxKind::FUNCTION_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PRIVATE_KW,
    ];

    for kind in sample_keywords {
        let name = kind_to_name(kind);
        assert_ne!(
            name, "keyword",
            "{:?} should have a specific name, not 'keyword'",
            kind
        );
    }
}

#[test]
fn test_punctuation_has_quoted_names() {
    let punctuation = [
        (SyntaxKind::SEMICOLON, "';'"),
        (SyntaxKind::COLON, "':'"),
        (SyntaxKind::COMMA, "','"),
        (SyntaxKind::L_BRACE, "'{'"),
        (SyntaxKind::R_BRACE, "'}'"),
        (SyntaxKind::L_PAREN, "'('"),
        (SyntaxKind::R_PAREN, "')'"),
        (SyntaxKind::L_BRACKET, "'['"),
        (SyntaxKind::R_BRACKET, "']'"),
        (SyntaxKind::COLON_GT, "':>'"),
        (SyntaxKind::COLON_GT_GT, "':>>'"),
    ];

    for (kind, expected) in punctuation {
        let actual = kind_to_name(kind);
        assert_eq!(
            actual, expected,
            "kind_to_name({:?}) returned '{}', expected '{}'",
            kind, actual, expected
        );
    }
}

#[test]
fn test_literals_have_descriptive_names() {
    assert_eq!(kind_to_name(SyntaxKind::IDENT), "identifier");
    assert_eq!(kind_to_name(SyntaxKind::INTEGER), "integer");
    assert_eq!(kind_to_name(SyntaxKind::DECIMAL), "number");
    assert_eq!(kind_to_name(SyntaxKind::STRING), "string");
}
