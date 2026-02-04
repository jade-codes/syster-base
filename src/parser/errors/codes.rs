//! Error code definitions for parser diagnostics
//!
//! Error codes follow a naming convention: E{category}{number}
//! - E01xx: Lexical errors (invalid tokens)
//! - E02xx: Structural errors (braces, semicolons)
//! - E03xx: Declaration errors (definitions, usages)
//! - E04xx: Expression errors
//! - E05xx: Import/namespace errors
//! - E09xx: Generic/fallback errors

use std::fmt;

/// Error codes for parser diagnostics
///
/// Each error code represents a specific category of parse error,
/// enabling filtering, documentation, and IDE integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // =========================================================================
    // E01xx: Lexical errors (invalid tokens)
    // =========================================================================
    /// Invalid or unexpected character in source
    E0101,
    /// Unterminated string literal
    E0102,
    /// Unterminated block comment
    E0103,
    /// Invalid numeric literal
    E0104,

    // =========================================================================
    // E02xx: Structural errors (braces, semicolons, delimiters)
    // =========================================================================
    /// Missing semicolon
    E0201,
    /// Unclosed brace `{`
    E0202,
    /// Unclosed parenthesis `(`
    E0203,
    /// Unclosed bracket `[`
    E0204,
    /// Unexpected closing delimiter
    E0205,
    /// Empty body where content expected
    E0206,
    /// Mismatched delimiters
    E0207,

    // =========================================================================
    // E03xx: Declaration errors (definitions, usages)
    // =========================================================================
    /// Missing identifier/name
    E0301,
    /// Missing `def` keyword for definition
    E0302,
    /// Invalid definition prefix combination
    E0303,
    /// Unexpected token in definition body
    E0304,
    /// Missing type annotation
    E0305,
    /// Invalid usage declaration
    E0306,
    /// Missing body (neither `;` nor `{`)
    E0307,

    // =========================================================================
    // E04xx: Expression errors
    // =========================================================================
    /// Invalid expression
    E0401,
    /// Missing operand in expression
    E0402,
    /// Invalid operator
    E0403,
    /// Unclosed function/method call
    E0404,
    /// Invalid argument in function call
    E0405,
    /// Missing expression where expected
    E0406,

    // =========================================================================
    // E05xx: Import/namespace errors
    // =========================================================================
    /// Invalid import path
    E0501,
    /// Missing package name
    E0502,
    /// Invalid alias declaration
    E0503,
    /// Invalid filter expression
    E0504,

    // =========================================================================
    // E06xx: Relationship errors
    // =========================================================================
    /// Invalid relationship target
    E0601,
    /// Missing relationship operand
    E0602,

    // =========================================================================
    // E07xx: Action/state machine errors
    // =========================================================================
    /// Invalid action body element
    E0701,
    /// Invalid state body element
    E0702,
    /// Invalid transition syntax
    E0703,
    /// Missing `then` in transition
    E0704,

    // =========================================================================
    // E08xx: Requirement/constraint errors
    // =========================================================================
    /// Invalid requirement body element
    E0801,
    /// Invalid constraint expression
    E0802,

    // =========================================================================
    // E09xx: Generic/fallback errors
    // =========================================================================
    /// Unexpected token in current context
    E0901,
    /// Expected a specific token
    E0902,
    /// Internal parser error
    E0999,
}

