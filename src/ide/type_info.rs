//! Type information at cursor position.
//!
//! Provides detailed information about type annotations,
//! including resolution and navigation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, ResolveResult, SymbolIndex, TypeRef};

/// Information about a type reference at a position.
#[derive(Clone, Debug)]
pub struct TypeInfo {
    /// The target type name as written in source.
    pub target_name: Arc<str>,
    /// The type reference span information.
    pub type_ref: TypeRef,
    /// The resolved target symbol (if found).
    pub resolved_symbol: Option<HirSymbol>,
    /// The containing symbol's qualified name (for context).
    pub container: Option<Arc<str>>,
}

impl TypeInfo {
    /// Get the resolved qualified name, falling back to the written name.
    pub fn resolved_name(&self) -> &str {
        self.type_ref
            .resolved_target
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(self.target_name.as_ref())
    }
}

/// Get type information at a specific position.
///
/// Returns info if the cursor is on a type annotation (`:`, `:>`, `::>`, etc.).
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
///
/// # Returns
/// Type information if cursor is on a type reference, None otherwise.
pub fn type_info_at(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> Option<TypeInfo> {
    let (target_name, type_ref, containing_symbol) =
        find_type_ref_at_position(index, file, line, col)?;

    // Try to resolve the target symbol
    let resolved_symbol = resolve_type_ref(index, type_ref, &target_name, containing_symbol);

    Some(TypeInfo {
        target_name,
        type_ref: type_ref.clone(),
        resolved_symbol,
        container: containing_symbol.map(|s| s.qualified_name.clone()),
    })
}

/// Resolve a type reference to its target symbol.
pub fn resolve_type_ref(
    index: &SymbolIndex,
    type_ref: &TypeRef,
    target_name: &str,
    containing_symbol: Option<&HirSymbol>,
) -> Option<HirSymbol> {
    // Use pre-resolved target if available (computed during semantic analysis)
    if let Some(resolved) = &type_ref.resolved_target {
        return index.lookup_qualified(resolved).cloned();
    }

    // Fallback: try to resolve at query time
    let scope = containing_symbol
        .map(|s| s.qualified_name.as_ref())
        .unwrap_or("");
    let resolver = index.resolver_for_scope(scope);

    match resolver.resolve(target_name) {
        ResolveResult::Found(sym) => Some(sym),
        ResolveResult::Ambiguous(syms) => syms.into_iter().next(),
        ResolveResult::NotFound => {
            // Try qualified name directly
            index.lookup_qualified(target_name).cloned()
        }
    }
}

/// Find a type reference at a specific position in a file.
///
/// Returns the target type name, the TypeRef containing the position,
/// and the symbol that contains this type_ref (for scope resolution).
pub fn find_type_ref_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<(Arc<str>, &TypeRef, Option<&HirSymbol>)> {
    let symbols = index.symbols_in_file(file);

    for symbol in symbols {
        for type_ref_kind in symbol.type_refs.iter() {
            if type_ref_kind.contains(line, col) {
                // Find which part contains the position
                if let Some((_part_idx, tr)) = type_ref_kind.part_at(line, col) {
                    return Some((tr.target.clone(), tr, Some(symbol)));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{RefKind, SymbolKind, new_element_id};

    fn make_symbol_with_type_ref(
        name: &str,
        qualified: &str,
        kind: SymbolKind,
        type_ref_target: &str,
        line: u32,
    ) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            element_id: new_element_id(),
            kind,
            file: FileId::new(0),
            start_line: line,
            start_col: 0,
            end_line: line + 1,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: vec![Arc::from(type_ref_target)],
            relationships: Vec::new(),
            type_refs: vec![crate::hir::TypeRefKind::Simple(TypeRef::new(
                type_ref_target,
                RefKind::TypedBy,
                line,
                10,
                line,
                20,
            ))],
            is_public: false,
            view_data: None,
        }
    }

    #[test]
    fn test_type_info_at_type_ref() {
        let mut index = SymbolIndex::new();

        // Add a definition
        let def = HirSymbol {
            name: Arc::from("Engine"),
            short_name: None,
            qualified_name: Arc::from("Engine"),
            element_id: new_element_id(),
            kind: SymbolKind::PartDef,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 5,
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
            view_data: None,
        };

        // Add a usage with type_ref
        let usage =
            make_symbol_with_type_ref("engine", "Car::engine", SymbolKind::PartUsage, "Engine", 10);

        index.add_file(FileId::new(0), vec![def, usage]);

        // Query at the type_ref position
        let info = type_info_at(&index, FileId::new(0), 10, 15);
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.target_name.as_ref(), "Engine");
        assert!(info.resolved_symbol.is_some());
        assert_eq!(
            info.resolved_symbol.unwrap().qualified_name.as_ref(),
            "Engine"
        );
    }

    #[test]
    fn test_type_info_not_on_type_ref() {
        let mut index = SymbolIndex::new();

        let symbol = HirSymbol {
            name: Arc::from("Car"),
            short_name: None,
            qualified_name: Arc::from("Car"),
            element_id: new_element_id(),
            kind: SymbolKind::PartDef,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 10,
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
            view_data: None,
        };

        index.add_file(FileId::new(0), vec![symbol]);

        // Query at position without type_ref
        let info = type_info_at(&index, FileId::new(0), 5, 5);
        assert!(info.is_none());
    }
}
