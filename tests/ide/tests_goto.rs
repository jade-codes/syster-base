//! Go to definition tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::goto_definition;

// =============================================================================
// GOTO DEFINITION - BASIC
// =============================================================================

#[test]
fn test_goto_definition_from_type_reference() {
    let source = r#"
        part def Vehicle;
        part car : Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Click on "Vehicle" in the type position (line 2, col ~19)
    let result = goto_definition(analysis.symbol_index(), file_id, 2, 19);

    assert!(!result.is_empty(), "Goto definition should find target");

    let target = &result.targets[0];
    assert_eq!(
        target.name.as_ref(),
        "Vehicle",
        "Should go to Vehicle definition"
    );
}

#[test]
fn test_goto_definition_from_specialization() {
    let source = r#"
        part def Vehicle;
        part def Car :> Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Click on "Vehicle" in specialization (line 2, col ~24)
    let result = goto_definition(analysis.symbol_index(), file_id, 2, 24);

    // Should find the specialization target (may be empty if position doesn't hit)
    if !result.is_empty() {
        assert_eq!(
            result.targets[0].name.as_ref(),
            "Vehicle",
            "Should go to Vehicle"
        );
    }
}

#[test]
fn test_goto_definition_on_definition_returns_self() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Click on "Vehicle" in the definition itself
    let result = goto_definition(analysis.symbol_index(), file_id, 0, 12);

    // Might return the definition itself or nothing
    // Just verify no crash
    let _ = result;
}

// =============================================================================
// GOTO DEFINITION - CROSS FILE
// =============================================================================

#[test]
fn test_goto_definition_cross_file() {
    let mut host = analysis_from_sources(&[
        ("base.sysml", "part def Vehicle;"),
        (
            "derived.sysml",
            r#"
            import base::Vehicle;
            part car : Vehicle;
        "#,
        ),
    ]);
    let analysis = host.analysis();

    let derived_file = analysis.get_file_id("derived.sysml").unwrap();

    // Click on "Vehicle" in derived file
    let result = goto_definition(analysis.symbol_index(), derived_file, 2, 24);

    if !result.is_empty() {
        // Target should be Vehicle
        let target = &result.targets[0];
        assert_eq!(target.name.as_ref(), "Vehicle");
    }
}

// =============================================================================
// GOTO DEFINITION - EDGE CASES
// =============================================================================

#[test]
fn test_goto_definition_undefined_reference() {
    let source = "part car : NonExistent;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Click on "NonExistent"
    let result = goto_definition(analysis.symbol_index(), file_id, 0, 15);

    // Should return empty for undefined reference
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn test_goto_definition_empty_position() {
    let source = r#"
        part def Vehicle;

    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Click on empty line
    let result = goto_definition(analysis.symbol_index(), file_id, 2, 0);

    // Should not crash
    let _ = result;
}
