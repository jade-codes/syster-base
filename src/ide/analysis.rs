//! AnalysisHost and Analysis — Unified state management for IDE features.
//!
//! The `AnalysisHost` owns all mutable state and provides `Analysis` snapshots
//! for querying. This pattern ensures consistent reads across multiple queries.
//!
//! ## Usage
//!
//! ```ignore
//! let mut host = AnalysisHost::new();
//!
//! // Apply file changes
//! host.set_file_content(file_id, content);
//!
//! // Get a snapshot for queries
//! let analysis = host.analysis();
//! let hover = analysis.hover(file_id, line, col);
//! let symbols = analysis.document_symbols(file_id);
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use crate::base::FileId;
use crate::hir::{SymbolIndex, extract_with_filters};
use crate::syntax::SyntaxFile;

use super::{
    CompletionItem, DocumentLink, FoldingRange, GotoResult, HoverResult, InlayHint,
    ReferenceResult, SelectionRange, SemanticToken, SymbolInfo,
};

/// Owns all mutable state for the IDE layer.
///
/// Apply changes via `set_file_content()` and `remove_file()`,
/// then get a consistent snapshot via `analysis()`.
pub struct AnalysisHost {
    /// Parsed files stored directly (no Workspace dependency)
    files: HashMap<PathBuf, SyntaxFile>,
    /// HIR-based symbol index built from parsed files
    symbol_index: SymbolIndex,
    /// Map from file path to FileId
    file_id_map: HashMap<String, FileId>,
    /// Reverse map from FileId to file path
    file_path_map: HashMap<FileId, String>,
    /// Whether the index needs rebuilding
    index_dirty: bool,
}

impl Default for AnalysisHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisHost {
    /// Create a new empty AnalysisHost.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            symbol_index: SymbolIndex::new(),
            file_id_map: HashMap::new(),
            file_path_map: HashMap::new(),
            index_dirty: false,
        }
    }

    /// Set the content of a file, parsing it and storing the result.
    ///
    /// Returns parse errors if any.
    pub fn set_file_content(
        &mut self,
        path: &str,
        content: &str,
    ) -> Vec<crate::syntax::parser::ParseError> {
        use crate::syntax::parser::parse_with_result;
        use std::path::Path;

        let path_buf = PathBuf::from(path);

        // Parse the content
        let result = parse_with_result(content, Path::new(path));

        if let Some(syntax_file) = result.content {
            self.files.insert(path_buf, syntax_file);
        }

        self.index_dirty = true;
        result.errors
    }

    /// Remove a file from storage.
    pub fn remove_file(&mut self, path: &str) {
        let path_buf = PathBuf::from(path);
        self.files.remove(&path_buf);
        self.index_dirty = true;
    }

    /// Remove a file from storage using PathBuf.
    pub fn remove_file_path(&mut self, path: &PathBuf) {
        self.files.remove(path);
        self.index_dirty = true;
    }

    /// Check if a file exists in storage.
    pub fn has_file(&self, path: &str) -> bool {
        let path_buf = PathBuf::from(path);
        self.files.contains_key(&path_buf)
    }

    /// Check if a file exists in storage using Path.
    pub fn has_file_path(&self, path: &std::path::Path) -> bool {
        self.files.contains_key(path)
    }

    /// Update or add a file with pre-parsed content.
    /// Used when caller already has parsed SyntaxFile.
    pub fn set_file(&mut self, path: PathBuf, file: SyntaxFile) {
        self.files.insert(path, file);
        self.index_dirty = true;
    }

    /// Get access to the parsed files.
    pub fn files(&self) -> &HashMap<PathBuf, SyntaxFile> {
        &self.files
    }

    /// Get the number of files loaded.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Mark the index as needing rebuild (call after external changes).
    pub fn mark_dirty(&mut self) {
        self.index_dirty = true;
    }

    /// Rebuild the symbol index from the current files.
    ///
    /// This is called automatically by `analysis()` if the index is dirty.
    pub fn rebuild_index(&mut self) {
        // Build file ID map from file paths
        self.file_id_map.clear();
        self.file_path_map.clear();

        for (i, path) in self.files.keys().enumerate() {
            let path_str = path.to_string_lossy().to_string();
            let file_id = FileId::new(i as u32);
            self.file_id_map.insert(path_str.clone(), file_id);
            self.file_path_map.insert(file_id, path_str);
        }

        // Build symbol index directly from parsed files
        let mut new_index = SymbolIndex::new();

        for (path, syntax_file) in &self.files {
            let path_str = path.to_string_lossy().to_string();
            if let Some(&file_id) = self.file_id_map.get(&path_str) {
                // Extract symbols and filters using unified extraction (handles both SysML and KerML)
                let result = extract_with_filters(file_id, syntax_file);
                new_index.add_extraction_result(file_id, result);
            }
        }

        // Build visibility maps for import resolution
        new_index.ensure_visibility_maps();

        // Resolve all type references (pre-compute resolved_target)
        new_index.resolve_all_type_refs();

        self.symbol_index = new_index;
        self.index_dirty = false;
    }

    /// Get a consistent snapshot for querying.
    ///
    /// If the index is dirty, it will be rebuilt first.
    pub fn analysis(&mut self) -> Analysis<'_> {
        if self.index_dirty {
            self.rebuild_index();
        }

        Analysis {
            symbol_index: &self.symbol_index,
            file_id_map: &self.file_id_map,
            file_path_map: &self.file_path_map,
        }
    }

    /// Get the FileId for a path, if it exists.
    pub fn get_file_id(&self, path: &str) -> Option<FileId> {
        self.file_id_map.get(path).copied()
    }

    /// Get the FileId for a PathBuf, if it exists.
    pub fn get_file_id_for_path(&self, path: &std::path::Path) -> Option<FileId> {
        self.file_id_map
            .get(&path.to_string_lossy().to_string())
            .copied()
    }

    /// Get the path for a FileId, if it exists.
    pub fn get_file_path(&self, file_id: FileId) -> Option<&str> {
        self.file_path_map.get(&file_id).map(|s| s.as_str())
    }

    /// Get the path as PathBuf for a FileId, if it exists.
    pub fn get_file_path_buf(&self, file_id: FileId) -> Option<PathBuf> {
        self.file_path_map.get(&file_id).map(PathBuf::from)
    }

    /// Get the file_id_map (for compatibility during migration).
    pub fn file_id_map(&self) -> &HashMap<String, FileId> {
        &self.file_id_map
    }

    /// Get the symbol_index (for compatibility during migration).
    pub fn symbol_index(&self) -> &SymbolIndex {
        &self.symbol_index
    }
}

