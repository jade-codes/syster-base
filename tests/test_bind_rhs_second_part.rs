//! Minimal test for bind RHS second chain member failure

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

/// This test isolates the exact failure:
/// In `bind lhs.a.b = rhs.c.d;`, the `d` part fails to resolve
#[test]
fn test_bind_rhs_second_member_resolves() {
    let source = r#"
package Test {
    port def WheelPort;
    
    port def VehiclePort {
        port wheelPort1 : WheelPort;
    }
    
    part def Wheel {
        port wheelPort : WheelPort;
    }
    
    part def Axle {
        part wheel : Wheel;
    }
    
    part def Vehicle {
        part axle : Axle;
        port vehiclePort : VehiclePort;
        
        bind axle.wheel.wheelPort = vehiclePort.wheelPort1;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Print lines
    for (i, line) in source.lines().enumerate() {
        println!("Line {}: {}", i, line);
    }
    
    // Find bind line
    let bind_line = source.lines().enumerate()
        .find(|(_, l)| l.contains("bind axle"))
        .map(|(i, _)| i as u32)
        .unwrap();
    
    println!("\nBind is at line {}", bind_line);
    
    // Print type_refs for bind symbol
    println!("\nType refs for bind symbol:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.qualified_name.contains("bind") {
            println!("  Symbol: {}", sym.qualified_name);
            for (i, tr) in sym.type_refs.iter().enumerate() {
                for r in tr.as_refs() {
                    println!("    [{}] '{}' span=({},{}-{},{})", 
                        i, r.target, r.start_line, r.start_col, r.end_line, r.end_col);
                }
            }
        }
    }
    
    // Print hover at each position
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
    
    // THE ACTUAL TEST: hover on wheelPort1 (RHS second member) should resolve
    let mut found_wheel_port1 = false;
    for col in 40..70 {
        if let Some(hover) = analysis.hover(file_id, bind_line, col) {
            if hover.qualified_name.as_ref()
                .map(|s| s.contains("wheelPort1"))
                .unwrap_or(false)
            {
                found_wheel_port1 = true;
                println!("\nFOUND wheelPort1 at col {}: {:?}", col, hover.qualified_name);
                break;
            }
        }
    }
    
    assert!(found_wheel_port1, 
        "hover on 'wheelPort1' (RHS second member) should resolve to VehiclePort::wheelPort1");
}
