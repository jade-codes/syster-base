//! Debug test for interface endpoint chain resolution
//!
//! Tests `connect lugNutPort ::> lugNutCompositePort.lugNutPort1` pattern

use syster::ide::AnalysisHost;
use syster::hir::TypeRefKind;

fn get_test_file_content() -> &'static str {
    include_str!("sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml")
}

#[test]
fn test_debug_interface_endpoint_chain() {
    let mut host = AnalysisHost::new();
    let content = get_test_file_content();
    let file_uri = "test/SimpleVehicleModel.sysml";
    host.set_file_content(file_uri, content);
    let analysis = host.analysis();
    
    let index = analysis.symbol_index();
    
    // Line 1001 (0-indexed: 1000): connect lugNutPort ::> lugNutCompositePort.lugNutPort1 to ...
    // The chain "lugNutCompositePort.lugNutPort1" starts around col 37
    let test_line = 1000u32; // 0-indexed
    let test_col = 55u32; // Pointing at "lugNutPort1"
    
    println!("\n=== Looking for symbols containing the chain ===");
    
    // Find symbols that have type_refs containing "lugNutPort1"
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            match trk {
                TypeRefKind::Chain(chain) => {
                    let names: Vec<&str> = chain.parts.iter().map(|p| p.target.as_ref()).collect();
                    if names.contains(&"lugNutPort1") {
                        println!("Found chain {:?} on symbol:", names);
                        println!("  Symbol: {} ({:?})", sym.qualified_name, sym.kind);
                        println!("  Symbol line: {}", sym.start_line);
                        for (i, part) in chain.parts.iter().enumerate() {
                            println!("    Part {}: '{}' resolved={:?} kind={:?}", 
                                i, part.target, part.resolved_target, part.kind);
                        }
                    }
                }
                TypeRefKind::Simple(tr) => {
                    if tr.target.contains("lugNutPort1") {
                        println!("Found simple ref containing 'lugNutPort1':");
                        println!("  Symbol: {} ({:?})", sym.qualified_name, sym.kind);
                        println!("  Target: '{}' resolved={:?}", tr.target, tr.resolved_target);
                    }
                }
            }
        }
    }
    
    let file_id = analysis.get_file_id(file_uri).expect("file should exist");
    
    println!("\n=== Hovering at line {} col {} ===", test_line + 1, test_col);
    let hover = analysis.hover(file_id, test_line, test_col);
    println!("Hover result: {:?}", hover);
    
    // Also try hovering on "lugNutCompositePort" (first part of chain)
    let test_col_first = 37u32; // Pointing at "lugNutCompositePort"
    println!("\n=== Hovering at line {} col {} (first part) ===", test_line + 1, test_col_first);
    let hover2 = analysis.hover(file_id, test_line, test_col_first);
    println!("Hover result: {:?}", hover2);
    
    // Check what the expected symbol would be
    println!("\n=== Looking for lugNutPort1 definition ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "lugNutPort1" {
            println!("  {} ({:?}) at line {}", sym.qualified_name, sym.kind, sym.start_line);
        }
    }
}
