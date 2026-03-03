//! Symbol extraction from AST — pure functions that return symbols.
//!
//! This module provides functions to extract symbols from a parsed AST.
//! Extraction works directly with the typed AST wrapper types from
//! `crate::parser` (e.g., `Definition`, `Usage`, `Package`), producing
//! `HirSymbol` values without any intermediate representation.
//!
//! # Module structure
//!
//! - [`types`] — Public and internal type definitions (HirSymbol, SymbolKind, etc.)
//! - [`context`] — ExtractionContext for tracking scope during extraction
//! - [`helpers`] — AST → internal type conversion helpers
//! - [`extract`] — Unified extraction entry points and dispatch
//! - [`extract_leaf`] — Leaf node extractors (comment, alias, import, dependency)
//! - [`extract_package`] — Package and filter extractors
//! - [`extract_definition`] — Definition extraction
//! - [`extract_usage`] — Usage and metadata extraction
//! - [`extract_special`] — Special variant helpers (bind, succession, connector, etc.)

mod context;
mod extract;
mod extract_definition;
mod extract_leaf;
mod extract_package;
mod extract_special;
mod extract_usage;
mod helpers;
mod types;

#[cfg(test)]
mod tests;

// Re-export all public types and functions
pub use types::{
    ExtractionResult, HirRelationship, HirSymbol, RefKind, RelationshipKind, SymbolKind, TypeRef,
    TypeRefChain, TypeRefKind,
};

pub use types::new_element_id;

pub use extract::{extract_symbols_unified, extract_with_filters};
