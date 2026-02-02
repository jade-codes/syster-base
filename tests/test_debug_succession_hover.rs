//! Debug tests for hover on succession (then) and chain access (sendSpeed.sourceEvent)

use syster::hir::{SymbolKind, TypeRefKind};
use syster::ide::AnalysisHost;

/// Test `then event X` hover - the `then` keyword and target
#[test]
fn test_hover_on_then_succession() {
    let mut host = AnalysisHost::new();

    // Load necessary stdlib parts
    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

    let source = r#"
package TestPkg {
    import Flows::*;
    
    occurrence CruiseControl {
        part vehicle {
            port speedSensorPort {
                event occurrence setSpeedReceived;
                then event occurrence sendData;
            }
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{}: {}", i, line);
    }

    // Find symbols related to speedSensorPort and its contents
    println!("\n=== Symbols in speedSensorPort scope ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("speedSensorPort") {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "  Location: line {} cols {}-{}",
                sym.start_line, sym.start_col, sym.end_col
            );
            println!("  Supertypes: {:?}", sym.supertypes);
            println!("  Relationships: {:?}", sym.relationships);
        }
    }

    // Look for any succession-related symbols
    println!("\n=== Succession-related symbols ===");
    for sym in index.all_symbols() {
        if sym.name.contains("then")
            || matches!(sym.kind, SymbolKind::OccurrenceUsage)
                && sym.qualified_name.contains("speedSensorPort")
        {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "  Location: line {} cols {}-{}",
                sym.start_line, sym.start_col, sym.end_col
            );
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 8 (0-indexed): "then event occurrence sendData;"
    // sendData is at cols 38-46, let's hover there
    println!("\n=== Testing hover on line 8 (then event occurrence sendData) ===");
    for col in [16u32, 17, 18, 19, 20, 21, 38, 40, 45] {
        let hover = analysis.hover(file_id, 8, col);
        println!(
            "Hover at line 8, col {}: {:?}",
            col,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }
}

/// Test `sendSpeed.sourceEvent` chain access hover
#[test]
fn test_hover_on_message_source_event() {
    let mut host = AnalysisHost::new();

    // Load Flows stdlib
    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

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

    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{}: {}", i, line);
    }

    // Find the symbol containing the chain
    println!("\n=== Looking for chain type refs ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            if let TypeRefKind::Chain(chain) = trk {
                let names: Vec<&str> = chain.parts.iter().map(|p| p.target.as_ref()).collect();
                println!("Found chain {:?} on symbol '{}'", names, sym.qualified_name);
                for (i, part) in chain.parts.iter().enumerate() {
                    println!(
                        "  Part {}: '{}' at line {} cols {}-{}",
                        i, part.target, part.start_line, part.start_col, part.end_col
                    );
                    println!("    resolved: {:?}", part.resolved_target);
                }
            }
        }
    }

    // Look for sendSpeed symbol and its type
    println!("\n=== Looking for sendSpeed symbol ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "sendSpeed" {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Supertypes: {:?}", sym.supertypes);
            for trk in &sym.type_refs {
                for tr in trk.as_refs() {
                    println!(
                        "  TypeRef: {} kind={:?} resolved={:?}",
                        tr.target, tr.kind, tr.resolved_target
                    );
                }
            }
        }
    }

    // Look for Message symbol in stdlib
    println!("\n=== Looking for Message and sourceEvent in stdlib ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "Message"
            || sym.name.as_ref() == "sourceEvent"
            || sym.name.as_ref() == "targetEvent"
        {
            println!("Found: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Supertypes: {:?}", sym.supertypes);
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 6 (0-indexed): "            event sendSpeed.sourceEvent;"
    // Type ref spans line 6 cols 18-39
    println!("\n=== Testing hover on line 6 (event sendSpeed.sourceEvent) ===");
    for col in [12u32, 18, 20, 26, 28, 30, 35, 38] {
        let hover = analysis.hover(file_id, 6, col);
        println!(
            "Hover at line 6, col {}: {:?}",
            col,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }
}

/// Test redefines hover (from the failing test)
#[test]
fn test_hover_on_redefines_basic() {
    let mut host = AnalysisHost::new();

    let source = r#"
package TestPkg {
    part def Vehicle {
        port fuelPort;
    }
    
    part vehicle : Vehicle {
        port redefines fuelPort;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    println!("\n=== Source with line numbers (showing cols) ===");
    for (i, line) in source.lines().enumerate() {
        println!("{}: {}", i, line);
        if line.contains("redefines") {
            // Show column positions
            print!("   ");
            for (col, _) in line.chars().enumerate() {
                print!("{}", col % 10);
            }
            println!();
        }
    }

    // Find the redefines relationship
    println!("\n=== Looking for redefines relationships ===");
    for sym in index.all_symbols() {
        if !sym.relationships.is_empty() {
            for rel in &sym.relationships {
                if matches!(rel.kind, syster::hir::RelationshipKind::Redefines) {
                    println!("Symbol '{}' redefines '{}'", sym.qualified_name, rel.target);
                    println!(
                        "  Location: line {} cols {}-{}",
                        sym.start_line, sym.start_col, sym.end_col
                    );
                }
            }
        }
    }

    // Look for type refs with redefines
    println!("\n=== Looking for redefines type refs ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if matches!(tr.kind, syster::hir::RefKind::Redefines) {
                    println!(
                        "Symbol '{}' has redefines typeref to '{}' at line {} cols {}-{}",
                        sym.qualified_name, tr.target, tr.start_line, tr.start_col, tr.end_col
                    );
                    println!("  resolved: {:?}", tr.resolved_target);
                }
            }
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 7 (0-indexed): "        port redefines fuelPort;"
    // Hover on "fuelPort" should resolve to Vehicle::fuelPort
    // cols 23-31 based on type_ref output
    println!("\n=== Testing hover on line 7 (port redefines fuelPort) ===");
    for col in [8u32, 13, 20, 23, 25, 27, 30, 31] {
        let hover = analysis.hover(file_id, 7, col);
        println!(
            "Hover at line 7, col {}: {:?}",
            col,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }
}
