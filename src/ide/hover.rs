//! Hover information implementation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirRelationship, HirSymbol, RelationshipKind, SymbolIndex, SymbolKind};
use crate::ide::type_info::{find_type_ref_at_position, resolve_type_ref_with_chain};

/// A resolved relationship with target location info for building links.
#[derive(Clone, Debug)]
pub struct ResolvedRelationship {
    /// The kind of relationship.
    pub kind: RelationshipKind,
    /// The target name as written.
    pub target_name: Arc<str>,
    /// The resolved target's file (if found).
    pub target_file: Option<FileId>,
    /// The resolved target's start line (if found).
    pub target_line: Option<u32>,
}

/// Result of a hover request.
#[derive(Clone, Debug)]
pub struct HoverResult {
    /// The hover content (markdown).
    pub contents: String,
    /// Qualified name of the hovered symbol (for reference lookup).
    pub qualified_name: Option<Arc<str>>,
    /// Whether this is a definition (for determining if we should show references).
    pub is_definition: bool,
    /// Resolved relationships with target location info for building links.
    pub relationships: Vec<ResolvedRelationship>,
    /// Start line of the hovered range (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed).
    pub end_col: u32,
}

impl HoverResult {
    /// Create a new hover result with resolved relationships.
    pub fn new(contents: String, symbol: &HirSymbol, index: &SymbolIndex) -> Self {
        Self {
            contents,
            qualified_name: Some(symbol.qualified_name.clone()),
            is_definition: symbol.kind.is_definition(),
            relationships: resolve_relationships(&symbol.relationships, index),
            start_line: symbol.start_line,
            start_col: symbol.start_col,
            end_line: symbol.end_line,
            end_col: symbol.end_col,
        }
    }
}

/// Resolve relationships to get target file/line info.
fn resolve_relationships(
    relationships: &[HirRelationship],
    index: &SymbolIndex,
) -> Vec<ResolvedRelationship> {
    relationships
        .iter()
        .map(|rel| {
            let target_name = rel.target.clone();

            // Try multiple lookup strategies:
            // 1. Direct qualified name lookup (e.g., "Parts::Part")
            // 2. Definition lookup by simple name
            // 3. Simple name lookup (returns all matches, take first)
            // 4. If the name contains ::, try the last segment as simple name
            let target_symbol = index
                .lookup_qualified(&target_name)
                .or_else(|| index.lookup_definition(&target_name))
                .or_else(|| index.lookup_simple(&target_name).into_iter().next())
                .or_else(|| {
                    // For qualified names like "Parts::Part", try looking up by the last segment
                    if let Some(simple_name) = target_name.rsplit("::").next() {
                        index
                            .lookup_definition(simple_name)
                            .or_else(|| index.lookup_simple(simple_name).into_iter().next())
                    } else {
                        None
                    }
                });

            ResolvedRelationship {
                kind: rel.kind,
                target_name,
                target_file: target_symbol.map(|s| s.file),
                target_line: target_symbol.map(|s| s.start_line),
            }
        })
        .collect()
}

