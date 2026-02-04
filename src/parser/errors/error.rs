//! Enhanced syntax error types
//!
//! Provides rich error information including:
//! - Error codes for categorization
//! - Severity levels
//! - Hints/suggestions for fixes
//! - Related source locations

use rowan::{TextRange, TextSize};

use super::codes::ErrorCode;
use super::context::ParseContext;

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Severity {
    /// A hard error that prevents valid parsing
    #[default]
    Error,
    /// A warning that doesn't prevent parsing
    Warning,
    /// An informational hint
    Hint,
}

impl Severity {
    /// Check if this is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Hint => "hint",
        }
    }
}

/// Related location information for an error
///
/// Used to point to related source locations, e.g.,
/// "unclosed brace opened here" pointing to the opening `{`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedInfo {
    /// Description of this related location
    pub message: String,
    /// Source range
    pub range: TextRange,
}

impl RelatedInfo {
    /// Create a new related info
    pub fn new(message: impl Into<String>, range: TextRange) -> Self {
        Self {
            message: message.into(),
            range,
        }
    }
}

/// A syntax error with enhanced information
///
/// Provides:
/// - Human-readable error message
/// - Source location (range)
/// - Categorized error code
/// - Severity level
/// - Optional hint for fixing
/// - Related source locations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    /// Human-readable error message
    pub message: String,
    /// Source location
    pub range: TextRange,
    /// Categorized error code
    pub code: ErrorCode,
    /// Error severity
    pub severity: Severity,
    /// Optional suggestion for fixing the error
    pub hint: Option<String>,
    /// Related source locations
    pub related: Vec<RelatedInfo>,
}

impl SyntaxError {
    /// Create a new syntax error with minimal information
    pub fn new(message: impl Into<String>, range: TextRange, code: ErrorCode) -> Self {
        Self {
            message: message.into(),
            range,
            code,
            severity: Severity::Error,
            hint: None,
            related: vec![],
        }
    }

    /// Create an error at a specific offset with zero-width range
    pub fn at_offset(message: impl Into<String>, offset: TextSize, code: ErrorCode) -> Self {
        Self::new(message, TextRange::empty(offset), code)
    }

    /// Create a builder for more complex error construction
    pub fn builder(code: ErrorCode) -> SyntaxErrorBuilder {
        SyntaxErrorBuilder::new(code)
    }

    /// Add a hint to this error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add related information
    pub fn with_related(mut self, info: RelatedInfo) -> Self {
        self.related.push(info);
        self
    }

    /// Set the severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Check if this error has a hint
    pub fn has_hint(&self) -> bool {
        self.hint.is_some()
    }

    /// Check if this error has related information
    pub fn has_related(&self) -> bool {
        !self.related.is_empty()
    }

    /// Format the error for display
    pub fn format(&self) -> String {
        let mut result = format!("{}: {}", self.code, self.message);
        if let Some(hint) = &self.hint {
            result.push_str(&format!("\n  hint: {}", hint));
        }
        result
    }
}

/// Builder for creating complex syntax errors
pub struct SyntaxErrorBuilder {
    code: ErrorCode,
    message: Option<String>,
    range: Option<TextRange>,
    severity: Severity,
    hint: Option<String>,
    related: Vec<RelatedInfo>,
}

impl SyntaxErrorBuilder {
    /// Create a new builder with an error code
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: None,
            range: None,
            severity: Severity::Error,
            hint: None,
            related: vec![],
        }
    }

    /// Set the error message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the source range
    pub fn range(mut self, range: TextRange) -> Self {
        self.range = Some(range);
        self
    }

    /// Set the offset (creates an empty range at that position)
    pub fn at_offset(mut self, offset: TextSize) -> Self {
        self.range = Some(TextRange::empty(offset));
        self
    }

    /// Set the severity
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Add a hint
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add related information
    pub fn related(mut self, message: impl Into<String>, range: TextRange) -> Self {
        self.related.push(RelatedInfo::new(message, range));
        self
    }

    /// Build the syntax error
    ///
    /// # Panics
    /// Panics if message or range are not set
    pub fn build(self) -> SyntaxError {
        SyntaxError {
            message: self
                .message
                .unwrap_or_else(|| self.code.default_message().to_string()),
            range: self
                .range
                .unwrap_or_else(|| TextRange::empty(TextSize::new(0))),
            code: self.code,
            severity: self.severity,
            hint: self.hint,
            related: self.related,
        }
    }
}

