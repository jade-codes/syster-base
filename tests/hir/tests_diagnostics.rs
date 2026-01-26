//! Diagnostic tests for the HIR layer.
//!
//! These tests verify that semantic errors are correctly detected and reported.

use crate::helpers::hir_helpers::*;
use syster::hir::{Diagnostic, Severity, check_file};

// =============================================================================
// HELPERS
// =============================================================================

fn get_diagnostics_for_source(source: &str) -> Vec<Diagnostic> {
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    check_file(analysis.symbol_index(), file_id)
}

fn get_errors_for_source(source: &str) -> Vec<Diagnostic> {
    get_diagnostics_for_source(source)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn has_error_containing(diagnostics: &[Diagnostic], substring: &str) -> bool {
    diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.message.contains(substring))
}

// =============================================================================
// UNDEFINED REFERENCE ERRORS
// =============================================================================

#[test]
fn test_undefined_type_reference_detected() {
    let source = r#"
        package Test {
            part car : NonExistentType;
        }
    "#;

    let errors = get_errors_for_source(source);

    assert!(
        has_error_containing(&errors, "undefined")
            || has_error_containing(&errors, "NonExistentType"),
        "Should detect undefined type reference. Got: {:?}",
        errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_undefined_specialization_detected() {
    let source = r#"
        package Test {
            part def Car :> NonExistentBase;
        }
    "#;

    let errors = get_errors_for_source(source);

    assert!(
        has_error_containing(&errors, "undefined")
            || has_error_containing(&errors, "NonExistentBase"),
        "Should detect undefined specialization target. Got: {:?}",
        errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_valid_type_reference_no_error() {
    let source = r#"
        package Test {
            part def Vehicle;
            part car : Vehicle;
        }
    "#;

    let errors = get_errors_for_source(source);

    // Should not have undefined reference errors for Vehicle
    let vehicle_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.message.contains("Vehicle"))
        .collect();

    assert!(
        vehicle_errors.is_empty(),
        "Should not have errors for valid type reference. Got: {:?}",
        vehicle_errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_valid_specialization_no_error() {
    let source = r#"
        package Test {
            part def Vehicle;
            part def Car :> Vehicle;
        }
    "#;

    let errors = get_errors_for_source(source);

    // Should not have undefined reference errors for Vehicle
    let vehicle_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.message.contains("Vehicle"))
        .collect();

    assert!(
        vehicle_errors.is_empty(),
        "Should not have errors for valid specialization. Got: {:?}",
        vehicle_errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// DIAGNOSTIC SPAN TESTS
// =============================================================================

#[test]
fn test_diagnostic_has_correct_file() {
    let source = r#"
        package Test {
            part car : NonExistent;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let diagnostics = check_file(analysis.symbol_index(), file_id);

    // All diagnostics should be for the correct file
    for diag in &diagnostics {
        assert_eq!(diag.file, file_id, "Diagnostic should be for the test file");
    }
}

#[test]
fn test_diagnostic_has_position_info() {
    let source = r#"
        package Test {
            part car : NonExistent;
        }
    "#;

    let diagnostics = get_errors_for_source(source);

    // Check that diagnostics have valid position info
    for diag in &diagnostics {
        assert!(
            diag.start_line > 0 || diag.start_col > 0 || diag.end_line > 0 || diag.end_col > 0,
            "Diagnostic should have position info: {:?}",
            diag
        );
    }
}

// =============================================================================
// SEVERITY LEVEL TESTS
// =============================================================================

#[test]
fn test_undefined_reference_is_error_severity() {
    let source = r#"
        package Test {
            part car : NonExistent;
        }
    "#;

    let diagnostics = get_diagnostics_for_source(source);

    // Undefined reference should be an error, not a warning
    let undefined_diag = diagnostics
        .iter()
        .find(|d| d.message.contains("undefined") || d.message.contains("NonExistent"));

    if let Some(diag) = undefined_diag {
        assert_eq!(
            diag.severity,
            Severity::Error,
            "Undefined reference should be Error severity"
        );
    }
}

// =============================================================================
// ERROR CODE TESTS
// =============================================================================

#[test]
fn test_diagnostic_has_error_code() {
    let source = r#"
        package Test {
            part car : NonExistent;
        }
    "#;

    let diagnostics = get_errors_for_source(source);

    // Check that error diagnostics have codes
    for diag in &diagnostics {
        assert!(
            diag.code.is_some(),
            "Error diagnostic should have an error code: {}",
            diag.message
        );
    }
}

// =============================================================================
// CROSS-FILE DIAGNOSTIC TESTS
// =============================================================================

#[test]
fn test_cross_file_reference_resolved_no_error() {
    let mut host = analysis_from_sources(&[
        ("base.sysml", "package Base { part def Vehicle; }"),
        (
            "consumer.sysml",
            r#"
            package Consumer {
                import Base::*;
                part car : Vehicle;
            }
        "#,
        ),
    ]);
    let analysis = host.analysis();

    // Get diagnostics for consumer file
    let consumer_file = analysis.get_file_id("consumer.sysml").unwrap();
    let diagnostics = check_file(analysis.symbol_index(), consumer_file);

    // Should not have undefined reference errors for Vehicle
    let vehicle_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error && d.message.contains("Vehicle"))
        .collect();

    assert!(
        vehicle_errors.is_empty(),
        "Cross-file reference should resolve. Got: {:?}",
        vehicle_errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_cross_file_undefined_detected() {
    let mut host = analysis_from_sources(&[
        ("base.sysml", "package Base { part def Vehicle; }"),
        (
            "consumer.sysml",
            r#"
            package Consumer {
                // Note: no import
                part car : Vehicle;
            }
        "#,
        ),
    ]);
    let analysis = host.analysis();

    // Get diagnostics for consumer file
    let consumer_file = analysis.get_file_id("consumer.sysml").unwrap();
    let diagnostics = check_file(analysis.symbol_index(), consumer_file);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();

    // Should have undefined reference for Vehicle (no import)
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("undefined") || d.message.contains("Vehicle")),
        "Should detect undefined cross-file reference without import. Got: {:?}",
        errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// IMPORT DIAGNOSTICS
// =============================================================================

#[test]
fn test_import_nonexistent_package_no_crash() {
    // Importing a nonexistent package shouldn't crash - just produce an error or empty import
    let source = r#"
        package Consumer {
            import NonExistent::*;
        }
    "#;

    // Should not panic
    let (mut host, _) = analysis_from_sysml(source);
    let _analysis = host.analysis();
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_empty_package_no_diagnostics() {
    let source = "package Empty;";

    let diagnostics = get_diagnostics_for_source(source);

    // Empty package should have no errors
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();

    assert!(
        errors.is_empty(),
        "Empty package should have no errors. Got: {:?}",
        errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_self_referential_type_no_crash() {
    // Self-referential types shouldn't cause infinite loops
    let source = r#"
        package Test {
            part def Node {
                part children : Node;
            }
        }
    "#;

    // Should not panic or hang
    let diagnostics = get_diagnostics_for_source(source);

    // No errors expected - self-reference is valid
    let node_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error && d.message.contains("Node"))
        .collect();

    assert!(
        node_errors.is_empty(),
        "Self-referential type should be valid. Got: {:?}",
        node_errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_qualified_reference_no_false_positive() {
    let source = r#"
        package Outer {
            package Inner {
                part def Widget;
            }
            part w : Inner::Widget;
        }
    "#;

    let diagnostics = get_errors_for_source(source);

    // Qualified reference should resolve
    let widget_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("Widget"))
        .collect();

    assert!(
        widget_errors.is_empty(),
        "Qualified reference should resolve. Got: {:?}",
        widget_errors
            .iter()
            .map(|d| d.message.as_ref())
            .collect::<Vec<_>>()
    );
}