/// An immutable snapshot of the analysis state.
///
/// All IDE queries go through this struct to ensure consistent results.
pub struct Analysis<'a> {
    symbol_index: &'a SymbolIndex,
    file_id_map: &'a HashMap<String, FileId>,
    file_path_map: &'a HashMap<FileId, String>,
}

impl<'a> Analysis<'a> {
    // ==================== Symbol-based features ====================

    /// Get hover information at a position.
    pub fn hover(&self, file_id: FileId, line: u32, col: u32) -> Option<HoverResult> {
        super::hover(self.symbol_index, file_id, line, col)
    }

    /// Get type information at a position.
    ///
    /// Returns info if cursor is on a type annotation (`:`, `:>`, `::>`, etc.).
    pub fn type_info_at(&self, file_id: FileId, line: u32, col: u32) -> Option<super::TypeInfo> {
        super::type_info_at(self.symbol_index, file_id, line, col)
    }

    /// Go to definition at a position.
    pub fn goto_definition(&self, file_id: FileId, line: u32, col: u32) -> GotoResult {
        super::goto_definition(self.symbol_index, file_id, line, col)
    }

    /// Go to type definition at a position.
    ///
    /// Navigates from a usage to its type definition (e.g., from `engine : Engine` to `part def Engine`).
    pub fn goto_type_definition(&self, file_id: FileId, line: u32, col: u32) -> GotoResult {
        super::goto_type_definition(self.symbol_index, file_id, line, col)
    }

    /// Find all references to a symbol at a position.
    pub fn find_references(
        &self,
        file_id: FileId,
        line: u32,
        col: u32,
        include_declaration: bool,
    ) -> ReferenceResult {
        super::find_references(self.symbol_index, file_id, line, col, include_declaration)
    }

    /// Get completions at a position.
    pub fn completions(
        &self,
        file_id: FileId,
        line: u32,
        col: u32,
        trigger: Option<char>,
    ) -> Vec<CompletionItem> {
        super::completions(self.symbol_index, file_id, line, col, trigger)
    }

    /// Get all symbols in a document.
    pub fn document_symbols(&self, file_id: FileId) -> Vec<SymbolInfo> {
        super::document_symbols(self.symbol_index, file_id)
    }

    /// Search for symbols across the workspace.
    pub fn workspace_symbols(&self, query: Option<&str>) -> Vec<SymbolInfo> {
        super::workspace_symbols(self.symbol_index, query)
    }

    /// Get document links (import paths, etc.).
    pub fn document_links(&self, file_id: FileId) -> Vec<DocumentLink> {
        super::document_links(self.symbol_index, file_id)
    }

    // ==================== AST-based features ====================

    /// Get folding ranges for a file.
    pub fn folding_ranges(&self, file_id: FileId) -> Vec<FoldingRange> {
        super::folding_ranges(self.symbol_index, file_id)
    }

    /// Get selection ranges at positions.
    pub fn selection_ranges(&self, file_id: FileId, line: u32, col: u32) -> Vec<SelectionRange> {
        super::selection_ranges(self.symbol_index, file_id, line, col)
    }

    /// Get inlay hints for a file (optionally within a range).
    pub fn inlay_hints(
        &self,
        file_id: FileId,
        range: Option<(u32, u32, u32, u32)>,
    ) -> Vec<InlayHint> {
        super::inlay_hints(self.symbol_index, file_id, range)
    }

    /// Get semantic tokens for a file.
    pub fn semantic_tokens(&self, file_id: FileId) -> Vec<SemanticToken> {
        super::semantic_tokens(self.symbol_index, file_id)
    }

    // ==================== Accessors ====================

    /// Get the symbol index.
    pub fn symbol_index(&self) -> &SymbolIndex {
        self.symbol_index
    }

    /// Get the file ID map.
    pub fn file_id_map(&self) -> &HashMap<String, FileId> {
        self.file_id_map
    }

    /// Get the file path for a FileId.
    pub fn get_file_path(&self, file_id: FileId) -> Option<&str> {
        self.file_path_map.get(&file_id).map(|s| s.as_str())
    }

    /// Get the FileId for a path.
    pub fn get_file_id(&self, path: &str) -> Option<FileId> {
        self.file_id_map.get(path).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_host_basic() {
        let mut host = AnalysisHost::new();

        // Add a file
        let errors = host.set_file_content("test.sysml", "package Test {}");
        assert!(errors.is_empty());

        // Get analysis
        let analysis = host.analysis();

        // Should have the file
        assert!(analysis.get_file_id("test.sysml").is_some());
    }

    #[test]
    fn test_file_removal() {
        let mut host = AnalysisHost::new();

        // Add and remove a file
        host.set_file_content("test.sysml", "package Test {}");
        host.remove_file("test.sysml");

        let analysis = host.analysis();
        assert!(analysis.get_file_id("test.sysml").is_none());
    }
}
