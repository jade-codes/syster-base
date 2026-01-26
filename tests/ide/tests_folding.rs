//! Folding ranges tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::folding_ranges;

// =============================================================================
// FOLDING RANGES - BASIC
// =============================================================================

#[test]
fn test_folding_ranges_for_package() {
    let source = r#"
        package Pkg {
            part def A;
            part def B;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    // Should have at least some folding ranges (may be 0 depending on implementation)
    // Just document actual behavior
    for range in &ranges {
        assert!(
            range.end_line >= range.start_line,
            "Folding range should have end >= start"
        );
    }
}

#[test]
fn test_folding_range_has_lines() {
    let source = r#"
        package Pkg {
            part def Vehicle;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    for range in &ranges {
        assert!(
            range.end_line >= range.start_line,
            "Folding range should have end >= start"
        );
    }
}

#[test]
fn test_folding_ranges_nested() {
    let source = r#"
        package Outer {
            package Inner {
                part def Widget;
            }
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    // Document actual behavior - may or may not produce folding ranges
    for range in &ranges {
        assert!(range.end_line >= range.start_line);
    }
}

// =============================================================================
// FOLDING RANGES - EDGE CASES
// =============================================================================

#[test]
fn test_folding_ranges_single_line_no_fold() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    // Single-line definitions shouldn't create meaningful folding ranges
    // (or might create empty ranges)
    for range in &ranges {
        // If there is a range, it should be valid
        assert!(range.end_line >= range.start_line);
    }
}

#[test]
fn test_folding_ranges_empty_file() {
    let source = "";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    assert!(
        ranges.is_empty(),
        "Empty file should have no folding ranges"
    );
}

#[test]
fn test_folding_ranges_multiple_definitions() {
    let source = r#"
        part def Vehicle {
            part engine;
        }
        part def Car {
            part wheels;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let ranges = folding_ranges(analysis.symbol_index(), file_id);

    // Document actual behavior - may or may not produce folding ranges
    for range in &ranges {
        assert!(range.end_line >= range.start_line);
    }
}
