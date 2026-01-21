use super::symbol::SymbolId;
use crate::core::Span;
use std::collections::HashMap;

/// Import declaration in a scope (as written in source)
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub is_recursive: bool,
    pub is_namespace: bool,
    pub is_public: bool,
    pub span: Option<Span>,
    pub file: Option<String>,
}

/// Resolved import with fully qualified path
/// Created during the import resolution phase after all symbols are registered
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// Original path as written (e.g., "Definitions::*")
    pub raw_path: String,
    /// Fully qualified path after resolution (e.g., "SimpleVehicleModel::Definitions::*")
    pub resolved_path: String,
    /// Whether this is a wildcard import (::* or ::**)
    pub is_namespace: bool,
    /// Whether this is a recursive import (::**)
    pub is_recursive: bool,
    /// Whether this import re-exports its contents (public import)
    pub is_public: bool,
}

/// Represents a lexical scope in the symbol table
#[derive(Debug)]
pub struct Scope {
    pub parent: Option<usize>,
    /// Maps symbol name to SymbolId (symbols are stored in arena)
    pub symbols: HashMap<String, SymbolId>,
    pub children: Vec<usize>,
    /// Raw imports as written in source (used during Phase 1)
    pub imports: Vec<Import>,
    /// Resolved imports with fully qualified paths (populated in Phase 2)
    pub resolved_imports: Vec<ResolvedImport>,
    /// Export map: name -> SymbolId for fast O(1) lookup (populated in Phase 3)
    /// Includes: direct children + re-exports from public wildcard imports
    pub export_map: HashMap<String, SymbolId>,
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            symbols: HashMap::new(),
            children: Vec::new(),
            imports: Vec::new(),
            resolved_imports: Vec::new(),
            export_map: HashMap::new(),
        }
    }
}
