use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();
    host.set_file_content(
        "/test.sysml",
        r#"package Requirements {
    requirement def MassRequirement;
    
    requirement vehicleSpecification {
        requirement vehicleMassRequirement : MassRequirement;
    }
    
    requirement engineSpecification {
        requirement engineMassRequirement : MassRequirement;
    }
    
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#,
    );

    // Get file_id before taking analysis snapshot
    let file_id = host.get_file_id("/test.sysml").expect("file should exist");
    let analysis = host.analysis();

    println!("=== Symbols ===");
    for sym in analysis.symbol_index().all_symbols() {
        println!(
            "  {} ({:?}) @ {}:{}-{}:{}",
            sym.qualified_name, sym.kind, sym.start_line, sym.start_col, sym.end_line, sym.end_col
        );
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                println!(
                    "    type_ref: {} @ {}:{}-{}:{}",
                    tr.target, tr.start_line, tr.start_col, tr.end_line, tr.end_col
                );
            }
        }
    }

    println!("\n=== Hover at line 12, col 30 ===");
    match analysis.hover(file_id, 12, 30) {
        Some(h) => println!("Found: {}", h.contents),
        None => println!("Not found"),
    }
}
