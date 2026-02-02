use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

    let source = include_str!(
        "../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml"
    );
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Find the line with sourceEvent
    println!("=== Lines containing sourceEvent or targetEvent ===");
    for (i, line) in source.lines().enumerate() {
        if line.contains("sourceEvent") || line.contains("targetEvent") {
            println!("{:3}: {}", i, line);
        }
    }

    println!("\n=== All type_refs with chains containing sourceEvent/targetEvent ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            if let TypeRefKind::Chain(chain) = trk {
                let names: Vec<&str> = chain.parts.iter().map(|p| p.target.as_ref()).collect();
                if names.iter().any(|n| n.contains("Event")) {
                    println!("Symbol: {}", sym.qualified_name);
                    for part in &chain.parts {
                        println!(
                            "  Part '{}': line {} cols {}-{} resolved={:?}",
                            part.target,
                            part.start_line,
                            part.start_col,
                            part.end_col,
                            part.resolved_target
                        );
                    }
                }
            }
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Test hover on specific lines - find them first
    println!("\n=== Testing hover on sourceEvent lines ===");
    for (line_num, line) in source.lines().enumerate() {
        if line.contains(".sourceEvent") {
            println!("\nLine {}: {}", line_num, line.trim());
            // Find the position of sourceEvent
            if let Some(pos) = line.find("sourceEvent") {
                let col = pos as u32;
                for test_col in [col, col + 5, col + 10] {
                    let hover = analysis.hover(file_id, line_num as u32, test_col);
                    println!(
                        "  col {}: {:?}",
                        test_col,
                        hover.as_ref().map(|h| h.qualified_name.as_ref())
                    );
                }
            }
        }
    }
}
