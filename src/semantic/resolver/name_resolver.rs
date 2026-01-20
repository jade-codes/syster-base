use crate::semantic::symbol_table::{Symbol, SymbolTable};
use std::collections::HashSet;

/// Resolver provides symbol resolution algorithms.
///
/// All resolution logic lives here, keeping SymbolTable as a pure data structure.
pub struct Resolver<'a> {
    symbol_table: &'a SymbolTable,
}

impl<'a> Resolver<'a> {
    pub fn new(symbol_table: &'a SymbolTable) -> Self {
        Self { symbol_table }
    }

    pub fn symbol_table(&self) -> &SymbolTable {
        self.symbol_table
    }

    // ============================================================
    // Primary Resolution API
    // ============================================================

    /// Resolve a name (qualified or simple) using current scope.
    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.resolve_qualified(name)
            .or_else(|| self.walk_scope_chain(name, self.symbol_table.current_scope_id()))
    }

    /// Resolve a name within a specific scope.
    /// Checks: qualified names → scope chain → inherited members.
    pub fn resolve_in_scope(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        // 1. Try as a fully qualified name
        if let Some(symbol) = self.resolve_qualified(name) {
            return Some(symbol);
        }

        // 2. For relative qualified names like "Inner::Vehicle"
        if let Some(colon_pos) = name.find("::") {
            let first_segment = &name[..colon_pos];
            let rest = &name[colon_pos + 2..];
            if let Some(first_symbol) = self.walk_scope_chain(first_segment, scope_id) {
                let full_qualified = format!("{}::{}", first_symbol.qualified_name(), rest);
                return self.resolve_qualified(&full_qualified);
            }
        }

        // 3. Walk scope chain (parent packages/namespaces)
        if let Some(symbol) = self.walk_scope_chain(name, scope_id) {
            return Some(symbol);
        }

        // 4. Check inherited members from enclosing type
        self.resolve_inherited(name, scope_id)
    }

    /// Resolve a member within a parent symbol's type hierarchy.
    /// Used for feature chains like `takePicture.focus`.
    pub fn resolve_member(
        &self,
        member_name: &str,
        parent_symbol: &Symbol,
        source_scope_id: usize,
    ) -> Option<&Symbol> {
        let mut visited = HashSet::new();
        self.resolve_member_recursive(member_name, parent_symbol, source_scope_id, &mut visited)
    }

    /// Resolve a feature chain like `localClock.currentTime`.
    /// 
    /// This is the SINGLE place for feature chain resolution logic.
    /// The LSP layer should call this instead of doing its own resolution.
    ///
    /// # Arguments
    /// * `parts` - The chain segments, e.g., ["localClock", "currentTime"]
    /// * `target_index` - Which part we want to resolve (0-indexed)
    /// * `scope_id` - The scope where the chain appears
    pub fn resolve_feature_chain(
        &self,
        parts: &[&str],
        target_index: usize,
        scope_id: usize,
    ) -> Option<&Symbol> {
        let mut visited = HashSet::new();
        self.resolve_feature_chain_internal(parts, target_index, scope_id, &mut visited)
    }
    
    /// Internal feature chain resolution that reuses a visited set to avoid loops.
    fn resolve_feature_chain_internal(
        &self,
        parts: &[&str],
        target_index: usize,
        scope_id: usize,
        visited: &mut HashSet<String>,
    ) -> Option<&Symbol> {
        if parts.is_empty() || target_index >= parts.len() {
            return None;
        }

        let full_chain = parts.join(".");
        let first_part = parts[0];
        
        // Resolve first part - may be inherited from enclosing type
        let mut current = self.resolve_in_scope(first_part, scope_id)?;

        // Handle edge case: we found a symbol that redefines the ENTIRE chain we're resolving.
        // This happens with `attribute :>> localClock.currentTime` - it creates a symbol named
        // "localClock" that redefines the chain. We need to look past this to find the REAL
        // inherited localClock from stdlib.
        let redefines_chain = current.redefines().iter().any(|r| *r == full_chain);
        
        if redefines_chain {
            // This symbol IS the redefining usage - we need the inherited one instead.
            // Skip the current type's direct children to find what's being redefined.
            if let Some(enclosing) = self.find_enclosing_definition(scope_id) {
                if let Some(inherited) = self.resolve_in_type_hierarchy(first_part, enclosing, visited, true) {
                    current = inherited;
                }
            }
        }
        
        // Handle case: found a symbol that subsets the chain we're resolving.
        // e.g., `perform takePicture.focus` creates symbol that subsets "takePicture.focus"
        // We need to look past this to find the outer `takePicture`.
        let subsets_chain = current.subsets().iter().any(|s| {
            s.starts_with(first_part) && (s == &full_chain || s.starts_with(&format!("{}.", first_part)))
        });
        
        if subsets_chain {
            // This symbol IS the subsetting usage - look in parent scope
            if let Some(parent_scope) = self.symbol_table.get_scope_parent(scope_id) {
                if let Some(outer) = self.resolve_in_scope(first_part, parent_scope) {
                    current = outer;
                }
            }
        }

        // Also handle duplicate name segments (anonymous usages that shadow)
        let qname = current.qualified_name();
        let qname_parts: Vec<&str> = qname.split("::").collect();
        let first_part_count = qname_parts.iter().filter(|&p| *p == first_part).count();
        if first_part_count > 1 {
            if let Some(parent_scope) = self.symbol_table.get_scope_parent(scope_id) {
                if let Some(better) = self.resolve_in_scope(first_part, parent_scope) {
                    current = better;
                }
            }
        }

        // If we only want the first part, return it
        if target_index == 0 {
            return Some(current);
        }

        // Walk through the chain to reach target_index
        for i in 1..=target_index {
            let part = parts[i];
            current = self.resolve_member_recursive(part, current, current.scope_id(), visited)?;
        }

        Some(current)
    }

    // ============================================================
    // Inherited Member Resolution
    // ============================================================

    /// Resolve a name as an inherited member from the enclosing type.
    /// e.g., `portionOfLife` inside `part def X` resolves from `Occurrence`.
    fn resolve_inherited(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        let enclosing = self.find_enclosing_definition(scope_id)?;
        let mut visited = HashSet::new();
        self.resolve_in_type_hierarchy(name, enclosing, &mut visited, false)
    }

    /// Search for a member in a type's inheritance hierarchy.
    /// 
    /// * `skip_self` - If true, skip direct children of type_symbol and only check supertypes.
    ///                 Used when the current type has a local shadow we need to look past.
    fn resolve_in_type_hierarchy(
        &self,
        name: &str,
        type_symbol: &Symbol,
        visited: &mut HashSet<String>,
        skip_self: bool,
    ) -> Option<&Symbol> {
        let qname = type_symbol.qualified_name();
        if visited.contains(qname) {
            return None;
        }
        visited.insert(qname.to_string());

        // Check direct children first (unless skip_self is true)
        if !skip_self {
            let child_qname = format!("{}::{}", qname, name);
            if let Some(symbol) = self.symbol_table.find_by_qualified_name(&child_qname) {
                return Some(symbol);
            }
        }

        // Check supertypes - use walk_scope_chain, NOT resolve_in_scope (avoids recursion)
        let scope_id = type_symbol.scope_id();
        for spec_target in type_symbol.specializes() {
            let supertype = self.resolve_qualified(spec_target)
                .or_else(|| self.walk_scope_chain(spec_target, scope_id));
            
            if let Some(supertype) = supertype {
                // Supertypes always check their own children (skip_self=false)
                if let Some(result) = self.resolve_in_type_hierarchy(name, supertype, visited, false) {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Find the enclosing Definition or Classifier for a scope.
    fn find_enclosing_definition(&self, scope_id: usize) -> Option<&Symbol> {
        let mut current = scope_id;
        loop {
            if let Some(scope) = self.symbol_table.scopes().get(current) {
                for &symbol_id in scope.symbols.values() {
                    if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                        if matches!(symbol, Symbol::Definition { .. } | Symbol::Classifier { .. }) {
                            return Some(symbol);
                        }
                    }
                }
            }
            current = self.symbol_table.get_scope_parent(current)?;
        }
    }

    // ============================================================
    // Member Resolution (for feature chains)
    // ============================================================

    fn resolve_member_recursive(
        &self,
        member_name: &str,
        parent_symbol: &Symbol,
        source_scope_id: usize,
        visited: &mut HashSet<String>,
    ) -> Option<&Symbol> {
        let parent_qname = parent_symbol.qualified_name();
        let parent_scope_id = parent_symbol.scope_id();

        if visited.contains(parent_qname) {
            return None;
        }
        visited.insert(parent_qname.to_string());

        // 1. Direct child lookup
        let child_qname = format!("{}::{}", parent_qname, member_name);
        if let Some(symbol) = self.symbol_table.find_by_qualified_name(&child_qname) {
            return Some(symbol);
        }

        // 2. Check subsets relationships
        for subset_target in parent_symbol.subsets() {
            // Handle feature chain subsets (e.g., "takePicture.focus")
            if subset_target.contains('.') {
                let parts: Vec<&str> = subset_target.split('.').collect();
                // We need the LAST part of the chain resolved to find its type
                // Use resolve_feature_chain_internal to avoid infinite loops
                if let Some(chain_result) = self.resolve_feature_chain_internal(&parts, parts.len() - 1, source_scope_id, visited) {
                    // The chain result IS the thing being subsetted
                    // Look for member in its type hierarchy
                    if let Some(result) = self.resolve_member_recursive(
                        member_name,
                        chain_result,
                        source_scope_id,
                        visited,
                    ) {
                        return Some(result);
                    }
                }
            } else if let Some(subset_symbol) = self.resolve_in_scope(subset_target, source_scope_id) {
                if let Some(result) =
                    self.resolve_member_recursive(member_name, subset_symbol, source_scope_id, visited)
                {
                    return Some(result);
                }
            }
        }

        // 3. Check redefines (including feature chains)
        for redef_target in parent_symbol.redefines() {
            if let Some(result) =
                self.resolve_via_redefines(member_name, redef_target, source_scope_id, visited)
            {
                return Some(result);
            }
        }

        // 4. Check specializes (inheritance)
        for spec_target in parent_symbol.specializes() {
            if let Some(spec_symbol) = self.resolve_in_scope(spec_target, parent_scope_id) {
                if let Some(result) = self.resolve_member_recursive(
                    member_name,
                    spec_symbol,
                    spec_symbol.scope_id(),
                    visited,
                ) {
                    return Some(result);
                }
            }
        }

        // 5. Check typed_by (for Usages)
        if let Symbol::Usage { usage_type, .. } = parent_symbol {
            if let Some(type_name) = usage_type {
                if let Some(type_symbol) = self.resolve_in_scope(type_name, parent_scope_id) {
                    if let Some(result) = self.resolve_member_recursive(
                        member_name,
                        type_symbol,
                        type_symbol.scope_id(),
                        visited,
                    ) {
                        return Some(result);
                    }
                }
            }
        }

        // 6. Check feature_type (for Features)
        if let Symbol::Feature { feature_type, .. } = parent_symbol {
            if let Some(type_name) = feature_type {
                if let Some(type_symbol) = self.resolve_in_scope(type_name, parent_scope_id) {
                    if let Some(result) = self.resolve_member_recursive(
                        member_name,
                        type_symbol,
                        type_symbol.scope_id(),
                        visited,
                    ) {
                        return Some(result);
                    }
                }
            }
        }

        None
    }

    /// Resolve through a redefines target (may be a feature chain like `a.b.c`).
    fn resolve_via_redefines(
        &self,
        member_name: &str,
        redef_target: &str,
        scope_id: usize,
        visited: &mut HashSet<String>,
    ) -> Option<&Symbol> {
        if redef_target.contains('.') {
            // Feature chain: resolve each part in sequence
            let parts: Vec<&str> = redef_target.split('.').collect();
            let member_pos = parts.iter().position(|&p| p == member_name)?;

            // First part is inherited from enclosing type
            let enclosing = self.find_enclosing_definition(scope_id)?;
            let mut current = self.resolve_in_type_hierarchy(parts[0], enclosing, visited, false)?;

            // Walk chain to member_name
            for &part in &parts[1..=member_pos] {
                current = self.resolve_member_recursive(part, current, current.scope_id(), visited)?;
            }
            Some(current)
        } else {
            // Simple redefines - resolve directly (may be inherited)
            self.resolve_in_scope(redef_target, scope_id)
        }
    }

    // ============================================================
    // Qualified Name Resolution
    // ============================================================

    pub fn resolve_qualified(&self, qualified_name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbol_table.find_by_qualified_name(qualified_name) {
            return Some(symbol);
        }

        // Try resolving via public re-exports
        if let Some(colon_pos) = qualified_name.rfind("::") {
            let namespace = &qualified_name[..colon_pos];
            let member_name = &qualified_name[colon_pos + 2..];

            if let Some(ns_symbol) = self.symbol_table.find_by_qualified_name(namespace) {
                let definition_scope_id = ns_symbol.scope_id();
                if let Some(scope) = self.symbol_table.scopes().get(definition_scope_id) {
                    for &child_scope_id in &scope.children {
                        if let Some(symbol) =
                            self.resolve_via_public_imports(member_name, child_scope_id)
                        {
                            return Some(symbol);
                        }
                    }
                }
            }
        }

        None
    }

    // ============================================================
    // Scope Chain Resolution
    // ============================================================

    fn walk_scope_chain(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        let mut current = scope_id;
        loop {
            if let Some(symbol) = self.symbol_table.get_symbol_in_scope(current, name) {
                return self.resolve_alias(symbol);
            }
            if let Some(symbol) = self.resolve_via_imports(name, current) {
                return self.resolve_alias(symbol);
            }
            current = self.symbol_table.get_scope_parent(current)?;
        }
    }

    pub fn resolve_from_scope_direct(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        let mut current = scope_id;
        loop {
            if let Some(symbol) = self.symbol_table.get_symbol_in_scope(current, name) {
                return Some(symbol);
            }
            current = self.symbol_table.get_scope_parent(current)?;
        }
    }

    // ============================================================
    // Alias Resolution
    // ============================================================

    fn resolve_alias<'b>(&self, symbol: &'b Symbol) -> Option<&'b Symbol>
    where
        'a: 'b,
    {
        match symbol {
            Symbol::Alias { target, .. } => self.symbol_table.find_by_qualified_name(target),
            _ => Some(symbol),
        }
    }
}
