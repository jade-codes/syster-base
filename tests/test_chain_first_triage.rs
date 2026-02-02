//! Bottom-up triage for CHAIN_FIRST hover failures
//!
//! Failures to investigate:
//! 1. Line 556: 'mop' in: #mop attribute mass redefines mass=dryMass+cargoMass+fuelTank.fuel.fuelMass
//! 2. Line 654: 'generateTorque' in: subject generateTorque redefines generateTorque = vehicle_b.engine.generateTorque

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::parser::parse_sysml;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

fn print_ast(source: &str) {
    let parsed = parse_sysml(source);
    println!("AST:\n{:#?}", parsed.syntax());
}

/// Test: #mop prefix metadata in expression context
/// `#mop attribute mass redefines mass=dryMass+cargoMass+fuelTank.fuel.fuelMass`
/// The `mop` after # should resolve to the metadata def
#[test]
fn test_chain_first_prefix_metadata_in_redefines() {
    let source = r#"
package Test {
    metadata def mop;
    part def Vehicle {
        attribute mass : Real;
    }
    part vehicle : Vehicle {
        #mop attribute mass redefines mass = 100;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    println!("\n=== Test: #mop prefix metadata in redefines ===");
    println!("\nAST:");
    print_ast(source);

    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }

    // Line 7 (0-indexed) is: `#mop attribute mass redefines mass = 100;`
    println!("\nHover scan on redefines line (line 7):");
    for col in 8..50 {
        if let Some(hover) = analysis.hover(file_id, 7, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }

    // mop should resolve at cols 9-11 (the identifier after #)
    let hover = analysis.hover(file_id, 7, 9);
    assert!(
        hover.is_some() && hover.as_ref().unwrap().qualified_name.as_ref().map(|q| q.contains("mop")).unwrap_or(false),
        "mop in #mop should resolve to metadata def"
    );
}

/// Test: subject with redefines and expression chain
/// `subject generateTorque redefines generateTorque = vehicle_b.engine.generateTorque`
#[test]
fn test_chain_first_subject_redefines_chain() {
    let source = r#"
package Test {
    part def Engine {
        calc generateTorque : Real;
    }
    part def Vehicle {
        part engine : Engine;
    }
    
    requirement def TorqueReq {
        subject generateTorque : Real;
    }
    
    part vehicle_b : Vehicle;
    
    requirement torqueReq : TorqueReq {
        subject generateTorque redefines generateTorque = vehicle_b.engine.generateTorque;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    println!("\n=== Test: subject redefines with chain ===");
    println!("\nAST:");
    print_ast(source);

    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }

    // Line 16 (0-indexed) is the subject line
    println!("\nHover scan on subject line (line 16):");
    for col in 8..90 {
        if let Some(hover) = analysis.hover(file_id, 16, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }

    // The first generateTorque (being defined) should resolve
    let hover = analysis.hover(file_id, 16, 16);
    assert!(
        hover.is_some(),
        "First generateTorque (subject name) should resolve"
    );
}

/// Test: expression chain first element
/// In `dryMass+cargoMass+fuelTank.fuel.fuelMass`, `dryMass` and `cargoMass` should resolve
#[test]
fn test_chain_first_expression_refs() {
    let source = r#"
package Test {
    part def Vehicle {
        attribute dryMass : Real;
        attribute cargoMass : Real;
        part fuelTank {
            part fuel {
                attribute fuelMass : Real;
            }
        }
        attribute totalMass : Real = dryMass + cargoMass + fuelTank.fuel.fuelMass;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    println!("\n=== Test: expression chain first elements ===");
    println!("\nAST:");
    print_ast(source);

    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }

    // Line 10 (0-indexed) has the expression
    println!("\nHover scan on totalMass line (line 10):");
    for col in 30..85 {
        if let Some(hover) = analysis.hover(file_id, 10, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }

    // dryMass should resolve
    let hover = analysis.hover(file_id, 10, 38);
    assert!(
        hover.is_some(),
        "dryMass in expression should resolve"
    );
}
