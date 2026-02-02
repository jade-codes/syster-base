//! Debug test for flow endpoint extraction

use std::path::PathBuf;
use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;
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

#[test]
fn debug_flow_extraction() {
    let source = r#"
package Test {
    item def Signal;
    
    part def Sender {
        out item output : Signal;
    }
    
    part def Receiver {
        in item input : Signal;
    }
    
    part def System {
        part sender : Sender;
        part receiver : Receiver;
        
        flow of Signal from sender.output to receiver.input;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\n=== ALL SYMBOLS ===");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("\nSymbol: {} ({:?})", sym.qualified_name, sym.kind);
        println!("  Line: {}, Supertypes: {:?}", sym.start_line, sym.supertypes);
        println!("  Type refs ({}):", sym.type_refs.len());
        for (i, tr) in sym.type_refs.iter().enumerate() {
            for r in tr.as_refs() {
                println!("    [{}] '{}' kind={:?} span=({},{}-{},{})", 
                    i, r.target, r.kind, r.start_line, r.start_col, r.end_line, r.end_col);
            }
        }
        println!("  Relationships ({}):", sym.relationships.len());
        for rel in &sym.relationships {
            println!("    {:?} -> {}", rel.kind, rel.target);
        }
    }
    
    // Now test hover on the flow line
    // Line: flow of Signal from sender.output to receiver.input;
    // This should be around line 16 (0-indexed)
    println!("\n=== HOVER TESTS ===");
    let line = 16;
    println!("Testing line {}", line);
    
    for col in 0..70 {
        if let Some(hover) = analysis.hover(file_id, line, col) {
            println!("  col {}: {:?}", col, hover.qualified_name);
        }
    }
}

#[test]
fn debug_succession_extraction() {
    let source = r#"
package Test {
    part def Vehicle {
        event occurrence started;
    }
    
    part def Driver {
        event occurrence acknowledged;
    }
    
    action def StartSequence {
        part vehicle : Vehicle;
        part driver : Driver;
        
        first vehicle.started then driver.acknowledged;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\n=== ALL SYMBOLS (SUCCESSION) ===");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("\nSymbol: {} ({:?})", sym.qualified_name, sym.kind);
        println!("  Line: {}, Supertypes: {:?}", sym.start_line, sym.supertypes);
        println!("  Type refs ({}):", sym.type_refs.len());
        for (i, tr) in sym.type_refs.iter().enumerate() {
            for r in tr.as_refs() {
                println!("    [{}] '{}' kind={:?} span=({},{}-{},{})", 
                    i, r.target, r.kind, r.start_line, r.start_col, r.end_line, r.end_col);
            }
        }
    }
    
    // Test hover on succession line
    println!("\n=== HOVER TESTS (SUCCESSION) ===");
    let line = 15;
    println!("Testing line {}", line);
    
    for col in 0..60 {
        if let Some(hover) = analysis.hover(file_id, line, col) {
            println!("  col {}: {:?}", col, hover.qualified_name);
        }
    }
}

#[test]
fn debug_message_extraction() {
    let source = r#"
package Test {
    item def Command;
    
    part def Driver {
        event occurrence sendCmd;
    }
    
    part def Vehicle {
        event occurrence receiveCmd;
    }
    
    part def Interaction {
        part driver : Driver;
        part vehicle : Vehicle;
        
        message of Command from driver.sendCmd to vehicle.receiveCmd;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\n=== ALL SYMBOLS (MESSAGE) ===");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.qualified_name.contains("Interaction") || sym.qualified_name.contains("message") || sym.qualified_name.contains("Message") {
            println!("\nSymbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Line: {}, Supertypes: {:?}", sym.start_line, sym.supertypes);
            println!("  Type refs ({}):", sym.type_refs.len());
            for (i, tr) in sym.type_refs.iter().enumerate() {
                for r in tr.as_refs() {
                    println!("    [{}] '{}' kind={:?} span=({},{}-{},{})", 
                        i, r.target, r.kind, r.start_line, r.start_col, r.end_line, r.end_col);
                }
            }
        }
    }
    
    // Test hover on message line
    println!("\n=== HOVER TESTS (MESSAGE) ===");
    let line = 16;
    println!("Testing line {}", line);
    
    for col in 0..75 {
        if let Some(hover) = analysis.hover(file_id, line, col) {
            println!("  col {}: {:?}", col, hover.qualified_name);
        }
    }
}
