//! Debug test for transition extraction
//! 
//! Checks that transition source/target references and feature chains 
//! are being extracted properly at the symbol level.

use syster::base::FileId;
use syster::hir::{extract_symbols_unified, TypeRefKind};
use syster::syntax::parser::parse_content;
use syster::syntax::SyntaxFile;
use std::path::Path;

fn print_all_symbols_and_refs(source: &str) {
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    
    println!("\n=== All symbols with type_refs ===");
    for sym in &symbols {
        println!("\n{} ({:?})", sym.qualified_name, sym.kind);
        if sym.type_refs.is_empty() {
            println!("  (no type_refs)");
        } else {
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, tr);
            }
        }
    }
}

fn get_all_ref_targets(source: &str) -> Vec<String> {
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    
    let mut targets = Vec::new();
    for sym in &symbols {
        for tr in &sym.type_refs {
            for r in tr.as_refs() {
                targets.push(r.target.to_string());
            }
        }
    }
    targets
}

#[test]
fn test_transition_targets_in_symbols() {
    let source = r#"
        state def VehicleState {
            state off;
            state on;
            transition initial then off;
            transition off_to_on first off then on;
        }
    "#;
    
    print_all_symbols_and_refs(source);
    let targets = get_all_ref_targets(source);
    
    println!("\n=== All ref targets ===");
    for t in &targets {
        println!("  {}", t);
    }
    
    // Check that transition source/target are captured
    assert!(targets.iter().any(|t| t == "initial"), "Should have 'initial' as target");
    assert!(targets.iter().any(|t| t == "off"), "Should have 'off' as target");
    assert!(targets.iter().any(|t| t == "on"), "Should have 'on' as target");
}

#[test]
fn test_perform_chain_in_symbols() {
    let source = r#"
        action def ProvidePower {
            action distributeTorque;
            action generateTorque;
        }
        part vehicle {
            perform providePower.distributeTorque;
        }
    "#;
    
    print_all_symbols_and_refs(source);
    let targets = get_all_ref_targets(source);
    
    println!("\n=== All ref targets ===");
    for t in &targets {
        println!("  {}", t);
    }
    
    // Check that feature chain parts are captured (either as full chain or parts)
    let has_chain = targets.iter().any(|t| t.contains("distributeTorque") || t.contains("providePower"));
    assert!(has_chain, "Should have feature chain reference to providePower.distributeTorque");
}

#[test]
fn test_constraint_has_name_in_symbols() {
    let source = r#"
        part def FuelTank {
            attribute fuel;
            assert constraint fuelConstraint { fuel > 0 }
        }
    "#;
    
    print_all_symbols_and_refs(source);
    
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    
    let constraint_names: Vec<_> = symbols.iter()
        .filter(|s| s.qualified_name.contains("fuelConstraint"))
        .map(|s| s.qualified_name.clone())
        .collect();
    
    println!("\n=== Constraint symbols ===");
    for n in &constraint_names {
        println!("  {}", n);
    }
    
    assert!(!constraint_names.is_empty(), "Should have symbol for 'fuelConstraint'");
}
