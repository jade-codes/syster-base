use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let source = include_str!(
        "../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml"
    );
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Test hover on line 866 (0-indexed): requirement <'1'> vehicleMassRequirement
    // File uses 1-indexed, so it's line 867 in the file = line 866 0-indexed
    println!("=== Line 866 (0-indexed): requirement <'1'> vehicleMassRequirement ===");
    let line = source.lines().nth(866).unwrap();
    println!("Line: {}", line);

    // Print char positions
    println!("Chars:");
    for (i, c) in line.chars().enumerate() {
        if (20..=70).contains(&i) {
            print!("{}:{} ", i, c);
        }
    }
    println!();

    for col in [20u32, 25, 30, 32, 35, 37, 40, 45, 50, 55, 60] {
        let hover = analysis.hover(file_id, 866, col);
        println!(
            "  col {}: {:?}",
            col,
            hover.as_ref().map(|h| h.qualified_name.as_ref())
        );
    }

    // Check the symbol span
    println!("\n=== vehicleMassRequirement symbol ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "vehicleMassRequirement"
            && sym.qualified_name.contains("vehicleSpecification")
        {
            println!("Symbol: {}", sym.qualified_name);
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
}
