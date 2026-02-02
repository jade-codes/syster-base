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

    // Find transition symbols with quoted names
    println!("=== Transition symbols with quoted names ===");
    for sym in index.all_symbols() {
        if sym.name.contains("off-on")
            || sym.name.contains("on-off")
            || sym.name.contains("wait-wait")
        {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "  Location: line {} cols {}-{}",
                sym.start_line, sym.start_col, sym.end_col
            );
        }
    }

    // Test hover on line 169 (0-indexed): transition 'off-on'
    println!("\n=== Line 169 (0-indexed): transition 'off-on' ===");
    let line = source.lines().nth(169).unwrap();
    println!("Line: {}", line);

    for col in [20u32, 24, 28, 32, 35, 38, 40] {
        let hover = analysis.hover(file_id, 169, col);
        println!(
            "  col {}: {:?}",
            col,
            hover.as_ref().map(|h| h.qualified_name.as_ref())
        );
    }
}
