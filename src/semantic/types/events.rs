//! # Semantic Event System
//!
//! Defines all event types emitted by semantic analysis components.
//!
//! These events enable observability and decoupled logic for:
//! - LSP implementations (subscribe to changes)
//! - Testing (verify events fire without side effects)
//! - Extensibility (add listeners without modifying core code)
//!
//! ## Event Types
//!
//! - **WorkspaceEvent**: File additions, updates, removals
//! - **DependencyEvent**: Dependency graph changes
//!
//! Future additions:
//! - SymbolTableEvent: Symbol insertions, import additions
//! - RelationshipEvent: Relationship graph changes
//! - AnalyzerEvent: Semantic error detection

use crate::core::events::Event;
use std::path::PathBuf;

/// Events emitted by the workspace during file operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEvent {
    /// A file was added to the workspace
    FileAdded { path: PathBuf },

    /// A file's content was updated
    FileUpdated { path: PathBuf },

    /// A file was removed from the workspace
    FileRemoved { path: PathBuf },
}

impl Event for WorkspaceEvent {}

/// Events emitted by the dependency graph during updates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyEvent {
    /// A dependency was added between two files
    DependencyAdded {
        /// The file that imports another file
        from: PathBuf,
        /// The file being imported
        to: PathBuf,
    },

    /// A file and all its dependencies were removed
    FileRemoved { path: PathBuf },
}

impl Event for DependencyEvent {}

/// Events emitted by the symbol table during symbol operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolTableEvent {
    /// A symbol was inserted into the symbol table
    SymbolInserted {
        /// The qualified name of the symbol
        qualified_name: String,
        /// The unique identifier of the symbol
        symbol_id: usize,
    },

    /// An import was added to a file's import list
    ImportAdded {
        /// The import path that was added
        import_path: String,
    },

    /// The current file context was changed
    FileChanged {
        /// The path of the file that is now current
        file_path: String,
    },
}

impl Event for SymbolTableEvent {}