/// Get hover information for a position.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
///
/// # Returns
/// Hover information, or None if nothing to show.
pub fn hover(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> Option<HoverResult> {
    // First, check if cursor is on a type reference (e.g., ::>, :, :>)
    if let Some(ctx) = find_type_ref_at_position(index, file, line, col) {
        // Try to resolve and show hover for the target type
        if let Some(target_symbol) = resolve_type_ref_with_chain(index, &ctx) {
            let contents = build_hover_content(&target_symbol, index);
            // Return with the type_ref's span (where the cursor is)
            return Some(HoverResult {
                contents,
                qualified_name: Some(target_symbol.qualified_name.clone()),
                is_definition: target_symbol.kind.is_definition(),
                relationships: resolve_relationships(&target_symbol.relationships, index),
                start_line: ctx.type_ref.start_line,
                start_col: ctx.type_ref.start_col,
                end_line: ctx.type_ref.end_line,
                end_col: ctx.type_ref.end_col,
            });
        }
    }

    // Otherwise, find the symbol at the cursor position
    let symbol = find_symbol_at_position(index, file, line, col)?;

    // Build hover content
    let contents = build_hover_content(symbol, index);

    Some(HoverResult::new(contents, symbol, index))
}

/// Build markdown hover content for a symbol.
fn build_hover_content(symbol: &HirSymbol, _index: &SymbolIndex) -> String {
    let mut content = String::new();

    // Symbol signature
    content.push_str("```sysml\n");
    content.push_str(&build_signature(symbol));
    content.push_str("\n```\n");

    // Documentation
    if let Some(ref doc) = symbol.doc {
        content.push_str("\n---\n\n");
        content.push_str(doc);
        content.push('\n');
    }

    // Note: Relationships are formatted at the LSP layer with clickable links.

    // Qualified name for context
    content.push_str("\n**Qualified Name:** `");
    content.push_str(&symbol.qualified_name);
    content.push_str("`\n");

    // Note: "Referenced by:" section is added at the LSP layer.

    content
}

/// Build a signature string for a symbol.
fn build_signature(symbol: &HirSymbol) -> String {
    let kind_str = symbol.kind.display();

    // Build name with short name alias if present
    let name_with_alias = if let Some(ref short) = symbol.short_name {
        if short.as_ref() != symbol.name.as_ref() {
            format!("<{}> {}", short, symbol.name)
        } else {
            symbol.name.to_string()
        }
    } else {
        symbol.name.to_string()
    };

    match symbol.kind {
        // Definitions
        SymbolKind::PartDef
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
        | SymbolKind::InteractionDef => {
            let mut sig = format!("{} {}", kind_str, name_with_alias);
            if !symbol.supertypes.is_empty() {
                sig.push_str(" :> ");
                sig.push_str(&symbol.supertypes.join(", "));
            }
            sig
        }

        // Usages
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
        | SymbolKind::FlowUsage => {
            let mut sig = format!("{} {}", kind_str, name_with_alias);
            if !symbol.supertypes.is_empty() {
                sig.push_str(" : ");
                sig.push_str(symbol.supertypes[0].as_ref());
            }
            sig
        }

        // Package
        SymbolKind::Package => format!("package {}", name_with_alias),

        // Import
        SymbolKind::Import => format!("import {}", symbol.name),

        // Alias
        SymbolKind::Alias => {
            if !symbol.supertypes.is_empty() {
                format!("alias {} for {}", name_with_alias, symbol.supertypes[0])
            } else {
                format!("alias {}", name_with_alias)
            }
        }

        // Other
        SymbolKind::Comment | SymbolKind::Other | SymbolKind::Dependency => name_with_alias,
    }
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
        if contains_position(symbol, line, col) || contains_short_name_position(symbol, line, col) {
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

/// Check if position is within the symbol's short_name span (for hover on short names).
fn contains_short_name_position(symbol: &HirSymbol, line: u32, col: u32) -> bool {
    // All four span components must be present
    let (Some(start_line), Some(start_col), Some(end_line), Some(end_col)) = (
        symbol.short_name_start_line,
        symbol.short_name_start_col,
        symbol.short_name_end_line,
        symbol.short_name_end_col,
    ) else {
        return false;
    };

    let after_start = line > start_line || (line == start_line && col >= start_col);
    let before_end = line < end_line || (line == end_line && col <= end_col);
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

    fn make_symbol(name: &str, qualified: &str, kind: SymbolKind, line: u32) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            kind,
            file: FileId::new(0),
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: 20,
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
    fn test_hover_part_def() {
        let mut index = SymbolIndex::new();
        let mut def = make_symbol("Car", "Vehicle::Car", SymbolKind::PartDef, 5);
        def.doc = Some(Arc::from("A car is a vehicle."));
        def.supertypes = vec![Arc::from("Vehicle")];
        index.add_file(FileId::new(0), vec![def]);

        let result = hover(&index, FileId::new(0), 5, 5);

        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.contents.contains("Part def Car"));
        assert!(hover.contents.contains(":> Vehicle"));
        assert!(hover.contents.contains("A car is a vehicle"));
    }

    #[test]
    fn test_hover_usage() {
        let mut index = SymbolIndex::new();
        let mut usage = make_symbol("engine", "Car::engine", SymbolKind::PartUsage, 10);
        usage.supertypes = vec![Arc::from("Engine")];
        index.add_file(FileId::new(0), vec![usage]);

        let result = hover(&index, FileId::new(0), 10, 5);

        assert!(result.is_some());
        let hover = result.unwrap();
        assert!(hover.contents.contains("Part engine"));
        assert!(hover.contents.contains(": Engine"));
    }

    #[test]
    fn test_hover_not_found() {
        let index = SymbolIndex::new();
        let result = hover(&index, FileId::new(0), 0, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_signature_package() {
        let symbol = make_symbol("Vehicle", "Vehicle", SymbolKind::Package, 0);
        let sig = build_signature(&symbol);
        assert_eq!(sig, "package Vehicle");
    }
}
