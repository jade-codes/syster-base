//! Foundation types for the Syster toolchain.
//!
//! This module provides fundamental types used throughout the compiler:
//! - [`FileId`] - Interned file identifiers
//! - [`TextRange`], [`TextSize`] - Source positions (byte offsets)
//! - [`LineCol`], [`LineIndex`] - Line/column conversion
//! - [`Position`], [`Span`] - Line/column positions for AST nodes
//! - [`Name`], [`Interner`] - String interning
//! - Domain constants (file extensions, relationship types)
//!
//! This module has NO dependencies on other syster modules.

pub mod constants;
mod file_id;
mod intern;
mod position;
mod span;

pub use file_id::FileId;
pub use intern::{Interner, Name};
pub use position::{Position, Span};
pub use span::{LineCol, LineIndex, TextRange, TextSize};

// Re-export text-size types for convenience
pub use text_size;
