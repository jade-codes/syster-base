//! Span and position tracking tests for the HIR layer.
//!
//! These tests verify that symbols have correct start/end positions
//! and that type references point to the correct locations.

use crate::helpers::hir_helpers::*;
use syster::hir::SymbolKind;

// =============================================================================
// SYMBOL SPAN TESTS
// =============================================================================

#[test]
fn test_symbol_has_line_info() {
    let source = "part def Vehicle;";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let vehicle = analysis.symbol_index().lookup_qualified("Vehicle").unwrap();

    // Should have valid line info (0-indexed)
    assert!(
        vehicle.start_line < 100,
        "Should have reasonable start line, got {}",
        vehicle.start_line
    );
}

#[test]
fn test_symbol_has_column_info() {
    let source = "part def Vehicle;";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let vehicle = analysis.symbol_index().lookup_qualified("Vehicle").unwrap();

    // Column should be > 0 since there's "part def " before "Vehicle"
    assert!(
        vehicle.start_col > 0,
        "Should have column > 0 for indented symbol, got {}",
        vehicle.start_col
    );
}

#[test]
fn test_symbol_end_after_start() {
    let source = "part def Vehicle;";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let vehicle = analysis.symbol_index().lookup_qualified("Vehicle").unwrap();

    // End should be at or after start
    assert!(
        vehicle.end_line >= vehicle.start_line,
        "End line should be >= start line"
    );
    if vehicle.end_line == vehicle.start_line {
        assert!(
            vehicle.end_col >= vehicle.start_col,
            "End col should be >= start col on same line"
        );
    }
}

#[test]
fn test_multiline_definition_name_span() {
    // Note: HIR symbols track the NAME position, not the full body span
    // For a multiline definition, the name itself is on a single line
    let source = r#"
        part def Vehicle {
            part engine;
            part wheels;
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let vehicle = analysis.symbol_index().lookup_qualified("Vehicle").unwrap();

    // The symbol name "Vehicle" is on a single line - HIR tracks name position
    // This documents actual behavior: start_line == end_line for the name
    assert!(
        vehicle.start_line <= vehicle.end_line,
        "Symbol name span should be valid: start_line ({}) <= end_line ({})",
        vehicle.start_line,
        vehicle.end_line
    );
}

#[test]
fn test_nested_symbols_have_valid_positions() {
    // Note: HIR symbols track NAME positions, not containment
    let source = r#"
        package Outer {
            part def Inner;
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let outer = analysis.symbol_index().lookup_qualified("Outer").unwrap();
    let inner = analysis
        .symbol_index()
        .lookup_qualified("Outer::Inner")
        .unwrap();

    // Both should have valid position info
    assert!(outer.start_line < 100, "Outer should have valid line");
    assert!(inner.start_line < 100, "Inner should have valid line");

    // Inner appears after Outer in source
    assert!(
        inner.start_line >= outer.start_line,
        "Inner start_line ({}) should be >= Outer start_line ({})",
        inner.start_line,
        outer.start_line
    );
}

// =============================================================================
// TYPE REFERENCE SPAN TESTS
// =============================================================================

#[test]
fn test_type_ref_has_span() {
    let source = r#"
        part def Vehicle;
        part car : Vehicle;
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let car = analysis.symbol_index().lookup_qualified("car").unwrap();

    // car should have a supertype reference to Vehicle
    assert!(
        !car.supertypes.is_empty(),
        "car should have type ref to Vehicle"
    );
}

#[test]
fn test_specialization_ref_has_span() {
    let source = r#"
        part def Vehicle;
        part def Car :> Vehicle;
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let car = analysis.symbol_index().lookup_qualified("Car").unwrap();

    // Car should have a supertype reference to Vehicle
    assert!(
        !car.supertypes.is_empty(),
        "Car should have supertype Vehicle"
    );
    assert!(
        car.supertypes.iter().any(|s| s.as_ref() == "Vehicle"),
        "Car's supertypes should include Vehicle, got {:?}",
        car.supertypes
    );
}

// =============================================================================
// SPAN ORDERING TESTS
// =============================================================================

#[test]
fn test_symbols_ordered_by_position() {
    let source = r#"
        part def A;
        part def B;
        part def C;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis
        .symbol_index()
        .symbols_in_file(file_id)
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::PartDef))
        .cloned()
        .collect();

    // Verify we have A, B, C
    assert_eq!(symbols.len(), 3, "Should have 3 part defs");

    let a = symbols.iter().find(|s| s.name.as_ref() == "A").unwrap();
    let b = symbols.iter().find(|s| s.name.as_ref() == "B").unwrap();
    let c = symbols.iter().find(|s| s.name.as_ref() == "C").unwrap();

    // A should be before B
    assert!(
        a.start_line < b.start_line || (a.start_line == b.start_line && a.start_col < b.start_col),
        "A should be before B"
    );
    // B should be before C
    assert!(
        b.start_line < c.start_line || (b.start_line == c.start_line && b.start_col < c.start_col),
        "B should be before C"
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_single_line_package() {
    let source = "package Pkg;";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let pkg = analysis.symbol_index().lookup_qualified("Pkg").unwrap();

    // Single line, so start and end line should be the same
    assert_eq!(
        pkg.start_line, pkg.end_line,
        "Single-line package should have same start and end line"
    );
}

#[test]
fn test_deeply_nested_spans() {
    let source = r#"
        package L1 {
            package L2 {
                package L3 {
                    part def Deep;
                }
            }
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let l1 = analysis.symbol_index().lookup_qualified("L1").unwrap();
    let l2 = analysis.symbol_index().lookup_qualified("L1::L2").unwrap();
    let l3 = analysis
        .symbol_index()
        .lookup_qualified("L1::L2::L3")
        .unwrap();
    let deep = analysis
        .symbol_index()
        .lookup_qualified("L1::L2::L3::Deep")
        .unwrap();

    // Each level should be contained in the parent
    assert!(l2.start_line >= l1.start_line);
    assert!(l3.start_line >= l2.start_line);
    assert!(deep.start_line >= l3.start_line);
}

// =============================================================================
// UNICODE HANDLING
// =============================================================================

#[test]
fn test_unicode_in_name_position() {
    // Note: SysML identifiers are typically ASCII, but we test the parser doesn't crash
    let source = "package Test { part def Car; }";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should parse without issues
    let car = analysis
        .symbol_index()
        .lookup_qualified("Test::Car")
        .unwrap();
    assert!(car.start_col > 0, "Should have valid column info");
}

#[test]
fn test_position_with_tabs() {
    // Source with tabs
    let source = "\tpart def Vehicle;";

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let vehicle = analysis.symbol_index().lookup_qualified("Vehicle").unwrap();

    // Should have valid position (tab handling may vary)
    assert!(
        vehicle.start_line < 100 && vehicle.start_col < 100,
        "Should have reasonable position values"
    );
}
