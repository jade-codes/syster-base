use crate::semantic::symbol_table::{Symbol, SymbolTable};
use std::collections::HashSet;
use tracing::trace;

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
        trace!("[RESOLVE_IN_SCOPE] name='{}' scope_id={}", name, scope_id);
        
        // 1. Try as a fully qualified name
        if let Some(symbol) = self.resolve_qualified(name) {
            trace!("[RESOLVE_IN_SCOPE] -> resolved as qualified: {}", symbol.qualified_name());
            return Some(symbol);
        }

        // 2. For relative qualified names like "Inner::Vehicle"
        if let Some(colon_pos) = name.find("::") {
            let first_segment = &name[..colon_pos];
            let rest = &name[colon_pos + 2..];
            if let Some(first_symbol) = self.walk_scope_chain(first_segment, scope_id) {
                let full_qualified = format!("{}::{}", first_symbol.qualified_name(), rest);
                trace!("[RESOLVE_IN_SCOPE] -> resolved relative: {}", full_qualified);
                return self.resolve_qualified(&full_qualified);
            }
        }

        // 3. Walk scope chain (parent packages/namespaces)
        if let Some(symbol) = self.walk_scope_chain(name, scope_id) {
            trace!("[RESOLVE_IN_SCOPE] -> resolved via scope chain: {}", symbol.qualified_name());
            return Some(symbol);
        }

        // 4. Check inherited members from enclosing type
        let result = self.resolve_inherited(name, scope_id);
        trace!("[RESOLVE_IN_SCOPE] -> inherited result: {:?}", result.map(|s| s.qualified_name()));
        result
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
        trace!("[RESOLVE_INHERITED] Looking for '{}' from scope {}", name, scope_id);
        
        // First, try to find an enclosing Definition or Classifier
        if let Some(enclosing) = self.find_enclosing_definition(scope_id) {
            trace!("[RESOLVE_INHERITED] Enclosing definition: {}", enclosing.qualified_name());
            let mut visited = HashSet::new();
            if let Some(result) = self.resolve_in_type_hierarchy(name, enclosing, &mut visited, false) {
                trace!("[RESOLVE_INHERITED] Result: {:?}", Some(result.qualified_name()));
                return Some(result);
            }
        }
        
        // Try to find via enclosing typed usage(s)
        // Walk through ALL enclosing usages, not just the first
        let mut current_scope = scope_id;
        let mut checked_usages = HashSet::new();
        
        loop {
            if let Some((usage, usage_type)) = self.find_enclosing_typed_usage(current_scope) {
                let usage_qname = usage.qualified_name().to_string();
                
                if checked_usages.contains(&usage_qname) {
                    break;
                }
                checked_usages.insert(usage_qname.clone());
                

                
                let mut visited = HashSet::new();
                if let Some(result) = self.resolve_in_type_hierarchy(name, usage_type, &mut visited, false) {
                    trace!("[RESOLVE_INHERITED] Result from usage type: {:?}", Some(result.qualified_name()));
                    return Some(result);
                }
                
                // Move up to the usage's parent scope to check grandparent usages
                current_scope = usage.scope_id();
            } else {
                break;
            }
        }
        
        trace!("[RESOLVE_INHERITED] Result: None");
        None
    }
    
    fn is_in_type_hierarchy_recursive(
        &self, 
        symbol: &Symbol, 
        type_symbol: &Symbol,
        visited: &mut HashSet<String>,
    ) -> bool {
        let type_qname = type_symbol.qualified_name();
        if visited.contains(type_qname) {
            return false;
        }
        visited.insert(type_qname.to_string());
        
        // Check if symbol is a direct child of this type
        let expected_prefix = format!("{}::", type_qname);
        if symbol.qualified_name().starts_with(&expected_prefix) {
            // Make sure it's a direct child, not nested deeper
            let remainder = &symbol.qualified_name()[expected_prefix.len()..];
            if !remainder.contains("::") {
                return true;
            }
        }
        
        // Check supertypes
        for spec_target in type_symbol.specializes() {
            if let Some(supertype) = self.resolve_qualified(spec_target)
                .or_else(|| self.walk_scope_chain(spec_target, type_symbol.scope_id())) 
            {
                if self.is_in_type_hierarchy_recursive(symbol, supertype, visited) {
                    return true;
                }
            }
        }
        
        false
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

        // Check supertypes - resolve relative qualified names from the type's scope
        let scope_id = type_symbol.scope_id();
        for spec_target in type_symbol.specializes() {
            let supertype = self.resolve_qualified(spec_target)
                .or_else(|| self.resolve_relative_qualified(spec_target, scope_id));
            
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
    /// 
    /// Given a scope_id, this walks up the scope tree looking for a Definition
    /// or Classifier whose body scope contains (directly or indirectly) the given scope.
    fn find_enclosing_definition(&self, scope_id: usize) -> Option<&Symbol> {
        let mut current = scope_id;
        loop {
            // Get the parent scope
            let parent_scope_id = self.symbol_table.get_scope_parent(current)?;
            
            // Look for a Definition/Classifier in the parent scope that owns 'current' as its body
            if let Some(parent_scope) = self.symbol_table.scopes().get(parent_scope_id) {
                for &symbol_id in parent_scope.symbols.values() {
                    if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                        if matches!(symbol, Symbol::Definition { .. } | Symbol::Classifier { .. }) {
                            // Check if this definition's body scope is our 'current' scope
                            // by checking if symbols in 'current' have qualified names starting with this definition
                            let prefix = format!("{}::", symbol.qualified_name());
                            if let Some(scope) = self.symbol_table.scopes().get(current) {
                                for child_id in scope.symbols.values() {
                                    if let Some(child) = self.symbol_table.get_symbol(*child_id) {
                                        if child.qualified_name().starts_with(&prefix) {
                                            return Some(symbol);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            current = parent_scope_id;
        }
    }

    /// Find an enclosing Usage with a resolved type.
    /// 
    /// This is similar to find_enclosing_definition, but looks for Usage symbols
    /// that have a `usage_type` field set. Returns both the Usage and its resolved type.
    fn find_enclosing_typed_usage(&self, scope_id: usize) -> Option<(&Symbol, &Symbol)> {
        trace!("[FIND_ENCLOSING_TYPED_USAGE] Starting from scope {}", scope_id);
        let mut current = scope_id;
        loop {
            // Get the parent scope
            let parent_scope_id = self.symbol_table.get_scope_parent(current)?;
            trace!("[FIND_ENCLOSING_TYPED_USAGE] current={}, parent={}", current, parent_scope_id);
            
            // Look for a Usage in the parent scope that owns 'current' as its body
            if let Some(parent_scope) = self.symbol_table.scopes().get(parent_scope_id) {
                for &symbol_id in parent_scope.symbols.values() {
                    if let Some(symbol) = self.symbol_table.get_symbol(symbol_id) {
                        // Check for Usage with a type
                        if let Symbol::Usage { usage_type: Some(type_name), .. } = symbol {
                            trace!("[FIND_ENCLOSING_TYPED_USAGE] Found typed usage {} with type {}", 
                                   symbol.qualified_name(), type_name);
                            // Check if this usage's body scope is our 'current' scope
                            let prefix = format!("{}::", symbol.qualified_name());
                            if let Some(scope) = self.symbol_table.scopes().get(current) {
                                for child_id in scope.symbols.values() {
                                    if let Some(child) = self.symbol_table.get_symbol(*child_id) {
                                        if child.qualified_name().starts_with(&prefix) {
                                            trace!("[FIND_ENCLOSING_TYPED_USAGE] Matched! {} starts with {}", 
                                                   child.qualified_name(), prefix);
                                            // Found the enclosing usage, now resolve its type
                                            let usage_scope = symbol.scope_id();
                                            // Use resolve_in_scope_no_inherit to handle relative qualified names
                                            // but avoid recursion into resolve_inherited
                                            let resolved_type = self.resolve_qualified(type_name)
                                                .or_else(|| self.resolve_relative_qualified(type_name, usage_scope));
                                            
                                            if let Some(resolved_type) = resolved_type {
                                                return Some((symbol, resolved_type));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            current = parent_scope_id;
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
            } else {
                // Try resolving from both the source scope and the parent's scope
                // (the subset may be visible from either scope)
                let subset_symbol = self.resolve_in_scope(subset_target, source_scope_id)
                    .or_else(|| self.resolve_in_scope(subset_target, parent_scope_id));
                    
                if let Some(subset_symbol) = subset_symbol {
                    if let Some(result) =
                        self.resolve_member_recursive(member_name, subset_symbol, source_scope_id, visited)
                    {
                        return Some(result);
                    }
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

        // 7. Check nested children with subsets (for perform actions, parts, etc.)
        // This handles cases like `driver.turnVehicleOn` where `turnVehicleOn` isn't a direct
        // child of `driver`, but `driver` has a child (e.g., `driver::startVehicle`) that
        // subsets something containing `turnVehicleOn` (e.g., subsets=["startVehicle.turnVehicleOn"])
        let prefix = format!("{}::", parent_qname);
        for child in self.symbol_table.iter_symbols() {
            let child_qname = child.qualified_name();
            // Only check direct children (one level down)
            if child_qname.starts_with(&prefix) {
                let suffix = &child_qname[prefix.len()..];
                // Skip if this has more :: in suffix (nested deeper)
                if suffix.contains("::") {
                    continue;
                }
                // Check if this child has any subset containing the member name as its last part
                // e.g., subsets=["startVehicle.turnVehicleOn"] and member_name="turnVehicleOn"
                for subset in child.subsets() {
                    if subset.ends_with(&format!(".{}", member_name)) {
                        // Resolve the full chain to find the actual symbol
                        let parts: Vec<&str> = subset.split('.').collect();
                        if let Some(result) = self.resolve_feature_chain_internal(
                            &parts,
                            parts.len() - 1,  // Get the last part (member_name)
                            source_scope_id,
                            visited,
                        ) {
                            return Some(result);
                        }
                    }
                }
            }
        }

        // 8. Check kind-based metaclass hierarchy (for usages with standard library base types)
        // This handles cases like `message sendSensedSpeed : SensedSpeed` where `sourceEvent`
        // is inherited from `Flows::Message`, not from `SensedSpeed`. The kind "Message" maps
        // to the metaclass `Flows::Message` which has the `sourceEvent` and `targetEvent` members.
        if let Symbol::Usage { kind, .. } = parent_symbol {
            // Map usage kinds to their standard library base definitions
            let metaclass_qname = match kind.as_str() {
                "Message" => Some("Flows::Message"),
                "Flow" => Some("Flows::FlowConnectionUsage"),
                "Interface" => Some("Interfaces::InterfaceUsage"),
                "Connection" => Some("Connections::ConnectionUsage"),
                "Allocation" => Some("Allocations::AllocationUsage"),
                "Binding" => Some("Connections::BindingUsage"),
                "Succession" => Some("Connections::SuccessionUsage"),
                _ => None,
            };
            
            if let Some(qname) = metaclass_qname {
                if let Some(metaclass_symbol) = self.resolve_qualified(qname) {
                    if let Some(result) = self.resolve_member_recursive(
                        member_name,
                        metaclass_symbol,
                        metaclass_symbol.scope_id(),
                        visited,
                    ) {
                        return Some(result);
                    }
                }
            }
        }

        // 9. Check performs relationships (for perform action usages)
        // This handles cases like `vehicle.sendStatus` where `vehicle` has:
        //   `perform startVehicle.sendStatus;`
        // The performs field contains ["startVehicle.sendStatus"], so we resolve
        // `sendStatus` by finding the full chain and returning the last element.
        for perform_target in parent_symbol.performs() {
            if perform_target.ends_with(&format!(".{}", member_name)) 
                || perform_target == member_name 
            {
                // Resolve the full perform chain to find the actual symbol
                let parts: Vec<&str> = perform_target.split('.').collect();
                if let Some(result) = self.resolve_feature_chain_internal(
                    &parts,
                    parts.len() - 1,  // Get the last part (member_name)
                    source_scope_id,
                    visited,
                ) {
                    return Some(result);
                }
            }
        }

        // 10. Check references relationships (for ::> featured_by)
        // This handles cases like `lugNutCompositePort.lugNutPort1` where `lugNutCompositePort` has:
        //   `references ::> wheel1.lugNutCompositePort`
        // The references field contains the target, so we resolve the member
        // as a member of the referenced symbol (which has the actual children).
        for ref_target in parent_symbol.references() {
            trace!("[RESOLVE_MEMBER] checking references target: {}", ref_target);
            // Resolve the reference target
            let parts: Vec<&str> = ref_target.split('.').collect();
            if let Some(referenced_symbol) = self.resolve_feature_chain_internal(
                &parts,
                parts.len() - 1,
                source_scope_id,
                visited,
            ) {
                // Now resolve the member from the referenced symbol
                if let Some(result) = self.resolve_member_recursive(
                    member_name,
                    referenced_symbol,
                    referenced_symbol.scope_id(),
                    visited,
                ) {
                    return Some(result);
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
                
                // Find the namespace's body scope (not sibling scopes)
                if let Some(body_scope_id) = self.symbol_table.find_namespace_body_scope(namespace, definition_scope_id) {
                    if let Some(symbol) = self.resolve_via_public_imports(member_name, body_scope_id) {
                        return Some(symbol);
                    }
                }
            }
        }

        None
    }

    /// Resolve a relative qualified name (like "ContextDefinitions::MissionContext")
    /// from a given scope, without calling resolve_inherited (to avoid recursion).
    fn resolve_relative_qualified(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        // First try simple walk_scope_chain for non-qualified names
        if !name.contains("::") {
            return self.walk_scope_chain(name, scope_id);
        }
        
        // For relative qualified names like "ContextDefinitions::MissionContext"
        if let Some(colon_pos) = name.find("::") {
            let first_segment = &name[..colon_pos];
            let rest = &name[colon_pos + 2..];
            if let Some(first_symbol) = self.walk_scope_chain(first_segment, scope_id) {
                let full_qualified = format!("{}::{}", first_symbol.qualified_name(), rest);
                return self.resolve_qualified(&full_qualified);
            }
        }
        
        None
    }

    // ============================================================
    // Scope Chain Resolution
    // ============================================================

    fn walk_scope_chain(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        trace!("[WALK_SCOPE_CHAIN] name='{}' starting_scope={}", name, scope_id);
        let mut current = scope_id;
        loop {
            trace!("[WALK_SCOPE_CHAIN] checking scope {}", current);
            if let Some(symbol) = self.symbol_table.get_symbol_in_scope(current, name) {
                trace!("[WALK_SCOPE_CHAIN] -> found direct in scope: {}", symbol.qualified_name());
                return self.resolve_alias(symbol);
            }
            if let Some(symbol) = self.resolve_via_imports(name, current) {
                trace!("[WALK_SCOPE_CHAIN] -> found via imports: {}", symbol.qualified_name());
                return self.resolve_alias(symbol);
            }
            let parent = self.symbol_table.get_scope_parent(current);
            trace!("[WALK_SCOPE_CHAIN] -> not found in scope {}, parent={:?}", current, parent);
            current = parent?;
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

    fn resolve_alias(&'a self, symbol: &'a Symbol) -> Option<&'a Symbol> {
        match symbol {
            Symbol::Alias { target, scope_id, .. } => {
                // Try qualified name lookup first
                if let Some(sym) = self.symbol_table.find_by_qualified_name(target) {
                    return Some(sym);
                }
                // Fall back to scope-aware resolution (handles both simple names and
                // relative qualified names like `ISQ::TorqueValue`)
                self.resolve_in_scope(target, *scope_id)
            }
            _ => Some(symbol),
        }
    }
}
