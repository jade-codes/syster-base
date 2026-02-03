//! Salsa database definition and queries.

use std::sync::Arc;

use crate::base::FileId;
use crate::syntax::SyntaxFile;
use crate::syntax::file::FileExtension;

use super::input::SourceRoot;
use super::symbols::{HirSymbol, extract_symbols_unified};

// ============================================================================
// INPUTS
// ============================================================================

/// Input: The raw text content of a file.
///
/// Set this explicitly when a file is opened or changed.
#[salsa::input]
pub struct FileText {
    pub file: FileId,
    #[return_ref]
    pub text: String,
}

/// Input: Configuration for the source root.
#[salsa::input]
pub struct SourceRootInput {
    #[return_ref]
    pub root: SourceRoot,
}

// ============================================================================
// DATABASE
// ============================================================================

/// The root Salsa database for HIR operations.
///
/// This provides memoization for expensive operations like parsing and
/// symbol extraction. All queries are automatically invalidated when
/// their inputs change.
#[salsa::db]
#[derive(Default, Clone)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for RootDatabase {
    fn salsa_event(&self, _event: &dyn Fn() -> salsa::Event) {
        // Default no-op implementation
    }
}

impl RootDatabase {
    /// Create a new, empty database.
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// PARSE RESULT
// ============================================================================

/// Parse result with optional AST and errors.
///
/// Uses our new SyntaxFile type from the rowan parser.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseResult {
    /// The parse was successful (no fatal errors).
    pub success: bool,
    /// Parse errors (may be present even with partial success).
    pub errors: Vec<String>,
    /// The parsed syntax file (if successful).
    pub syntax_file: Option<Arc<SyntaxFile>>,
}

// Manual Eq impl for Salsa tracking
impl Eq for ParseResult {}

impl ParseResult {
    /// Create a successful parse result.
    pub fn ok(syntax_file: SyntaxFile) -> Self {
        Self {
            success: true,
            errors: Vec::new(),
            syntax_file: Some(Arc::new(syntax_file)),
        }
    }

    /// Create a successful parse result with errors (warnings).
    pub fn ok_with_errors(syntax_file: SyntaxFile, errors: Vec<String>) -> Self {
        Self {
            success: true,
            errors,
            syntax_file: Some(Arc::new(syntax_file)),
        }
    }

    /// Create a failed parse result with errors.
    pub fn err(errors: Vec<String>) -> Self {
        Self {
            success: false,
            errors,
            syntax_file: None,
        }
    }

    /// Check if parsing succeeded.
    pub fn is_ok(&self) -> bool {
        self.success
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the syntax file if parsing succeeded.
    pub fn get_syntax_file(&self) -> Option<&SyntaxFile> {
        self.syntax_file.as_deref()
    }
}

// ============================================================================
// TRACKED QUERIES
// ============================================================================

/// Parse a file and return whether it succeeded.
///
/// This is a tracked Salsa query - results are memoized and automatically
/// invalidated when the input `FileText` changes.
#[salsa::tracked]
pub fn parse_file(db: &dyn salsa::Database, file_text: FileText) -> ParseResult {
    let text = file_text.text(db);

    // Parse using the rowan parser via SyntaxFile
    let syntax_file = SyntaxFile::new(text, FileExtension::SysML);

    if syntax_file.has_errors() {
        let errors: Vec<String> = syntax_file
            .errors()
            .iter()
            .map(|e| e.message.clone())
            .collect();
        ParseResult::ok_with_errors(syntax_file, errors)
    } else {
        ParseResult::ok(syntax_file)
    }
}

/// Extract symbols from a parsed file.
///
/// This is a pure function that takes a FileId and SyntaxFile, then returns symbols.
/// It's designed to be composable with other queries.
pub fn file_symbols(file: FileId, syntax_file: &SyntaxFile) -> Vec<HirSymbol> {
    extract_symbols_unified(file, syntax_file)
}

/// Extract symbols from a file given its text.
///
/// This is a tracked Salsa query that combines parsing + symbol extraction.
/// Results are memoized per-file.
#[salsa::tracked]
pub fn file_symbols_from_text(db: &dyn salsa::Database, file_text: FileText) -> Vec<HirSymbol> {
    let file = file_text.file(db);
    let result = parse_file(db, file_text);
    match result.syntax_file {
        Some(ref syntax_file) => extract_symbols_unified(file, syntax_file),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::symbols::SymbolKind;

    #[test]
    fn test_database_creation() {
        let _db = RootDatabase::new();
    }

    #[test]
    fn test_parse_result() {
        let syntax_file = SyntaxFile::sysml("part def Test;");

        let ok = ParseResult::ok(syntax_file.clone());
        assert!(ok.is_ok());
        assert!(!ok.has_errors());
        assert!(ok.get_syntax_file().is_some());

        let err = ParseResult::err(vec!["error".to_string()]);
        assert!(!err.is_ok());
        assert!(err.has_errors());
        assert!(err.get_syntax_file().is_none());
    }

    #[test]
    fn test_file_symbols_empty() {
        let syntax_file = SyntaxFile::sysml("");
        let file = FileId::new(0);
        let symbols = file_symbols(file, &syntax_file);
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_file_symbols_from_real_sysml() {
        let sysml = r#"
            package Vehicle {
                part def Car {
                    attribute mass : Real;
                    part engine : Engine;
                }
                
                part def Engine {
                    attribute power : Real;
                }
            }
        "#;

        let syntax_file = SyntaxFile::sysml(sysml);
        let file = FileId::new(1);
        let symbols = file_symbols(file, &syntax_file);

        // Should have symbols extracted
        assert!(!symbols.is_empty(), "Expected symbols, got empty");

        // Find the Vehicle package
        let vehicle = symbols.iter().find(|s| s.name.as_ref() == "Vehicle");
        assert!(vehicle.is_some(), "Vehicle package not found");
        assert_eq!(vehicle.unwrap().kind, SymbolKind::Package);
    }

    #[test]
    fn test_salsa_tracked_parse_query() {
        // Test that the tracked parse_file query works through the database
        let db = RootDatabase::new();

        let sysml = "part def Car;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // Call the tracked query
        let result = parse_file(&db, file_text);
        assert!(
            result.is_ok(),
            "Parse failed with errors: {:?}",
            result.errors
        );
        assert!(result.get_syntax_file().is_some());
    }

    #[test]
    fn test_salsa_tracked_symbols_query() {
        // Test that the tracked file_symbols_from_text query works
        let db = RootDatabase::new();

        let sysml = r#"
            package Test {
                part def Widget;
            }
        "#;
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // Call the tracked query
        let symbols = file_symbols_from_text(&db, file_text);

        assert!(!symbols.is_empty());
        let widget = symbols.iter().find(|s| s.name.as_ref() == "Widget");
        assert!(widget.is_some(), "Widget not found in symbols");
        assert_eq!(widget.unwrap().kind, SymbolKind::PartDefinition);
    }

    #[test]
    fn test_salsa_memoization() {
        // Test that queries are memoized (same input returns same result)
        let db = RootDatabase::new();

        let sysml = "part def MemoTest;";
        let file_text = FileText::new(&db, FileId::new(0), sysml.to_string());

        // Call twice - should be memoized
        let symbols1 = file_symbols_from_text(&db, file_text);
        let symbols2 = file_symbols_from_text(&db, file_text);

        // Results should be equal (memoized)
        assert_eq!(symbols1, symbols2);
    }
}