impl ErrorCode {
    /// Get the string representation of the error code (e.g., "E0201")
    pub fn as_str(&self) -> &'static str {
        match self {
            // Lexical
            Self::E0101 => "E0101",
            Self::E0102 => "E0102",
            Self::E0103 => "E0103",
            Self::E0104 => "E0104",
            // Structural
            Self::E0201 => "E0201",
            Self::E0202 => "E0202",
            Self::E0203 => "E0203",
            Self::E0204 => "E0204",
            Self::E0205 => "E0205",
            Self::E0206 => "E0206",
            Self::E0207 => "E0207",
            // Declaration
            Self::E0301 => "E0301",
            Self::E0302 => "E0302",
            Self::E0303 => "E0303",
            Self::E0304 => "E0304",
            Self::E0305 => "E0305",
            Self::E0306 => "E0306",
            Self::E0307 => "E0307",
            // Expression
            Self::E0401 => "E0401",
            Self::E0402 => "E0402",
            Self::E0403 => "E0403",
            Self::E0404 => "E0404",
            Self::E0405 => "E0405",
            Self::E0406 => "E0406",
            // Import
            Self::E0501 => "E0501",
            Self::E0502 => "E0502",
            Self::E0503 => "E0503",
            Self::E0504 => "E0504",
            // Relationship
            Self::E0601 => "E0601",
            Self::E0602 => "E0602",
            // Action/state
            Self::E0701 => "E0701",
            Self::E0702 => "E0702",
            Self::E0703 => "E0703",
            Self::E0704 => "E0704",
            // Requirement
            Self::E0801 => "E0801",
            Self::E0802 => "E0802",
            // Generic
            Self::E0901 => "E0901",
            Self::E0902 => "E0902",
            Self::E0999 => "E0999",
        }
    }

    /// Get a short description of the error category
    pub fn category_description(&self) -> &'static str {
        match self {
            Self::E0101 | Self::E0102 | Self::E0103 | Self::E0104 => "lexical error",
            Self::E0201 | Self::E0202 | Self::E0203 | Self::E0204 | Self::E0205 | Self::E0206 | Self::E0207 => {
                "structural error"
            }
            Self::E0301 | Self::E0302 | Self::E0303 | Self::E0304 | Self::E0305 | Self::E0306 | Self::E0307 => {
                "declaration error"
            }
            Self::E0401 | Self::E0402 | Self::E0403 | Self::E0404 | Self::E0405 | Self::E0406 => "expression error",
            Self::E0501 | Self::E0502 | Self::E0503 | Self::E0504 => "import error",
            Self::E0601 | Self::E0602 => "relationship error",
            Self::E0701 | Self::E0702 | Self::E0703 | Self::E0704 => "action/state error",
            Self::E0801 | Self::E0802 => "requirement error",
            Self::E0901 | Self::E0902 | Self::E0999 => "syntax error",
        }
    }

    /// Get the default message template for this error code
    pub fn default_message(&self) -> &'static str {
        match self {
            // Lexical
            Self::E0101 => "invalid character",
            Self::E0102 => "unterminated string literal",
            Self::E0103 => "unterminated block comment",
            Self::E0104 => "invalid numeric literal",
            // Structural
            Self::E0201 => "missing semicolon",
            Self::E0202 => "unclosed brace",
            Self::E0203 => "unclosed parenthesis",
            Self::E0204 => "unclosed bracket",
            Self::E0205 => "unexpected closing delimiter",
            Self::E0206 => "empty body",
            Self::E0207 => "mismatched delimiters",
            // Declaration
            Self::E0301 => "missing identifier",
            Self::E0302 => "missing 'def' keyword",
            Self::E0303 => "invalid definition prefix",
            Self::E0304 => "unexpected token in definition",
            Self::E0305 => "missing type annotation",
            Self::E0306 => "invalid usage declaration",
            Self::E0307 => "missing body",
            // Expression
            Self::E0401 => "invalid expression",
            Self::E0402 => "missing operand",
            Self::E0403 => "invalid operator",
            Self::E0404 => "unclosed function call",
            Self::E0405 => "invalid argument",
            Self::E0406 => "expected expression",
            // Import
            Self::E0501 => "invalid import path",
            Self::E0502 => "missing package name",
            Self::E0503 => "invalid alias",
            Self::E0504 => "invalid filter expression",
            // Relationship
            Self::E0601 => "invalid relationship target",
            Self::E0602 => "missing relationship operand",
            // Action/state
            Self::E0701 => "invalid action body element",
            Self::E0702 => "invalid state body element",
            Self::E0703 => "invalid transition syntax",
            Self::E0704 => "missing 'then' keyword",
            // Requirement
            Self::E0801 => "invalid requirement body element",
            Self::E0802 => "invalid constraint expression",
            // Generic
            Self::E0901 => "unexpected token",
            Self::E0902 => "expected token",
            Self::E0999 => "internal parser error",
        }
    }

    /// Check if this is a structural error (delimiter-related)
    pub fn is_structural(&self) -> bool {
        matches!(
            self,
            Self::E0201
                | Self::E0202
                | Self::E0203
                | Self::E0204
                | Self::E0205
                | Self::E0206
                | Self::E0207
        )
    }

    /// Check if this is a recoverable error (parsing can continue)
    pub fn is_recoverable(&self) -> bool {
        !matches!(self, Self::E0999)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::E0201.as_str(), "E0201");
        assert_eq!(ErrorCode::E0901.as_str(), "E0901");
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(format!("{}", ErrorCode::E0201), "E0201");
    }

    #[test]
    fn test_error_code_default_message() {
        assert_eq!(ErrorCode::E0201.default_message(), "missing semicolon");
        assert_eq!(ErrorCode::E0202.default_message(), "unclosed brace");
    }

    #[test]
    fn test_error_code_category() {
        assert_eq!(ErrorCode::E0201.category_description(), "structural error");
        assert_eq!(ErrorCode::E0301.category_description(), "declaration error");
        assert_eq!(ErrorCode::E0401.category_description(), "expression error");
    }

    #[test]
    fn test_is_structural() {
        assert!(ErrorCode::E0201.is_structural());
        assert!(ErrorCode::E0202.is_structural());
        assert!(!ErrorCode::E0301.is_structural());
    }

    #[test]
    fn test_is_recoverable() {
        assert!(ErrorCode::E0201.is_recoverable());
        assert!(!ErrorCode::E0999.is_recoverable());
    }
}
