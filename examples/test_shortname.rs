use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let source = include_str!(
        "../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml"
    );
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Find symbols with short names (quoted)
    println!("=== Symbols with short names ===");
    for sym in index.all_symbols() {
        if sym.short_name.is_some() {
            println!(
                "Symbol: {} (short: {:?})",
                sym.qualified_name, sym.short_name
            );
            println!(
                "  Location: line {} cols {}-{}",
                sym.start_line, sym.start_col, sym.end_col
            );
            if let (Some(sl), Some(sc), Some(_el), Some(ec)) = (
                sym.short_name_start_line,
                sym.short_name_start_col,
                sym.short_name_end_line,
                sym.short_name_end_col,
            ) {
                println!("  Short name span: line {} cols {}-{}", sl, sc, ec);
            }
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Test hover on line 867: requirement <'1'> vehicleMassRequirement
    println!("\n=== Line 867: requirement <'1'> vehicleMassRequirement ===");
    let line = source.lines().nth(867).unwrap();
    println!("Line: {}", line);
    for col in [20u32, 25, 30, 35, 40, 45, 50, 55, 60] {
        let hover = analysis.hover(file_id, 867, col);
        println!(
            "  col {}: {:?}",
            col,
            hover.as_ref().map(|h| h.qualified_name.as_ref())
        );
    }
}
