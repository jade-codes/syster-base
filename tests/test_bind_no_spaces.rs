//! Reproduce exact failure from SimpleVehicleModel line 714

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

/// Test using exact syntax from line 714 - note NO SPACES around =
#[test]
fn test_bind_no_spaces_around_equals() {
    // Exact syntax: bind lhs.a.b=rhs.c.d;  (no spaces around =)
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
        
        bind axle.wheel.wheelPort=vehiclePort.wheelPort1;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    let bind_line = source.lines().enumerate()
        .find(|(_, l)| l.contains("bind axle"))
        .map(|(i, _)| i as u32)
        .unwrap();
    
    println!("Bind line {}: {}", bind_line, source.lines().nth(bind_line as usize).unwrap());
    
    // Print type_refs
    println!("\nType refs:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.qualified_name.contains("bind") {
            for (i, tr) in sym.type_refs.iter().enumerate() {
                for r in tr.as_refs() {
                    println!("  [{}] '{}' span=({},{}-{},{})", 
                        i, r.target, r.start_line, r.start_col, r.end_line, r.end_col);
                }
            }
        }
    }
    
    // Print hover
    println!("\nHover results:");
    let mut last: Option<String> = None;
    for col in 0..70 {
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
    
    // Test: wheelPort1 should resolve
    let mut found = false;
    for col in 40..60 {
        if let Some(hover) = analysis.hover(file_id, bind_line, col) {
            if hover.qualified_name.as_ref()
                .map(|s| s.contains("wheelPort1"))
                .unwrap_or(false)
            {
                found = true;
                println!("\nFOUND at col {}", col);
                break;
            }
        }
    }
    
    assert!(found, "wheelPort1 should resolve (no spaces around =)");
}
