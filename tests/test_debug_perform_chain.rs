use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;

#[test]
fn test_debug_perform_chain() {
    let source = r#"
package VehicleModel {
    action def StartVehicle;
    action def TurnVehicleOn;
    
    part def Vehicle {
        part driver {
            perform startVehicle.turnVehicleOn;
        }
        action startVehicle : StartVehicle {
            action turnVehicleOn : TurnVehicleOn;
        }
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();
    
    // Find the driver part
    let driver = index.all_symbols()
        .find(|s| &*s.name == "driver")
        .expect("Should find driver");
    
    println!("\n=== DRIVER SYMBOL ===");
    println!("name: {:?}", driver.name);
    println!("kind: {:?}", driver.kind);
    println!("type_refs count: {}", driver.type_refs.len());
    
    for (i, trk) in driver.type_refs.iter().enumerate() {
        match trk {
            TypeRefKind::Simple(tr) => {
                println!("  type_ref[{}] SIMPLE: target='{}' kind={:?}", i, tr.target, tr.kind);
            }
            TypeRefKind::Chain(chain) => {
                println!("  type_ref[{}] CHAIN with {} parts:", i, chain.parts.len());
                for (j, part) in chain.parts.iter().enumerate() {
                    println!("    part[{}]: target='{}' kind={:?}", j, part.target, part.kind);
                }
            }
        }
    }
}
