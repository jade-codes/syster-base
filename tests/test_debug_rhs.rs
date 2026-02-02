//! Debug RHS chain resolution

use std::path::PathBuf;
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
fn debug_rhs_chain() {
    // Simplified test case for RHS chain resolution
    let source = r#"
package Test {
    port def WheelToRoadPort;
    
    port def VehicleToRoadPort {
        port wheelToRoadPort1 : WheelToRoadPort;
    }
    
    part def Vehicle {
        port vehicleToRoadPort : VehicleToRoadPort;
        
        bind something = vehicleToRoadPort.wheelToRoadPort1;
    }
}
"#;

    for (i, line) in source.lines().enumerate() {
        println!("Line {}: {}", i, line);
    }
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    let bind_line = source.lines().enumerate()
        .find(|(_, l)| l.contains("bind something"))
        .map(|(i, _)| i as u32)
        .unwrap();
    
    println!("\nBind is at line {}", bind_line);
    
    // Check symbols
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.qualified_name.contains("bind") || sym.qualified_name.contains("Vehicle::") {
            println!("\n{} ({:?})", sym.qualified_name, sym.kind);
            for (i, tr) in sym.type_refs.iter().enumerate() {
                for r in tr.as_refs() {
                    println!("  [{}] '{}' span=({},{}-{},{})", i, r.target, r.start_line, r.start_col, r.end_line, r.end_col);
                }
            }
        }
    }
    
    println!("\nHover results:");
    let mut last: Option<String> = None;
    for col in 0..80 {
        let result = analysis.hover(file_id, bind_line, col)
            .and_then(|h| h.qualified_name.as_ref().map(|s| s.to_string()));
        if result != last {
            if let Some(ref r) = result {
                println!("  col {}: {}", col, r);
            } else if last.is_some() {
                println!("  col {}: (none)", col);
            }
            last = result;
        }
    }
}
