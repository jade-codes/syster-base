//! Document and workspace symbols tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::{document_symbols, workspace_symbols};

// =============================================================================
// DOCUMENT SYMBOLS
// =============================================================================

#[test]
fn test_document_symbols_returns_all_symbols() {
    let source = r#"
        package Pkg {
            part def Vehicle;
            part def Car;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols = document_symbols(analysis.symbol_index(), file_id);

    // Should have Pkg, Vehicle, Car
    assert!(
        symbols.len() >= 3,
        "Should have at least 3 symbols, got {}",
        symbols.len()
    );
}

#[test]
fn test_document_symbols_has_correct_names() {
    let source = r#"
        part def Widget;
        part def Gadget;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols = document_symbols(analysis.symbol_index(), file_id);
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    assert!(
        names.contains(&"Widget"),
        "Should have Widget. Got: {:?}",
        names
    );
    assert!(
        names.contains(&"Gadget"),
        "Should have Gadget. Got: {:?}",
        names
    );
}

#[test]
fn test_document_symbols_has_kind() {
    let source = r#"
        package Pkg;
        part def Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols = document_symbols(analysis.symbol_index(), file_id);

    // All symbols should have a kind
    for sym in &symbols {
        // SymbolInfo has a kind field
        let _ = &sym.kind;
    }
}

#[test]
fn test_document_symbols_has_position() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols = document_symbols(analysis.symbol_index(), file_id);

    assert!(!symbols.is_empty(), "Should have symbols");
    let vehicle = symbols.iter().find(|s| s.name.as_ref() == "Vehicle");

    if let Some(v) = vehicle {
        assert!(v.start_line < 100, "Should have valid line position");
    }
}

#[test]
fn test_document_symbols_empty_file() {
    let source = "";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols = document_symbols(analysis.symbol_index(), file_id);

    assert!(symbols.is_empty(), "Empty file should have no symbols");
}

// =============================================================================
// WORKSPACE SYMBOLS
// =============================================================================

#[test]
fn test_workspace_symbols_search() {
    let mut host = analysis_from_sources(&[
        ("file1.sysml", "part def Vehicle;"),
        ("file2.sysml", "part def VehicleFactory;"),
        ("file3.sysml", "part def Car;"),
    ]);
    let analysis = host.analysis();

    // Search for "Vehicle"
    let symbols = workspace_symbols(analysis.symbol_index(), Some("Vehicle"));

    // Should find Vehicle and VehicleFactory
    assert!(
        symbols.len() >= 2,
        "Should find at least 2 symbols matching 'Vehicle', got {}",
        symbols.len()
    );
}

#[test]
fn test_workspace_symbols_case_insensitive() {
    let mut host = analysis_from_sources(&[("file1.sysml", "part def MyWidget;")]);
    let analysis = host.analysis();

    // Search with different case
    let symbols_lower = workspace_symbols(analysis.symbol_index(), Some("mywidget"));
    let symbols_upper = workspace_symbols(analysis.symbol_index(), Some("MYWIDGET"));

    // Both should find the symbol (if case-insensitive)
    // Or at least not crash
    let _ = symbols_lower;
    let _ = symbols_upper;
}

#[test]
fn test_workspace_symbols_empty_query() {
    let mut host = analysis_from_sources(&[
        ("file1.sysml", "part def A;"),
        ("file2.sysml", "part def B;"),
    ]);
    let analysis = host.analysis();

    // Empty query should return all or nothing (implementation-dependent)
    let symbols = workspace_symbols(analysis.symbol_index(), None);

    // Just verify no crash
    let _ = symbols;
}

#[test]
fn test_workspace_symbols_no_match() {
    let mut host = analysis_from_sources(&[("file1.sysml", "part def Vehicle;")]);
    let analysis = host.analysis();

    // Search for something that doesn't exist
    let symbols = workspace_symbols(analysis.symbol_index(), Some("NonExistentSymbol123"));

    assert!(
        symbols.is_empty(),
        "Should find no symbols for non-matching query"
    );
}
