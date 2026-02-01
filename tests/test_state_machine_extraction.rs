//! Tests for state machine reference extraction
//!
//! These tests verify that references in state machine constructs (then, first, accept)
//! are properly extracted and can be resolved for hover support.

use std::path::Path;
use syster::base::FileId;
use syster::hir::{TypeRefKind, extract_symbols_unified};
use syster::syntax::parser::parse_content;

fn extract_type_ref_targets(source: &str) -> Vec<(String, Vec<String>)> {
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    symbols
        .into_iter()
        .filter(|s| !s.type_refs.is_empty())
        .map(|s| {
            let targets: Vec<String> = s
                .type_refs
                .iter()
                .flat_map(|tr: &TypeRefKind| {
                    tr.as_refs()
                        .iter()
                        .map(|r| r.target.to_string())
                        .collect::<Vec<String>>()
                })
                .collect();
            (s.qualified_name.to_string(), targets)
        })
        .collect()
}

fn print_all_symbols(source: &str) {
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("=== All symbols ===");
    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, tr);
            }
        }
    }
}

// ==============================================================================
// PATTERN: then (transition) - 11 occurrences
// state s1 then s2;
// ==============================================================================

#[test]
fn test_then_simple_transition() {
    let source = r#"package P {
    state def Machine {
        state s1;
        state s2;
        transition s1 then s2;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // The transition should have refs to s1 and s2
    let has_s1_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"s1".to_string()));
    let has_s2_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"s2".to_string()));

    assert!(has_s1_ref, "Should have reference to 's1', got: {:?}", refs);
    assert!(has_s2_ref, "Should have reference to 's2', got: {:?}", refs);
}

#[test]
fn test_then_in_state_body() {
    let source = r#"package P {
    state def Machine {
        state idle;
        state running;
        state idle {
            then running;
        }
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // Should have ref to 'running' from inside idle's body
    let has_running_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"running".to_string()));

    assert!(
        has_running_ref,
        "Should have reference to 'running', got: {:?}",
        refs
    );
}

// ==============================================================================
// PATTERN: first (transition) - 5 occurrences
// first s1;
// ==============================================================================

#[test]
fn test_first_state() {
    let source = r#"package P {
    state def Machine {
        state idle;
        state running;
        first idle;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // Should have ref to 'idle' from the first declaration
    let has_idle_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"idle".to_string()));

    assert!(
        has_idle_ref,
        "Should have reference to 'idle', got: {:?}",
        refs
    );
}

// ==============================================================================
// PATTERN: accept (state machine) - 3 occurrences
// accept sig : Signal then targetState;
// ==============================================================================

#[test]
fn test_accept_signal() {
    let source = r#"package P {
    part def Signal;
    state def Machine {
        state idle;
        state running;
        accept sig : Signal then running;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // Should have ref to 'Signal' type and 'running' state
    let has_signal_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"Signal".to_string()));
    let has_running_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"running".to_string()));

    assert!(
        has_signal_ref,
        "Should have reference to 'Signal', got: {:?}",
        refs
    );
    assert!(
        has_running_ref,
        "Should have reference to 'running', got: {:?}",
        refs
    );
}

#[test]
fn test_accept_with_when() {
    let source = r#"package P {
    part def TempSignal { attribute temp; }
    state def Thermostat {
        attribute maxTemp;
        state normal;
        state alarm;
        accept when senseTemp.temp > maxTemp then alarm;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // Should have ref to 'alarm' state and expression refs
    let has_alarm_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"alarm".to_string()));

    assert!(
        has_alarm_ref,
        "Should have reference to 'alarm', got: {:?}",
        refs
    );
}
