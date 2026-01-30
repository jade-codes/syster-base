use syster::ide::AnalysisHost;

fn main() {
    let content = r#"calc def ComputeBSFC {
    return : Real;
}"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("/test.sysml", content);

    let analysis = host.analysis();
    println!("=== Symbols ===");
    for sym in analysis.symbol_index().all_symbols() {
        println!("  {} (type_refs: {:?})", sym.qualified_name, sym.type_refs);
    }
}
