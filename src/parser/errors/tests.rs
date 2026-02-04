//! Integration tests for the errors module

use super::*;
use crate::parser::SyntaxKind;
use rowan::{TextRange, TextSize};

#[test]
fn test_error_module_exports() {
    // Verify all public types are accessible
    let _code = ErrorCode::E0201;
    let _severity = Severity::Error;
    let _context = ParseContext::TopLevel;

    let _err = SyntaxError::new(
        "test error",
        TextRange::empty(TextSize::new(0)),
        ErrorCode::E0901,
    );
}

#[test]
fn test_complete_error_workflow() {
    // Simulate a complete error creation workflow
    
    // 1. Detect unclosed brace at position 50
    let opening_brace_pos = TextRange::new(TextSize::new(10), TextSize::new(11));
    let error_pos = TextRange::empty(TextSize::new(50));
    
    // 2. Create error with context
    let err = SyntaxError::builder(ErrorCode::E0202)
        .message("unclosed '{' in package body")
        .range(error_pos)
        .hint("add '}' to close the package body")
        .related("opening brace here", opening_brace_pos)
        .build();
    
    // 3. Verify error properties
    assert_eq!(err.code, ErrorCode::E0202);
    assert!(err.message.contains("unclosed"));
    assert!(err.has_hint());
    assert!(err.has_related());
    assert_eq!(err.related[0].range, opening_brace_pos);
}

#[test]
fn test_error_code_exhaustiveness() {
    // Ensure all error codes have required properties
    let codes = [
        ErrorCode::E0101,
        ErrorCode::E0102,
        ErrorCode::E0103,
        ErrorCode::E0104,
        ErrorCode::E0201,
        ErrorCode::E0202,
        ErrorCode::E0203,
        ErrorCode::E0204,
        ErrorCode::E0205,
        ErrorCode::E0206,
        ErrorCode::E0207,
        ErrorCode::E0301,
        ErrorCode::E0302,
        ErrorCode::E0303,
        ErrorCode::E0304,
        ErrorCode::E0305,
        ErrorCode::E0306,
        ErrorCode::E0307,
        ErrorCode::E0401,
        ErrorCode::E0402,
        ErrorCode::E0403,
        ErrorCode::E0404,
        ErrorCode::E0405,
        ErrorCode::E0406,
        ErrorCode::E0501,
        ErrorCode::E0502,
        ErrorCode::E0503,
        ErrorCode::E0504,
        ErrorCode::E0601,
        ErrorCode::E0602,
        ErrorCode::E0701,
        ErrorCode::E0702,
        ErrorCode::E0703,
        ErrorCode::E0704,
        ErrorCode::E0801,
        ErrorCode::E0802,
        ErrorCode::E0901,
        ErrorCode::E0902,
        ErrorCode::E0999,
    ];

    for code in codes {
        // Every code must have a string representation
        assert!(!code.as_str().is_empty(), "code {:?} has empty as_str()", code);
        
        // Every code must have a default message
        assert!(
            !code.default_message().is_empty(),
            "code {:?} has empty default_message()",
            code
        );
        
        // Every code must have a category
        assert!(
            !code.category_description().is_empty(),
            "code {:?} has empty category_description()",
            code
        );
        
        // String representation should match pattern E####
        let s = code.as_str();
        assert!(s.starts_with('E'), "code {:?} doesn't start with E", code);
        assert_eq!(s.len(), 5, "code {:?} should be 5 chars", code);
    }
}

#[test]
fn test_context_recovery_tokens_validity() {
    // Ensure recovery tokens are valid for each context
    let contexts = [
        ParseContext::TopLevel,
        ParseContext::PackageBody,
        ParseContext::ActionBody,
        ParseContext::StateBody,
        ParseContext::RequirementBody,
        ParseContext::Expression,
        ParseContext::Multiplicity,
        ParseContext::Import,
    ];

    for ctx in contexts {
        let tokens = ctx.recovery_tokens();
        assert!(
            !tokens.is_empty(),
            "context {:?} has no recovery tokens",
            ctx
        );
        
        // Recovery tokens should include some kind of closing/terminating token
        let has_terminator = tokens.iter().any(|t| {
            matches!(
                t,
                SyntaxKind::R_BRACE
                    | SyntaxKind::R_PAREN
                    | SyntaxKind::R_BRACKET
                    | SyntaxKind::SEMICOLON
            )
        });
        assert!(
            has_terminator,
            "context {:?} should have at least one terminator in recovery tokens",
            ctx
        );
    }
}

#[test]
fn test_related_info_creation() {
    let info = RelatedInfo::new(
        "opened here",
        TextRange::new(TextSize::new(5), TextSize::new(6)),
    );
    
    assert_eq!(info.message, "opened here");
    assert_eq!(info.range.start(), TextSize::new(5));
    assert_eq!(info.range.end(), TextSize::new(6));
}

#[test]
fn test_error_severity_default() {
    let err = SyntaxError::new(
        "test",
        TextRange::empty(TextSize::new(0)),
        ErrorCode::E0901,
    );
    
    // Default severity should be Error
    assert_eq!(err.severity, Severity::Error);
    assert!(err.severity.is_error());
}

#[test]
fn test_error_at_offset() {
    let err = SyntaxError::at_offset("test", TextSize::new(42), ErrorCode::E0901);
    
    assert_eq!(err.range.start(), TextSize::new(42));
    assert_eq!(err.range.end(), TextSize::new(42)); // Empty range
}
