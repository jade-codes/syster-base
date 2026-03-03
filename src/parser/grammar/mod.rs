//! Grammar modules for KerML and SysML parsing
//!
//! This module contains the language-specific parsing logic organized by grammar:
//! - `kerml` - Core KerML constructs (definitions, usages, relationships)
//! - `kerml_expressions` - Expression parsing (shared between KerML and SysML)
//! - `sysml` - SysML-specific extensions (action bodies, state machines, requirements)
//!
//! The parsing functions are generic over a trait (`ExpressionParser` / `KerMLParser` / `SysMLParser`)
//! so they can be used with any parser implementation.

pub mod kerml;
pub mod kerml_expressions;
pub mod sysml;

pub use kerml::{KerMLParser, parse_kerml_file, parse_namespace_element};
pub use kerml_expressions::ExpressionParser;
pub use sysml::SysMLParser;

use super::syntax_kind::SyntaxKind;

// =============================================================================
// BaseParser — shared interface for KerML and SysML parsers
// =============================================================================

/// Methods shared identically between `KerMLParser` and `SysMLParser`.
///
/// Both sub-traits extend this, so grammar-specific code bounded on either
/// trait automatically gains access to these methods.
pub trait BaseParser: ExpressionParser {
    /// Get the current token text (for error messages and inspection)
    fn current_token_text(&self) -> Option<&str>;

    /// Parse an identification (name or short name)
    fn parse_identification(&mut self);

    /// Skip trivia except block comments
    fn skip_trivia_except_block_comments(&mut self);

    /// Parse a comma-separated list of qualified names
    fn parse_qualified_name_list(&mut self);

    /// Report a parse error
    fn error(&mut self, message: impl Into<String>);

    /// Error recovery — skip to recovery tokens
    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]);
}

// =============================================================================
// Shared constants used by both KerML and SysML grammars
// =============================================================================

/// Standalone relationship keywords (shared between KerML and SysML)
pub const STANDALONE_RELATIONSHIP_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::SPECIALIZATION_KW,
    SyntaxKind::SUBCLASSIFIER_KW,
    SyntaxKind::REDEFINITION_KW,
    SyntaxKind::SUBSET_KW,
    SyntaxKind::SUBTYPE_KW,
    SyntaxKind::TYPING_KW,
    SyntaxKind::CONJUGATION_KW,
    SyntaxKind::DISJOINING_KW,
    SyntaxKind::FEATURING_KW,
    SyntaxKind::INVERTING_KW,
];

/// Relationship operator keywords (shared between KerML and SysML)
pub const RELATIONSHIP_OPERATORS: &[SyntaxKind] = &[
    SyntaxKind::SPECIALIZES_KW,
    SyntaxKind::COLON_GT,
    SyntaxKind::SUBSETS_KW,
    SyntaxKind::REDEFINES_KW,
    SyntaxKind::COLON_GT_GT,
    SyntaxKind::TYPED_KW,
    SyntaxKind::COLON,
    SyntaxKind::CONJUGATES_KW,
    SyntaxKind::TILDE,
    SyntaxKind::INVERSE_KW,
    SyntaxKind::OF_KW,
];
