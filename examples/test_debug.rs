use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn main() {
    let mut host = AnalysisHost::new();
    let stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library");
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }

    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml",
    );
    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();
    let _ = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    // Find wheelFastenerInterface1 symbol
    println!("=== wheelFastenerInterface1 in wheelHubAssy2 ===");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.qualified_name.contains("wheelHubAssy2")
            && sym.qualified_name.contains("wheelFastenerInterface1")
            && !sym.qualified_name.contains("::wheelFastenerInterface1::")
        {
            println!("Symbol: {}", sym.qualified_name);
            println!(
                "  start_line: {}, start_col: {}",
                sym.start_line, sym.start_col
            );
            println!("  end_line: {}, end_col: {}", sym.end_line, sym.end_col);
            println!("  kind: {:?}", sym.kind);
            println!("\nType refs:");
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  [{}]: {:?}", i, tr);
            }
        }
    }

    // Show lines around 969 (0-indexed)
    let lines: Vec<&str> = content.lines().collect();
    println!("\n=== File content (0-indexed lines 967-971) ===");
    for i in 967..=971 {
        if i < lines.len() {
            println!("  [{}]: {}", i, lines[i].trim());
        }
    }

    // Test hover at position (969, 93) - shankPort first occurrence
    println!("\n=== Testing hover at (969, 93) ===");
    let hover = analysis.hover(file_id, 969, 93);
    println!("  hover result: {:?}", hover.is_some());
}
