//! High-level IR (HIR) — Semantic model with Salsa queries.
//!
//! This module contains the incremental computation engine using Salsa.
//! All semantic analysis is expressed as queries that are automatically
//! memoized and invalidated when inputs change.
//!
//! ## Key Types
//!
//! - [`Db`] — The Salsa database trait defining all queries
//! - [`RootDatabase`] — Concrete implementation of the database
//! - [`DefId`] — Identifier for a definition (part, port, action, etc.)
//! - [`HirSymbol`] — A symbol extracted from the AST
//! - [`SymbolIndex`] — Workspace-wide symbol index for name resolution
//! - [`Resolver`] — Name resolver with import handling
//!
//! ## Query Layers
//!
//! ```text
//! file_text(file)           ← INPUT: raw source text
//!     │
//!     ▼
//! parse(file)               ← Parse into AST (per-file)
//!     │
//!     ▼
//! file_symbols(file)        ← Extract symbols (per-file)
//!     │
//!     ▼
//! symbol_index              ← Workspace-wide index
//!     │
//!     ▼
//! resolve_name(scope, name) ← Name resolution
//!     │
//!     ▼
//! file_diagnostics(file)    ← Semantic errors
//! ```

mod db;
mod diagnostics;
mod ids;
mod input;
mod resolve;
mod source;
mod symbols;

pub use db::{RootDatabase, FileText, SourceRootInput, ParseResult, parse_file, file_symbols, file_symbols_from_text};
pub use diagnostics::{Diagnostic, DiagnosticCollector, Severity, RelatedInfo, SemanticChecker, check_file};
pub use ids::{DefId, LocalDefId};
pub use input::SourceRoot;
pub use resolve::{SymbolIndex, Resolver, ResolveResult};
pub use source::FileSet;
pub use symbols::{HirSymbol, SymbolKind, TypeRef, TypeRefKind, TypeRefChain, RefKind, extract_symbols_unified};
