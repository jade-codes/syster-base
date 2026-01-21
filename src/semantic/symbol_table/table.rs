use std::collections::HashMap;

use crate::core::Span;
use crate::core::events::EventEmitter;
use crate::core::operation::{EventBus, OperationResult};
use crate::semantic::SymbolTableEvent;
use crate::semantic::types::normalize_path;

use super::scope::{Import, ResolvedImport, Scope};
use super::symbol::Symbol;

use super::symbol::SymbolId;

pub struct SymbolTable {
    /// Arena storage for all symbols - single source of truth
    pub(super) arena: Vec<Symbol>,
    pub(super) scopes: Vec<Scope>,
    pub(super) current_scope: usize,
    current_file: Option<String>,
    pub events: EventEmitter<SymbolTableEvent, SymbolTable>,
    /// Index mapping file paths to SymbolIds of symbols defined in that file
    pub(super) symbols_by_file: HashMap<String, Vec<SymbolId>>,
    /// Index mapping file paths to imports originating from that file
    pub(super) imports_by_file: HashMap<String, Vec<Import>>,
    /// Index for O(1) qualified name lookups: qname -> SymbolId
    pub(super) symbols_by_qname: HashMap<String, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            arena: Vec::new(),
            scopes: vec![Scope::new(None)],
            current_scope: 0,
            current_file: None,
            events: EventEmitter::new(),
            symbols_by_file: HashMap::new(),
            imports_by_file: HashMap::new(),
            symbols_by_qname: HashMap::new(),
        }
    }

    pub fn set_current_file(&mut self, file_path: Option<String>) {
        let _ = {
            self.current_file = file_path.clone();
            let event = file_path.map(|path| SymbolTableEvent::FileChanged { file_path: path });
            OperationResult::<(), String, SymbolTableEvent>::success((), event)
        }
        .publish(self);
    }

    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    pub fn get_current_scope(&self) -> usize {
        self.current_scope
    }

    pub fn enter_scope(&mut self) -> usize {
        let parent = self.current_scope;
        let scope_id = self.scopes.len();
        self.scopes.push(Scope::new(Some(parent)));
        self.scopes[parent].children.push(scope_id);
        self.current_scope = scope_id;
        scope_id
    }

    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    pub fn insert(&mut self, name: String, symbol: Symbol) -> Result<(), String> {
        {
            let qualified_name = symbol.qualified_name().to_string();
            let source_file = symbol.source_file().map(normalize_path);

            // Check for duplicate in scope
            let scope = &self.scopes[self.current_scope];
            if scope.symbols.contains_key(&name) {
                return OperationResult::failure(format!(
                    "Symbol '{name}' already defined in this scope"
                ))
                .publish(self);
            }

            // Add symbol to arena and get its ID
            let symbol_id = SymbolId::new(self.arena.len());
            self.arena.push(symbol);

            // Store SymbolId in scope
            self.scopes[self.current_scope]
                .symbols
                .insert(name, symbol_id);

            // Update the qname -> SymbolId index for O(1) lookup
            self.symbols_by_qname
                .insert(qualified_name.clone(), symbol_id);

            // Update the file -> SymbolIds index (using normalized path)
            if let Some(file_path) = source_file {
                self.symbols_by_file
                    .entry(file_path)
                    .or_default()
                    .push(symbol_id);
            }

            let event = SymbolTableEvent::SymbolInserted {
                qualified_name,
                symbol_id: symbol_id.index(),
            };
            OperationResult::success((), Some(event))
        }
        .publish(self)
    }

    pub fn add_import(
        &mut self,
        path: String,
        is_recursive: bool,
        is_public: bool,
        span: Option<crate::core::Span>,
        file: Option<String>,
    ) {
        let _ = {
            let is_namespace = path.ends_with("::*") || path.ends_with("::**");
            let import = Import {
                path: path.clone(),
                is_recursive,
                is_namespace,
                is_public,
                span,
                file: file.clone(),
            };
            self.scopes[self.current_scope].imports.push(import.clone());

            // Update the file -> imports index (using normalized path)
            if let Some(file_path) = file {
                let normalized = normalize_path(&file_path);
                self.imports_by_file
                    .entry(normalized)
                    .or_default()
                    .push(import);
            }

            let event = SymbolTableEvent::ImportAdded { import_path: path };
            OperationResult::<(), String, SymbolTableEvent>::success((), Some(event))
        }
        .publish(self);
    }

    pub fn current_scope_id(&self) -> usize {
        self.current_scope
    }

    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    // ============================================================
    // Data Access Methods (for Resolver)
    // ============================================================

    /// Get read-only access to all scopes.
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// Get a symbol directly from a specific scope (no chain walking).
    pub fn get_symbol_in_scope(&self, scope_id: usize, name: &str) -> Option<&Symbol> {
        let id = self.scopes.get(scope_id)?.symbols.get(name)?;
        self.get_symbol(*id)
    }

    /// Get a SymbolId from a specific scope
    pub fn get_symbol_id_in_scope(&self, scope_id: usize, name: &str) -> Option<SymbolId> {
        self.scopes.get(scope_id)?.symbols.get(name).copied()
    }

    /// Get the parent of a scope.
    pub fn get_scope_parent(&self, scope_id: usize) -> Option<usize> {
        self.scopes.get(scope_id)?.parent
    }

    /// Get the scope ID for a file that contains its imports.
    ///
    /// This returns the scope where imports are registered for the given file,
    /// which is typically the scope of the top-level package body.
    /// Falls back to the scope of the first symbol if no imports are found.
    pub fn get_scope_for_file(&self, file_path: &str) -> Option<usize> {
        // First, try to find a scope with imports from this file
        for (scope_id, scope) in self.scopes.iter().enumerate() {
            if scope
                .imports
                .iter()
                .any(|import| import.file.as_deref() == Some(file_path))
            {
                return Some(scope_id);
            }
        }

        // Fall back to the scope of the first symbol defined in the file
        self.get_symbols_for_file(file_path)
            .first()
            .map(|symbol| symbol.scope_id())
    }

    pub fn get_scope_imports(&self, scope_id: usize) -> Vec<super::scope::Import> {
        self.scopes
            .get(scope_id)
            .map(|scope| scope.imports.clone())
            .unwrap_or_default()
    }

    /// Get all imports from a specific file
    ///
    /// Returns a vector of (import_path, span) tuples for all imports in the given file.
    /// Uses an internal index for O(1) file lookup. Paths are normalized.
    pub fn get_file_imports(&self, file_path: &str) -> Vec<(String, Span)> {
        let normalized = normalize_path(file_path);
        self.imports_by_file
            .get(&normalized)
            .into_iter()
            .flatten()
            .filter_map(|import| import.span.map(|span| (import.path.clone(), span)))
            .collect()
    }

    /// Get all symbols defined in a specific file
    ///
    /// Returns a Vec of symbols whose source_file matches the given path.
    /// Uses an internal index for O(1) file lookup instead of iterating all symbols.
    /// Paths are normalized before lookup (handles stdlib path variations).
    pub fn get_symbols_for_file(&self, file_path: &str) -> Vec<&Symbol> {
        let normalized = normalize_path(file_path);
        self.symbols_by_file
            .get(&normalized)
            .into_iter()
            .flatten()
            .filter_map(|id| self.get_symbol(*id))
            .collect()
    }

    /// Get a symbol by its SymbolId (O(1) arena lookup)
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.arena.get(id.index())
    }

    /// Get a mutable symbol by its SymbolId (O(1) arena lookup)
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.arena.get_mut(id.index())
    }

    /// Find a symbol by its exact qualified name (data access, not resolution)
    /// Uses O(1) index lookup.
    pub fn find_by_qualified_name(&self, qualified_name: &str) -> Option<&Symbol> {
        let id = self.symbols_by_qname.get(qualified_name)?;
        self.get_symbol(*id)
    }

    /// Find a SymbolId by qualified name (for callers that need the ID)
    pub fn find_id_by_qualified_name(&self, qualified_name: &str) -> Option<SymbolId> {
        self.symbols_by_qname.get(qualified_name).copied()
    }

    /// Find a mutable symbol by its exact qualified name (data access, not resolution)
    /// Uses O(1) index lookup.
    pub fn find_by_qualified_name_mut(&mut self, qualified_name: &str) -> Option<&mut Symbol> {
        let id = self.symbols_by_qname.get(qualified_name).copied()?;
        self.get_symbol_mut(id)
    }

    /// Get qualified names of all symbols defined in a specific file
    ///
    /// Returns a Vec of qualified names for the file.
    /// Used by enable_auto_invalidation to know which symbols to remove.
    /// Paths are normalized.
    pub fn get_qualified_names_for_file(&self, file_path: &str) -> Vec<String> {
        let normalized = normalize_path(file_path);
        self.symbols_by_file
            .get(&normalized)
            .into_iter()
            .flatten()
            .filter_map(|id| self.get_symbol(*id).map(|s| s.qualified_name().to_string()))
            .collect()
    }

    // ============================================================
    // Phase 2 & 3: Import Resolution and Export Map Building
    // ============================================================

    /// Add a resolved import to a scope (Phase 2)
    pub fn add_resolved_import(&mut self, scope_id: usize, resolved: ResolvedImport) {
        if let Some(scope) = self.scopes.get_mut(scope_id) {
            scope.resolved_imports.push(resolved);
        }
    }

    /// Add an entry to a scope's export map (Phase 3)
    pub fn add_to_export_map(&mut self, scope_id: usize, name: String, symbol_id: SymbolId) {
        if let Some(scope) = self.scopes.get_mut(scope_id) {
            // Don't overwrite existing entries (local symbols take precedence)
            scope.export_map.entry(name).or_insert(symbol_id);
        }
    }

    /// Get the resolved imports for a scope
    pub fn get_resolved_imports(&self, scope_id: usize) -> &[ResolvedImport] {
        self.scopes
            .get(scope_id)
            .map(|s| s.resolved_imports.as_slice())
            .unwrap_or(&[])
    }

    /// Clear all export maps (called before rebuilding them)
    pub fn clear_export_maps(&mut self) {
        for scope in &mut self.scopes {
            scope.export_map.clear();
        }
    }

    /// Get the export map for a scope
    pub fn get_export_map(&self, scope_id: usize) -> Option<&HashMap<String, SymbolId>> {
        self.scopes.get(scope_id).map(|s| &s.export_map)
    }

    /// Look up a symbol in a scope's export map (O(1))
    pub fn lookup_in_export_map(&self, scope_id: usize, name: &str) -> Option<&Symbol> {
        let export_map = self.get_export_map(scope_id)?;
        tracing::trace!(
            "[EXPORT_MAP] lookup scope={} name='{}' export_map_keys={:?}",
            scope_id,
            name,
            export_map.keys().take(10).collect::<Vec<_>>()
        );
        let symbol_id = export_map.get(name)?;
        let result = self.get_symbol(*symbol_id);
        tracing::trace!(
            "[EXPORT_MAP] -> found symbol_id={:?} symbol={:?}",
            symbol_id,
            result.map(|s| s.qualified_name())
        );
        result
    }

    /// Get all children symbols of a scope (direct children only)
    pub fn get_scope_children_symbols(&self, scope_id: usize) -> Vec<(String, SymbolId)> {
        self.scopes
            .get(scope_id)
            .map(|s| s.symbols.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default()
    }

    /// Find the body scope of a namespace (package/class) by looking for
    /// a child scope that contains symbols or imports belonging to the namespace.
    ///
    /// When we define `package Foo { ... }`, the package symbol's scope_id is where
    /// Foo is defined (parent scope), but Foo's body creates a new child scope.
    /// This method finds that body scope.
    pub fn find_namespace_body_scope(
        &self,
        namespace_qname: &str,
        definition_scope_id: usize,
    ) -> Option<usize> {
        let prefix = format!("{}::", namespace_qname);
        let scope = self.scopes.get(definition_scope_id)?;

        for &child_scope_id in &scope.children {
            if let Some(child_scope) = self.scopes.get(child_scope_id) {
                // Check if any symbol in this scope has the right prefix
                for symbol_id in child_scope.symbols.values() {
                    if let Some(symbol) = self.get_symbol(*symbol_id) {
                        if symbol.qualified_name().starts_with(&prefix) {
                            return Some(child_scope_id);
                        }
                    }
                }
            }
        }

        // If no symbols found with the prefix, check for scopes with imports
        // that have public namespace imports - likely a package with only imports
        for &child_scope_id in &scope.children {
            if let Some(child_scope) = self.scopes.get(child_scope_id) {
                // Check resolved_imports first (Phase 2 completed)
                let has_public_namespace_resolved = child_scope
                    .resolved_imports
                    .iter()
                    .any(|ri| ri.is_public && ri.is_namespace);

                // Also check raw imports (Phase 2 not completed)
                let has_public_namespace_raw = child_scope
                    .imports
                    .iter()
                    .any(|i| i.is_public && i.is_namespace);

                if has_public_namespace_resolved || has_public_namespace_raw {
                    // This scope has public wildcard imports - likely the body scope
                    // But we need to verify it's not a sibling namespace
                    // Check if this scope has NO symbols with a different namespace prefix
                    let has_wrong_prefix = child_scope.symbols.values().any(|id| {
                        self.get_symbol(*id)
                            .map(|s| {
                                let qname = s.qualified_name();
                                // Symbol belongs to a different namespace
                                !qname.starts_with(&prefix)
                                    && qname.contains("::")
                                    && !qname.starts_with("import::")
                            })
                            .unwrap_or(false)
                    });

                    if !has_wrong_prefix {
                        return Some(child_scope_id);
                    }
                }
            }
        }

        None
    }

    /// Get mutable access to a scope (for phase operations)
    pub fn get_scope_mut(&mut self, scope_id: usize) -> Option<&mut Scope> {
        self.scopes.get_mut(scope_id)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus<SymbolTableEvent> for SymbolTable {
    fn publish(&mut self, event: &SymbolTableEvent) {
        let emitter = std::mem::take(&mut self.events);
        self.events = emitter.emit(event.clone(), self);
    }
}
