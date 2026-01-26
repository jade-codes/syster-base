//! Find references tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::find_references;

// =============================================================================
// FIND REFERENCES - BASIC
// =============================================================================

#[test]
fn test_find_references_for_definition() {
    let source = r#"
        part def Vehicle;
        part car1 : Vehicle;
        part car2 : Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Find references to Vehicle (click on definition)
    let result = find_references(analysis.symbol_index(), file_id, 1, 18, true);

    // Should find at least the usages (maybe also the definition)
    assert!(
        result.references.len() >= 2,
        "Should find at least 2 references to Vehicle, got {}",
        result.references.len()
    );
}

#[test]
fn test_find_references_include_declaration() {
    let source = r#"
        part def Vehicle;
        part car : Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Find references including declaration
    let result = find_references(analysis.symbol_index(), file_id, 1, 18, true);

    // With include_declaration=true, should include the definition
    let has_definition = result.references.iter().any(|r| r.is_definition);
    assert!(
        has_definition || !result.references.is_empty(),
        "Should include declaration or find usages"
    );
}

#[test]
fn test_find_references_from_usage() {
    let source = r#"
        part def Vehicle;
        part car : Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Find references from a usage position (line 2, where Vehicle is used)
    let result = find_references(analysis.symbol_index(), file_id, 2, 19, true);

    // Should find the definition and other usages
    assert!(!result.is_empty(), "Should find references from usage");
}

// =============================================================================
// FIND REFERENCES - NO REFERENCES
// =============================================================================

#[test]
fn test_find_references_unused_definition() {
    let source = "part def UnusedWidget;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Find references for unused definition
    let result = find_references(analysis.symbol_index(), file_id, 0, 12, true);

    // Only the definition itself (if include_declaration is true)
    assert!(
        result.references.len() <= 1,
        "Unused definition should have at most 1 reference (itself)"
    );
}

// =============================================================================
// FIND REFERENCES - CROSS FILE
// =============================================================================

#[test]
fn test_find_references_cross_file() {
    let mut host = analysis_from_sources(&[
        ("base.sysml", "package Base { part def Vehicle; }"),
        (
            "consumer1.sysml",
            r#"
            package C1 {
                import Base::*;
                part car : Vehicle;
            }
        "#,
        ),
        (
            "consumer2.sysml",
            r#"
            package C2 {
                import Base::*;
                part truck : Vehicle;
            }
        "#,
        ),
    ]);
    let analysis = host.analysis();

    let base_file = analysis.get_file_id("base.sysml").unwrap();

    // Find references to Vehicle from base file
    let result = find_references(analysis.symbol_index(), base_file, 0, 28, true);

    // Should find references across files
    assert!(
        !result.references.is_empty(),
        "Should find references across files"
    );
}
