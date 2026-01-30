//! Rowan-based incremental parser for SysML v2
//!
//! This module provides a lossless, incremental parser using:
//! - **logos** for fast lexing
//! - **rowan** for the CST (Concrete Syntax Tree)
//!
//! This is the rust-analyzer approach: we build a lossless CST that preserves
//! all whitespace and comments, then extract an AST layer on top.
//!
//! ## Architecture
//!
//! ```text
//! Source Text
//!     ↓
//! Lexer (logos) → Tokens with SyntaxKind
//!     ↓
//! Parser → GreenNode tree (immutable, cheap to clone)
//!     ↓
//! SyntaxNode (rowan) → CST with parent pointers
//!     ↓
//! AST layer → Typed wrappers over SyntaxNode
//!     ↓
//! HIR → Semantic model
//! ```
//!
//! ## Incremental Reparsing
//!
//! When text changes, we:
//! 1. Find the smallest subtree containing the change
//! 2. Reparse only that subtree
//! 3. Reuse unchanged green nodes (they're immutable and cheap to share)

mod syntax_kind;
mod lexer;
pub mod grammar;
mod parser;
pub mod result;
pub mod ast;
pub mod keywords;
// mod converter;  // TODO: Fix converter after migration tests pass

#[cfg(test)]
mod tests_migration;

pub use syntax_kind::{SyntaxKind, SysMLLanguage, SyntaxNode, SyntaxToken, SyntaxElement};
pub use lexer::{Lexer, Token};
pub use parser::{parse, parse_sysml, parse_kerml, Parse, SyntaxError, LanguageMode};
pub use result::{ParseResult, ParseError, ParseErrorKind};
pub use ast::*;
// pub use converter::{parse_sysml_to_ast, parse_kerml_to_ast, ConvertResult};

/// Re-export rowan types for convenience
pub use rowan::{GreenNode, TextRange, TextSize};
