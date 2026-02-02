//! Tests for expression reference extraction
//!
//! These tests verify that references within expressions (like `speed * time`)
//! are properly extracted and can be resolved for hover support.

use std::path::Path;
use syster::base::FileId;
use syster::hir::{extract_symbols_unified, TypeRefKind};
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

#[test]
fn test_expression_simple_multiplication() {
    let source = r#"package P {
    part def Vehicle {
        attribute speed;
        attribute time;
        attribute distance = speed * time;
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let distance_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Vehicle::distance")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    assert!(
        distance_refs.contains(&"speed".to_string()),
        "distance should reference 'speed', got: {:?}",
        distance_refs
    );
    assert!(
        distance_refs.contains(&"time".to_string()),
        "distance should reference 'time', got: {:?}",
        distance_refs
    );
}

#[test]
fn test_expression_feature_chain() {
    let source = r#"package P {
    part def FuelTank { attribute mass; }
    part def Vehicle {
        part fuelTank : FuelTank;
        attribute totalMass = fuelTank.mass + 100;
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let total_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Vehicle::totalMass")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    assert!(
        total_refs.contains(&"fuelTank".to_string())
            || total_refs.contains(&"fuelTank.mass".to_string()),
        "totalMass should reference 'fuelTank' or chain, got: {:?}",
        total_refs
    );
}

#[test]
fn test_expression_qualified_name() {
    let source = r#"package P {
    package Units { attribute def Length; }
    part def Vehicle {
        attribute length : Units::Length = 5;
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let length_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Vehicle::length")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    assert!(
        length_refs
            .iter()
            .any(|t| t.contains("Units") || t.contains("Length")),
        "length should reference Units::Length, got: {:?}",
        length_refs
    );
}

#[test]
fn test_expression_comparison() {
    let source = r#"package P {
    part def Vehicle {
        attribute temp;
        attribute maxTemp;
        constraint { temp < maxTemp }
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let has_temp_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"temp".to_string()));
    let has_max_ref = refs
        .iter()
        .any(|(_, targets)| targets.contains(&"maxTemp".to_string()));

    assert!(
        has_temp_ref,
        "Should have reference to 'temp', got: {:?}",
        refs
    );
    assert!(
        has_max_ref,
        "Should have reference to 'maxTemp', got: {:?}",
        refs
    );
}

#[test]
fn test_expression_in_constraint_block() {
    let source = r#"package P {
    constraint def SpeedLimit {
        attribute maxSpeed;
        attribute currentSpeed;
        constraint { currentSpeed <= maxSpeed }
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let has_speed_refs = refs.iter().any(|(_, targets)| {
        targets.contains(&"currentSpeed".to_string()) || targets.contains(&"maxSpeed".to_string())
    });

    assert!(
        has_speed_refs,
        "Should extract expression refs from constraint, got: {:?}",
        refs
    );
}

#[test]
fn test_expression_enum_literal() {
    let source = r#"package P {
    enum def StatusKind { open; closed; }
    part def Issue {
        attribute status : StatusKind = StatusKind::open;
    }
}"#;

    let refs = extract_type_ref_targets(source);

    let status_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Issue::status")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    assert!(
        status_refs.iter().any(|t| t.contains("StatusKind")),
        "status should reference StatusKind, got: {:?}",
        status_refs
    );
}
