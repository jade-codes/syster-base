//! IDE features — High-level APIs for LSP handlers.
//!
//! This module provides the interface between the semantic model (HIR)
//! and the LSP server. Each function corresponds to an LSP request.
//!
//! ## Design Principles
//!
//! 1. **Pure functions**: Take data in, return data out
//! 2. **No LSP types**: Uses our own types, converted at LSP boundary
//! 3. **Composable**: Built on top of HIR queries
//!
//! ## Usage
//!
//! The recommended way to use this module is through `AnalysisHost`:
//!
//! ```ignore
//! use syster::ide::AnalysisHost;
//!
//! let mut host = AnalysisHost::new();
//! host.set_file_content("test.sysml", "package Test {}");
//!
//! let analysis = host.analysis();
//! let symbols = analysis.document_symbols(file_id);
//! ```

mod analysis;
mod completion;
mod document_links;
mod folding;
mod goto;
mod hover;
mod inlay_hints;
mod references;
mod selection;
mod semantic_tokens;
mod symbols;
pub mod text_utils;

pub use analysis::{Analysis, AnalysisHost};
pub use completion::{CompletionItem, CompletionKind, completions};
pub use document_links::{DocumentLink, document_links};
pub use folding::{FoldingRange, folding_ranges};
pub use goto::{GotoResult, GotoTarget, goto_definition};
pub use hover::{HoverResult, hover};
pub use inlay_hints::{InlayHint, InlayHintKind, inlay_hints};
pub use references::{Reference, ReferenceResult, find_references};
pub use selection::{SelectionRange, selection_ranges};
pub use semantic_tokens::{SemanticToken, TokenType, semantic_tokens};
pub use symbols::{SymbolInfo, document_symbols, workspace_symbols};
pub use text_utils::{extract_qualified_name_at_cursor, extract_word_at_cursor};
