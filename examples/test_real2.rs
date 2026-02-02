use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

    let source = r#"
package TestPkg {
    import Flows::*;
    
    part def SensedSpeed;
    
    occurrence CruiseControl2 {
        part vehicle_b {
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

    println!("\n=== All symbols in speedSensorPort ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("speedSensorPort") {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "  Span: line {} cols {}-{} to line {} col {}",
                sym.start_line, sym.start_col, sym.end_col, sym.end_line, sym.end_col
            );
        }
    }
}
