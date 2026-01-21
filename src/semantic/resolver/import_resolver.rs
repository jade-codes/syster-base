use crate::semantic::resolver::Resolver;
use crate::semantic::symbol_table::Symbol;

impl<'a> Resolver<'a> {
    // ============================================================
    // Import Path Utilities
    // ============================================================

    /// Parse an import path into components (split by ::)
    pub fn parse_import_path(path: &str) -> Vec<String> {
        path.split("::").map(|s| s.to_string()).collect()
    }

    /// Check if an import is a wildcard import (ends with *)
    pub fn is_wildcard_import(path: &str) -> bool {
        path.ends_with("::*") || path == "*"
    }

    // ============================================================
    // Import Resolution (for expanding import statements)
    // ============================================================

    /// Resolve an import path to the symbols it brings into scope
    pub fn resolve_import(&self, import_path: &str) -> Vec<String> {
        if Self::is_wildcard_import(import_path) {
            self.resolve_wildcard_import(import_path)
        } else if self.resolve_qualified(import_path).is_some() {
            vec![import_path.to_string()]
        } else {
            vec![]
        }
    }

    fn resolve_wildcard_import(&self, import_path: &str) -> Vec<String> {
        if import_path == "*" {
            return self
                .symbol_table()
                .iter_symbols()
                .filter_map(|symbol| {
                    let qname = symbol.qualified_name();
                    if !qname.contains("::") {
                        Some(qname.to_string())
                    } else {
                        None
                    }
                })
                .collect();
        }

        let prefix = import_path.strip_suffix("::*").unwrap_or(import_path);

        self.symbol_table()
            .iter_symbols()
            .filter_map(|symbol| {
                let qname = symbol.qualified_name();
                if let Some(remainder) = qname.strip_prefix(prefix)
                    && let Some(remainder) = remainder.strip_prefix("::")
                    && !remainder.contains("::")
                {
                    return Some(qname.to_string());
                }
                None
            })
            .collect()
    }

    // ============================================================
    // Import-based Name Resolution (O(1) via export_map)
    // ============================================================

    /// Resolve a simple name via imports registered in a scope.
    /// Uses precomputed export_map for O(1) lookup.
    pub(super) fn resolve_via_imports(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        self.symbol_table().lookup_in_export_map(scope_id, name)
    }

    /// Resolve a simple name via public imports only (for re-export resolution).
    /// Uses precomputed export_map for O(1) lookup.
    pub(super) fn resolve_via_public_imports(
        &self,
        name: &str,
        scope_id: usize,
    ) -> Option<&Symbol> {
        self.symbol_table().lookup_in_export_map(scope_id, name)
    }
}
