//! Completion suggestions implementation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, SymbolIndex, SymbolKind};

/// Kind of completion item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionKind {
    Package,
    Definition,
    Usage,
    Keyword,
    Snippet,
}

impl CompletionKind {
    /// Convert to LSP completion item kind number.
    pub fn to_lsp(&self) -> u32 {
        match self {
            CompletionKind::Package => 9,    // Module
            CompletionKind::Definition => 7, // Class
            CompletionKind::Usage => 5,      // Field
            CompletionKind::Keyword => 14,   // Keyword
            CompletionKind::Snippet => 15,   // Snippet
        }
    }
}

/// A completion suggestion.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// The text to insert.
    pub label: Arc<str>,
    /// The kind of completion.
    pub kind: CompletionKind,
    /// Detail text (shown after label).
    pub detail: Option<Arc<str>>,
    /// Documentation (shown in popup).
    pub documentation: Option<Arc<str>>,
    /// Text to insert (if different from label).
    pub insert_text: Option<Arc<str>>,
    /// Sort priority (lower = higher priority).
    pub sort_priority: u32,
}

impl CompletionItem {
    /// Create a new completion item.
    pub fn new(label: impl Into<Arc<str>>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            sort_priority: 100,
        }
    }

    /// Set the detail text.
    pub fn with_detail(mut self, detail: impl Into<Arc<str>>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the documentation.
    pub fn with_documentation(mut self, doc: impl Into<Arc<str>>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Set the insert text.
    pub fn with_insert_text(mut self, text: impl Into<Arc<str>>) -> Self {
        self.insert_text = Some(text.into());
        self
    }

    /// Set the sort priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.sort_priority = priority;
        self
    }

    /// Create from a HirSymbol.
    pub fn from_symbol(symbol: &HirSymbol) -> Self {
        let kind = if symbol.kind.is_definition() {
            CompletionKind::Definition
        } else if symbol.kind == SymbolKind::Package {
            CompletionKind::Package
        } else {
            CompletionKind::Usage
        };

        let mut item = Self::new(symbol.name.clone(), kind);

        // Add type info as detail
        if !symbol.supertypes.is_empty() {
            item.detail = Some(Arc::from(format!(": {}", symbol.supertypes.join(", "))));
        } else {
            item.detail = Some(Arc::from(symbol.kind.display()));
        }

        // Add doc if available
        if let Some(ref doc) = symbol.doc {
            item.documentation = Some(doc.clone());
        }

        item
    }
}

/// Get completion suggestions at a position.
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
/// * `trigger` - The trigger character (if any)
///
/// # Returns
/// List of completion suggestions.
pub fn completions(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
    trigger: Option<char>,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Determine context
    let context = determine_context(index, file, line, col, trigger);

    match context {
        CompletionContext::TypeReference => {
            // Suggest definitions (types)
            for symbol in index.all_definitions() {
                if symbol.kind.is_definition() {
                    let mut item = CompletionItem::from_symbol(symbol);
                    item.sort_priority = 10;
                    items.push(item);
                }
            }
        }
        CompletionContext::MemberAccess(scope) => {
            // Suggest members of the scope
            for symbol in index.all_symbols() {
                if symbol.qualified_name.starts_with(&format!("{}::", scope)) {
                    let depth = symbol.qualified_name.matches("::").count();
                    let scope_depth = scope.matches("::").count() + 1;
                    // Only direct children
                    if depth == scope_depth + 1 {
                        items.push(CompletionItem::from_symbol(symbol));
                    }
                }
            }
        }
        CompletionContext::General => {
            // Suggest keywords
            items.extend(keyword_completions());

            // Suggest all visible symbols
            for symbol in index.all_definitions() {
                let mut item = CompletionItem::from_symbol(symbol);
                item.sort_priority = 50;
                items.push(item);
            }

            // Suggest symbols in the same file with higher priority
            for symbol in index.symbols_in_file(file) {
                let mut item = CompletionItem::from_symbol(symbol);
                item.sort_priority = 20;
                items.push(item);
            }
        }
    }

    // Sort by priority
    items.sort_by_key(|item| item.sort_priority);

    // Deduplicate by label
    items.dedup_by(|a, b| a.label == b.label);

    items
}