/// Helper function to create a context-aware error message
#[allow(dead_code)] // Will be used when context stack is integrated
pub fn format_context_error(found: &str, context: ParseContext, code: ErrorCode) -> SyntaxError {
    let message = format!(
        "unexpected {} {}—expected {}",
        found,
        context.description(),
        context.expected_description()
    );

    SyntaxError::builder(code).message(message).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_error_new() {
        let err = SyntaxError::new(
            "expected ';'",
            TextRange::new(TextSize::new(10), TextSize::new(11)),
            ErrorCode::E0201,
        );

        assert_eq!(err.message, "expected ';'");
        assert_eq!(err.code, ErrorCode::E0201);
        assert_eq!(err.severity, Severity::Error);
        assert!(err.hint.is_none());
        assert!(err.related.is_empty());
    }

    #[test]
    fn test_syntax_error_with_hint() {
        let err = SyntaxError::new(
            "expected ';'",
            TextRange::empty(TextSize::new(10)),
            ErrorCode::E0201,
        )
        .with_hint("add ';' at the end of the statement");

        assert!(err.has_hint());
        assert_eq!(
            err.hint.as_ref().unwrap(),
            "add ';' at the end of the statement"
        );
    }

    #[test]
    fn test_syntax_error_with_related() {
        let err = SyntaxError::new(
            "unclosed brace",
            TextRange::empty(TextSize::new(50)),
            ErrorCode::E0202,
        )
        .with_related(RelatedInfo::new(
            "opening brace here",
            TextRange::new(TextSize::new(10), TextSize::new(11)),
        ));

        assert!(err.has_related());
        assert_eq!(err.related.len(), 1);
        assert_eq!(err.related[0].message, "opening brace here");
    }

    #[test]
    fn test_syntax_error_builder() {
        let err = SyntaxError::builder(ErrorCode::E0201)
            .message("expected ';' after part definition")
            .range(TextRange::new(TextSize::new(10), TextSize::new(15)))
            .hint("add ';' at the end")
            .severity(Severity::Error)
            .related(
                "definition started here",
                TextRange::empty(TextSize::new(0)),
            )
            .build();

        assert_eq!(err.message, "expected ';' after part definition");
        assert_eq!(err.code, ErrorCode::E0201);
        assert!(err.has_hint());
        assert!(err.has_related());
    }

    #[test]
    fn test_syntax_error_builder_defaults() {
        let err = SyntaxError::builder(ErrorCode::E0201).build();

        // Should use default message from error code
        assert_eq!(err.message, "missing semicolon");
        assert_eq!(err.severity, Severity::Error);
    }

    #[test]
    fn test_severity() {
        assert!(Severity::Error.is_error());
        assert!(!Severity::Warning.is_error());
        assert!(!Severity::Hint.is_error());

        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Hint.as_str(), "hint");
    }

    #[test]
    fn test_format_error() {
        let err = SyntaxError::new(
            "expected ';'",
            TextRange::empty(TextSize::new(10)),
            ErrorCode::E0201,
        )
        .with_hint("add semicolon");

        let formatted = err.format();
        assert!(formatted.contains("E0201"));
        assert!(formatted.contains("expected ';'"));
        assert!(formatted.contains("hint"));
        assert!(formatted.contains("add semicolon"));
    }

    #[test]
    fn test_format_context_error() {
        let err = format_context_error("'}'", ParseContext::ActionBody, ErrorCode::E0901);

        assert!(err.message.contains("'}'"));
        assert!(err.message.contains("in action body"));
        assert!(err.message.contains("expected"));
    }
}
