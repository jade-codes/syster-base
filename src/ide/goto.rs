//! Go-to-definition implementation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, RefKind, ResolveResult, Resolver, SymbolIndex, SymbolKind, TypeRef};

/// Result of a go-to-definition request.
#[derive(Clone, Debug)]
pub struct GotoResult {
    /// The targets to jump to.
    pub targets: Vec<GotoTarget>,
}

impl GotoResult {
    /// Create an empty result (no targets found).
    pub fn empty() -> Self {
        Self {
            targets: Vec::new(),
        }
    }

    /// Create a result with a single target.
    pub fn single(target: GotoTarget) -> Self {
        Self {
            targets: vec![target],
        }
    }

    /// Create a result with multiple targets.
    pub fn multiple(targets: Vec<GotoTarget>) -> Self {
        Self { targets }
    }

    /// Check if any targets were found.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

/// A target location for go-to-definition.
#[derive(Clone, Debug)]
pub struct GotoTarget {
    /// The file containing the target.
    pub file: FileId,
    /// Start line (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed).
    pub end_col: u32,
    /// The symbol kind.
    pub kind: SymbolKind,
    /// The symbol name.
    pub name: Arc<str>,
}

impl From<&HirSymbol> for GotoTarget {
    fn from(symbol: &HirSymbol) -> Self {
        Self {
            file: symbol.file,
            start_line: symbol.start_line,
            start_col: symbol.start_col,
            end_line: symbol.end_line,
            end_col: symbol.end_col,
            kind: symbol.kind,
            name: symbol.name.clone(),
        }
    }
}

/// Find the definition of a symbol at the given position.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
///
/// # Returns
/// The location(s) of the definition, or empty if not found.
pub fn goto_definition(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> GotoResult {
    // First, check if cursor is on a type reference
    if let Some((target_name, type_ref, source_symbol)) =
        find_type_ref_at_position(index, file, line, col)
    {
        // Build resolver with scope from the source symbol
        let scope = extract_scope(&source_symbol.qualified_name);
        let resolver = Resolver::new(index).with_scope(scope);

        // For Expression refs (like unit bracket [spatialCF]), we want to find the symbol
        // even if it's a usage, not just definitions. For other refs (TypedBy, etc.),
        // we only want definitions.
        let resolve_result = if type_ref.kind == RefKind::Expression {
            resolver.resolve(&target_name)
        } else {
            resolver.resolve_type(&target_name)
        };

        match resolve_result {
            ResolveResult::Found(def) => {
                return GotoResult::single(GotoTarget::from(&def));
            }
            ResolveResult::Ambiguous(defs) => {
                let targets = defs.iter().map(GotoTarget::from).collect();
                return GotoResult::multiple(targets);
            }
            ResolveResult::NotFound => {
                // Try without scope as a fallback
                if let Some(def) = index.lookup_definition(&target_name) {
                    return GotoResult::single(GotoTarget::from(def));
                }
            }
        }
    }

    // Find the symbol at the cursor position
    let symbol = match find_symbol_at_position(index, file, line, col) {
        Some(s) => s,
        None => return GotoResult::empty(),
    };

    // If this is already a definition, return it
    if symbol.kind.is_definition() {
        return GotoResult::single(GotoTarget::from(symbol));
    }

    // For usages, find the definition via type reference
    if !symbol.supertypes.is_empty() {
        let type_name = &symbol.supertypes[0];

        // Build resolver with scope
        let scope = extract_scope(&symbol.qualified_name);
        let resolver = Resolver::new(index).with_scope(scope);

        match resolver.resolve_type(type_name) {
            ResolveResult::Found(def) => {
                return GotoResult::single(GotoTarget::from(&def));
            }
            ResolveResult::Ambiguous(defs) => {
                let targets = defs.iter().map(GotoTarget::from).collect();
                return GotoResult::multiple(targets);
            }
            ResolveResult::NotFound => {}
        }
    }

    // Try to find definition by name (for cases without explicit typing)
    if let Some(def) = index.lookup_definition(&symbol.qualified_name) {
        return GotoResult::single(GotoTarget::from(def));
    }

    GotoResult::empty()
}

/// Go to the type definition of a symbol at the given position.
///
/// This navigates from a usage to its type definition. For example:
/// - `engine : Engine` → navigates to `part def Engine`
/// - `vehicle :> VehiclePart` → navigates to `part def VehiclePart`
///
/// Unlike `goto_definition`, this always navigates to the TYPE, not the symbol itself.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
///
/// # Returns
/// The location(s) of the type definition, or empty if not found.
pub fn goto_type_definition(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> GotoResult {
    // First, check if cursor is directly on a type reference
    if let Some((target_name, _type_ref, source_symbol)) =
        find_type_ref_at_position(index, file, line, col)
    {
        let scope = extract_scope(&source_symbol.qualified_name);
        let resolver = Resolver::new(index).with_scope(scope);

        match resolver.resolve_type(&target_name) {
            ResolveResult::Found(def) => {
                return GotoResult::single(GotoTarget::from(&def));
            }
            ResolveResult::Ambiguous(defs) => {
                let targets = defs.iter().map(GotoTarget::from).collect();
                return GotoResult::multiple(targets);
            }
            ResolveResult::NotFound => {
                // Try without scope
                if let Some(def) = index.lookup_definition(&target_name) {
                    return GotoResult::single(GotoTarget::from(def));
                }
            }
        }
    }

    // Find the symbol at the cursor position
    let symbol = match find_symbol_at_position(index, file, line, col) {
        Some(s) => s,
        None => return GotoResult::empty(),
    };

    // If this is a usage with a type, navigate to the type
    if !symbol.supertypes.is_empty() {
        let type_name = &symbol.supertypes[0];
        let scope = extract_scope(&symbol.qualified_name);
        let resolver = Resolver::new(index).with_scope(scope);

        match resolver.resolve_type(type_name) {
            ResolveResult::Found(def) => {
                return GotoResult::single(GotoTarget::from(&def));
            }
            ResolveResult::Ambiguous(defs) => {
                let targets = defs.iter().map(GotoTarget::from).collect();
                return GotoResult::multiple(targets);
            }
            ResolveResult::NotFound => {
                // Try direct lookup
                if let Some(def) = index.lookup_definition(type_name) {
                    return GotoResult::single(GotoTarget::from(def));
                }
            }
        }
    }

    // Check type_refs for typed_by relationships
    for type_ref_kind in &symbol.type_refs {
        for tr in type_ref_kind.as_refs() {
            if tr.kind == RefKind::TypedBy || tr.kind == RefKind::Specializes {
                let scope = extract_scope(&symbol.qualified_name);
                let resolver = Resolver::new(index).with_scope(scope);

                match resolver.resolve_type(&tr.target) {
                    ResolveResult::Found(def) => {
                        return GotoResult::single(GotoTarget::from(&def));
                    }
                    ResolveResult::Ambiguous(defs) => {
                        let targets = defs.iter().map(GotoTarget::from).collect();
                        return GotoResult::multiple(targets);
                    }
                    ResolveResult::NotFound => {
                        if let Some(def) = index.lookup_definition(&tr.target) {
                            return GotoResult::single(GotoTarget::from(def));
                        }
                    }
                }
            }
        }
    }

    GotoResult::empty()
}

/// Find a type reference at a specific position in a file.
///
/// Returns the target type name, the TypeRef, and the symbol containing the reference.
fn find_type_ref_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<(Arc<str>, &TypeRef, &HirSymbol)> {
    let symbols = index.symbols_in_file(file);

    for symbol in symbols {
        for type_ref_kind in &symbol.type_refs {
            if type_ref_kind.contains(line, col) {
                // Find which part contains the position and return its target
                if let Some((_, tr)) = type_ref_kind.part_at(line, col) {
                    return Some((tr.target.clone(), tr, symbol));
                }
            }
        }
    }

    None
}

/// Find the symbol at a specific position in a file.
fn find_symbol_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<&HirSymbol> {
    let symbols = index.symbols_in_file(file);

    // Find smallest symbol containing the position
    let mut best: Option<&HirSymbol> = None;

    for symbol in symbols {
        if contains_position(symbol, line, col) {
            match best {
                None => best = Some(symbol),
                Some(current) => {
                    // Prefer smaller (more specific) symbols
                    if symbol_size(symbol) < symbol_size(current) {
                        best = Some(symbol);
                    }
                }
            }
        }
    }

    best
}

/// Check if a symbol's range contains a position.
fn contains_position(symbol: &HirSymbol, line: u32, col: u32) -> bool {
    // Check if position is after start
    let after_start =
        line > symbol.start_line || (line == symbol.start_line && col >= symbol.start_col);

    // Check if position is before end
    let before_end = line < symbol.end_line || (line == symbol.end_line && col <= symbol.end_col);

    after_start && before_end
}

/// Calculate approximate size of a symbol's range.
fn symbol_size(symbol: &HirSymbol) -> u32 {
    let line_diff = symbol.end_line.saturating_sub(symbol.start_line);
    let col_diff = symbol.end_col.saturating_sub(symbol.start_col);
    line_diff * 1000 + col_diff
}

/// Extract the scope from a qualified name.
fn extract_scope(qualified_name: &str) -> String {
    if let Some(pos) = qualified_name.rfind("::") {
        qualified_name[..pos].to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::new_element_id;

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
            metadata_annotations: Vec::new(),
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            direction: None,
            multiplicity: None,
            value: None,
        }
    }

    #[test]
    fn test_goto_definition_direct() {
        let mut index = SymbolIndex::new();
        let def = make_symbol("Car", "Vehicle::Car", SymbolKind::PartDefinition, 0, 5);
        index.add_file(FileId::new(0), vec![def]);

        let result = goto_definition(&index, FileId::new(0), 5, 5);

        assert!(!result.is_empty());
        assert_eq!(result.targets.len(), 1);
        assert_eq!(result.targets[0].name.as_ref(), "Car");
    }

    #[test]
    fn test_goto_definition_from_usage() {
        let mut index = SymbolIndex::new();

        // Definition
        let def = make_symbol("Engine", "Engine", SymbolKind::PartDefinition, 0, 1);

        // Usage with type reference
        let mut usage = make_symbol("engine", "Car::engine", SymbolKind::PartUsage, 0, 10);
        usage.supertypes = vec![Arc::from("Engine")];

        index.add_file(FileId::new(0), vec![def, usage]);

        // Click on the usage
        let result = goto_definition(&index, FileId::new(0), 10, 5);

        assert!(!result.is_empty());
        assert_eq!(result.targets[0].name.as_ref(), "Engine");
        assert_eq!(result.targets[0].start_line, 1); // Goes to definition
    }

    #[test]
    fn test_goto_definition_from_type_ref() {
        use crate::hir::{RefKind, TypeRef, TypeRefKind};

        let mut index = SymbolIndex::new();

        // Definition at line 1
        let def = make_symbol("Engine", "Engine", SymbolKind::PartDefinition, 0, 1);

        // Usage at line 10, with type reference at columns 15-21 (where "Engine" appears)
        let mut usage = make_symbol("engine", "Car::engine", SymbolKind::PartUsage, 0, 10);
        usage.supertypes = vec![Arc::from("Engine")];
        usage.type_refs = vec![TypeRefKind::Simple(TypeRef::new(
            "Engine",
            RefKind::TypedBy,
            10,
            15,
            10,
            21,
        ))];

        index.add_file(FileId::new(0), vec![def, usage]);

        // Click on the type reference "Engine" (at column 17)
        let result = goto_definition(&index, FileId::new(0), 10, 17);

        assert!(!result.is_empty());
        assert_eq!(result.targets[0].name.as_ref(), "Engine");
        assert_eq!(result.targets[0].start_line, 1); // Goes to definition
    }

    #[test]
    fn test_goto_definition_not_found() {
        let index = SymbolIndex::new();
        let result = goto_definition(&index, FileId::new(0), 0, 0);
        assert!(result.is_empty());
    }
}
