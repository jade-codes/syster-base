use syster::base::FileId;
use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();
    let code = r#"package Test {
    requirement def MassRequirement;
    
    requirement <'1'> engineMassRequirement : MassRequirement;
}"#;
    host.set_file_content("/test.sysml", code);

    let analysis = host.analysis();

    println!("\n=== Symbols with spans ===");
    for sym in analysis.symbol_index().all_symbols() {
        println!(
            "  {} (short_name: {:?}) at line {} col {}-{}",
            sym.qualified_name, sym.short_name, sym.start_line, sym.start_col, sym.end_col
        );
        if let (Some(sn_start_line), Some(sn_start_col), Some(sn_end_line), Some(sn_end_col)) = (
            sym.short_name_start_line,
            sym.short_name_start_col,
            sym.short_name_end_line,
            sym.short_name_end_col,
        ) {
            println!(
                "    short_name_span: line {} col {}-{}:{}",
                sn_start_line, sn_start_col, sn_end_line, sn_end_col
            );
        }
    }

    // The text:
    // line 3:     requirement <'1'> engineMassRequirement : MassRequirement;
    //             0         1         2         3         4
    //             0123456789012345678901234567890123456789012345678901234567890
    //                          ^'1' is at around col 17-20 (inside quotes)
    //                               ^engineMassRequirement starts at 22

    println!("\n=== Hover on engineMassRequirement (line 3, col 25) ===");
    if let Some(hover) = analysis.hover(FileId::new(0), 3, 25) {
        println!("Hover result:\n{}", hover.contents);
    } else {
        println!("No hover result at col 25");
    }

    println!("\n=== Hover on short name '1' (line 3, col 18) ===");
    if let Some(hover) = analysis.hover(FileId::new(0), 3, 18) {
        println!("Hover result:\n{}", hover.contents);
    } else {
        println!("No hover result at col 18 - short name hover NOT working!");
    }
}
