//! Multi-pass import resolution and export map building
//!
//! Phase 1 (during population): Register symbols, store raw imports
//! Phase 2 (resolve_imports): Resolve import paths to fully qualified paths  
//! Phase 3 (build_export_maps): Build export maps with fixpoint iteration

use crate::semantic::symbol_table::{ResolvedImport, SymbolId, SymbolTable};

/// Phase 2: Resolve all import paths to fully qualified paths
///
/// For each scope, resolve the first segment of each import path from that scope,
/// then build the fully qualified path.
pub fn resolve_imports(symbol_table: &mut SymbolTable) {
    let scope_count = symbol_table.scope_count();

    for scope_id in 0..scope_count {
        let imports = symbol_table.get_scope_imports(scope_id);

        for import in imports {
            // Try to resolve the import path
            if let Some(resolved) = resolve_import_path(symbol_table, scope_id, &import.path) {
                let resolved_import = ResolvedImport {
                    raw_path: import.path.clone(),
                    resolved_path: resolved,
                    is_namespace: import.is_namespace,
                    is_recursive: import.is_recursive,
                    is_public: import.is_public,
                };
                symbol_table.add_resolved_import(scope_id, resolved_import);
            } else {
                // Import path couldn't be resolved - store as-is (might be absolute)
                let resolved_import = ResolvedImport {
                    raw_path: import.path.clone(),
                    resolved_path: import.path.clone(),
                    is_namespace: import.is_namespace,
                    is_recursive: import.is_recursive,
                    is_public: import.is_public,
                };
                symbol_table.add_resolved_import(scope_id, resolved_import);
            }
        }
    }
}

/// Resolve an import path from a given scope
///
/// Strategy:
/// 1. Try resolving first segment by walking scope chain (handles relative imports)
/// 2. If that fails, assume it's already absolute (stdlib imports like "ISQ::*")
fn resolve_import_path(
    symbol_table: &SymbolTable,
    scope_id: usize,
    import_path: &str,
) -> Option<String> {
    // Strip the wildcard suffix for resolution
    let path_without_wildcard = import_path.trim_end_matches("::**").trim_end_matches("::*");

    // Split into first segment and rest
    let (first_segment, rest) = match path_without_wildcard.find("::") {
        Some(pos) => (&path_without_wildcard[..pos], &path_without_wildcard[pos..]),
        None => (path_without_wildcard, ""),
    };

    // Try to resolve first segment by walking scope chain
    if let Some(resolved_first) = resolve_from_scope_chain(symbol_table, scope_id, first_segment) {
        // Get the wildcard suffix back
        let suffix = if import_path.ends_with("::**") {
            "::**"
        } else if import_path.ends_with("::*") {
            "::*"
        } else {
            ""
        };
        return Some(format!("{}{}{}", resolved_first, rest, suffix));
    }

    // First segment not found in scope chain - path might already be absolute
    None
}

/// Walk up the scope chain looking for a symbol with the given name
/// Returns the symbol's qualified name if found
fn resolve_from_scope_chain(
    symbol_table: &SymbolTable,
    scope_id: usize,
    name: &str,
) -> Option<String> {
    let mut current = scope_id;
    loop {
        // Check if this scope has a direct child with this name
        if let Some(symbol) = symbol_table.get_symbol_in_scope(current, name) {
            return Some(symbol.qualified_name().to_string());
        }

        // Check if this name is available via imports (from export_map)
        if let Some(symbol_id) = symbol_table
            .get_export_map(current)
            .and_then(|m| m.get(name))
        {
            if let Some(symbol) = symbol_table.get_symbol(*symbol_id) {
                return Some(symbol.qualified_name().to_string());
            }
        }

        // Move to parent scope
        current = symbol_table.get_scope_parent(current)?;
    }
}

