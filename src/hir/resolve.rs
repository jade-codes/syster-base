//! Name resolution — resolving references to their definitions.
//!
//! This module provides name resolution for SysML/KerML.
//! It builds on top of the symbol extraction layer.
//!
//! # Architecture (January 2026)
//!
//! Name resolution follows a rust-analyzer inspired pattern:
//!
//! 1. **Symbol Extraction** - HIR extraction captures raw names/references with spans
//! 2. **Visibility Maps** - A separate pass builds per-scope visibility maps with resolved imports
//! 3. **Query-time Resolution** - Uses pre-computed visibility maps for O(1) lookups
//!
//! ## Key Data Structures
//!
//! - [`ScopeVisibility`] - Per-scope map of visible symbols (direct + imported)
//! - [`SymbolIndex`] - Global index with all symbols + pre-computed visibility maps
//! - [`Resolver`] - Query-time resolution using visibility maps

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::symbols::{HirSymbol, RefKind, SymbolKind, TypeRefKind};
use crate::base::FileId;

/// Type alias for resolution cache: (name, starting_scope) -> resolved_qname
type ResolutionCache = HashMap<(Arc<str>, Arc<str>), Option<Arc<str>>>;

// ============================================================================
// SCOPE VISIBILITY (Pre-computed at index time)
// ============================================================================

/// Per-scope visibility map capturing what names are visible and where they resolve to.
///
/// Built once during index construction, used at query time for O(1) resolution.
///
/// # Example
///
/// For a scope like `ISQ` with `public import ISQSpaceTime::*`:
/// - `direct_defs` contains symbols defined directly in ISQ
/// - `imports` contains symbols from ISQSpaceTime (via the wildcard import)
/// - `public_reexports` tracks that ISQSpaceTime's symbols are re-exported
#[derive(Clone, Debug, Default)]
pub struct ScopeVisibility {
    /// The scope this visibility applies to (e.g., "ISQ", "Automotive::Torque").
    scope: Arc<str>,

    /// Symbols defined directly in this scope.
    /// SimpleName → QualifiedName
    direct_defs: HashMap<Arc<str>, Arc<str>>,

    /// Symbols visible via imports (includes transitive public re-exports).
    /// SimpleName → QualifiedName (the resolved target)
    imports: HashMap<Arc<str>, Arc<str>>,

    /// Namespaces that are publicly re-exported from this scope.
    /// Used for transitive import resolution.
    public_reexports: Vec<Arc<str>>,
}

impl ScopeVisibility {
    /// Create a new empty visibility map for a scope.
    pub fn new(scope: impl Into<Arc<str>>) -> Self {
        Self {
            scope: scope.into(),
            direct_defs: HashMap::new(),
            imports: HashMap::new(),
            public_reexports: Vec::new(),
        }
    }

    /// Get the scope this visibility applies to.
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// Look up a simple name in this scope's visibility.
    ///
    /// Checks direct definitions first, then imports.
    /// Returns the qualified name if found.
    pub fn lookup(&self, name: &str) -> Option<&Arc<str>> {
        self.direct_defs
            .get(name)
            .or_else(|| self.imports.get(name))
    }

    /// Look up only in direct definitions.
    pub fn lookup_direct(&self, name: &str) -> Option<&Arc<str>> {
        self.direct_defs.get(name)
    }

    /// Look up only in imports.
    pub fn lookup_import(&self, name: &str) -> Option<&Arc<str>> {
        self.imports.get(name)
    }

    /// Add a direct definition to this scope.
    pub fn add_direct(&mut self, simple_name: Arc<str>, qualified_name: Arc<str>) {
        self.direct_defs.insert(simple_name, qualified_name);
    }

    /// Add an imported symbol to this scope.
    pub fn add_import(&mut self, simple_name: Arc<str>, qualified_name: Arc<str>) {
        // Don't overwrite direct definitions with imports
        if !self.direct_defs.contains_key(&simple_name) {
            self.imports.insert(simple_name, qualified_name);
        }
    }

    /// Add a public re-export (for transitive import resolution).
    pub fn add_public_reexport(&mut self, namespace: Arc<str>) {
        if !self.public_reexports.contains(&namespace) {
            self.public_reexports.push(namespace);
        }
    }

    /// Get all public re-exports.
    pub fn public_reexports(&self) -> &[Arc<str>] {
        &self.public_reexports
    }

    /// Get iterator over all direct definitions.
    pub fn direct_defs(&self) -> impl Iterator<Item = (&Arc<str>, &Arc<str>)> {
        self.direct_defs.iter()
    }

    /// Get iterator over all imports.
    pub fn imports(&self) -> impl Iterator<Item = (&Arc<str>, &Arc<str>)> {
        self.imports.iter()
    }

    /// Get count of visible symbols (direct + imported).
    pub fn len(&self) -> usize {
        self.direct_defs.len() + self.imports.len()
    }

    /// Check if visibility map is empty.
    pub fn is_empty(&self) -> bool {
        self.direct_defs.is_empty() && self.imports.is_empty()
    }

    /// Debug: dump contents of this visibility map.
    pub fn debug_dump(&self) -> String {
        let mut s = format!(
            "Scope '{}': {} direct, {} imports\n",
            self.scope,
            self.direct_defs.len(),
            self.imports.len()
        );
        for (name, qname) in self.direct_defs.iter().take(10) {
            s.push_str(&format!("  direct: {} -> {}\n", name, qname));
        }
        if self.direct_defs.len() > 10 {
            s.push_str(&format!(
                "  ... and {} more direct defs\n",
                self.direct_defs.len() - 10
            ));
        }
        for (name, qname) in self.imports.iter().take(10) {
            s.push_str(&format!("  import: {} -> {}\n", name, qname));
        }
        if self.imports.len() > 10 {
            s.push_str(&format!(
                "  ... and {} more imports\n",
                self.imports.len() - 10
            ));
        }
        s
    }
}

// ============================================================================
// SYMBOL INDEX
// ============================================================================

/// Index into the symbols vector.
pub type SymbolIdx = usize;

/// An index of all symbols across multiple files.
///
/// This is the main data structure for workspace-wide name resolution.
/// It includes pre-computed visibility maps for efficient query-time resolution.
///
/// Symbols are stored in a single vector (`symbols`) and referenced by index
/// from all other maps. This ensures consistency when symbols are mutated
/// (e.g., when resolving type references).
#[derive(Clone, Debug, Default)]
pub struct SymbolIndex {
    /// The single source of truth for all symbols.
    symbols: Vec<HirSymbol>,
    /// Index by qualified name -> symbol index.
    by_qualified_name: HashMap<Arc<str>, SymbolIdx>,
    /// Index by simple name -> symbol indices (may have multiple).
    by_simple_name: HashMap<Arc<str>, Vec<SymbolIdx>>,
    /// Index by short name (alias) -> symbol indices (for lookups like `kg` -> `SI::kilogram`).
    by_short_name: HashMap<Arc<str>, Vec<SymbolIdx>>,
    /// Index by file -> symbol indices.
    by_file: HashMap<FileId, Vec<SymbolIdx>>,
    /// Definitions only (not usages) -> symbol indices.
    definitions: HashMap<Arc<str>, SymbolIdx>,
    /// Pre-computed visibility map for each scope (built after all files added).
    visibility_map: HashMap<Arc<str>, ScopeVisibility>,
    /// Filters for each scope (e.g., "SafetyGroup" -> ["Safety"])
    /// Elements must have ALL listed metadata to be visible in that scope.
    /// These come from `filter @Metadata;` statements.
    scope_filters: HashMap<Arc<str>, Vec<Arc<str>>>,
    /// Filters for specific imports (import qualified name -> metadata names)
    /// These come from bracket syntax: `import X::*[@Filter]`
    import_filters: HashMap<Arc<str>, Vec<Arc<str>>>,
    /// Flag to track if visibility maps are stale and need rebuilding.
    visibility_dirty: bool,
}

