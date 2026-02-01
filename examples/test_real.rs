use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

    let source = r#"
package TestPkg {
    import Flows::*;
    
    part def SensedSpeed;
    part def FuelCmd;
    
    occurrence CruiseControl2 {
        part vehicle_b {
            part speedSensor {
                port speedSensorPort {
                    event sendSensedSpeed.sourceEvent;
                }
            }
            part vehicleController {
                port speedSensorPort {
                    event occurrence setSpeedReceived;
                    then event sendSensedSpeed.targetEvent;
                }
            }
            message sendSensedSpeed of SensedSpeed;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{:2}: {}", i, line);
    }

    println!("\n=== All type_refs with chains ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            if let TypeRefKind::Chain(chain) = trk {
                println!("Symbol: {}", sym.qualified_name);
                for part in &chain.parts {
                    println!(
                        "  Part '{}': line {} cols {}-{} (kind={:?})",
                        part.target, part.start_line, part.start_col, part.end_col, part.kind
                    );
                }
            }
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 11: event sendSensedSpeed.sourceEvent;
    println!("\n=== Hover on line 11 (event sendSensedSpeed.sourceEvent) ===");
    for col in [20u32, 25, 30, 35, 40, 45, 50, 55] {
        let hover = analysis.hover(file_id, 11, col);
        println!("col {}: {:?}", col, hover.as_ref().map(|h| h.qualified_name.as_ref()));
    }

    // Line 16: then event sendSensedSpeed.targetEvent;
    println!("\n=== Hover on line 17 (then event sendSensedSpeed.targetEvent) ===");
    for col in [20u32, 25, 30, 35, 40, 45, 50, 55] {
        let hover = analysis.hover(file_id, 17, col);
        println!("col {}: {:?}", col, hover.as_ref().map(|h| h.qualified_name.as_ref()));
    }
}
