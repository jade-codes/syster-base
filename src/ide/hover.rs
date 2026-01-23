//! Hover information implementation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{SymbolIndex, HirSymbol, SymbolKind, TypeRef, ResolveResult};

/// Result of a hover request.
#[derive(Clone, Debug)]
pub struct HoverResult {
    /// The hover content (markdown).
    pub contents: String,
    /// Qualified name of the hovered symbol (for reference lookup).
    pub qualified_name: Option<Arc<str>>,
    /// Whether this is a definition (for determining if we should show references).
    pub is_definition: bool,
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
    /// Create a new hover result.
    pub fn new(contents: String, symbol: &HirSymbol) -> Self {
        Self {
            contents,
            qualified_name: Some(symbol.qualified_name.clone()),
            is_definition: symbol.kind.is_definition(),
            start_line: symbol.start_line,
            start_col: symbol.start_col,
            end_line: symbol.end_line,
            end_col: symbol.end_col,
        }
    }
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
pub fn hover(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<HoverResult> {
    
    // First, check if cursor is on a type reference (e.g., ::>, :, :>)
    if let Some((_target_name, type_ref, _containing_symbol)) = find_type_ref_at_position(index, file, line, col) {
        // Use pre-resolved target if available (computed during semantic analysis)
        let target_symbol = if let Some(resolved) = &type_ref.resolved_target {
            index.lookup_qualified(resolved).cloned()
        } else {
            // Fallback: try to resolve at hover time (for backwards compatibility)
            let scope = _containing_symbol.map(|s| s.qualified_name.as_ref()).unwrap_or("");
            let resolver = index.resolver_for_scope(scope);
            
            match resolver.resolve(&_target_name) {
                ResolveResult::Found(sym) => {
                    Some(sym)
                }
                ResolveResult::Ambiguous(syms) => {
                    syms.into_iter().next()
                }
                ResolveResult::NotFound => {
                    // Try qualified name directly
                    let result = index.lookup_qualified(&_target_name).cloned();
                    result
                }
            }
        };
        
        if let Some(target_symbol) = target_symbol {
            let contents = build_hover_content(&target_symbol, index);
            // Return with the type_ref's span (where the cursor is)
            return Some(HoverResult {
                contents,
                qualified_name: Some(target_symbol.qualified_name.clone()),
                is_definition: target_symbol.kind.is_definition(),
                start_line: type_ref.start_line,
                start_col: type_ref.start_col,
                end_line: type_ref.end_line,
                end_col: type_ref.end_col,
            });
        }
    }

    // Otherwise, find the symbol at the cursor position
    let symbol = find_symbol_at_position(index, file, line, col)?;
    
    // Build hover content
    let contents = build_hover_content(symbol, index);
    
    Some(HoverResult::new(contents, symbol))
}

/// Build markdown hover content for a symbol.
fn build_hover_content(symbol: &HirSymbol, index: &SymbolIndex) -> String {
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
    
    // Type information for usages
    if symbol.kind.is_usage() && !symbol.supertypes.is_empty() {
        content.push_str("\n**Typed by:** ");
        content.push_str(&symbol.supertypes.join(", "));
        
        // Try to add info about the type
        if let Some(type_symbol) = index.lookup_definition(&symbol.supertypes[0]) {
            if let Some(ref doc) = type_symbol.doc {
                content.push_str("\n\n*");
                // First sentence of doc
                let first_sentence = doc.split('.').next().unwrap_or(doc);
                content.push_str(first_sentence.trim());
                content.push_str("*");
            }
        }
        content.push('\n');
    }
    
    // Qualified name for context
    content.push_str("\n**Qualified Name:** `");
    content.push_str(&symbol.qualified_name);
    content.push_str("`\n");
    
    // Note: "Referenced by:" section is added at the LSP layer 
    // because it needs file path resolution for clickable links.
    
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
        SymbolKind::PartDef | SymbolKind::ItemDef | SymbolKind::ActionDef |
        SymbolKind::PortDef | SymbolKind::AttributeDef | SymbolKind::ConnectionDef |
        SymbolKind::InterfaceDef | SymbolKind::AllocationDef | SymbolKind::RequirementDef |
        SymbolKind::ConstraintDef | SymbolKind::StateDef | SymbolKind::CalculationDef |
        SymbolKind::UseCaseDef | SymbolKind::AnalysisCaseDef | SymbolKind::ConcernDef |
        SymbolKind::ViewDef | SymbolKind::ViewpointDef | SymbolKind::RenderingDef |
        SymbolKind::EnumerationDef => {
            let mut sig = format!("{} {}", kind_str, name_with_alias);
            if !symbol.supertypes.is_empty() {
                sig.push_str(" :> ");
                sig.push_str(&symbol.supertypes.join(", "));
            }
            sig
        }
        
        // Usages
        SymbolKind::PartUsage | SymbolKind::ItemUsage | SymbolKind::ActionUsage |
        SymbolKind::PortUsage | SymbolKind::AttributeUsage | SymbolKind::ConnectionUsage |
        SymbolKind::InterfaceUsage | SymbolKind::AllocationUsage | SymbolKind::RequirementUsage |
        SymbolKind::ConstraintUsage | SymbolKind::StateUsage | SymbolKind::CalculationUsage |
        SymbolKind::ReferenceUsage | SymbolKind::OccurrenceUsage | SymbolKind::FlowUsage => {
            let mut sig = format!("{} {}", kind_str, name_with_alias);
            if !symbol.supertypes.is_empty() {
                sig.push_str(" : ");
                sig.push_str(&symbol.supertypes[0].as_ref());
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
fn find_symbol_at_position(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> Option<&HirSymbol> {
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
    let after_start = line > symbol.start_line 
        || (line == symbol.start_line && col >= symbol.start_col);
    let before_end = line < symbol.end_line 
        || (line == symbol.end_line && col <= symbol.end_col);
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
    
    let after_start = line > start_line 
        || (line == start_line && col >= start_col);
    let before_end = line < end_line 
        || (line == end_line && col <= end_col);
    after_start && before_end
}

fn symbol_size(symbol: &HirSymbol) -> u32 {
    let line_diff = symbol.end_line.saturating_sub(symbol.start_line);
    let col_diff = symbol.end_col.saturating_sub(symbol.start_col);
    line_diff * 1000 + col_diff
}

/// Find a type reference at a specific position in a file.
///
/// Returns the target type name, the TypeRef containing the position,
/// and the symbol that contains this type_ref (for scope resolution).
fn find_type_ref_at_position<'a>(index: &'a SymbolIndex, file: FileId, line: u32, col: u32) -> Option<(Arc<str>, &'a TypeRef, Option<&'a HirSymbol>)> {
    let symbols = index.symbols_in_file(file);
    
    for symbol in symbols {
        for (_idx, type_ref_kind) in symbol.type_refs.iter().enumerate() {
            // Debug: print all type_refs for message symbols on line 780
            if cfg!(debug_assertions) && symbol.name.contains("ignitionCmd") && line == 780 {
                eprintln!("[HOVER DEBUG] Checking symbol '{}' type_ref at line={} col={}", 
                    symbol.name, line, col);
                for tr in type_ref_kind.as_refs() {
                    eprintln!("[HOVER DEBUG]   TypeRef: target='{}' span={}:{}-{}:{} contains={} resolved_target={:?}", 
                        tr.target, tr.start_line, tr.start_col, tr.end_line, tr.end_col,
                        tr.contains(line, col), tr.resolved_target);
                }
            }
            
            let contains = type_ref_kind.contains(line, col);
            
            if contains {
                // Find which part contains the position
                if let Some((_part_idx, tr)) = type_ref_kind.part_at(line, col) {
                    if cfg!(debug_assertions) && symbol.name.contains("ignitionCmd") && line == 780 {
                        eprintln!("[HOVER DEBUG]   FOUND! part_at returned target='{}' resolved_target={:?}", tr.target, tr.resolved_target);
                    }
                    return Some((tr.target.clone(), tr, Some(symbol)));
                } else if cfg!(debug_assertions) && symbol.name.contains("ignitionCmd") {
                    eprintln!("[HOVER DEBUG]   contains=true but part_at returned None!");
                }
            }
        }
    }
    
    None
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
            type_refs: Vec::new(),
            is_public: false,
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