impl SymbolIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add symbols and filters from an extraction result.
    pub fn add_extraction_result(
        &mut self,
        file: FileId,
        result: crate::hir::symbols::ExtractionResult,
    ) {
        // Add symbols
        self.add_file(file, result.symbols);

        // Add scope filters (from `filter @X;` statements)
        for (scope, metadata_names) in result.scope_filters {
            for name in metadata_names {
                self.add_scope_filter(scope.clone(), name);
            }
        }

        // Add import filters (from bracket syntax `import X::*[@Filter]`)
        for (import_qname, metadata_names) in result.import_filters {
            for name in metadata_names {
                self.import_filters
                    .entry(import_qname.clone())
                    .or_default()
                    .push(Arc::from(name));
            }
        }
    }

    /// Add symbols from a file to the index.
    pub fn add_file(&mut self, file: FileId, symbols: Vec<HirSymbol>) {
        // Remove existing symbols from this file first
        self.remove_file(file);

        // Mark visibility maps as dirty
        self.visibility_dirty = true;

        let mut file_indices = Vec::with_capacity(symbols.len());

        for symbol in symbols {
            let idx = self.symbols.len();

            // Index by qualified name
            self.by_qualified_name
                .insert(symbol.qualified_name.clone(), idx);

            // Index by simple name
            self.by_simple_name
                .entry(symbol.name.clone())
                .or_default()
                .push(idx);

            // Index by short name (e.g., <kg> for "kilogram")
            if let Some(ref short) = symbol.short_name {
                self.by_short_name
                    .entry(short.clone())
                    .or_default()
                    .push(idx);
            }

            // Track definitions separately
            if symbol.kind.is_definition() {
                self.definitions.insert(symbol.qualified_name.clone(), idx);
            }

            // Track for file index
            file_indices.push(idx);

            // Store the symbol
            self.symbols.push(symbol);
        }

        // Index by file
        self.by_file.insert(file, file_indices);
    }

    /// Add a filter for a scope. Elements imported into this scope must have
    /// the specified metadata to be visible.
    pub fn add_scope_filter(
        &mut self,
        scope: impl Into<Arc<str>>,
        metadata_name: impl Into<Arc<str>>,
    ) {
        self.visibility_dirty = true;
        self.scope_filters
            .entry(scope.into())
            .or_default()
            .push(metadata_name.into());
    }

    /// Remove all symbols from a file.
    ///
    /// Note: This marks indices as invalid but doesn't compact the symbols vec
    /// to avoid invalidating other indices. For a full cleanup, rebuild the index.
    pub fn remove_file(&mut self, file: FileId) {
        if let Some(indices) = self.by_file.remove(&file) {
            // Mark visibility maps as dirty
            self.visibility_dirty = true;

            for &idx in &indices {
                if let Some(symbol) = self.symbols.get(idx) {
                    let qname = symbol.qualified_name.clone();
                    let sname = symbol.name.clone();
                    let short = symbol.short_name.clone();

                    self.by_qualified_name.remove(&qname);
                    self.definitions.remove(&qname);

                    // Remove from simple name index
                    if let Some(list) = self.by_simple_name.get_mut(&sname) {
                        list.retain(|&i| i != idx);
                        if list.is_empty() {
                            self.by_simple_name.remove(&sname);
                        }
                    }

                    // Remove from short name index
                    if let Some(short_name) = short {
                        if let Some(list) = self.by_short_name.get_mut(&short_name) {
                            list.retain(|&i| i != idx);
                            if list.is_empty() {
                                self.by_short_name.remove(&short_name);
                            }
                        }
                    }
                }
            }
            // Note: We don't remove from self.symbols to preserve indices
            // A rebuild would be needed for true cleanup
        }
    }

    /// Look up a symbol by qualified name.
    pub fn lookup_qualified(&self, name: &str) -> Option<&HirSymbol> {
        self.by_qualified_name
            .get(name)
            .and_then(|&idx| self.symbols.get(idx))
    }

    /// Look up a symbol by qualified name (mutable).
    pub fn lookup_qualified_mut(&mut self, name: &str) -> Option<&mut HirSymbol> {
        self.by_qualified_name
            .get(name)
            .copied()
            .and_then(move |idx| self.symbols.get_mut(idx))
    }

    /// Look up all symbols with a simple name (also checks short names/aliases).
    pub fn lookup_simple(&self, name: &str) -> Vec<&HirSymbol> {
        let mut results = Vec::new();

        // Check by simple name
        if let Some(indices) = self.by_simple_name.get(name) {
            for &idx in indices {
                if let Some(sym) = self.symbols.get(idx) {
                    results.push(sym);
                }
            }
        }

        // Also check by short name (aliases like <kg> for "kilogram")
        if let Some(indices) = self.by_short_name.get(name) {
            for &idx in indices {
                if let Some(sym) = self.symbols.get(idx) {
                    // Avoid duplicates if name == short_name
                    if !results
                        .iter()
                        .any(|s| Arc::ptr_eq(&s.qualified_name, &sym.qualified_name))
                    {
                        results.push(sym);
                    }
                }
            }
        }

        results
    }

    /// Look up all symbols with a short name only.
    pub fn lookup_by_short_name(&self, name: &str) -> Vec<&HirSymbol> {
        self.by_short_name
            .get(name)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.symbols.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Debug: Find which scopes contain a name in their visibility map.
    pub fn debug_find_name_in_visibility(&self, name: &str) -> Vec<String> {
        let mut results = Vec::new();
        for (scope, vis) in &self.visibility_map {
            if vis.lookup_direct(name).is_some() {
                results.push(format!("{}: direct", scope));
            }
            if vis.lookup_import(name).is_some() {
                results.push(format!("{}: import", scope));
            }
        }
        results
    }

    /// Debug: Dump visibility map for a scope.
    pub fn debug_dump_scope(&self, scope: &str) -> String {
        self.visibility_map
            .get(scope)
            .map(|vis| vis.debug_dump())
            .unwrap_or_else(|| format!("No visibility map for scope '{}'", scope))
    }

    /// Look up a definition by qualified name.
    pub fn lookup_definition(&self, name: &str) -> Option<&HirSymbol> {
        self.definitions
            .get(name)
            .and_then(|&idx| self.symbols.get(idx))
    }

    /// Get all symbols in a file.
    pub fn symbols_in_file(&self, file: FileId) -> Vec<&HirSymbol> {
        self.by_file
            .get(&file)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.symbols.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all definitions in the index.
    pub fn all_definitions(&self) -> impl Iterator<Item = &HirSymbol> {
        self.definitions
            .values()
            .filter_map(|&idx| self.symbols.get(idx))
    }

    /// Get all symbols in the index.
    pub fn all_symbols(&self) -> impl Iterator<Item = &HirSymbol> {
        self.by_qualified_name
            .values()
            .filter_map(|&idx| self.symbols.get(idx))
    }

    /// Get the total number of symbols.
    pub fn len(&self) -> usize {
        self.by_qualified_name.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.by_qualified_name.is_empty()
    }

    /// Get number of files indexed.
    pub fn file_count(&self) -> usize {
        self.by_file.len()
    }

    /// Insert a single symbol into the index.
    /// This is a convenience wrapper around add_file for single-symbol insertion.
    pub fn insert(&mut self, symbol: HirSymbol) {
        // Use a dummy file ID for test/debug purposes
        let file = FileId::new(0);
        let idx = self.symbols.len();

        // Index by qualified name
        self.by_qualified_name
            .insert(symbol.qualified_name.clone(), idx);

        // Index by simple name
        self.by_simple_name
            .entry(symbol.name.clone())
            .or_default()
            .push(idx);

        // Index by short name
        if let Some(ref short) = symbol.short_name {
            self.by_short_name
                .entry(short.clone())
                .or_default()
                .push(idx);
        }

        // Track definitions
        if symbol.kind.is_definition() {
            self.definitions.insert(symbol.qualified_name.clone(), idx);
        }

        // Track for file index
        self.by_file.entry(file).or_default().push(idx);

        // Store the symbol
        self.symbols.push(symbol);

        // Mark visibility maps as dirty
        self.visibility_dirty = true;
    }

    /// Get a reference to the visibility maps.
    /// Call ensure_visibility_maps() first to ensure they are up-to-date.
    pub fn visibility_maps(&self) -> &HashMap<Arc<str>, ScopeVisibility> {
        &self.visibility_map
    }

    // ========================================================================
    // VISIBILITY MAP CONSTRUCTION
    // ========================================================================

    /// Ensure visibility maps are up-to-date, rebuilding if necessary.
    ///
    /// Call this before using visibility-based resolution.
    pub fn ensure_visibility_maps(&mut self) {
        if self.visibility_dirty {
            self.build_visibility_maps();
            self.visibility_dirty = false;
        }
    }

    /// Resolve all type references in all symbols.
    ///
    /// This is called after visibility maps are built to fill in `resolved_target`
    /// on all TypeRefs. This is the "semantic resolution pass" that pre-computes
    /// what each type reference points to.
    ///
    /// Feature chains (like `takePicture.focus`) are now preserved explicitly
    /// as TypeRefKind::Chain from the parser. Simple refs use TypeRefKind::Simple.
    pub fn resolve_all_type_refs(&mut self) {
        use crate::hir::symbols::TypeRefKind;

        // Ensure visibility maps are built first
        self.ensure_visibility_maps();

        // Memoization cache for scope walk results: (name, starting_scope) -> resolved_qname
        // This avoids re-resolving the same name from the same scope multiple times
        let mut resolution_cache: ResolutionCache = HashMap::new();

        // Two-pass resolution to handle dependencies:
        // Pass 1: Resolve simple refs and chain first-parts (they don't depend on other refs)
        // Pass 2: Resolve chain subsequent parts (they depend on the first part's resolved type)

        use std::rc::Rc;

        // Collect work items, separating first-parts from subsequent chain parts
        // Each item: (sym_idx, trk_idx, part_idx, target, chain_context, ref_kind)
        type WorkItem = (
            SymbolIdx,
            usize,
            usize,
            Arc<str>,
            Option<(Rc<Vec<Arc<str>>>, usize)>,
            RefKind,
        );
        let mut pass1_work: Vec<WorkItem> = Vec::new();
        let mut pass2_work: Vec<WorkItem> = Vec::new();

        for (sym_idx, sym) in self.symbols.iter().enumerate() {
            for (trk_idx, trk) in sym.type_refs.iter().enumerate() {
                match trk {
                    TypeRefKind::Simple(tr) => {
                        // Simple refs go in pass 1
                        pass1_work.push((sym_idx, trk_idx, 0, tr.target.clone(), None, tr.kind));
                    }
                    TypeRefKind::Chain(chain) => {
                        let chain_parts: Rc<Vec<Arc<str>>> =
                            Rc::new(chain.parts.iter().map(|p| p.target.clone()).collect());
                        for (part_idx, part) in chain.parts.iter().enumerate() {
                            let item = (
                                sym_idx,
                                trk_idx,
                                part_idx,
                                part.target.clone(),
                                Some((Rc::clone(&chain_parts), part_idx)),
                                part.kind,
                            );
                            if part_idx == 0 {
                                // First part of chain - pass 1
                                pass1_work.push(item);
                            } else {
                                // Subsequent parts - pass 2 (depend on first part's type)
                                pass2_work.push(item);
                            }
                        }
                    }
                }
            }
        }

        // Pass 1: Resolve simple refs and chain first-parts
        for (sym_idx, trk_idx, part_idx, target, chain_context, ref_kind) in pass1_work {
            let symbol_qname = self.symbols[sym_idx].qualified_name.clone();
            let mut resolved = self.resolve_type_ref_cached(
                &symbol_qname,
                &target,
                &chain_context,
                &mut resolution_cache,
            );

            // For unresolved Redefines refs, try satisfy/perform context resolution
            if resolved.is_none() && ref_kind == RefKind::Redefines {
                resolved = self.resolve_redefines_in_context(&symbol_qname, &target);
            }

            if let Some(trk) = self.symbols[sym_idx].type_refs.get_mut(trk_idx) {
                match trk {
                    TypeRefKind::Simple(tr) => {
                        tr.resolved_target = resolved;
                    }
                    TypeRefKind::Chain(chain) => {
                        if let Some(part) = chain.parts.get_mut(part_idx) {
                            part.resolved_target = resolved;
                        }
                    }
                }
            }
        }

        // Pass 2: Resolve chain subsequent parts (can now use resolved types from pass 1)
        for (sym_idx, trk_idx, part_idx, target, chain_context, _ref_kind) in pass2_work {
            let symbol_qname = self.symbols[sym_idx].qualified_name.clone();
            let resolved = self.resolve_type_ref_cached(
                &symbol_qname,
                &target,
                &chain_context,
                &mut resolution_cache,
            );

            if let Some(TypeRefKind::Chain(chain)) =
                self.symbols[sym_idx].type_refs.get_mut(trk_idx)
            {
                if let Some(part) = chain.parts.get_mut(part_idx) {
                    part.resolved_target = resolved;
                }
            }
        }
    }

    /// Resolve a single type reference within a symbol's scope (with caching).
    ///
    /// For regular references: uses lexical scoping + imports
    /// For feature chain members: resolves through type membership
    fn resolve_type_ref_cached(
        &self,
        containing_symbol: &str,
        target: &str,
        chain_context: &Option<(std::rc::Rc<Vec<Arc<str>>>, usize)>,
        cache: &mut ResolutionCache,
    ) -> Option<Arc<str>> {
        // Get the scope for resolution
        // For import symbols (e.g., "Pkg::import:Target" or "import:Target"), use the parent scope
        let scope = if let Some(import_pos) = containing_symbol.find("::import:") {
            &containing_symbol[..import_pos]
        } else if containing_symbol.starts_with("import:") {
            // Root-level import - use empty scope
            ""
        } else {
            containing_symbol
        };

        // Check if this is a feature chain member (index > 0)
        // Chain members can't be cached the same way (they depend on the full chain)
        if let Some((chain_parts, chain_idx)) = chain_context {
            if *chain_idx > 0 {
                return self.resolve_feature_chain_member(
                    scope,
                    chain_parts.as_slice(),
                    *chain_idx,
                );
            }
        }

        // For simple references, use cache
        let cache_key = (Arc::from(target), Arc::from(scope));
        if let Some(cached) = cache.get(&cache_key) {
            return cached.clone();
        }

        // Not in cache - do the actual resolution
        let result = if let Some(sym) = self.resolve_with_scope_walk(target, scope) {
            Some(sym.qualified_name.clone())
        } else {
            self.lookup_qualified(target)
                .map(|s| s.qualified_name.clone())
        };

        // Store in cache
        cache.insert(cache_key, result.clone());
        result
    }

    /// Follow a typing chain to find the actual type definition.
    ///
    /// For example, if we have:
    ///   action takePicture : TakePicture;  // usage typed by definition
    ///   action a :> takePicture;           // usage subsets usage
    ///
    /// When resolving from `a`, we need to follow: a -> takePicture -> TakePicture
    ///
    /// IMPORTANT: If the input symbol is already a definition, return it immediately.
    /// We only follow the chain for usages, not for definition inheritance.
    fn follow_typing_chain(&self, sym: &HirSymbol, scope: &str) -> Arc<str> {
        // If the input is already a definition, return it - don't follow inheritance
        if sym.kind.is_definition() {
            return sym.qualified_name.clone();
        }

        let mut current_qname = sym.qualified_name.clone();
        let mut visited = std::collections::HashSet::new();
        visited.insert(current_qname.clone());

        // Keep following supertypes until we find a definition or loop
        while let Some(current) = self.lookup_qualified(&current_qname) {
            let Some(type_name) = current.supertypes.first() else {
                // No supertypes
                break;
            };

            let type_resolver = Resolver::new(self).with_scope(scope);
            let ResolveResult::Found(type_sym) = type_resolver.resolve(type_name) else {
                // Can't resolve further, use what we have
                break;
            };

            if visited.contains(&type_sym.qualified_name) {
                // Cycle detected, stop here
                break;
            }
            visited.insert(type_sym.qualified_name.clone());

            // If this symbol is a definition, return it
            if type_sym.kind.is_definition() {
                return type_sym.qualified_name.clone();
            }

            // Otherwise continue following
            current_qname = type_sym.qualified_name.clone();
        }

        current_qname
    }

    /// Resolve a feature chain member (e.g., `focus` in `takePicture.focus`).
    ///
    /// Chain resolution follows rust-analyzer's approach:
    /// 1. Resolve first part using full lexical scoping (walks up parent scopes)
    /// 2. Get that symbol's type definition
    /// 3. Resolve subsequent parts as members of that type
    /// 4. For each member, follow its type to resolve the next part
    ///
    /// IMPORTANT: SysML usages can have nested members defined directly within them,
    /// even when they have a type annotation. We must check the usage's own scope
    /// BEFORE falling back to its type definition.
    pub fn resolve_feature_chain_member(
        &self,
        scope: &str,
        chain_parts: &[Arc<str>],
        chain_idx: usize,
    ) -> Option<Arc<str>> {
        if chain_idx == 0 || chain_parts.is_empty() {
            return None;
        }

        // Step 1: Resolve the first part using full lexical scoping
        // This walks up the scope hierarchy to find the symbol
        let first_part = &chain_parts[0];
        let first_sym = self.resolve_with_scope_walk(first_part, scope)?;

        // Track the current symbol (for checking nested members) and its type scope (for inheritance)
        let mut current_sym_qname = first_sym.qualified_name.clone();
        let mut current_type_scope = self.get_member_lookup_scope(&first_sym, scope);

        // Step 2: Walk through the chain, resolving each part
        for (i, part) in chain_parts.iter().enumerate().take(chain_idx + 1).skip(1) {
            // SysML Pattern: Usages can have nested members defined directly within them.
            // For example: part differential:Differential { port leftDiffPort:DiffPort; }
            // Here `leftDiffPort` is a member of the usage, not the Differential definition.
            //
            // Strategy: First try to find member in the symbol's own scope (nested members),
            // then fall back to the type scope (inherited members).

            let member_sym = {
                // Try 1: Look for nested member directly in the current symbol
                if let Some(sym) = self.find_member_in_scope(&current_sym_qname, part) {
                    sym
                } else if current_sym_qname != current_type_scope {
                    // Try 2: Look in the type scope (inherited members)
                    self.find_member_in_scope(&current_type_scope, part)?
                } else {
                    return None;
                }
            };

            if i == chain_idx {
                // This is the target - return it
                return Some(member_sym.qualified_name.clone());
            }

            // Update for next iteration: track both the symbol and its type scope
            current_sym_qname = member_sym.qualified_name.clone();
            current_type_scope = self.get_member_lookup_scope(&member_sym, scope);
        }

        None
    }

    /// Resolve a name using visibility maps (which already handle scope hierarchy).
    ///
    /// NOTE: Resolver::resolve() already walks up the scope hierarchy internally,
    /// so we just need to call it once with the starting scope.
    fn resolve_with_scope_walk(&self, name: &str, starting_scope: &str) -> Option<HirSymbol> {
        let resolver = Resolver::new(self).with_scope(starting_scope);
        match resolver.resolve(name) {
            ResolveResult::Found(sym) => Some(sym),
            _ => None,
        }
    }

    /// Get the scope to use for member lookups on a symbol.
    /// If the symbol has a type, returns the type's qualified name.
    /// Otherwise, returns the symbol's own qualified name (for nested members).
    ///
    /// Checks the symbol's resolved type_refs first (if available), then falls back
    /// to resolving the supertype name. This ensures we use the same type resolution
    /// that was computed for the symbol's own type annotation.
    ///
    /// For interface endpoints with `::>` (References), we follow the reference to find
    /// where members actually live. E.g., `connect lugNutPort ::> wheel1.lugNutCompositePort`
    /// means members of `lugNutPort` are actually in `wheel1.lugNutCompositePort`.
    fn get_member_lookup_scope(&self, sym: &HirSymbol, resolution_scope: &str) -> Arc<str> {
        // First, check if the symbol has a resolved type_ref (from its : TypeAnnotation)
        // This is more accurate than re-resolving the name because it uses the same
        // resolution context that was used for the symbol's own typing.
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                // Look for typed-by refs with resolved targets
                if tr.kind == crate::hir::symbols::RefKind::TypedBy {
                    if let Some(ref resolved) = tr.resolved_target {
                        // Got a pre-resolved type - use it
                        if let Some(type_sym) = self.lookup_qualified(resolved) {
                            if type_sym.kind.is_definition() {
                                return type_sym.qualified_name.clone();
                            }
                            // If it's a usage, follow the typing chain
                            return self.follow_typing_chain(type_sym, resolution_scope);
                        }
                    }
                }
            }
        }

        // For interface endpoints: check for ::> (References) relationships
        // These indicate the endpoint is a proxy for another scope where members live.
        // E.g., `connect lugNutPort ::> wheel1.lugNutCompositePort` means members
        // of lugNutPort are actually defined in wheel1.lugNutCompositePort.
        for trk in &sym.type_refs {
            match trk {
                crate::hir::TypeRefKind::Chain(chain) => {
                    // Check if this is a References chain
                    if let Some(first_part) = chain.parts.first() {
                        if first_part.kind == crate::hir::symbols::RefKind::References {
                            // This is a ::> chain - follow the last resolved part
                            if let Some(last_part) = chain.parts.last() {
                                if let Some(ref resolved) = last_part.resolved_target {
                                    return resolved.clone();
                                }
                            }
                        }
                    }
                }
                crate::hir::TypeRefKind::Simple(tr) => {
                    // Also handle simple References (non-chain)
                    if tr.kind == crate::hir::symbols::RefKind::References {
                        if let Some(ref resolved) = tr.resolved_target {
                            return resolved.clone();
                        }
                    }
                }
            }
        }

        // Fallback: resolve the supertype name (for symbols without resolved type_refs yet)
        if let Some(type_name) = sym.supertypes.first() {
            let sym_scope = Self::parent_scope(&sym.qualified_name).unwrap_or("");

            if let Some(type_sym) = self.resolve_with_scope_walk(type_name, sym_scope) {
                if type_sym.kind.is_usage() {
                    return type_sym.qualified_name.clone();
                }
                return self.follow_typing_chain(&type_sym, resolution_scope);
            }

            if let Some(type_sym) = self.lookup_qualified(type_name) {
                if type_sym.kind.is_usage() {
                    return type_sym.qualified_name.clone();
                }
                return self.follow_typing_chain(type_sym, resolution_scope);
            }
        }

        // No type - use the symbol itself as the scope for nested members
        sym.qualified_name.clone()
    }

    /// Find a member within a type scope.
    /// Tries direct lookup, then searches inherited members from supertypes,
    /// then checks chain-based relationships (perform, exhibit, satisfy, etc.).
    pub fn find_member_in_scope(&self, type_scope: &str, member_name: &str) -> Option<HirSymbol> {
        let mut visited = HashSet::new();
        self.find_member_in_scope_internal(type_scope, member_name, &mut visited)
    }

    /// Internal implementation with visited tracking to prevent infinite loops.
    fn find_member_in_scope_internal(
        &self,
        type_scope: &str,
        member_name: &str,
        visited: &mut HashSet<String>,
    ) -> Option<HirSymbol> {
        // Check for cycles - if we've already visited this scope, skip it
        if !visited.insert(type_scope.to_string()) {
            return None;
        }

        // Strategy 1: Direct qualified lookup
        let direct_qname = format!("{}::{}", type_scope, member_name);
        if let Some(sym) = self.lookup_qualified(&direct_qname) {
            return Some(sym.clone());
        }

        // Strategy 2: Check visibility map for the type scope
        if let Some(vis) = self.visibility_for_scope(type_scope) {
            if let Some(qname) = vis.lookup(member_name) {
                if let Some(sym) = self.lookup_qualified(qname) {
                    return Some(sym.clone());
                }
            }
        }

        // Strategy 3: Look in supertypes (inheritance)
        if let Some(type_sym) = self.lookup_qualified(type_scope) {
            for supertype in &type_sym.supertypes {
                // Resolve the supertype name
                let parent_scope = Self::parent_scope(type_scope).unwrap_or("");
                if let Some(super_sym) = self.resolve_with_scope_walk(supertype, parent_scope) {
                    // Recursively search in the supertype (with visited tracking)
                    if let Some(found) = self.find_member_in_scope_internal(
                        &super_sym.qualified_name,
                        member_name,
                        visited,
                    ) {
                        return Some(found);
                    }
                }
            }

            // Strategy 4: Check chain-based type_refs on the symbol
            // In SysML, relationships like `perform x.y`, `exhibit a.b`, `satisfy r.s`
            // make the target accessible as a member of the containing symbol.
            // e.g., `part driver { perform startVehicle.turnVehicleOn; }`
            // makes `turnVehicleOn` accessible as `driver.turnVehicleOn`
            for trk in &type_sym.type_refs {
                use crate::hir::symbols::TypeRefKind;
                match trk {
                    TypeRefKind::Chain(chain) => {
                        // Check if the last part of the chain matches the member we're looking for
                        if let Some(last_part) = chain.parts.last() {
                            if &*last_part.target == member_name {
                                // Use the pre-resolved target (should be set by resolve_all_type_refs)
                                if let Some(ref resolved) = last_part.resolved_target {
                                    if let Some(sym) = self.lookup_qualified(resolved) {
                                        return Some(sym.clone());
                                    }
                                }
                            }
                        }
                    }
                    TypeRefKind::Simple(tr) => {
                        // Legacy: also check simple refs that might contain dots
                        let target_name = tr.target.as_ref();
                        if target_name.contains('.') {
                            let last_part = target_name.rsplit('.').next().unwrap_or(target_name);
                            if last_part == member_name {
                                // Use the pre-resolved target (should be set by resolve_all_type_refs)
                                if let Some(ref resolved) = tr.resolved_target {
                                    if let Some(sym) = self.lookup_qualified(resolved) {
                                        return Some(sym.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve a Redefines ref by looking at the parent's satisfy/perform context.
    /// For `satisfy Req by Subject { :>> reqMember; }`, the reqMember should resolve
    /// to a member of Req (the satisfied requirement), not Subject.
    fn resolve_redefines_in_context(
        &self,
        symbol_qname: &str,
        member_name: &str,
    ) -> Option<Arc<str>> {
        // Get the parent scope
        let parent_qname = symbol_qname.rsplit_once("::")?.0;

        // Look up the parent symbol
        let parent = self.lookup_qualified(parent_qname)?;

        // First, check the parent's type_refs for satisfy context
        if let Some(result) = self.check_satisfy_context(parent, member_name) {
            return Some(result);
        }

        // If not found on parent, check ALL symbols in the same parent scope and its descendants
        // This handles cases where the parser places the satisfy relationship
        // on a nested symbol rather than the parent
        let parent_prefix = format!("{}::", parent_qname);
        for sym in &self.symbols {
            // Check if this symbol is under the parent scope
            if sym.qualified_name.starts_with(parent_prefix.as_str())
                && sym.qualified_name.as_ref() != symbol_qname
            {
                if let Some(result) = self.check_satisfy_context(sym, member_name) {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Check a symbol's type_refs for satisfy context and try to resolve member in that context
    fn check_satisfy_context(&self, sym: &HirSymbol, member_name: &str) -> Option<Arc<str>> {
        for type_ref_kind in &sym.type_refs {
            let type_ref = match type_ref_kind {
                TypeRefKind::Simple(tr) => tr,
                TypeRefKind::Chain(chain) => {
                    // For chains, use the first part if it has a resolved target
                    if let Some(part) = chain.parts.first() {
                        part
                    } else {
                        continue;
                    }
                }
            };

            // Skip refs without resolved targets
            let Some(resolved_qname) = type_ref.resolved_target.as_ref() else {
                continue;
            };

            // Look for satisfied requirement - can be Subsets (from :>) or Other (from satisfy)
            // The satisfy relationship creates a ref to the requirement being satisfied
            // We check the target's kind rather than the ref kind to be more robust
            let Some(target_type) = self.lookup_qualified(resolved_qname) else {
                continue;
            };

            // Check if target is a requirement-like definition
            if matches!(
                target_type.kind,
                SymbolKind::RequirementDef
                    | SymbolKind::RequirementUsage
                    | SymbolKind::ConstraintDef
                    | SymbolKind::ConstraintUsage
            ) {
                // Look for the member in the requirement's scope
                let member_qname = format!("{}::{}", resolved_qname, member_name);
                if self.lookup_qualified(&member_qname).is_some() {
                    return Some(Arc::from(member_qname));
                }

                // Also check visibility map for inherited members
                if let Some(vis) = self.visibility_for_scope(resolved_qname) {
                    if let Some(qname) = vis.lookup(member_name) {
                        if self.lookup_qualified(qname).is_some() {
                            return Some(Arc::from(qname.as_ref()));
                        }
                    }
                }
            }
        }

        None
    }

    /// Get the visibility map for a scope (if built).
    pub fn visibility_for_scope(&self, scope: &str) -> Option<&ScopeVisibility> {
        self.visibility_map.get(scope)
    }

    /// Build visibility maps for all scopes.
    ///
    /// This is the main entry point for constructing visibility information.
    /// It performs:
    /// 1. Single-pass scope collection and direct definition grouping
    /// 2. Inheritance propagation (supertypes' members become visible)
    /// 3. Import processing with transitive public re-export handling
    fn build_visibility_maps(&mut self) {
        eprintln!("[TRACE build_visibility_maps] Starting visibility map build");
        // 1. Single pass: collect scopes AND group symbols by parent scope
        // This is O(symbols) instead of O(scopes × symbols)
        self.visibility_map.clear();

        // Pre-create root scope
        self.visibility_map
            .insert(Arc::from(""), ScopeVisibility::new(""));

        for symbol in &self.symbols {
            eprintln!("[TRACE build_visibility_maps] Processing symbol: {} ({:?})", symbol.qualified_name, symbol.kind);
            // Ensure this symbol's scope exists (for namespace-creating symbols)
            // Include usages too - they can have nested members and need inherited members from their type
            if symbol.kind == SymbolKind::Package
                || symbol.kind.is_definition()
                || symbol.kind.is_usage()
            {
                eprintln!("[TRACE build_visibility_maps]   Creating scope for: {}", symbol.qualified_name);
                self.visibility_map
                    .entry(symbol.qualified_name.clone())
                    .or_insert_with(|| ScopeVisibility::new(symbol.qualified_name.clone()));
            }

            // Skip adding import symbols as direct definitions - they're processed separately
            // and shouldn't shadow global packages with the same name
            if symbol.kind == SymbolKind::Import {
                eprintln!("[TRACE build_visibility_maps]   Skipping import symbol (handled separately): {}", symbol.qualified_name);
                continue;
            }

            // Add symbol to its parent scope's direct definitions
            let parent_scope: Arc<str> = Self::parent_scope(&symbol.qualified_name)
                .map(Arc::from)
                .unwrap_or_else(|| Arc::from(""));

            eprintln!("[TRACE build_visibility_maps]   Adding '{}' to parent scope '{}'", symbol.name, parent_scope);

            // Ensure parent scope exists
            let vis = self
                .visibility_map
                .entry(parent_scope.clone())
                .or_insert_with(|| ScopeVisibility::new(parent_scope.clone()));

            vis.add_direct(symbol.name.clone(), symbol.qualified_name.clone());

            // Also register by short_name if available
            if let Some(ref short_name) = symbol.short_name {
                vis.add_direct(short_name.clone(), symbol.qualified_name.clone());
            }
            
            // If the parent scope is anonymous (contains `<` which indicates generated names),
            // also add this symbol to the grandparent scope so it's accessible from siblings.
            // This handles cases like `then action foo { ... }` where `foo` needs to be visible
            // from the enclosing scope, not just from the anonymous succession scope.
            if parent_scope.contains('<') {
                if let Some(grandparent) = Self::parent_scope(&parent_scope) {
                    eprintln!("[TRACE build_visibility_maps]   Also adding '{}' to grandparent scope '{}' (parent is anonymous)", symbol.name, grandparent);
                    let grandparent_arc: Arc<str> = Arc::from(grandparent);
                    let gp_vis = self
                        .visibility_map
                        .entry(grandparent_arc.clone())
                        .or_insert_with(|| ScopeVisibility::new(grandparent_arc));
                    gp_vis.add_direct(symbol.name.clone(), symbol.qualified_name.clone());
                    if let Some(ref short_name) = symbol.short_name {
                        gp_vis.add_direct(short_name.clone(), symbol.qualified_name.clone());
                    }
                }
            }
        }

        eprintln!("[TRACE build_visibility_maps] Processing imports...");
        // 3. Process all imports FIRST (needed for inheritance to resolve types via imports)
        let mut visited: HashSet<(Arc<str>, Arc<str>)> = HashSet::new();
        let scope_keys: Vec<_> = self.visibility_map.keys().cloned().collect();

        for scope in &scope_keys {
            eprintln!("[TRACE build_visibility_maps] Processing imports for scope: '{}'", scope);
            self.process_imports_recursive(&scope, &mut visited);
        }

        eprintln!("[TRACE build_visibility_maps] Propagating inherited members...");
        // 4. Propagate inherited members from supertypes (can now resolve types via imports)
        self.propagate_inherited_members();
        
        eprintln!("[TRACE build_visibility_maps] Final visibility maps:");
        for (scope, vis) in &self.visibility_map {
            eprintln!("[TRACE build_visibility_maps]   Scope '{}': direct={:?}, imports={:?}", 
                scope, 
                vis.direct_defs().map(|(n, _)| n.to_string()).collect::<Vec<_>>(),
                vis.imports().map(|(n, _)| n.to_string()).collect::<Vec<_>>()
            );
        }
    }

    /// Propagate inherited members from supertypes into scope visibility maps.
    /// When `Shape :> Path`, members of `Path` become visible in `Shape`.
    fn propagate_inherited_members(&mut self) {
        // Collect inheritance info: (scope, resolved_supertype_qname)
        let mut inheritance_pairs: Vec<(Arc<str>, Arc<str>)> = Vec::new();

        for symbol in &self.symbols {
            if !symbol.supertypes.is_empty() {
                let scope = &symbol.qualified_name;
                let parent_scope = Self::parent_scope(scope).unwrap_or("");

                for supertype in &symbol.supertypes {
                    // Resolve supertype name to qualified name
                    if let Some(resolved) =
                        self.resolve_supertype_for_inheritance(supertype, parent_scope)
                    {
                        inheritance_pairs.push((scope.clone(), resolved));
                    }
                }
            }
        }

        // Now propagate: for each (child_scope, parent_scope), add parent's direct members to child
        for (child_scope, parent_scope) in inheritance_pairs {
            // Get parent's direct members
            let parent_members: Vec<(Arc<str>, Arc<str>)> = self
                .visibility_map
                .get(&parent_scope)
                .map(|vis| {
                    vis.direct_defs
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .unwrap_or_default();

            // Add to child's visibility (if not already present - direct takes priority)
            if let Some(child_vis) = self.visibility_map.get_mut(&child_scope) {
                for (name, qname) in parent_members {
                    child_vis.direct_defs.entry(name).or_insert(qname);
                }
            }
        }
    }

    /// Resolve a supertype reference for inheritance propagation.
    /// Uses visibility maps (including imports) for resolution.
    fn resolve_supertype_for_inheritance(
        &self,
        name: &str,
        starting_scope: &str,
    ) -> Option<Arc<str>> {
        // Try qualified lookup first
        if let Some(sym) = self.lookup_qualified(name) {
            return Some(sym.qualified_name.clone());
        }

        // Walk up scopes looking for the name
        let mut current_scope = starting_scope;
        loop {
            // Try direct qualified name in this scope
            let qname = if current_scope.is_empty() {
                name.to_string()
            } else {
                format!("{}::{}", current_scope, name)
            };

            if let Some(sym) = self.lookup_qualified(&qname) {
                return Some(sym.qualified_name.clone());
            }

            // Check visibility map for this scope (both direct defs AND imports)
            if let Some(vis) = self.visibility_map.get(current_scope) {
                // Check direct definitions first
                if let Some(resolved) = vis.direct_defs.get(name) {
                    return Some(resolved.clone());
                }
                // Also check imports (important for types imported via `import X::*`)
                if let Some(resolved) = vis.imports.get(name) {
                    return Some(resolved.clone());
                }
            }

            if current_scope.is_empty() {
                break;
            }
            current_scope = Self::parent_scope(current_scope).unwrap_or("");
        }
        None
    }

    /// Helper to check if a symbol passes a given list of filters.
    fn symbol_passes_filters_list(&self, symbol_qname: &str, filters: &[Arc<str>]) -> bool {
        // Find the symbol by qualified name
        let symbol = match self.by_qualified_name.get(symbol_qname) {
            Some(&idx) => &self.symbols[idx],
            None => return true, // If we can't find the symbol, let it through
        };

        // Check if symbol has ALL required metadata
        for required_metadata in filters {
            let has_metadata = symbol
                .metadata_annotations
                .iter()
                .any(|ann| ann.as_ref() == required_metadata.as_ref());
            if !has_metadata {
                return false;
            }
        }
        true
    }

    /// Process imports for a scope recursively, handling transitive public re-exports.
    fn process_imports_recursive(
        &mut self,
        scope: &str,
        visited: &mut HashSet<(Arc<str>, Arc<str>)>,
    ) {
        eprintln!("[TRACE process_imports_recursive] Processing scope: '{}'", scope);
        
        // Find import symbols in this scope - extract needed fields
        let imports_to_process: Vec<(Arc<str>, Arc<str>, bool)> = self
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Import)
            .filter(|s| {
                let qname = s.qualified_name.as_ref();
                if let Some(idx) = qname.find("::import:") {
                    &qname[..idx] == scope
                } else if qname.starts_with("import:") {
                    scope.is_empty()
                } else {
                    false
                }
            })
            .map(|s| (s.name.clone(), s.qualified_name.clone(), s.is_public))
            .collect();

        eprintln!("[TRACE process_imports_recursive]   Found {} imports to process", imports_to_process.len());

        for (import_name, import_qname, is_public) in imports_to_process {
            eprintln!("[TRACE process_imports_recursive]   Import: '{}' (qname='{}', public={})", import_name, import_qname, is_public);
            let is_wildcard = import_name.ends_with("::*") && !import_name.ends_with("::**");
            let is_recursive = import_name.ends_with("::**");

            // Trim the wildcard/recursive suffix to get the base target
            let import_target = if is_recursive {
                import_name.trim_end_matches("::**")
            } else {
                import_name.trim_end_matches("::*")
            };
            let resolved_target = self.resolve_import_target(scope, import_target);
            eprintln!("[TRACE process_imports_recursive]     is_wildcard={}, is_recursive={}, import_target='{}', resolved_target='{}'", 
                is_wildcard, is_recursive, import_target, resolved_target);

            if is_wildcard || is_recursive {
                // Wildcard or recursive import: import symbols from target scope

                // Skip if already visited this (scope, target) pair
                let key = (Arc::from(scope), Arc::from(resolved_target.as_str()));
                if visited.contains(&key) {
                    eprintln!("[TRACE process_imports_recursive]     SKIPPING - already visited");
                    continue;
                }
                visited.insert(key);

                // Recursively process the target's imports first (to get transitive symbols)
                self.process_imports_recursive(&resolved_target, visited);

                // Get filter info - both scope filters and import-specific filters
                let scope_filters = self.scope_filters.get(scope).cloned();
                let import_filters = self.import_filters.get(import_qname.as_ref()).cloned();

                // Combine filters: import filters take precedence, then scope filters
                let active_filters = import_filters.or(scope_filters);

                // Now copy symbols from target to this scope
                if let Some(target_vis) = self.visibility_map.get(&resolved_target as &str).cloned()
                {
                    eprintln!("[TRACE process_imports_recursive]     target_vis direct_defs={:?}", 
                        target_vis.direct_defs().map(|(n, _)| n.to_string()).collect::<Vec<_>>());
                    // Collect symbols to import (applying filter)
                    let direct_defs_to_import: Vec<_> = target_vis
                        .direct_defs()
                        .filter(|(_, qname)| {
                            // Apply filter if present
                            if let Some(ref filters) = active_filters {
                                if !filters.is_empty() {
                                    return self.symbol_passes_filters_list(qname, filters);
                                }
                            }
                            true
                        })
                        .map(|(n, q)| (n.clone(), q.clone()))
                        .collect();

                    let vis = self
                        .visibility_map
                        .get_mut(scope)
                        .expect("scope must exist");

                    // Copy direct definitions (filtered)
                    for (name, qname) in direct_defs_to_import {
                        vis.add_import(name, qname);
                    }

                    // Only copy imports that come from publicly re-exported namespaces
                    // Private imports should NOT be transitively visible
                    let public_reexports = target_vis.public_reexports();
                    for (name, qname) in target_vis.imports() {
                        // Check if this import comes from a publicly re-exported namespace
                        let is_from_public_reexport = public_reexports.iter().any(|ns| {
                            qname.starts_with(ns.as_ref())
                                && (qname.len() == ns.len()
                                    || qname.as_bytes().get(ns.len()) == Some(&b':'))
                        });
                        if is_from_public_reexport {
                            vis.add_import(name.clone(), qname.clone());
                        }
                    }

                    if is_public {
                        vis.add_public_reexport(Arc::from(resolved_target.as_str()));
                        // Also propagate the target's public reexports for transitive chains
                        for reexport in public_reexports {
                            vis.add_public_reexport(reexport.clone());
                        }
                    }
                }

                // For recursive imports, also import all descendants
                if is_recursive {
                    self.import_descendants(scope, &resolved_target, &active_filters);
                }
            } else {
                // Specific import: import a single symbol
                // E.g., `import EngineDefs::Engine;` makes `Engine` visible as `EngineDefs::Engine`

                // Get the simple name (last component of path)
                let simple_name = resolved_target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&resolved_target);

                // Add to this scope's imports
                if let Some(vis) = self.visibility_map.get_mut(scope) {
                    vis.add_import(Arc::from(simple_name), Arc::from(resolved_target.as_str()));
                }
            }
        }
    }

    /// Import all descendants of a scope (for recursive imports like ::**).
    ///
    /// This imports all symbols that are nested under the target scope,
    /// not just direct children.
    fn import_descendants(
        &mut self,
        importing_scope: &str,
        target_scope: &str,
        filters: &Option<Vec<Arc<str>>>,
    ) {
        let target_prefix = format!("{}::", target_scope);

        // Find all symbols that are descendants of target_scope
        let descendant_symbols: Vec<(Arc<str>, Arc<str>)> = self
            .symbols
            .iter()
            .filter(|s| {
                // Skip imports, they're processed separately
                if s.kind == SymbolKind::Import || !s.qualified_name.starts_with(&target_prefix) {
                    return false;
                }
                // Apply filter if present
                if let Some(filter_list) = filters {
                    if !filter_list.is_empty() {
                        return self.symbol_passes_filters_list_static(
                            &s.metadata_annotations,
                            filter_list,
                        );
                    }
                }
                true
            })
            .map(|s| (s.name.clone(), s.qualified_name.clone()))
            .collect();

        // Add each descendant to the importing scope
        if let Some(vis) = self.visibility_map.get_mut(importing_scope) {
            for (simple_name, qualified_name) in descendant_symbols {
                vis.add_import(simple_name, qualified_name);
            }
        }
    }

    /// Check if a symbol passes filters given its metadata annotations directly.
    /// This avoids lookup by qualified name since we already have the symbol.
    fn symbol_passes_filters_list_static(
        &self,
        metadata_annotations: &[Arc<str>],
        filters: &[Arc<str>],
    ) -> bool {
        // Check if symbol has ALL required metadata
        for required_metadata in filters {
            let has_metadata = metadata_annotations
                .iter()
                .any(|ann| ann.as_ref() == required_metadata.as_ref());
            if !has_metadata {
                return false;
            }
        }
        true
    }

    /// Resolve an import target relative to a scope.
    ///
    /// According to SysML spec, after importing a namespace with `import P1::*`,
    /// the imported members become visible by their simple names. So subsequent
    /// imports like `import C::*` should resolve `C` through prior imports.
    ///
    /// Resolution order:
    /// 1. Check if target is already fully qualified and exists
    /// 2. Check current scope's visibility map (direct defs + imports)
    /// 3. Walk up parent scopes
    /// 4. Fall back to target as-is
    fn resolve_import_target(&self, scope: &str, target: &str) -> String {
        eprintln!("[TRACE resolve_import_target] scope='{}', target='{}'", scope, target);
        
        // If already qualified with ::, check as-is first
        if target.contains("::") && self.visibility_map.contains_key(target) {
            eprintln!("[TRACE resolve_import_target]   Found as qualified: '{}'", target);
            return target.to_string();
        }

        // For simple names (no ::), first check if it's visible via imports in current scope
        // This handles the SysML pattern: import P1::*; import C::*;
        // where C was imported from P1
        if !target.contains("::") {
            if let Some(vis) = self.visibility_map.get(scope) {
                // Check if target is visible (either as direct def or import)
                if let Some(resolved_qname) = vis.lookup(target) {
                    // Found it - return the qualified name
                    eprintln!("[TRACE resolve_import_target]   Found via visibility lookup in scope '{}': '{}'", scope, resolved_qname);
                    return resolved_qname.to_string();
                }
            }
        }

        // Try relative to scope and parent scopes (nested namespace lookup)
        let mut current = scope.to_string();
        loop {
            let candidate = if current.is_empty() {
                target.to_string()
            } else {
                format!("{}::{}", current, target)
            };

            eprintln!("[TRACE resolve_import_target]   Trying candidate: '{}'", candidate);
            if self.visibility_map.contains_key(&candidate as &str) {
                eprintln!("[TRACE resolve_import_target]   Found candidate in visibility_map: '{}'", candidate);
                return candidate;
            }

            // Move up
            if let Some(idx) = current.rfind("::") {
                current = current[..idx].to_string();
            } else if !current.is_empty() {
                current = String::new();
            } else {
                break;
            }
        }

        // Fall back to target as-is (might be global)
        eprintln!("[TRACE resolve_import_target]   Falling back to target as-is: '{}'", target);
        target.to_string()
    }

    /// Get the parent scope of a qualified name.
    ///
    /// "A::B::C" -> Some("A::B")
    /// "A" -> Some("")
    /// "" -> None
    fn parent_scope(qualified_name: &str) -> Option<&str> {
        if qualified_name.is_empty() {
            return None;
        }
        // Handle import qualified names: "Scope::import:Path" -> parent is "Scope"
        if let Some(import_pos) = qualified_name.find("::import:") {
            if import_pos == 0 {
                return Some(""); // Root level import
            }
            return Some(&qualified_name[..import_pos]);
        }
        match qualified_name.rfind("::") {
            Some(idx) => Some(&qualified_name[..idx]),
            None => Some(""), // Root level
        }
    }

    /// Build a resolver for the given scope.
    ///
    /// The resolver uses pre-computed visibility maps for efficient resolution.
    /// No need to manually collect imports - they're already in the visibility map.
    pub fn resolver_for_scope(&self, scope: &str) -> Resolver<'_> {
        Resolver::new(self).with_scope(scope)
    }
}

// ============================================================================
// SYMBOL KIND HELPERS
// ============================================================================

impl SymbolKind {
    /// Check if this is a definition kind (vs usage).
    pub fn is_definition(&self) -> bool {
        matches!(
            self,
            SymbolKind::Package
                | SymbolKind::PartDef
                | SymbolKind::ItemDef
                | SymbolKind::ActionDef
                | SymbolKind::PortDef
                | SymbolKind::AttributeDef
                | SymbolKind::ConnectionDef
                | SymbolKind::InterfaceDef
                | SymbolKind::AllocationDef
                | SymbolKind::RequirementDef
                | SymbolKind::ConstraintDef
                | SymbolKind::StateDef
                | SymbolKind::CalculationDef
                | SymbolKind::UseCaseDef
                | SymbolKind::AnalysisCaseDef
                | SymbolKind::ConcernDef
                | SymbolKind::ViewDef
                | SymbolKind::ViewpointDef
                | SymbolKind::RenderingDef
                | SymbolKind::EnumerationDef
                | SymbolKind::MetaclassDef
                | SymbolKind::InteractionDef
        )
    }

    /// Check if this is a usage kind.
    pub fn is_usage(&self) -> bool {
        matches!(
            self,
            SymbolKind::PartUsage
                | SymbolKind::ItemUsage
                | SymbolKind::ActionUsage
                | SymbolKind::PortUsage
                | SymbolKind::AttributeUsage
                | SymbolKind::ConnectionUsage
                | SymbolKind::InterfaceUsage
                | SymbolKind::AllocationUsage
                | SymbolKind::RequirementUsage
                | SymbolKind::ConstraintUsage
                | SymbolKind::StateUsage
                | SymbolKind::CalculationUsage
                | SymbolKind::ReferenceUsage
                | SymbolKind::OccurrenceUsage
                | SymbolKind::FlowUsage
        )
    }

    /// Get the corresponding definition kind for a usage.
    pub fn to_definition_kind(&self) -> Option<SymbolKind> {
        match self {
            SymbolKind::PartUsage => Some(SymbolKind::PartDef),
            SymbolKind::ItemUsage => Some(SymbolKind::ItemDef),
            SymbolKind::ActionUsage => Some(SymbolKind::ActionDef),
            SymbolKind::PortUsage => Some(SymbolKind::PortDef),
            SymbolKind::AttributeUsage => Some(SymbolKind::AttributeDef),
            SymbolKind::ConnectionUsage => Some(SymbolKind::ConnectionDef),
            SymbolKind::InterfaceUsage => Some(SymbolKind::InterfaceDef),
            SymbolKind::AllocationUsage => Some(SymbolKind::AllocationDef),
            SymbolKind::RequirementUsage => Some(SymbolKind::RequirementDef),
            SymbolKind::ConstraintUsage => Some(SymbolKind::ConstraintDef),
            SymbolKind::StateUsage => Some(SymbolKind::StateDef),
            SymbolKind::CalculationUsage => Some(SymbolKind::CalculationDef),
            _ => None,
        }
    }
}

// ============================================================================
// RESOLUTION RESULT
// ============================================================================

/// Result of resolving a reference.
#[derive(Clone, Debug)]
pub enum ResolveResult {
    /// Successfully resolved to a single symbol.
    Found(HirSymbol),
    /// Resolved to multiple candidates (ambiguous).
    Ambiguous(Vec<HirSymbol>),
    /// Could not resolve the reference.
    NotFound,
}

impl ResolveResult {
    /// Get the resolved symbol if unambiguous.
    pub fn symbol(&self) -> Option<&HirSymbol> {
        match self {
            ResolveResult::Found(s) => Some(s),
            _ => None,
        }
    }

    /// Check if resolution was successful.
    pub fn is_found(&self) -> bool {
        matches!(self, ResolveResult::Found(_))
    }

    /// Check if the reference was ambiguous.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ResolveResult::Ambiguous(_))
    }
}

// ============================================================================
// RESOLVER
// ============================================================================

/// Resolver for name lookups using pre-computed visibility maps.
///
/// The resolver uses visibility maps built during index construction,
/// so there's no need to manually configure imports.
#[derive(Clone, Debug)]
pub struct Resolver<'a> {
    /// The symbol index to search.
    index: &'a SymbolIndex,
    /// Current scope prefix (e.g., "Vehicle::Powertrain").
    current_scope: Arc<str>,
}

impl<'a> Resolver<'a> {
    /// Create a new resolver.
    pub fn new(index: &'a SymbolIndex) -> Self {
        Self {
            index,
            current_scope: Arc::from(""),
        }
    }

    /// Set the current scope.
    pub fn with_scope(mut self, scope: impl Into<Arc<str>>) -> Self {
        self.current_scope = scope.into();
        self
    }

    /// Resolve a name using pre-computed visibility maps.
    pub fn resolve(&self, name: &str) -> ResolveResult {
        eprintln!("[TRACE Resolver::resolve] name='{}', current_scope='{}'", name, self.current_scope);
        
        // 1. Handle qualified paths like "ISQ::TorqueValue"
        if name.contains("::") {
            // For qualified paths, try exact match first
            if let Some(symbol) = self.index.lookup_qualified(name) {
                return ResolveResult::Found(symbol.clone());
            }
            return self.resolve_qualified_path(name);
        }

        // 2. For simple names, try scope walking FIRST (finds local Requirements before global)
        let mut current = self.current_scope.to_string();
        let mut scopes_checked = Vec::new();
        loop {
            scopes_checked.push(current.clone());
            eprintln!("[TRACE Resolver::resolve]   Checking scope: '{}'", current);
            if let Some(vis) = self.index.visibility_for_scope(&current) {
                // Check direct definitions first (higher priority)
                if let Some(qname) = vis.lookup_direct(name) {
                    eprintln!("[TRACE Resolver::resolve]   Found direct: {} -> {}", name, qname);
                    tracing::trace!(
                        "[RESOLVE] Found '{}' as direct def in scope '{}' -> {}",
                        name,
                        current,
                        qname
                    );
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return ResolveResult::Found(sym.clone());
                    }
                }

                // Check imports
                if let Some(qname) = vis.lookup_import(name) {
                    eprintln!("[TRACE Resolver::resolve]   Found import: {} -> {}", name, qname);
                    tracing::trace!(
                        "[RESOLVE] Found '{}' as import in scope '{}' -> {}",
                        name,
                        current,
                        qname
                    );
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return ResolveResult::Found(sym.clone());
                    }
                }
            } else {
                eprintln!("[TRACE Resolver::resolve]   No visibility map for scope");
            }

            // For usages AND definitions in scope, check inherited members from supertypes
            // E.g., missionContext: MissionContext has spatialCF via inheritance from Context
            // E.g., use case def MyUseCase has start/done via inheritance from Actions::Action
            if !current.is_empty() {
                if let Some(scope_sym) = self.index.lookup_qualified(&current) {
                    // Check inherited members for both usages and definitions
                    // (both can have supertypes that define members like start/done)
                    if !scope_sym.supertypes.is_empty() {
                        eprintln!("[TRACE Resolver::resolve]   Scope has supertypes, checking inherited members");
                        if let Some(result) = self.resolve_inherited_member(scope_sym, name) {
                            eprintln!("[TRACE Resolver::resolve]   Found inherited member");
                            return result;
                        }
                    }
                }
            }

            // Move up to parent scope
            if let Some(idx) = current.rfind("::") {
                current = current[..idx].to_string();
            } else if !current.is_empty() {
                current = String::new(); // Try root scope
            } else {
                break;
            }
        }

        tracing::debug!(
            "[RESOLVE] '{}' not found in any of {} scopes: {:?}",
            name,
            scopes_checked.len(),
            scopes_checked.first()
        );

        // 3. Fall back to exact qualified match for simple names
        // This handles cases like a global package named exactly "Requirements"
        if let Some(symbol) = self.index.lookup_qualified(name) {
            return ResolveResult::Found(symbol.clone());
        }

        ResolveResult::NotFound
    }

    /// Resolve a qualified path like "ISQ::TorqueValue" using visibility maps.
    ///
    /// This handles cases where:
    /// - ISQ is a package with `public import ISQSpaceTime::*`
    /// - TorqueValue is defined in ISQSpaceTime
    fn resolve_qualified_path(&self, path: &str) -> ResolveResult {
        let (first, rest) = match path.find("::") {
            Some(idx) => (&path[..idx], &path[idx + 2..]),
            None => return ResolveResult::NotFound,
        };

        // Resolve the first segment (it's a simple name, so resolve() won't recurse here)
        let first_sym = self.resolve(first);

        if let ResolveResult::Found(first_symbol) = first_sym {
            // Get the target scope (follow alias if needed)
            let target_scope = if first_symbol.kind == SymbolKind::Alias {
                if let Some(target) = first_symbol.supertypes.first() {
                    target.as_ref()
                } else {
                    first_symbol.qualified_name.as_ref()
                }
            } else {
                first_symbol.qualified_name.as_ref()
            };

            // Handle nested qualified paths (e.g., "A::B::C" where rest="B::C")
            if rest.contains("::") {
                // Recursively resolve with target scope
                let nested_resolver = Resolver::new(self.index).with_scope(target_scope);
                return nested_resolver.resolve(rest);
            }

            // Look up 'rest' in target scope's visibility map
            if let Some(vis) = self.index.visibility_for_scope(target_scope) {
                // Check direct definitions first
                if let Some(qname) = vis.lookup_direct(rest) {
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return ResolveResult::Found(sym.clone());
                    }
                }

                // Check imports (handles public import ISQSpaceTime::*)
                if let Some(qname) = vis.lookup_import(rest) {
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return ResolveResult::Found(sym.clone());
                    }
                }
            }

            // Try direct qualified lookup (might be nested definition)
            let full_path = format!("{}::{}", target_scope, rest);
            if let Some(sym) = self.index.lookup_qualified(&full_path) {
                return ResolveResult::Found(sym.clone());
            }
        }

        ResolveResult::NotFound
    }

    /// Resolve a member name inherited through a usage's type hierarchy.
    ///
    /// E.g., if `missionContext: MissionContext` and `MissionContext :> Context`
    /// and `Context` has `spatialCF`, this will find it.
    fn resolve_inherited_member(
        &self,
        usage_sym: &HirSymbol,
        member_name: &str,
    ) -> Option<ResolveResult> {
        // Get the usage's type from supertypes
        let type_name = usage_sym.supertypes.first()?;

        // Resolve the type name from the usage's scope
        // Use direct lookup to avoid recursion
        let usage_scope = SymbolIndex::parent_scope(&usage_sym.qualified_name).unwrap_or("");
        let type_sym = self.resolve_without_inheritance(type_name, usage_scope)?;

        // Walk up the type hierarchy looking for the member
        let mut current_type = type_sym;
        let mut visited = std::collections::HashSet::new();
        visited.insert(current_type.qualified_name.clone());

        loop {
            // Check if current_type defines this member
            let member_qname = format!("{}::{}", current_type.qualified_name, member_name);
            if let Some(member_sym) = self.index.lookup_qualified(&member_qname) {
                return Some(ResolveResult::Found(member_sym.clone()));
            }

            // Move up to parent type (via supertypes)
            let parent_type_name = current_type.supertypes.first()?;
            let parent_scope =
                SymbolIndex::parent_scope(&current_type.qualified_name).unwrap_or("");
            let parent_type = self.resolve_without_inheritance(parent_type_name, parent_scope)?;

            // Cycle detection
            if visited.contains(&parent_type.qualified_name) {
                return None;
            }
            visited.insert(parent_type.qualified_name.clone());

            current_type = parent_type;
        }
    }

    /// Resolve a name without checking inherited members (to avoid recursion).
    fn resolve_without_inheritance(&self, name: &str, starting_scope: &str) -> Option<HirSymbol> {
        // Handle qualified paths
        if name.contains("::") {
            if let Some(symbol) = self.index.lookup_qualified(name) {
                return Some(symbol.clone());
            }
            // Try qualified path resolution without recursion
            return self.resolve_qualified_without_inheritance(name, starting_scope);
        }

        // For simple names, do scope walking without inheritance check
        let mut current = starting_scope.to_string();
        loop {
            if let Some(vis) = self.index.visibility_for_scope(&current) {
                if let Some(qname) = vis.lookup_direct(name) {
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return Some(sym.clone());
                    }
                }
                if let Some(qname) = vis.lookup_import(name) {
                    if let Some(sym) = self.index.lookup_qualified(qname) {
                        return Some(sym.clone());
                    }
                }
            }

            if let Some(idx) = current.rfind("::") {
                current = current[..idx].to_string();
            } else if !current.is_empty() {
                current = String::new();
            } else {
                break;
            }
        }

        // Fall back to exact qualified match
        self.index.lookup_qualified(name).cloned()
    }

    /// Resolve a qualified path without inheritance check (to avoid recursion).
    fn resolve_qualified_without_inheritance(
        &self,
        path: &str,
        starting_scope: &str,
    ) -> Option<HirSymbol> {
        let (first, rest) = match path.find("::") {
            Some(idx) => (&path[..idx], &path[idx + 2..]),
            None => return None,
        };

        // Resolve first segment
        let first_symbol = self.resolve_without_inheritance(first, starting_scope)?;
        let target_scope = first_symbol.qualified_name.as_ref();

        if rest.contains("::") {
            return self.resolve_qualified_without_inheritance(rest, target_scope);
        }

        // Look up rest in target scope
        if let Some(vis) = self.index.visibility_for_scope(target_scope) {
            if let Some(qname) = vis.lookup_direct(rest) {
                if let Some(sym) = self.index.lookup_qualified(qname) {
                    return Some(sym.clone());
                }
            }
            if let Some(qname) = vis.lookup_import(rest) {
                if let Some(sym) = self.index.lookup_qualified(qname) {
                    return Some(sym.clone());
                }
            }
        }

        // Try direct qualified lookup
        let full_path = format!("{}::{}", target_scope, rest);
        self.index.lookup_qualified(&full_path).cloned()
    }

    /// Resolve a type reference (for : Type annotations).
    pub fn resolve_type(&self, name: &str) -> ResolveResult {
        let result = self.resolve(name);

        // Filter to only definition kinds
        match result {
            ResolveResult::Found(ref symbol) if symbol.kind.is_definition() => result,
            ResolveResult::Found(_) => ResolveResult::NotFound,
            ResolveResult::Ambiguous(symbols) => {
                let defs: Vec<_> = symbols
                    .into_iter()
                    .filter(|s| s.kind.is_definition())
                    .collect();
                match defs.len() {
                    0 => ResolveResult::NotFound,
                    1 => ResolveResult::Found(defs.into_iter().next().unwrap()),
                    _ => ResolveResult::Ambiguous(defs),
                }
            }
            ResolveResult::NotFound => ResolveResult::NotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_symbol(name: &str, qualified: &str, kind: SymbolKind, file: u32) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            kind,
            file: FileId::new(file),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            type_refs: Vec::new(),
            is_public: false,
            metadata_annotations: Vec::new(),
        }
    }

    #[test]
    fn test_symbol_index_basic() {
        let mut index = SymbolIndex::new();

        let symbols = vec![
            make_symbol("Vehicle", "Vehicle", SymbolKind::Package, 0),
            make_symbol("Car", "Vehicle::Car", SymbolKind::PartDef, 0),
            make_symbol("engine", "Vehicle::Car::engine", SymbolKind::PartUsage, 0),
        ];

        index.add_file(FileId::new(0), symbols);

        assert_eq!(index.len(), 3);
        assert!(index.lookup_qualified("Vehicle::Car").is_some());
        assert!(index.lookup_qualified("Vehicle::Car::engine").is_some());
        assert!(index.lookup_definition("Vehicle::Car").is_some());
        assert!(index.lookup_definition("Vehicle::Car::engine").is_none()); // Usage, not def
    }

    #[test]
    fn test_symbol_index_remove_file() {
        let mut index = SymbolIndex::new();

        index.add_file(
            FileId::new(0),
            vec![make_symbol("A", "A", SymbolKind::PartDef, 0)],
        );
        index.add_file(
            FileId::new(1),
            vec![make_symbol("B", "B", SymbolKind::PartDef, 1)],
        );

        assert_eq!(index.len(), 2);

        index.remove_file(FileId::new(0));

        assert_eq!(index.len(), 1);
        assert!(index.lookup_qualified("A").is_none());
        assert!(index.lookup_qualified("B").is_some());
    }

    #[test]
    fn test_resolver_qualified_name() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![make_symbol("Car", "Vehicle::Car", SymbolKind::PartDef, 0)],
        );

        let resolver = Resolver::new(&index);
        let result = resolver.resolve("Vehicle::Car");

        assert!(result.is_found());
        assert_eq!(result.symbol().unwrap().name.as_ref(), "Car");
    }

    #[test]
    fn test_resolver_with_scope() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("Car", "Vehicle::Car", SymbolKind::PartDef, 0),
                make_symbol("engine", "Vehicle::Car::engine", SymbolKind::PartUsage, 0),
            ],
        );
        index.ensure_visibility_maps();

        let resolver = Resolver::new(&index).with_scope("Vehicle::Car");
        let result = resolver.resolve("engine");

        assert!(result.is_found());
    }

    #[test]
    fn test_resolver_with_visibility_maps() {
        let mut index = SymbolIndex::new();
        // Create a package ISQ with Real defined inside
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("ISQ", "ISQ", SymbolKind::Package, 0),
                make_symbol("Real", "ISQ::Real", SymbolKind::AttributeDef, 0),
            ],
        );
        // Create an import from another scope
        let mut import_sym = make_symbol("ISQ::*", "TestPkg::import:ISQ::*", SymbolKind::Import, 1);
        import_sym.is_public = false;
        index.add_file(
            FileId::new(1),
            vec![
                make_symbol("TestPkg", "TestPkg", SymbolKind::Package, 1),
                import_sym,
            ],
        );
        index.ensure_visibility_maps();

        // Resolver from TestPkg should find Real via import
        let resolver = Resolver::new(&index).with_scope("TestPkg");
        let result = resolver.resolve("Real");

        assert!(result.is_found());
        assert_eq!(
            result.symbol().unwrap().qualified_name.as_ref(),
            "ISQ::Real"
        );
    }

    #[test]
    fn test_symbol_kind_is_definition() {
        assert!(SymbolKind::PartDef.is_definition());
        assert!(SymbolKind::ActionDef.is_definition());
        assert!(!SymbolKind::PartUsage.is_definition());
        assert!(!SymbolKind::Import.is_definition());
    }

    #[test]
    fn test_symbol_kind_is_usage() {
        assert!(SymbolKind::PartUsage.is_usage());
        assert!(SymbolKind::ActionUsage.is_usage());
        assert!(!SymbolKind::PartDef.is_usage());
        assert!(!SymbolKind::Package.is_usage());
    }

    #[test]
    fn test_debug_message_chain_resolution() {
        use crate::hir::symbols::extract_symbols_unified;
        use crate::syntax::SyntaxFile;

        let source = r#"
package Test {
    part def Sequence;
    part def Driver {
        action turnVehicleOn;
    }
    part def Vehicle {
        action trigger1;
    }
    part def IgnitionCmd;
    
    part sequence : Sequence {
        part driver : Driver;
        part vehicle : Vehicle;
        message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    }
}
"#;
        let file_id = FileId::new(0);
        let syntax = SyntaxFile::sysml(source);
        let symbols = extract_symbols_unified(file_id, &syntax);

        let mut index = SymbolIndex::new();
        index.add_file(file_id, symbols);
        index.ensure_visibility_maps();

        // Now resolve all type refs (this is what happens in the semantic analysis pass)
        index.resolve_all_type_refs();

        // Check what's in various scopes
        println!("\n=== Symbols and their type_refs ===");
        for sym in index.symbols_in_file(file_id) {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("    type_ref[{}]: {:?}", i, tr);
            }
        }

        // Find ignitionCmd and check its chain type_refs
        let ignition_cmd = index
            .lookup_qualified("Test::sequence::ignitionCmd")
            .expect("ignitionCmd should exist");
        println!("\n=== ignitionCmd type_refs ===");
        for (i, trk) in ignition_cmd.type_refs.iter().enumerate() {
            match trk {
                crate::hir::TypeRefKind::Chain(chain) => {
                    println!("  Chain[{}]:", i);
                    for (j, part) in chain.parts.iter().enumerate() {
                        println!(
                            "    part[{}]: {} -> resolved: {:?}",
                            j, part.target, part.resolved_target
                        );
                    }
                }
                crate::hir::TypeRefKind::Simple(tr) => {
                    println!(
                        "  Simple[{}]: {} -> resolved: {:?}",
                        i, tr.target, tr.resolved_target
                    );
                }
            }
        }

        // Check that driver.turnVehicleOn chain resolved correctly
        // The first part (driver) should resolve to Test::sequence::driver
        // The second part (turnVehicleOn) should resolve to Test::Driver::turnVehicleOn (via typing)
        let mut found_driver_chain = false;
        let mut turn_vehicle_on_tr: Option<&crate::hir::TypeRef> = None;
        for trk in &ignition_cmd.type_refs {
            if let crate::hir::TypeRefKind::Chain(chain) = trk {
                if chain.parts.len() >= 2 && chain.parts[0].target.as_ref() == "driver" {
                    found_driver_chain = true;
                    turn_vehicle_on_tr = Some(&chain.parts[1]);
                    assert!(
                        chain.parts[0].resolved_target.is_some(),
                        "driver (first part of chain) should be resolved"
                    );
                    assert_eq!(
                        chain.parts[0].resolved_target.as_deref(),
                        Some("Test::sequence::driver"),
                        "driver should resolve to Test::sequence::driver"
                    );
                    // turnVehicleOn should resolve to the action in Driver def
                    assert!(
                        chain.parts[1].resolved_target.is_some(),
                        "turnVehicleOn (second part of chain) should be resolved"
                    );
                }
            }
        }
        assert!(
            found_driver_chain,
            "Should have found driver.turnVehicleOn chain in ignitionCmd"
        );

        // Verify the turnVehicleOn part was found and resolved
        let _tr = turn_vehicle_on_tr.expect("Should have found turnVehicleOn");
        
        // NOTE: Hover on individual chain parts requires per-part position tracking,
        // which is a separate improvement. For now we verify chain resolution works.
    }
}