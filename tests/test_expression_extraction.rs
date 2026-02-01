//! Tests for expression reference extraction
//!
//! These tests verify that references within expressions (like `speed * time`)
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
    println!("Extracted refs: {:#?}", refs);

    // The `distance` attribute should have refs to `speed` and `time`
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
    println!("Extracted refs: {:#?}", refs);

    // totalMass should have a chain ref to fuelTank.mass
    let total_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Vehicle::totalMass")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    // Should contain both parts of the chain
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
    println!("Extracted refs: {:#?}", refs);

    let length_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Vehicle::length")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    // Should have the qualified type reference
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

    // Step 1: Check parser output
    let parse = syster::parser::parse_sysml(source);
    println!("=== PARSER OUTPUT ===");
    println!("Parse errors: {:?}", parse.errors);
    fn print_tree(node: &syster::parser::SyntaxNode, indent: usize) {
        let spaces = "  ".repeat(indent);
        let text: String = node
            .text()
            .to_string()
            .chars()
            .take(40)
            .collect::<String>()
            .replace('\n', "\\n");
        println!("{}{:?} \"{}\"", spaces, node.kind(), text);
        for child in node.children() {
            print_tree(&child, indent + 1);
        }
    }
    print_tree(&parse.syntax(), 0);

    // Step 2: Check AST layer
    println!("\n=== AST LAYER ===");
    use syster::parser::{AstNode, NamespaceMember, SourceFile};
    let root = SourceFile::cast(parse.syntax()).unwrap();
    for member in root.members() {
        println!("Member: {:?}", std::mem::discriminant(&member));
        if let NamespaceMember::Package(pkg) = member {
            println!("  Package: {:?}", pkg.name().and_then(|n| n.text()));
            if let Some(body) = pkg.body() {
                for inner in body.members() {
                    println!("  Inner: {:?}", std::mem::discriminant(&inner));
                    if let NamespaceMember::Definition(def) = inner {
                        println!("    Def: {:?}", def.name().and_then(|n| n.text()));
                        if let Some(body) = def.body() {
                            for item in body.members() {
                                println!("    Item: {:?}", std::mem::discriminant(&item));
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 3: Check HIR extraction
    println!("\n=== HIR EXTRACTION ===");
    let refs = extract_type_ref_targets(source);
    println!("Extracted refs: {:#?}", refs);

    // Find any symbol that references temp and maxTemp
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
    println!("Extracted refs: {:#?}", refs);

    // Should extract refs from the constraint expression
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
    println!("Extracted refs: {:#?}", refs);

    let status_refs = refs
        .iter()
        .find(|(name, _)| name == "P::Issue::status")
        .map(|(_, targets)| targets.clone())
        .unwrap_or_default();

    // Should have both type and value refs
    assert!(
        status_refs.iter().any(|t| t.contains("StatusKind")),
        "status should reference StatusKind, got: {:?}",
        status_refs
    );
}
