//! Find references implementation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, SymbolIndex, SymbolKind, TypeRef};

/// Result of a find-references request.
#[derive(Clone, Debug)]
pub struct ReferenceResult {
    /// All references found.
    pub references: Vec<Reference>,
    /// Include the definition in the results.
    pub include_declaration: bool,
}

impl ReferenceResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            references: Vec::new(),
            include_declaration: false,
        }
    }

    /// Check if any references were found.
    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    /// Get the number of references.
    pub fn len(&self) -> usize {
        self.references.len()
    }
}

/// A reference to a symbol.
#[derive(Clone, Debug)]
pub struct Reference {
    /// The file containing the reference.
    pub file: FileId,
    /// Start line (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed).
    pub end_col: u32,
    /// Whether this is the definition (vs a reference).
    pub is_definition: bool,
    /// The symbol kind.
    pub kind: SymbolKind,
}

impl Reference {
    /// Create from a symbol.
    pub fn from_symbol(symbol: &HirSymbol, is_definition: bool) -> Self {
        Self {
            file: symbol.file,
            start_line: symbol.start_line,
            start_col: symbol.start_col,
            end_line: symbol.end_line,
            end_col: symbol.end_col,
            is_definition,
            kind: symbol.kind,
        }
    }

    /// Create from a type reference.
    pub fn from_type_ref(type_ref: &TypeRef, file: FileId) -> Self {
        Self {
            file,
            start_line: type_ref.start_line,
            start_col: type_ref.start_col,
            end_line: type_ref.end_line,
            end_col: type_ref.end_col,
            is_definition: false,
            kind: SymbolKind::Other, // Type references don't have a specific kind
        }
    }
}

/// Find all references to the symbol at the given position.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
/// * `include_declaration` - Whether to include the definition
///
/// # Returns
/// All references to the symbol, or empty if not found.
pub fn find_references(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
    include_declaration: bool,
) -> ReferenceResult {
    // First, check if cursor is on a type reference
    if let Some((target_name, _source_symbol)) = find_type_ref_at_position(index, file, line, col) {
        return find_references_for_target(index, &target_name, include_declaration);
    }

    // Find the symbol at the cursor position
    let symbol = match find_symbol_at_position(index, file, line, col) {
        Some(s) => s,
        None => return ReferenceResult::empty(),
    };

    // Determine what we're looking for
    let target_name = if symbol.kind.is_definition() {
        // Looking for references TO this definition - use qualified name
        symbol.qualified_name.clone()
    } else {
        // Looking for references to the type this usage refers to
        if !symbol.supertypes.is_empty() {
            symbol.supertypes[0].clone()
        } else {
            symbol.qualified_name.clone()
        }
    };

    find_references_for_target(index, &target_name, include_declaration)
}

/// Find all references to a named target.
fn find_references_for_target(
    index: &SymbolIndex,
    target_name: &str,
    include_declaration: bool,
) -> ReferenceResult {
    let mut references = Vec::new();

    // Find the definition
    if let Some(def) = find_definition(index, target_name) {
        if include_declaration {
            references.push(Reference::from_symbol(def, true));
        }
    }

    // Extract simple name from qualified (for matching unqualified references)
    let _simple_name = target_name.rsplit("::").next().unwrap_or(target_name);

    // Collect all type references to this target (actual textual locations)
    // Use resolved_target for proper scoped matching
    let type_refs: Vec<_> = index
        .all_symbols()
        .flat_map(|sym| {
            sym.type_refs
                .iter()
                .flat_map(|trk| trk.as_refs()) // Flatten TypeRefKind to &TypeRef
                .filter(|tr| {
                    // Use effective_target (resolved if available) for accurate scoped matching
                    tr.effective_target().as_ref() == target_name
                })
                .map(move |tr| Reference::from_type_ref(tr, sym.file))
        })
        .collect();

    references.extend(type_refs);

    // Find direct name matches (for things like package references)
    for sym in index.all_symbols() {
        if sym.name.as_ref() == target_name && !sym.kind.is_definition() {
            // Avoid duplicates
            if !references.iter().any(|r| {
                r.file == sym.file && r.start_line == sym.start_line && r.start_col == sym.start_col
            }) {
                references.push(Reference::from_symbol(sym, false));
            }
        }
    }

    ReferenceResult {
        references,
        include_declaration,
    }
}

/// Find a type reference at a specific position.
fn find_type_ref_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<(Arc<str>, &HirSymbol)> {
    let symbols = index.symbols_in_file(file);

    for symbol in symbols {
        for type_ref_kind in &symbol.type_refs {
            if type_ref_kind.contains(line, col) {
                if let Some((_, tr)) = type_ref_kind.part_at(line, col) {
                    // Use effective_target to get resolved name for proper scoped matching
                    return Some((tr.effective_target().clone(), symbol));
                }
            }
        }
    }

    None
}

