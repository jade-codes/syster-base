//! # syster-base
//!
//! Core library for SysML v2 and KerML parsing, AST, and semantic analysis.
//!
//! ## Module Structure (dependency order)
//!
//! ```text
//! ide     → IDE features (completion, hover, goto-def)
//!   ↓
//! hir     → Semantic model with Salsa queries  
//!   ↓
//! syntax  → AST types, Span/Position
//!   ↓
//! parser  → Lexer + parser + constants
//!   ↓
//! base    → Primitives (FileId, Name interning, TextRange)
//! ```

// ============================================================================
// MODULES (dependency order: base → parser → syntax → hir → ide)
// ============================================================================

/// Foundation types: FileId, Name interning, TextRange
pub mod base;

/// Parser: pest grammars, ParseResult, file extensions
pub mod parser;

/// Syntax: AST types, Span/Position, traits
pub mod syntax;

/// High-level IR: Salsa-based semantic model
pub mod hir;

/// IDE features: completion, hover, goto-definition, find-references
pub mod ide;

/// Project management: workspace loading, stdlib
pub mod project;

// Re-export commonly needed items
pub use parser::keywords;

// Re-export foundation types
pub use base::{FileId, Interner, LineCol, LineIndex, Name, Position, Span, TextRange, TextSize};
