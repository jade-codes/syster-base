//! Document links — clickable references to definitions.

use crate::base::FileId;
use crate::hir::{SymbolIndex, SymbolKind, Resolver, ResolveResult};
use std::borrow::Cow;

/// A document link target.
#[derive(Debug, Clone)]
pub struct DocumentLink {
    /// The span of the link in the source file.
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// The target file containing the definition.
    pub target_file: FileId,
    /// The target position in the file.
    pub target_line: u32,
    pub target_col: u32,
    /// Tooltip text for the link.
    pub tooltip: Cow<'static, str>,
}

/// Extract the parent scope from a qualified name.
fn extract_scope(qualified_name: &str) -> String {
    if let Some(pos) = qualified_name.rfind("::") {
        qualified_name[..pos].to_string()
    } else {
        String::new()
    }
}

/// Get document links for a file.
///
/// Returns clickable links for:
/// 1. Import statements - link to the definition of the imported symbol
/// 2. Type references - link to the definition of the referenced type
pub fn document_links(index: &SymbolIndex, file: FileId) -> Vec<DocumentLink> {
    let mut links = Vec::new();
    
    // Get all symbols in this file
    let symbols = index.symbols_in_file(file);
    
    for sym in symbols {
        match sym.kind {
            SymbolKind::Import => {
                // For imports, the "name" field contains the import path (e.g., "Base::*", "Pkg::Thing")
                // Strip wildcard suffixes for resolution
                let import_path = sym.name.as_ref();
                let resolved_path = if import_path.ends_with("::*") {
                    &import_path[..import_path.len() - 3]
                } else if import_path.ends_with(":::**") {
                    &import_path[..import_path.len() - 5]
                } else {
                    import_path
                };
                
                // Use scope-aware resolver to find the target
                // The import's scope is its parent package
                let scope = extract_scope(&sym.qualified_name);
                let resolver = Resolver::new(index).with_scope(scope);
                
                let target = match resolver.resolve(resolved_path) {
                    ResolveResult::Found(t) => Some(t),
                    ResolveResult::Ambiguous(targets) => targets.into_iter().next(),
                    ResolveResult::NotFound => {
                        // Fallback to exact qualified lookup
                        index.lookup_qualified(resolved_path).cloned()
                    }
                };
                
                if let Some(target) = target {
                    links.push(DocumentLink {
                        start_line: sym.start_line,
                        start_col: sym.start_col,
                        end_line: sym.end_line,
                        end_col: sym.end_col,
                        target_file: target.file,
                        target_line: target.start_line,
                        target_col: target.start_col,
                        tooltip: Cow::Owned(format!("Go to {}", target.qualified_name)),
                    });
                }
            }
            _ => {
                // For other symbols, add links for their type references
                for type_ref_kind in &sym.type_refs {
                    for type_ref in type_ref_kind.as_refs() {
                        let target_qname = &type_ref.target;
                        if let Some(target) = index.lookup_qualified(target_qname) {
                            links.push(DocumentLink {
                                start_line: type_ref.start_line,
                                start_col: type_ref.start_col,
                                end_line: type_ref.end_line,
                                end_col: type_ref.end_col,
                                target_file: target.file,
                                target_line: target.start_line,
                                target_col: target.start_col,
                                tooltip: Cow::Owned(format!("Go to {}", target_qname)),
                            });
                        }
                    }
                }
            }
        }
    }
    
    links
}
