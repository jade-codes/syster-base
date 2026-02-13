//! Symbol listing for workspace and document views.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, SymbolIndex, SymbolKind};

/// A symbol for the workspace symbol list or document outline.
#[derive(Clone, Debug)]
pub struct SymbolInfo {
    /// Symbol name.
    pub name: Arc<str>,
    /// Qualified name (for grouping/hierarchy).
    pub qualified_name: Arc<str>,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// File containing the symbol.
    pub file: FileId,
    /// Start line (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed).
    pub end_col: u32,
}

impl SymbolInfo {
    /// Create from a HirSymbol.
    pub fn from_hir(symbol: &HirSymbol) -> Self {
        Self {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind,
            file: symbol.file,
            start_line: symbol.start_line,
            start_col: symbol.start_col,
            end_line: symbol.end_line,
            end_col: symbol.end_col,
        }
    }

    /// Get the container name (parent path) for hierarchy building.
    pub fn container_name(&self) -> Option<&str> {
        let qname = self.qualified_name.as_ref();
        qname.rfind("::").map(|idx| &qname[..idx])
    }
}

/// Get all symbols in the workspace, optionally filtered by a query.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `query` - Optional search query (case-insensitive substring match)
///
/// # Returns
/// List of matching symbols, sorted by name.
pub fn workspace_symbols(index: &SymbolIndex, query: Option<&str>) -> Vec<SymbolInfo> {
    let query_lower = query.map(|q| q.to_lowercase());

    let mut results: Vec<SymbolInfo> = index
        .all_symbols()
        .filter(|sym| {
            // Skip imports
            if matches!(sym.kind, SymbolKind::Import) {
                return false;
            }

            // Filter by query if provided
            if let Some(ref q) = query_lower {
                let name_lower = sym.name.to_lowercase();
                let qname_lower = sym.qualified_name.to_lowercase();
                name_lower.contains(q) || qname_lower.contains(q)
            } else {
                true
            }
        })
        .map(SymbolInfo::from_hir)
        .collect();

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

/// Get all symbols in a specific file for document outline.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file to get symbols for
///
/// # Returns
/// List of symbols in the file, in source order.
pub fn document_symbols(index: &SymbolIndex, file: FileId) -> Vec<SymbolInfo> {
    let mut results: Vec<SymbolInfo> = index
        .symbols_in_file(file)
        .into_iter()
        .filter(|sym| {
            // Skip imports and comments in document outline
            !matches!(sym.kind, SymbolKind::Import | SymbolKind::Comment)
        })
        .map(SymbolInfo::from_hir)
        .collect();

    // Sort by position in file
    results.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(a.start_col.cmp(&b.start_col))
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::new_element_id;

    fn make_symbol(name: &str, qname: &str, kind: SymbolKind, line: u32) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qname),
            element_id: new_element_id(),
            kind,
            file: FileId::new(0),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 10,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            doc: None,
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
    fn test_workspace_symbols_no_filter() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("Vehicle", "Vehicle", SymbolKind::PartDefinition, 0),
                make_symbol("Car", "Vehicle::Car", SymbolKind::PartDefinition, 5),
                make_symbol("engine", "Vehicle::Car::engine", SymbolKind::PartUsage, 10),
            ],
        );

        let results = workspace_symbols(&index, None);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_workspace_symbols_with_filter() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("Vehicle", "Vehicle", SymbolKind::PartDefinition, 0),
                make_symbol("Truck", "Truck", SymbolKind::PartDefinition, 5),
                make_symbol("engine", "Vehicle::engine", SymbolKind::PartUsage, 10),
            ],
        );

        // "truck" should only match the Truck definition, not the engine
        let results = workspace_symbols(&index, Some("truck"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name.as_ref(), "Truck");
    }

    #[test]
    fn test_document_symbols() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("Vehicle", "Vehicle", SymbolKind::Package, 0),
                make_symbol("Car", "Vehicle::Car", SymbolKind::PartDefinition, 5),
            ],
        );
        index.add_file(
            FileId::new(1),
            vec![make_symbol("Other", "Other", SymbolKind::Package, 0)],
        );

        let results = document_symbols(&index, FileId::new(0));
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name.as_ref(), "Vehicle");
        assert_eq!(results[1].name.as_ref(), "Car");
    }

    #[test]
    fn test_container_name() {
        let sym = SymbolInfo {
            name: Arc::from("engine"),
            qualified_name: Arc::from("Vehicle::Car::engine"),
            kind: SymbolKind::PartUsage,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 10,
        };

        assert_eq!(sym.container_name(), Some("Vehicle::Car"));
    }
}
