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

/// Model interchange formats: XMI, KPAR, JSON-LD
pub mod interchange;

// ============================================================================
// BACKWARDS COMPATIBILITY - `core` re-exports for syster-lsp migration
// ============================================================================

/// Backwards compatibility: re-export base and parser items as `core`
///
/// This allows syster-lsp to continue using `syster::core::*` imports
/// while we migrate to the new module paths.
pub mod core {
    // Re-export from base
    pub use crate::base::{FileId, Position};

    // Re-export from parser
    pub use crate::parser::{ParseError, ParseResult};

    // Re-export constants (with is_supported_extension)
    pub mod constants {
        pub use crate::base::constants::*;
    }

    // Re-export text_utils from ide
    pub use crate::ide::text_utils;
}

// Re-export commonly needed items
pub use parser::keywords;

// Re-export foundation types
pub use base::{FileId, Interner, LineCol, LineIndex, Name, Position, Span, TextRange, TextSize};
