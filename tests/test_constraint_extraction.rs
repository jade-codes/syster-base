//! Tests for constraint reference extraction
//!
//! These tests verify that references in constraint constructs (assert, assume)
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
// PATTERN: assert constraint
// assert constraint speedLimit { speed <= maxSpeed }
// ==============================================================================

#[test]
fn test_assert_constraint() {
    let source = r#"package P {
    constraint def SpeedLimit {
        attribute speed;
        attribute maxSpeed;
        constraint { speed <= maxSpeed }
    }
    part def Vehicle {
        attribute currentSpeed;
        attribute maxAllowed;
        assert constraint limit : SpeedLimit {
            speed = currentSpeed;
            maxSpeed = maxAllowed;
        }
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // The assert should have ref to SpeedLimit
    let has_speedlimit_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"SpeedLimit".to_string()));

    assert!(
        has_speedlimit_ref,
        "Should have reference to 'SpeedLimit', got: {:?}",
        refs
    );
}

#[test]
fn test_assume_constraint() {
    let source = r#"package P {
    constraint def SafetyMargin {
        attribute margin;
        constraint { margin > 0 }
    }
    requirement def SafeReq {
        assume constraint safety : SafetyMargin;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // The assume should have ref to SafetyMargin
    let has_margin_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"SafetyMargin".to_string()));

    assert!(
        has_margin_ref,
        "Should have reference to 'SafetyMargin', got: {:?}",
        refs
    );
}

#[test]
fn test_require_constraint() {
    let source = r#"package P {
    constraint def ValidRange {
        attribute value;
        attribute min;
        attribute max;
        constraint { value >= min && value <= max }
    }
    requirement def RangeReq {
        require constraint range : ValidRange;
    }
}"#;

    print_all_symbols(source);
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // The require should have ref to ValidRange
    let has_range_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"ValidRange".to_string()));

    assert!(
        has_range_ref,
        "Should have reference to 'ValidRange', got: {:?}",
        refs
    );
}
