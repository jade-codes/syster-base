//! # syster-base
//!
//! Core library for SysML v2 and KerML parsing, AST, and semantic analysis.
//!
//! ## Module Structure (dependency order)
//!
//! ```text
//! ide       → IDE features (completion, hover, goto-def)
//!   ↓
//! hir       → Semantic model with Salsa queries
//!   ↓
//! project   → Workspace loading, stdlib resolution
//!   ↓
//! syntax    → AST types, Span/Position, ParseError/ParseResult
//!   ↓
//! parser    → Logos lexer, recursive-descent parser, grammar traits
//!   ↓
//! base      → Primitives (FileId, Name interning, TextRange)
//! ```

// ============================================================================
// MODULES (dependency order: base → parser → syntax → project → hir → ide)
// ============================================================================

/// Foundation types: FileId, Name interning, TextRange
pub mod base;

/// Parser: Logos lexer, recursive-descent parser, grammar traits
pub mod parser;

/// Syntax: AST types, Span/Position, ParseError/ParseResult
pub mod syntax;

/// High-level IR: Salsa-based semantic model
pub mod hir;

/// IDE features: completion, hover, goto-definition, find-references
pub mod ide;

/// Project management: workspace loading, stdlib
pub mod project;

/// Model interchange formats: XMI, KPAR, JSON-LD
#[cfg(feature = "interchange")]
pub mod interchange;

// Re-export commonly needed items
pub use parser::keywords;

// Re-export foundation types
pub use base::{FileId, Interner, LineCol, LineIndex, Name, Position, Span, TextRange, TextSize};
