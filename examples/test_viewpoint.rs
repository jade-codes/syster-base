use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let source = include_str!("../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Find SafetyViewpoint lines
    println!("=== Lines containing SafetyViewpoint ===");
    for (i, line) in source.lines().enumerate() {
        if line.contains("SafetyViewpoint") {
            println!("{:3}: {}", i, line);
        }
    }

    // Find SafetyViewpoint symbol
    println!("\n=== SafetyViewpoint symbols ===");
    for sym in index.all_symbols() {
        if sym.name.contains("Safety") || sym.qualified_name.contains("Safety") {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Location: line {} cols {}-{}", sym.start_line, sym.start_col, sym.end_col);
        }
    }

    // Find the satisfy requirement line and test hover
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\n=== Testing hover on SafetyViewpoint ===");
    for (line_num, line) in source.lines().enumerate() {
        if line.contains("SafetyViewpoint") && line.contains("satisfy") {
            println!("Line {}: {}", line_num, line.trim());
            if let Some(pos) = line.find("SafetyViewpoint") {
                let col = pos as u32;
                for test_col in [col, col + 5, col + 10] {
                    let hover = analysis.hover(file_id, line_num as u32, test_col);
                    println!("  col {}: {:?}", test_col, hover.as_ref().map(|h| h.qualified_name.as_ref()));
                }
            }
        }
    }
}
