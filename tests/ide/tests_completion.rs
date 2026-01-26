//! Code completion tests for the IDE layer.
//!
//! These tests verify that completions are suggested while typing,
//! including with incomplete/partial syntax.

use crate::helpers::hir_helpers::*;
use syster::ide::AnalysisHost;
use syster::ide::completions;

// =============================================================================
// COMPLETION - INCOMPLETE SYNTAX (REAL TYPING SCENARIOS)
// =============================================================================

#[test]
fn test_completion_with_incomplete_type_reference() {
    // User is typing: "part x : " and hasn't finished yet
    // This is INCOMPLETE SYNTAX - exactly what happens in a real editor
    let incomplete_source = r#"
        part def Vehicle;
        part def Car;
        part x : 
    "#;

    // Don't use analysis_from_sysml as it asserts no errors
    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", incomplete_source);
    // We expect parse errors - that's fine for completion

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Request completions at the cursor position after ": "
    let items = completions(analysis.symbol_index(), file_id, 3, 17, None);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_ref()).collect();

    // Even with incomplete syntax, we should suggest Vehicle and Car
    assert!(
        labels.contains(&"Vehicle"),
        "Incomplete syntax should still suggest Vehicle. Got: {:?}",
        labels
    );
}

#[test]
fn test_completion_with_partial_word() {
    // User is typing: "part x : Veh" - partial word, incomplete syntax
    let incomplete_source = r#"
        part def Vehicle;
        part def Van;
        part def Car;
        part x : Veh
    "#;

    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", incomplete_source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Cursor after "Veh"
    let items = completions(analysis.symbol_index(), file_id, 4, 20, None);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_ref()).collect();

    // Should suggest Vehicle (starts with Veh), maybe not Van or Car
    assert!(
        labels.contains(&"Vehicle"),
        "Typing 'Veh' should suggest Vehicle. Got: {:?}",
        labels
    );
}

#[test]
fn test_completion_with_no_word_at_all() {
    // User just typed "part x : " with cursor right after the colon
    // Absolutely nothing typed for the type yet
    let incomplete_source = r#"
        part def Widget;
        part def Gadget;
        part myPart :
    "#;

    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", incomplete_source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Cursor right after ":"
    let items = completions(analysis.symbol_index(), file_id, 3, 21, None);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_ref()).collect();

    // Should suggest all available types
    assert!(
        labels.contains(&"Widget") && labels.contains(&"Gadget"),
        "Empty completion context should suggest all types. Got: {:?}",
        labels
    );
}

// =============================================================================
// COMPLETION - VALID SYNTAX SCENARIOS
// =============================================================================

#[test]
fn test_completion_suggests_available_types() {
    // User is defining a new part and needs to know what types exist
    let source = r#"
        part def Engine;
        part def Wheel;
        part def Chassis;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Request completions - should list all available definitions
    let items = completions(analysis.symbol_index(), file_id, 3, 0, None);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_ref()).collect();

    // All definitions should be available as completion candidates
    assert!(
        labels.contains(&"Engine"),
        "Should suggest Engine. Got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Wheel"),
        "Should suggest Wheel. Got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Chassis"),
        "Should suggest Chassis. Got: {:?}",
        labels
    );
}

#[test]
fn test_completion_from_imported_scope() {
    // User is in Consumer package and has imported Base::*
    // Completions should include Widget from the import
    let source = r#"
        package Base {
            part def Widget;
            part def Gadget;
        }
        package Consumer {
            import Base::*;
            part w : W;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Request completions at the type position in Consumer
    // where user is typing "W" for Widget
    let items = completions(analysis.symbol_index(), file_id, 7, 21, None);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_ref()).collect();

    // Widget should be available via import
    assert!(
        labels.contains(&"Widget"),
        "Should suggest Widget from import. Got: {:?}",
        labels
    );
}

#[test]
fn test_completion_has_correct_kind() {
    let source = r#"
        package Pkg {
            part def Vehicle;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let items = completions(analysis.symbol_index(), file_id, 2, 20, None);

    // Find Vehicle in completions and verify its kind
    let vehicle_item = items.iter().find(|i| i.label.as_ref() == "Vehicle");
    assert!(vehicle_item.is_some(), "Should find Vehicle in completions");
    assert_eq!(
        vehicle_item.unwrap().kind,
        syster::ide::CompletionKind::Definition,
        "Vehicle should have Definition kind"
    );
}

// =============================================================================
// COMPLETION - EDGE CASES
// =============================================================================

#[test]
fn test_completion_empty_file() {
    let source = "";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let items = completions(analysis.symbol_index(), file_id, 0, 0, None);

    // Empty file might return no completions or keywords
    // Just verify no crash
    let _ = items;
}

#[test]
fn test_completion_at_file_end() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Position at end of file
    let items = completions(analysis.symbol_index(), file_id, 0, 17, None);

    // Should not crash
    let _ = items;
}
