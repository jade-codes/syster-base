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

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, SymbolIndex, extract_with_filters};
use crate::syntax::SyntaxFile;

// ModelFormat trait needed for .write() on Xmi/JsonLd/Yaml
#[cfg(feature = "interchange")]
use crate::interchange::ModelFormat;

use super::{
    CompletionItem, DocumentLink, FoldingRange, GotoResult, HoverResult, InlayHint,
    ReferenceResult, SelectionRange, SemanticToken, SymbolInfo,
};

/// Owns all mutable state for the IDE layer.
///
/// Apply changes via `set_file_content()` and `remove_file()`,
/// then get a consistent snapshot via `analysis()`.
///
/// When the `interchange` feature is enabled, `AnalysisHost` also
/// provides a lazily-cached [`Model`](crate::interchange::Model)
/// projection via [`model()`](Self::model), navigation delegates,
/// and format export methods — eliminating the need for a separate
/// `ModelHost` when working from SysML text.
#[derive(Clone)]
pub struct AnalysisHost {
    /// Parsed files stored directly (no Workspace dependency)
    files: HashMap<PathBuf, SyntaxFile>,
    /// HIR-based symbol index built from parsed files
    symbol_index: SymbolIndex,
    /// Map from file path to FileId
    file_id_map: HashMap<String, FileId>,
    /// Reverse map from FileId to file path
    file_path_map: HashMap<FileId, String>,
    /// Files that have been modified and need re-extraction
    dirty_files: HashSet<PathBuf>,
    /// Files that have been removed (need to be removed from index)
    removed_files: HashSet<PathBuf>,
    /// Whether we need a full rebuild (e.g., first build)
    needs_full_rebuild: bool,
    /// Persistent cache: qualified_name → element_id
    /// Preserves IDs even when symbols are temporarily removed
    element_id_cache: HashMap<Arc<str>, Arc<str>>,
    /// Lazily-cached interchange `Model`, built from the `SymbolIndex`.
    /// Invalidated whenever file content changes.
    #[cfg(feature = "interchange")]
    model_cache: Option<crate::interchange::Model>,
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
            dirty_files: HashSet::new(),
            removed_files: HashSet::new(),
            needs_full_rebuild: true, // First analysis needs full build
            element_id_cache: HashMap::new(),
            #[cfg(feature = "interchange")]
            model_cache: None,
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
            self.files.insert(path_buf.clone(), syntax_file);
        }

        // Mark this file as dirty (needs re-extraction)
        self.dirty_files.insert(path_buf);
        // Invalidate cached Model — symbols changed
        #[cfg(feature = "interchange")]
        {
            self.model_cache = None;
        }
        result.errors
    }

    /// Remove a file from storage.
    pub fn remove_file(&mut self, path: &str) {
        let path_buf = PathBuf::from(path);
        self.files.remove(&path_buf);
        self.dirty_files.remove(&path_buf);
        self.removed_files.insert(path_buf);
        // Invalidate cached Model — symbols changed
        #[cfg(feature = "interchange")]
        {
            self.model_cache = None;
        }
    }

    /// Remove a file from storage using PathBuf.
    pub fn remove_file_path(&mut self, path: &PathBuf) {
        self.files.remove(path);
        self.dirty_files.remove(path);
        self.removed_files.insert(path.clone());
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
        self.dirty_files.insert(path.clone());
        self.files.insert(path, file);
    }

    /// Get access to the parsed files.
    pub fn files(&self) -> &HashMap<PathBuf, SyntaxFile> {
        &self.files
    }

    /// Get the number of files loaded.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Mark the index as needing full rebuild (call after external changes).
    pub fn mark_dirty(&mut self) {
        self.needs_full_rebuild = true;
        // Invalidate cached Model — symbols will change on next rebuild
        #[cfg(feature = "interchange")]
        {
            self.model_cache = None;
        }
    }

    /// Check if the index needs updating.
    fn needs_update(&self) -> bool {
        self.needs_full_rebuild || !self.dirty_files.is_empty() || !self.removed_files.is_empty()
    }

    /// Rebuild the symbol index from the current files.
    ///
    /// This is called automatically by `analysis()` if the index is dirty.
    pub fn rebuild_index(&mut self) {
        if self.needs_full_rebuild {
            self.full_rebuild();
        } else {
            self.incremental_rebuild();
        }
    }

    /// Full rebuild - used on first load or when structure changes significantly
    fn full_rebuild(&mut self) {
        // First, update cache with all current symbols' IDs
        for symbol in self.symbol_index.all_symbols() {
            if !symbol.element_id.as_ref().is_empty()
                && !symbol.element_id.starts_with("00000000-0000-0000-0000")
            {
                self.element_id_cache
                    .insert(symbol.qualified_name.clone(), symbol.element_id.clone());
            }
        }

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
                let mut result = extract_with_filters(file_id, syntax_file);

                // Preserve element IDs from cache (survives removal/re-add)
                for symbol in &mut result.symbols {
                    if let Some(cached_id) = self.element_id_cache.get(&symbol.qualified_name) {
                        symbol.element_id = cached_id.clone();
                    }
                }

                new_index.add_extraction_result(file_id, result);
            }
        }

        // Build visibility maps for import resolution
        new_index.ensure_visibility_maps();

        // Resolve all type references (pre-compute resolved_target)
        new_index.resolve_all_type_refs();

        self.symbol_index = new_index;
        self.needs_full_rebuild = false;
        self.dirty_files.clear();
        self.removed_files.clear();
    }

    /// Incremental rebuild - only re-extract changed files
    fn incremental_rebuild(&mut self) {
        use std::time::Instant;

        // Collect files that need type ref resolution
        let mut files_to_resolve: Vec<FileId> = Vec::new();

        // Handle removed files first - cache their element IDs before removal
        let t0 = Instant::now();
        for path in self.removed_files.drain() {
            let path_str = path.to_string_lossy().to_string();
            if let Some(&file_id) = self.file_id_map.get(&path_str) {
                // Cache element IDs before removing
                for symbol in self.symbol_index.symbols_in_file(file_id) {
                    if !symbol.element_id.as_ref().is_empty()
                        && !symbol.element_id.starts_with("00000000-0000-0000-0000")
                    {
                        self.element_id_cache
                            .insert(symbol.qualified_name.clone(), symbol.element_id.clone());
                    }
                }
                self.symbol_index.remove_file(file_id);
            }
        }

        // Re-extract only dirty files and track which need resolution
        for path in self.dirty_files.drain() {
            let path_str = path.to_string_lossy().to_string();

            let file_id = if let Some(&id) = self.file_id_map.get(&path_str) {
                // Cache element IDs before re-extraction (so modified symbols keep their IDs)
                for symbol in self.symbol_index.symbols_in_file(id) {
                    if !symbol.element_id.as_ref().is_empty()
                        && !symbol.element_id.starts_with("00000000-0000-0000-0000")
                    {
                        self.element_id_cache
                            .insert(symbol.qualified_name.clone(), symbol.element_id.clone());
                    }
                }
                id
            } else {
                let new_id = FileId::new(self.file_id_map.len() as u32);
                self.file_id_map.insert(path_str.clone(), new_id);
                self.file_path_map.insert(new_id, path_str.clone());
                new_id
            };

            if let Some(syntax_file) = self.files.get(&path) {
                let mut result = extract_with_filters(file_id, syntax_file);

                // Preserve element IDs from cache (survives removal/re-add)
                for symbol in &mut result.symbols {
                    if let Some(cached_id) = self.element_id_cache.get(&symbol.qualified_name) {
                        symbol.element_id = cached_id.clone();
                    }
                }

                self.symbol_index.add_extraction_result(file_id, result);
                files_to_resolve.push(file_id);
            }
        }
        let t1 = Instant::now();

        // Rebuild visibility maps (needed for correct resolution)
        self.symbol_index.mark_visibility_dirty();
        self.symbol_index.ensure_visibility_maps();
        let t2 = Instant::now();

        // Only resolve type refs for changed files (not the entire workspace)
        if !files_to_resolve.is_empty() {
            self.symbol_index
                .resolve_type_refs_for_files(&files_to_resolve);
        }
        let t3 = Instant::now();

        tracing::info!(
            "Incremental rebuild: extract={:?}, visibility={:?}, resolve={:?}",
            t1.duration_since(t0),
            t2.duration_since(t1),
            t3.duration_since(t2)
        );
    }

    /// Get a consistent snapshot for querying.
    ///
    /// If the index is dirty, it will be rebuilt first.
    pub fn analysis(&mut self) -> Analysis<'_> {
        if self.needs_update() {
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

    /// Get semantic diagnostics for a specific file.
    ///
    /// Returns a list of diagnostics (errors and warnings) found during semantic analysis.
    pub fn diagnostics(&self, file_id: FileId) -> Vec<crate::hir::Diagnostic> {
        crate::hir::check_file(&self.symbol_index, file_id)
    }

    /// Get all semantic diagnostics for all loaded files.
    ///
    /// Returns a map from file path to diagnostics for that file.
    pub fn all_diagnostics(&self) -> HashMap<String, Vec<crate::hir::Diagnostic>> {
        let mut result = HashMap::new();
        for (path, &file_id) in &self.file_id_map {
            let diags = self.diagnostics(file_id);
            if !diags.is_empty() {
                result.insert(path.clone(), diags);
            }
        }
        result
    }

    /// Get all semantic errors (severity = Error) for all loaded files.
    ///
    /// Returns a vec of (file_path, diagnostic) pairs for errors only.
    pub fn all_errors(&self) -> Vec<(String, crate::hir::Diagnostic)> {
        let mut result = Vec::new();
        for (path, &file_id) in &self.file_id_map {
            for diag in self.diagnostics(file_id) {
                if diag.severity == crate::hir::Severity::Error {
                    result.push((path.clone(), diag));
                }
            }
        }
        result
    }

    /// Get the file_id_map (for compatibility during migration).
    pub fn file_id_map(&self) -> &HashMap<String, FileId> {
        &self.file_id_map
    }

    /// Get the symbol_index (for compatibility during migration).
    pub fn symbol_index(&self) -> &SymbolIndex {
        &self.symbol_index
    }

    /// Update symbols in the index using a closure.
    /// The closure is called for each symbol and can modify it in place.
    /// This is useful for applying metadata after import (e.g., restoring element IDs).
    pub fn update_symbols<F>(&mut self, f: F)
    where
        F: FnMut(&mut HirSymbol),
    {
        self.symbol_index.update_symbols(f);
        // Invalidate cached Model — symbol metadata changed
        #[cfg(feature = "interchange")]
        {
            self.model_cache = None;
        }
    }

    // ── Interchange: Model projection ───────────────────────────────

    /// Get a lazily-cached [`Model`] built from the current `SymbolIndex`.
    ///
    /// The model is recomputed only when files have changed since the last
    /// call. This eliminates the need for a separate `ModelHost::from_text()`
    /// — the `AnalysisHost` is the single owner of both IDE queries and
    /// interchange `Model` access.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut host = AnalysisHost::new();
    /// host.set_file_content("model.sysml", "package P { part def A; }");
    /// let model = host.model();
    /// assert!(model.find_by_name("A").len() == 1);
    /// ```
    #[cfg(feature = "interchange")]
    pub fn model(&mut self) -> &crate::interchange::Model {
        self.ensure_model();
        self.model_cache.as_ref().unwrap()
    }

    /// Get a mutable reference to the cached [`Model`].
    ///
    /// This is useful when you need to perform edits via [`ChangeTracker`]
    /// on the model and then render the result. The cache is populated
    /// lazily on first access.
    ///
    /// **Note:** mutations applied here are *not* automatically synced
    /// back to the AnalysisHost's `SymbolIndex`. Use `set_file_content()`
    /// with the rendered text to update the index after editing.
    #[cfg(feature = "interchange")]
    pub fn model_mut(&mut self) -> &mut crate::interchange::Model {
        self.ensure_model();
        self.model_cache.as_mut().unwrap()
    }

    /// Take the cached [`Model`] out of the host, leaving the cache empty.
    ///
    /// Useful when you need an owned `Model` for functions that consume it
    /// (e.g., `restore_element_ids()`). Call [`set_model_cache()`](Self::set_model_cache)
    /// to put it back for further navigation.
    #[cfg(feature = "interchange")]
    pub fn take_model(&mut self) -> Option<crate::interchange::Model> {
        self.ensure_model();
        self.model_cache.take()
    }

    /// Replace the cached model with a new one.
    ///
    /// Useful for putting a model back after external transformations
    /// (e.g., metadata restoration).
    #[cfg(feature = "interchange")]
    pub fn set_model_cache(&mut self, model: crate::interchange::Model) {
        self.model_cache = Some(model);
    }

    /// Ensure the model cache is populated. Private helper to reduce duplication.
    #[cfg(feature = "interchange")]
    fn ensure_model(&mut self) {
        if self.needs_update() {
            self.rebuild_index();
        }
        if self.model_cache.is_none() {
            let symbols: Vec<_> = self.symbol_index.all_symbols().cloned().collect();
            self.model_cache = Some(crate::interchange::model_from_symbols(&symbols));
        }
    }

    // ── Interchange: Navigation delegates ────────────────────────────

    /// Views over root elements of the cached model.
    #[cfg(feature = "interchange")]
    pub fn root_views(&mut self) -> Vec<crate::interchange::views::ElementView<'_>> {
        self.ensure_model();
        self.model_cache.as_ref().unwrap().root_views()
    }

    /// Find elements by declared name in the cached model.
    #[cfg(feature = "interchange")]
    pub fn find_by_name(&mut self, name: &str) -> Vec<crate::interchange::views::ElementView<'_>> {
        self.ensure_model();
        self.model_cache.as_ref().unwrap().find_by_name(name)
    }

    /// Find an element by fully qualified name in the cached model.
    #[cfg(feature = "interchange")]
    pub fn find_by_qualified_name(
        &mut self,
        qn: &str,
    ) -> Option<crate::interchange::views::ElementView<'_>> {
        self.ensure_model();
        self.model_cache
            .as_ref()
            .unwrap()
            .find_by_qualified_name(qn)
    }

    /// View a specific element by ID in the cached model.
    #[cfg(feature = "interchange")]
    pub fn view(
        &mut self,
        id: &crate::interchange::ElementId,
    ) -> Option<crate::interchange::views::ElementView<'_>> {
        self.ensure_model();
        self.model_cache.as_ref().unwrap().view(id)
    }

    /// Find all elements of a specific metaclass kind in the cached model.
    #[cfg(feature = "interchange")]
    pub fn find_by_kind(
        &mut self,
        kind: crate::interchange::ElementKind,
    ) -> Vec<crate::interchange::views::ElementView<'_>> {
        self.ensure_model();
        self.model_cache.as_ref().unwrap().find_by_kind(kind)
    }

    // ── Interchange: Export methods ──────────────────────────────────

    /// Export the cached model to XMI bytes.
    #[cfg(feature = "interchange")]
    pub fn to_xmi(&mut self) -> Result<Vec<u8>, crate::interchange::InterchangeError> {
        self.ensure_model();
        crate::interchange::Xmi.write(self.model_cache.as_ref().unwrap())
    }

    /// Export the cached model to JSON-LD bytes.
    #[cfg(feature = "interchange")]
    pub fn to_jsonld(&mut self) -> Result<Vec<u8>, crate::interchange::InterchangeError> {
        self.ensure_model();
        crate::interchange::JsonLd.write(self.model_cache.as_ref().unwrap())
    }

    /// Export the cached model to YAML bytes.
    #[cfg(feature = "interchange")]
    pub fn to_yaml(&mut self) -> Result<Vec<u8>, crate::interchange::InterchangeError> {
        self.ensure_model();
        crate::interchange::Yaml.write(self.model_cache.as_ref().unwrap())
    }

    /// Add a model by decompiling it to SysML and adding as a synthetic file.
    /// The model is converted to SysML text, then parsed normally.
    /// Element IDs from the model are preserved via the element_id_cache.
    ///
    /// # Arguments
    /// * `model` - The interchange model to import
    /// * `virtual_path` - A virtual path for the generated file (e.g., "imported.sysml")
    #[cfg(feature = "interchange")]
    pub fn add_model(
        &mut self,
        model: &crate::interchange::Model,
        virtual_path: &str,
    ) -> Vec<crate::syntax::parser::ParseError> {
        use crate::interchange::decompile;

        // Pre-populate element_id_cache with IDs from the model
        // This ensures the original XMI IDs are preserved after parsing
        for element in model.iter_elements() {
            // Prefer qualified_name if available, fall back to simple name
            let key = element.qualified_name.as_ref().or(element.name.as_ref());

            if let Some(name) = key {
                self.element_id_cache
                    .insert(name.clone(), Arc::from(element.id.as_str()));
            }
        }

        // Decompile the model to SysML text
        let result = decompile(model);

        // Add as a normal file - the parsing pipeline handles everything
        // The element_id_cache will restore the original IDs during rebuild
        self.set_file_content(virtual_path, &result.text)
    }

    // ── Interchange: Edit bridge ────────────────────────────────────

    /// Apply a semantic edit to the cached Model and sync back to the host.
    ///
    /// This is the single method for performing Model-level edits (rename,
    /// add, remove, etc.) while keeping the `SymbolIndex` in sync:
    ///
    /// 1. Ensure the model cache is populated.
    /// 2. Build a `SourceMap` from the cached model.
    /// 3. Call `edit_fn` with `(&mut Model, &mut ChangeTracker)`.
    /// 4. Render the edits via `render_dirty`.
    /// 5. Feed the rendered text back into `set_file_content()`.
    /// 6. Restore element IDs from the edited model.
    ///
    /// # Example
    ///
    /// ```ignore
    /// host.set_file_content("model.sysml", "package P { part def Vehicle; }");
    ///
    /// let result = host.apply_model_edit("model.sysml", |model, tracker| {
    ///     let id = model.find_by_name("Vehicle")[0].id().clone();
    ///     tracker.rename(model, &id, "Car");
    /// });
    ///
    /// // AnalysisHost now reflects the rename
    /// let analysis = host.analysis();
    /// // analysis.symbol_index().lookup_qualified("P::Car") → Some(...)
    /// ```
    #[cfg(feature = "interchange")]
    pub fn apply_model_edit<F>(
        &mut self,
        file_path: &str,
        edit_fn: F,
    ) -> crate::interchange::ApplyEditsResult
    where
        F: FnOnce(&mut crate::interchange::Model, &mut crate::interchange::ChangeTracker),
    {
        use crate::interchange::ChangeTracker;
        use crate::interchange::render::{SourceMap, render_dirty};

        // 1. Ensure model is built
        self.ensure_model();

        // 2. Build source map from the current model
        let (original_text, source_map) = SourceMap::build(self.model_cache.as_ref().unwrap());

        // 3. Apply the edit via the closure
        let mut tracker = ChangeTracker::new();
        edit_fn(self.model_cache.as_mut().unwrap(), &mut tracker);

        // 4. Render the edits
        let rendered_text = render_dirty(
            &original_text,
            &source_map,
            self.model_cache.as_ref().unwrap(),
            &tracker,
        );

        // 5. Feed rendered text back into AnalysisHost (re-parses, rebuilds index)
        let parse_errors = self.set_file_content(file_path, &rendered_text);

        // 6. Rebuild index and restore element IDs from the edited model
        let _ = self.analysis(); // trigger rebuild
        if let Some(ref model) = self.model_cache {
            let model_snapshot = model.clone();
            self.update_symbols(|symbol| {
                for element in model_snapshot.elements.values() {
                    let qn = element.qualified_name.as_ref().or(element.name.as_ref());
                    if let Some(name) = qn {
                        if name.as_ref() == symbol.qualified_name.as_ref() {
                            symbol.element_id = Arc::from(element.id.as_str());
                            break;
                        }
                    }
                }
            });
        }

        crate::interchange::ApplyEditsResult {
            rendered_text,
            parse_errors,
        }
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

    // ── Interchange model projection tests ──────────────────────────

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_from_analysis_host() {
        let mut host = AnalysisHost::new();
        host.set_file_content(
            "model.sysml",
            "package Vehicle {\n    part def Engine;\n    part def Wheel;\n    part engine : Engine;\n}",
        );

        let model = host.model();
        assert!(model.element_count() > 0, "model should have elements");

        // Should find declared names
        assert_eq!(model.find_by_name("Engine").len(), 1);
        assert_eq!(model.find_by_name("Wheel").len(), 1);

        // Should have root views
        let roots = model.root_views();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name(), Some("Vehicle"));
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_cache_invalidation() {
        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def A; }");

        // First access — model is built
        let count_1 = host.model().element_count();
        assert!(count_1 > 0);

        // Edit the file — cache should be invalidated
        host.set_file_content("model.sysml", "package P { part def A; part def B; }");

        // Second access — model should reflect the new content
        let count_2 = host.model().element_count();
        assert!(
            count_2 > count_1,
            "model should have more elements after edit"
        );

        // Should find the new element
        assert_eq!(host.model().find_by_name("B").len(), 1);
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_navigation_delegates() {
        let mut host = AnalysisHost::new();
        host.set_file_content(
            "model.sysml",
            "package Outer { part def Inner; part x : Inner; }",
        );

        // root_views
        let roots = host.root_views();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name(), Some("Outer"));

        // find_by_name
        let inners = host.find_by_name("Inner");
        assert_eq!(inners.len(), 1);
        assert_eq!(
            inners[0].kind(),
            crate::interchange::ElementKind::PartDefinition
        );

        // find_by_qualified_name
        let found = host.find_by_qualified_name("Outer::Inner");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), Some("Inner"));

        // find_by_kind
        let usages = host.find_by_kind(crate::interchange::ElementKind::PartUsage);
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].name(), Some("x"));

        // view by ID
        let id = host.find_by_name("Inner")[0].id().clone();
        let viewed = host.view(&id);
        assert!(viewed.is_some());
        assert_eq!(viewed.unwrap().name(), Some("Inner"));
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_export_xmi_roundtrip() {
        use crate::interchange::ModelFormat;

        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def A; }");

        // Export to XMI
        let xmi_bytes = host.to_xmi().expect("XMI export should succeed");
        assert!(!xmi_bytes.is_empty());

        // Read back from XMI
        let model_back = crate::interchange::Xmi
            .read(&xmi_bytes)
            .expect("XMI read-back should succeed");
        assert!(model_back.element_count() > 0);
        assert_eq!(model_back.find_by_name("A").len(), 1);
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_cache_invalidated_on_remove() {
        let mut host = AnalysisHost::new();
        host.set_file_content("a.sysml", "package A { part def X; }");
        host.set_file_content("b.sysml", "package B { part def Y; }");

        // Build cache with both files
        let count = host.model().element_count();
        assert!(count > 0);

        // Remove one file — cache should invalidate
        host.remove_file("b.sysml");
        assert!(
            host.model().find_by_name("Y").is_empty(),
            "Y should be gone after removal"
        );
        assert_eq!(
            host.model().find_by_name("X").len(),
            1,
            "X should still exist"
        );
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_model_cache_invalidated_on_mark_dirty() {
        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def A; }");

        // Build cache
        let _ = host.model();

        // mark_dirty should invalidate
        host.mark_dirty();
        // Should still work (rebuilds lazily)
        assert_eq!(host.model().find_by_name("A").len(), 1);
    }

    // ── apply_model_edit tests ──────────────────────────────────────

    #[test]
    #[cfg(feature = "interchange")]
    fn test_apply_model_edit_rename() {
        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def Vehicle; }");

        let result = host.apply_model_edit("model.sysml", |model, tracker| {
            let id = model.find_by_name("Vehicle")[0].id().clone();
            tracker.rename(model, &id, "Car");
        });

        // Rendered text should contain the new name
        assert!(result.rendered_text.contains("Car"));
        assert!(!result.rendered_text.contains("Vehicle"));

        // AnalysisHost should reflect the rename in Salsa queries
        let analysis = host.analysis();
        let symbols: Vec<_> = analysis.symbol_index().all_symbols().collect();
        let has_car = symbols.iter().any(|s| s.name.as_ref() == "Car");
        assert!(has_car, "Should find 'Car' in symbols after rename");
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_apply_model_edit_add_element() {
        use crate::interchange::model::{Element, ElementId, ElementKind};

        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def A; }");

        let result = host.apply_model_edit("model.sysml", |model, tracker| {
            let parent_id = model.find_by_name("P")[0].id().clone();
            let new_elem =
                Element::new(ElementId::generate(), ElementKind::PartDefinition).with_name("B");
            tracker.add_element(model, new_elem, Some(&parent_id));
        });

        // Rendered text should contain both elements
        assert!(result.rendered_text.contains("part def A"));
        assert!(result.rendered_text.contains("part def B"));
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_apply_model_edit_preserves_ids() {
        let mut host = AnalysisHost::new();
        host.set_file_content(
            "model.sysml",
            "package P { part def Vehicle; part def Engine; }",
        );

        // Get the original ID
        let original_id = host.model().find_by_name("Engine")[0].id().clone();

        // Rename Vehicle → Car (Engine should keep its ID)
        host.apply_model_edit("model.sysml", |model, tracker| {
            let id = model.find_by_name("Vehicle")[0].id().clone();
            tracker.rename(model, &id, "Car");
        });

        // Engine's ID should be preserved in the symbol index
        let analysis = host.analysis();
        let engine_sym = analysis
            .symbol_index()
            .all_symbols()
            .find(|s| s.name.as_ref() == "Engine");
        assert!(engine_sym.is_some(), "Engine should still exist");
        assert_eq!(
            engine_sym.unwrap().element_id.as_ref(),
            original_id.as_str(),
            "Engine's element ID should be preserved after sibling rename"
        );
    }

    #[test]
    #[cfg(feature = "interchange")]
    fn test_apply_model_edit_noop() {
        let mut host = AnalysisHost::new();
        host.set_file_content("model.sysml", "package P { part def A; }");

        // No-op edit - don't modify anything
        let result = host.apply_model_edit("model.sysml", |_model, _tracker| {
            // intentionally empty
        });

        // Text should be preserved
        assert!(result.rendered_text.contains("package P"));
        assert!(result.rendered_text.contains("part def A"));
        assert!(result.parse_errors.is_empty());
    }
}