/// Phase 3: Build export maps for all scopes with fixpoint iteration
///
/// Export map contains:
/// - Direct children of the scope
/// - Symbols from all imports (public and private) for local resolution
/// - Transitive re-exports from public wildcard imports
///
/// We iterate until no changes because public imports can be transitive.
pub fn build_export_maps(symbol_table: &mut SymbolTable) {
    let scope_count = symbol_table.scope_count();

    // Clear existing export maps before rebuilding
    // This is critical for incremental updates where symbols may have been removed
    symbol_table.clear_export_maps();

    // First pass: populate with direct children
    for scope_id in 0..scope_count {
        let children = symbol_table.get_scope_children_symbols(scope_id);
        for (name, symbol_id) in children {
            symbol_table.add_to_export_map(scope_id, name, symbol_id);
        }
    }

    // Second pass: Fixpoint iteration for PUBLIC wildcard imports
    // This must happen first so that re-exported symbols are available
    // when we process private imports that reference them
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 100;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        for scope_id in 0..scope_count {
            let resolved_imports: Vec<ResolvedImport> =
                symbol_table.get_resolved_imports(scope_id).to_vec();

            for import in &resolved_imports {
                // Only process public wildcard imports in fixpoint
                if !import.is_public || !import.is_namespace {
                    continue;
                }

                let namespace = import
                    .resolved_path
                    .trim_end_matches("::**")
                    .trim_end_matches("::*");

                // Get the namespace's body scope and copy its export_map entries
                // First try direct qualified lookup, then try via export_map (for chained imports)
                let ns_symbol = symbol_table.find_by_qualified_name(namespace).or_else(|| {
                    // Not a qualified name - try finding it in this scope's export_map
                    symbol_table
                        .get_export_map(scope_id)
                        .and_then(|m| m.get(namespace))
                        .and_then(|id| symbol_table.get_symbol(*id))
                });

                if let Some(ns_symbol) = ns_symbol {
                    let ns_def_scope = ns_symbol.scope_id();
                    let ns_qname = ns_symbol.qualified_name();
                    if let Some(ns_body_scope) =
                        symbol_table.find_namespace_body_scope(ns_qname, ns_def_scope)
                    {
                        // Get exports from the namespace
                        // For recursive imports, collect all nested symbols
                        // For non-recursive imports, only get direct exports
                        let ns_exports: Vec<(String, SymbolId)> = if import.is_recursive {
                            collect_namespace_exports(symbol_table, ns_qname, true)
                        } else {
                            symbol_table
                                .get_export_map(ns_body_scope)
                                .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect())
                                .unwrap_or_default()
                        };

                        for (name, symbol_id) in ns_exports {
                            // Skip import entries
                            if name.starts_with("import::") {
                                continue;
                            }

                            let existing = symbol_table
                                .get_export_map(scope_id)
                                .and_then(|m| m.get(&name));

                            if existing.is_none() {
                                symbol_table.add_to_export_map(scope_id, name, symbol_id);
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if iterations >= MAX_ITERATIONS {
        tracing::warn!("Export map building hit iteration limit - possible circular imports");
    }

    // Third pass: add symbols from non-public/non-namespace imports
    // Now that public re-exports are populated, these can reference them
    for scope_id in 0..scope_count {
        let resolved_imports: Vec<ResolvedImport> =
            symbol_table.get_resolved_imports(scope_id).to_vec();

        for import in &resolved_imports {
            // Skip public namespace imports (already handled in fixpoint)
            if import.is_public && import.is_namespace {
                continue;
            }

            if import.is_namespace {
                // Non-public wildcard import: add all symbols from namespace's export_map
                let namespace = import
                    .resolved_path
                    .trim_end_matches("::**")
                    .trim_end_matches("::*");

                // Try to resolve unqualified namespace names via export_map
                let resolved_namespace = symbol_table
                    .find_by_qualified_name(namespace)
                    .map(|s| s.qualified_name().to_string())
                    .or_else(|| {
                        symbol_table
                            .get_export_map(scope_id)
                            .and_then(|m| m.get(namespace))
                            .and_then(|id| symbol_table.get_symbol(*id))
                            .map(|s| s.qualified_name().to_string())
                    });

                if let Some(ns_qname) = resolved_namespace {
                    let exports_to_add =
                        collect_namespace_exports(symbol_table, &ns_qname, import.is_recursive);

                    for (name, symbol_id) in exports_to_add {
                        symbol_table.add_to_export_map(scope_id, name, symbol_id);
                    }
                }
            } else {
                // Direct import: add the specific symbol
                let import_path = &import.resolved_path;

                // Try direct lookup first
                let symbol_id = symbol_table
                    .find_id_by_qualified_name(import_path)
                    .or_else(|| {
                        // If not found directly, try resolving through namespace re-exports
                        // e.g., ISQ::MassValue -> look up MassValue in ISQ's export_map
                        if let Some(colon_pos) = import_path.rfind("::") {
                            let namespace = &import_path[..colon_pos];
                            let member = &import_path[colon_pos + 2..];

                            // Find namespace and its body scope
                            if let Some(ns_symbol) = symbol_table.find_by_qualified_name(namespace)
                            {
                                let ns_def_scope = ns_symbol.scope_id();
                                if let Some(ns_body) =
                                    symbol_table.find_namespace_body_scope(namespace, ns_def_scope)
                                {
                                    // Look up in the namespace's export_map
                                    if let Some(export_map) = symbol_table.get_export_map(ns_body) {
                                        return export_map.get(member).copied();
                                    }
                                }
                            }
                        }
                        None
                    });

                if let Some(symbol_id) = symbol_id {
                    // Extract the simple name (last segment)
                    if let Some(name) = import_path.rsplit("::").next() {
                        symbol_table.add_to_export_map(scope_id, name.to_string(), symbol_id);
                    }
                }
            }
        }
    }
}

/// Collect all symbols that should be exported from a namespace
fn collect_namespace_exports(
    symbol_table: &SymbolTable,
    namespace: &str,
    is_recursive: bool,
) -> Vec<(String, SymbolId)> {
    let mut exports = Vec::new();

    // Find the namespace symbol
    let namespace_symbol = match symbol_table.find_by_qualified_name(namespace) {
        Some(s) => s,
        None => return exports,
    };

    // Get the namespace's definition scope and find its body scope
    let def_scope = namespace_symbol.scope_id();

    // For non-recursive imports, get from export_map (includes re-exports from public imports)
    if !is_recursive {
        // Try to find the namespace's body scope (where its members are defined)
        if let Some(body_scope) = symbol_table.find_namespace_body_scope(namespace, def_scope) {
            // Get from export_map which includes re-exported symbols from public imports
            if let Some(export_map) = symbol_table.get_export_map(body_scope) {
                for (name, symbol_id) in export_map.iter() {
                    // Skip import entries (they start with "import::")
                    if !name.starts_with("import::") {
                        exports.push((name.clone(), *symbol_id));
                    }
                }
            } else {
                // Fallback: get direct children if no export_map
                exports.extend(symbol_table.get_scope_children_symbols(body_scope));
            }
        } else {
            // Fallback: check definition scope's children (old behavior)
            if let Some(scope) = symbol_table.scopes().get(def_scope) {
                for &child_scope_id in &scope.children {
                    let children = symbol_table.get_scope_children_symbols(child_scope_id);
                    exports.extend(children);
                }
                exports.extend(symbol_table.get_scope_children_symbols(def_scope));
            }
        }
    } else {
        // For recursive imports, collect all nested symbols
        let prefix = format!("{}::", namespace);
        for (id, symbol) in symbol_table.iter_symbols_with_ids() {
            let qname = symbol.qualified_name();
            if qname.starts_with(&prefix) {
                // Extract the simple name (last segment)
                if let Some(name) = qname.rsplit("::").next() {
                    exports.push((name.to_string(), id));
                }
            }
        }
    }

    exports
}

#[cfg(test)]
mod tests {
    // Tests for import phases are covered by integration tests
}
