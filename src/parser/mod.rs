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

#[allow(clippy::module_inception)]
mod parser;

pub mod ast;
pub mod grammar;
pub mod keywords;
mod lexer;
mod syntax_kind;

pub use ast::*;
pub use lexer::{Lexer, Token};
pub use parser::{Parse, SyntaxError, parse_kerml, parse_sysml};
pub use syntax_kind::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, SysMLLanguage};

/// Re-export rowan types for convenience
pub use rowan::{GreenNode, TextRange, TextSize};