/// Find the definition for a name.
fn find_definition<'a>(index: &'a SymbolIndex, name: &str) -> Option<&'a HirSymbol> {
    // Try qualified name first
    if let Some(def) = index.lookup_definition(name) {
        return Some(def);
    }

    // Extract simple name for lookup
    let simple_name = name.rsplit("::").next().unwrap_or(name);

    // Try simple name lookup
    let simple_matches: Vec<_> = index
        .lookup_simple(simple_name)
        .into_iter()
        .filter(|s| s.kind.is_definition())
        .collect();

    if simple_matches.len() == 1 {
        return Some(simple_matches[0]);
    }

    // If name contains ::, try suffix matching on qualified names
    if name.contains("::") {
        let suffix = format!("::{}", name);
        for def in index.all_definitions() {
            if def.qualified_name.ends_with(&suffix) || def.qualified_name.as_ref() == name {
                return Some(def);
            }
        }
    }

    None
}

/// Find the symbol at a specific position.
fn find_symbol_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<&HirSymbol> {
    let symbols = index.symbols_in_file(file);

    let mut best: Option<&HirSymbol> = None;

    for symbol in symbols {
        if contains_position(symbol, line, col) {
            match best {
                None => best = Some(symbol),
                Some(current) => {
                    if symbol_size(symbol) < symbol_size(current) {
                        best = Some(symbol);
                    }
                }
            }
        }
    }

    best
}

fn contains_position(symbol: &HirSymbol, line: u32, col: u32) -> bool {
    let after_start =
        line > symbol.start_line || (line == symbol.start_line && col >= symbol.start_col);
    let before_end = line < symbol.end_line || (line == symbol.end_line && col <= symbol.end_col);
    after_start && before_end
}

fn symbol_size(symbol: &HirSymbol) -> u32 {
    let line_diff = symbol.end_line.saturating_sub(symbol.start_line);
    let col_diff = symbol.end_col.saturating_sub(symbol.start_col);
    line_diff * 1000 + col_diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{RefKind, new_element_id};

    fn make_symbol(
        name: &str,
        qualified: &str,
        kind: SymbolKind,
        file: u32,
        line: u32,
    ) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            element_id: new_element_id(),
            kind,
            file: FileId::new(file),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 10,
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
        }
    }

    #[test]
    fn test_find_references_from_definition() {
        use crate::hir::TypeRefKind;

        let mut index = SymbolIndex::new();

        // Definition
        let engine_def = make_symbol("Engine", "Engine", SymbolKind::PartDef, 0, 1);

        // Usages with type_refs (this is how references are found)
        let mut engine_usage1 = make_symbol("engine", "Car::engine", SymbolKind::PartUsage, 0, 10);
        engine_usage1.supertypes = vec![Arc::from("Engine")];
        engine_usage1.type_refs = vec![TypeRefKind::Simple(TypeRef::new(
            "Engine",
            RefKind::TypedBy,
            10,
            15,
            10,
            21,
        ))];

        let mut engine_usage2 = make_symbol("motor", "Truck::motor", SymbolKind::PartUsage, 1, 5);
        engine_usage2.supertypes = vec![Arc::from("Engine")];
        engine_usage2.type_refs = vec![TypeRefKind::Simple(TypeRef::new(
            "Engine",
            RefKind::TypedBy,
            5,
            12,
            5,
            18,
        ))];

        index.add_file(FileId::new(0), vec![engine_def, engine_usage1]);
        index.add_file(FileId::new(1), vec![engine_usage2]);

        // Click on the definition
        let result = find_references(&index, FileId::new(0), 1, 5, true);

        // Should find: definition + 2 type_refs
        assert_eq!(result.len(), 3);
        assert!(result.references.iter().any(|r| r.is_definition));
    }

    #[test]
    fn test_find_references_from_usage() {
        let mut index = SymbolIndex::new();

        let wheel_def = make_symbol("Wheel", "Wheel", SymbolKind::PartDef, 0, 1);

        let mut wheel_usage = make_symbol(
            "frontWheel",
            "Car::frontWheel",
            SymbolKind::PartUsage,
            0,
            10,
        );
        wheel_usage.supertypes = vec![Arc::from("Wheel")];

        index.add_file(FileId::new(0), vec![wheel_def, wheel_usage]);

        // Click on the usage
        let result = find_references(&index, FileId::new(0), 10, 5, true);

        // Should find definition + the usage
        assert!(!result.is_empty());
    }

    #[test]
    fn test_find_references_exclude_declaration() {
        let mut index = SymbolIndex::new();

        let part_def = make_symbol("Part", "Part", SymbolKind::PartDef, 0, 1);

        let mut usage = make_symbol("myPart", "myPart", SymbolKind::PartUsage, 0, 10);
        usage.supertypes = vec![Arc::from("Part")];

        index.add_file(FileId::new(0), vec![part_def, usage]);

        let result = find_references(&index, FileId::new(0), 1, 5, false);

        // Should NOT include the definition
        assert!(result.references.iter().all(|r| !r.is_definition));
    }

    #[test]
    fn test_find_references_not_found() {
        let index = SymbolIndex::new();
        let result = find_references(&index, FileId::new(0), 0, 0, true);
        assert!(result.is_empty());
    }
}
