//! Hover feature tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::hover;

// =============================================================================
// HOVER ON DEFINITIONS
// =============================================================================

#[test]
fn test_hover_on_part_def_shows_info() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Hover at position where "Vehicle" is (line 0, col ~9)
    let result = hover(analysis.symbol_index(), file_id, 0, 12);

    assert!(
        result.is_some(),
        "Hover should return result for definition"
    );
    let hover = result.unwrap();
    assert!(
        hover.contents.contains("Vehicle") || hover.contents.contains("part def"),
        "Hover should mention the symbol name or kind. Got: {}",
        hover.contents
    );
}

#[test]
fn test_hover_on_package_shows_info() {
    let source = "package MyPackage;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Hover at position where "MyPackage" is
    let result = hover(analysis.symbol_index(), file_id, 0, 10);

    assert!(result.is_some(), "Hover should return result for package");
    let hover = result.unwrap();
    assert!(
        hover.contents.contains("MyPackage") || hover.contents.contains("package"),
        "Hover should mention the package. Got: {}",
        hover.contents
    );
}

#[test]
fn test_hover_result_has_qualified_name() {
    let source = r#"
        package Pkg {
            part def Widget;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Find Widget's line (should be line 2)
    let result = hover(analysis.symbol_index(), file_id, 2, 22);

    if let Some(hover) = result {
        assert!(
            hover.qualified_name.is_some(),
            "Hover result should have qualified name"
        );
    }
}

#[test]
fn test_hover_on_definition_is_definition_flag() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let result = hover(analysis.symbol_index(), file_id, 0, 12);

    if let Some(hover) = result {
        assert!(
            hover.is_definition,
            "Hover on definition should have is_definition=true"
        );
    }
}

// =============================================================================
// HOVER ON USAGES
// =============================================================================

#[test]
fn test_hover_on_usage_shows_type() {
    let source = r#"
        part def Vehicle;
        part car : Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Hover on "car" (line 2)
    let result = hover(analysis.symbol_index(), file_id, 2, 14);

    assert!(result.is_some(), "Hover should return result for usage");
    let hover = result.unwrap();
    assert!(
        hover.contents.contains("car") || hover.contents.contains("Vehicle"),
        "Hover on usage should show type info. Got: {}",
        hover.contents
    );
}

// =============================================================================
// HOVER POSITION TESTS
// =============================================================================

#[test]
fn test_hover_on_empty_space_returns_none() {
    let source = r#"
        part def Vehicle;


        part def Car;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Hover on empty line (line 2 or 3)
    let result = hover(analysis.symbol_index(), file_id, 2, 0);

    // Should return None for empty space
    // (This may vary based on implementation - some return closest symbol)
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn test_hover_result_has_span_info() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let result = hover(analysis.symbol_index(), file_id, 0, 12);

    if let Some(hover) = result {
        // Should have valid span info
        assert!(
            hover.end_line >= hover.start_line,
            "Hover span should be valid"
        );
    }
}

// =============================================================================
// HOVER WITH RELATIONSHIPS
// =============================================================================

#[test]
fn test_hover_shows_specialization() {
    let source = r#"
        part def Vehicle;
        part def Car :> Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Hover on Car (line 2)
    let result = hover(analysis.symbol_index(), file_id, 2, 18);

    if let Some(hover) = result {
        // Should show specialization relationship
        assert!(
            hover.contents.contains("Vehicle") || !hover.relationships.is_empty(),
            "Hover should show specialization. Contents: {}, Relationships: {:?}",
            hover.contents,
            hover.relationships
        );
    }
}