/// Completion context.
#[derive(Debug)]
enum CompletionContext {
    /// After `:` or `:>` — expecting a type
    TypeReference,
    /// After `::` or `.` — expecting a member
    MemberAccess(String),
    /// General completion
    General,
}

fn determine_context(
    _index: &SymbolIndex,
    _file: FileId,
    _line: u32,
    _col: u32,
    trigger: Option<char>,
) -> CompletionContext {
    // Simple heuristic based on trigger character
    match trigger {
        Some(':') => CompletionContext::TypeReference,
        Some('.') => CompletionContext::MemberAccess(String::new()),
        _ => CompletionContext::General,
    }
}

/// Get keyword completions.
fn keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        ("part def", "part def ${1:Name} {\n\t$0\n}"),
        ("part", "part ${1:name} : ${2:Type};"),
        ("action def", "action def ${1:Name} {\n\t$0\n}"),
        ("action", "action ${1:name};"),
        ("item def", "item def ${1:Name} {\n\t$0\n}"),
        ("item", "item ${1:name} : ${2:Type};"),
        ("port def", "port def ${1:Name} {\n\t$0\n}"),
        ("port", "port ${1:name} : ${2:Type};"),
        ("attribute", "attribute ${1:name} : ${2:Type};"),
        ("requirement def", "requirement def ${1:Name} {\n\t$0\n}"),
        ("requirement", "requirement ${1:name} {\n\t$0\n}"),
        ("package", "package ${1:Name} {\n\t$0\n}"),
        ("import", "import ${1:path}::*;"),
        ("specializes", ":> ${1:Type}"),
        ("redefines", ":>> ${1:name}"),
        ("subsets", ":> ${1:name}"),
    ];

    keywords
        .iter()
        .enumerate()
        .map(|(i, (label, snippet))| {
            CompletionItem::new(*label, CompletionKind::Keyword)
                .with_insert_text(*snippet)
                .with_priority(i as u32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::new_element_id;

    fn make_symbol(name: &str, qualified: &str, kind: SymbolKind) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            element_id: new_element_id(),
            kind,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 0,
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
        }
    }

    #[test]
    fn test_completion_item_from_symbol() {
        let mut symbol = make_symbol("Engine", "Vehicle::Engine", SymbolKind::PartDef);
        symbol.supertypes = vec![Arc::from("Component")];
        symbol.doc = Some(Arc::from("An engine component"));

        let item = CompletionItem::from_symbol(&symbol);

        assert_eq!(item.label.as_ref(), "Engine");
        assert_eq!(item.kind, CompletionKind::Definition);
        assert!(item.detail.as_ref().unwrap().contains("Component"));
        assert!(item.documentation.is_some());
    }

    #[test]
    fn test_keyword_completions() {
        let keywords = keyword_completions();
        assert!(!keywords.is_empty());
        assert!(keywords.iter().any(|k| k.label.as_ref() == "part def"));
        assert!(keywords.iter().any(|k| k.label.as_ref() == "package"));
    }

    #[test]
    fn test_completions_general() {
        let mut index = SymbolIndex::new();
        index.add_file(
            FileId::new(0),
            vec![
                make_symbol("Car", "Car", SymbolKind::PartDef),
                make_symbol("Engine", "Engine", SymbolKind::PartDef),
            ],
        );

        let items = completions(&index, FileId::new(0), 0, 0, None);

        // Should have keywords + symbols
        assert!(items.len() > 2);
        assert!(items.iter().any(|i| i.label.as_ref() == "Car"));
        assert!(items.iter().any(|i| i.label.as_ref() == "Engine"));
    }

    #[test]
    fn test_completion_kind_to_lsp() {
        assert_eq!(CompletionKind::Package.to_lsp(), 9);
        assert_eq!(CompletionKind::Definition.to_lsp(), 7);
        assert_eq!(CompletionKind::Keyword.to_lsp(), 14);
    }
}
