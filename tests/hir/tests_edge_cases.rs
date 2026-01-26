//! Edge cases and error recovery tests for the HIR layer.
//!
//! These tests verify that the system handles unusual or error conditions
//! gracefully without crashing.

use crate::helpers::hir_helpers::*;
use syster::ide::AnalysisHost;

// =============================================================================
// EMPTY FILE HANDLING
// =============================================================================

#[test]
fn test_empty_file_no_crash() {
    let source = "";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should have no symbols
    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    assert!(symbols.is_empty(), "Empty file should have no symbols");
}

#[test]
fn test_whitespace_only_file() {
    let source = "   \n\n\t\t\n   ";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    assert!(
        symbols.is_empty(),
        "Whitespace-only file should have no symbols"
    );
}

#[test]
fn test_comment_only_file() {
    let source = "// This is a comment\n/* Block comment */";

    // Comments might cause parse errors, that's OK
    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);

    // Should not crash
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let _symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
}

// =============================================================================
// PARTIAL PARSE / ERROR RECOVERY
// =============================================================================

#[test]
fn test_partial_parse_recovers_valid_symbols() {
    // First definition is valid, second has an error
    let source = r#"
        part def ValidDef;
        part def 
        part def AnotherValid;
    "#;

    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", source);

    // Should have parse errors
    assert!(!errors.is_empty(), "Should have parse errors");

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Should still extract ValidDef at minimum
    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    assert!(
        names.contains(&"ValidDef"),
        "Should recover ValidDef before error. Got: {:?}",
        names
    );
}

#[test]
fn test_syntax_error_does_not_crash() {
    let source = "this is not valid sysml at all {{{{";

    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);

    // Should not crash
    let analysis = host.analysis();
    let _file_id = analysis.get_file_id("test.sysml");
}

#[test]
fn test_unclosed_brace_does_not_crash() {
    let source = r#"
        package Pkg {
            part def Vehicle;
        // Missing closing brace
    "#;

    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Should still extract what it can
    let _symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
}

// =============================================================================
// DEEPLY NESTED STRUCTURES
// =============================================================================

#[test]
fn test_deeply_nested_packages() {
    // 20 levels of nesting
    let mut source = String::new();
    for i in 0..20 {
        source.push_str(&format!("package L{} {{\n", i));
    }
    source.push_str("part def Deep;\n");
    for _ in 0..20 {
        source.push_str("}\n");
    }

    let (mut host, file_id) = analysis_from_sysml(&source);
    let analysis = host.analysis();

    // Should extract the deeply nested symbol
    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    let has_deep = symbols.iter().any(|s| s.name.as_ref() == "Deep");

    assert!(has_deep, "Should extract deeply nested symbol");
}

#[test]
fn test_wide_file_many_siblings() {
    // 100 sibling definitions
    let mut source = String::from("package Wide {\n");
    for i in 0..100 {
        source.push_str(&format!("    part def Item{};\n", i));
    }
    source.push_str("}\n");

    let (mut host, file_id) = analysis_from_sysml(&source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);

    // Should have Wide + 100 items = 101 symbols
    assert!(
        symbols.len() >= 100,
        "Should extract all 100 sibling definitions, got {}",
        symbols.len()
    );
}

// =============================================================================
// SPECIAL CHARACTERS
// =============================================================================

#[test]
fn test_underscore_in_names() {
    // Note: SysML identifiers follow specific rules
    // Underscores in the middle are typically allowed
    let source = r#"
        part def my_vehicle;
        part def vehicle_v2;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    assert!(
        names.contains(&"my_vehicle"),
        "Should handle underscores in names"
    );
    assert!(
        names.contains(&"vehicle_v2"),
        "Should handle underscore before number"
    );
}

#[test]
fn test_numbers_in_names() {
    let source = r#"
        part def Item1;
        part def Item2;
        part def V8Engine;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    let names: Vec<_> = symbols.iter().map(|s| s.name.as_ref()).collect();

    assert!(names.contains(&"Item1"), "Should handle numbers in names");
    assert!(
        names.contains(&"V8Engine"),
        "Should handle numbers in middle"
    );
}

// =============================================================================
// LARGE CONTENT
// =============================================================================

#[test]
fn test_long_name() {
    let long_name = "A".repeat(200);
    let source = format!("part def {};", long_name);

    let (mut host, file_id) = analysis_from_sysml(&source);
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);

    assert!(!symbols.is_empty(), "Should handle long names");
    assert_eq!(symbols[0].name.as_ref(), long_name);
}

#[test]
fn test_many_type_references() {
    // A definition with many type references
    let source = r#"
        part def A;
        part def B;
        part def C;
        part def D;
        part def E;
        part def Multi :> A, B, C, D, E;
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let multi = analysis.symbol_index().lookup_qualified("Multi").unwrap();

    // Should have multiple supertypes
    assert!(
        multi.supertypes.len() >= 5,
        "Should capture all supertypes, got {}",
        multi.supertypes.len()
    );
}

// =============================================================================
// DUPLICATE HANDLING
// =============================================================================

#[test]
fn test_duplicate_names_same_scope_no_crash() {
    // Same name defined twice - should not crash
    let source = r#"
        package Pkg {
            part def Duplicate;
            part def Duplicate;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should not crash, and should have symbols
    let symbols: Vec<_> = analysis.symbol_index().symbols_in_file(file_id);
    assert!(!symbols.is_empty());
}

#[test]
fn test_same_name_different_scopes() {
    let source = r#"
        package A {
            part def Widget;
        }
        package B {
            part def Widget;
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Both should exist with different qualified names
    let a_widget = analysis.symbol_index().lookup_qualified("A::Widget");
    let b_widget = analysis.symbol_index().lookup_qualified("B::Widget");

    assert!(a_widget.is_some(), "A::Widget should exist");
    assert!(b_widget.is_some(), "B::Widget should exist");
}

// =============================================================================
// CIRCULAR REFERENCES
// =============================================================================

#[test]
fn test_self_referential_no_crash() {
    let source = r#"
        part def Node {
            part children : Node;
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let node = analysis.symbol_index().lookup_qualified("Node");
    assert!(
        node.is_some(),
        "Self-referential type should be extractable"
    );
}

#[test]
fn test_mutual_references_no_crash() {
    let source = r#"
        part def A {
            part b : B;
        }
        part def B {
            part a : A;
        }
    "#;

    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert!(analysis.symbol_index().lookup_qualified("A").is_some());
    assert!(analysis.symbol_index().lookup_qualified("B").is_some());
}
