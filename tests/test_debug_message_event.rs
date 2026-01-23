//! Debug test for message event chain resolution
//!
//! Tests `sendSensedSpeed.sourceEvent` pattern - message event chains

use syster::ide::AnalysisHost;
use syster::hir::TypeRefKind;

#[test]
fn test_debug_message_event_chain() {
    let mut host = AnalysisHost::new();
    
    // Load the Flows library  
    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);
    
    // Simplified vehicle example with message
    let source = r#"
package TestPkg {
    import Flows::*;
    
    part vehicle {
        port speedSensorPort {
            event sendSpeed.sourceEvent;
        }
        message sendSpeed of Real;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();
    
    println!("\n=== Looking for sendSpeed symbol ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "sendSpeed" {
            println!("Found: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Supertypes: {:?}", sym.supertypes);
            println!("  Type refs:");
            for trk in &sym.type_refs {
                match trk {
                    TypeRefKind::Simple(tr) => {
                        println!("    Simple: '{}' kind={:?} resolved={:?}", 
                            tr.target, tr.kind, tr.resolved_target);
                    }
                    TypeRefKind::Chain(chain) => {
                        let names: Vec<&str> = chain.parts.iter().map(|p| p.target.as_ref()).collect();
                        println!("    Chain: {:?}", names);
                    }
                }
            }
        }
    }
    
    println!("\n=== Looking for Message/sourceEvent in stdlib ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "sourceEvent" || sym.name.as_ref() == "Message" {
            println!("Found: {} ({:?})", sym.qualified_name, sym.kind);
        }
    }
    
    println!("\n=== Looking for chain containing sourceEvent ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            if let TypeRefKind::Chain(chain) = trk {
                let names: Vec<&str> = chain.parts.iter().map(|p| p.target.as_ref()).collect();
                if names.contains(&"sourceEvent") {
                    println!("Found chain {:?} on symbol:", names);
                    println!("  Symbol: {} ({:?})", sym.qualified_name, sym.kind);
                    for (i, part) in chain.parts.iter().enumerate() {
                        println!("    Part {}: '{}' resolved={:?}", 
                            i, part.target, part.resolved_target);
                    }
                }
            }
        }
    }
    
    // Try hover
    let file_id = analysis.get_file_id("test.sysml").expect("file should exist");
    
    // Print the source with line numbers to verify positions
    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{}: {}", i, line);
    }
    
    // Line 6 (0-indexed: 5): event sendSpeed.sourceEvent;
    // Let's check what cols the typeref covers
    println!("\n=== Type ref positions for speedSensorPort ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "speedSensorPort" {
            for trk in &sym.type_refs {
                if let TypeRefKind::Chain(chain) = trk {
                    for part in &chain.parts {
                        println!("  Part '{}': line {} cols {}-{}", 
                            part.target, part.start_line, part.start_col, part.end_col);
                    }
                }
            }
        }
    }
    
    // Try hovering at different positions on line 6 (0-indexed)
    // The chain is on line 6 (from output: Part 'sendSpeed': line 6 cols 18-27)
    for col in [18u32, 20, 25, 28, 30, 35, 39] {
        let hover = analysis.hover(file_id, 6, col);
        println!("\n=== Hover at line 7 col {} ===", col);
        println!("Hover result: {:?}", hover);
    }
}
