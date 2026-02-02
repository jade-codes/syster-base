//! Debug actual CHAIN_MEMBER failures from SimpleVehicleModel.sysml

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

#[test]
fn debug_line_714() {
    let example_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    
    let source = std::fs::read_to_string(&example_path).expect("Failed to read");
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", &source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Line 714 (0-indexed: 713)
    let line = 713;
    let line_text = source.lines().nth(line as usize).unwrap();
    println!("\nLine {}: {}", line + 1, line_text.trim());
    
    // Find the symbol for this bind
    println!("\nSymbols around this line:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.start_line >= 710 && sym.start_line <= 720 {
            println!("\n{} (Line {}, {:?})", sym.qualified_name, sym.start_line, sym.kind);
            println!("  Type refs:");
            for (i, tr) in sym.type_refs.iter().enumerate() {
                for r in tr.as_refs() {
                    println!("    [{}] '{}' span=({},{}-{},{})", i, r.target, r.start_line, r.start_col, r.end_line, r.end_col);
                }
            }
        }
    }
    
    println!("\nHover results on line {}:", line + 1);
    let mut last: Option<String> = None;
    for col in 0..line_text.len().min(120) {
        let result = analysis.hover(file_id, line, col as u32)
            .and_then(|h| h.qualified_name.as_ref().map(|s| s.to_string()));
        if result != last {
            if let Some(ref r) = result {
                println!("  col {}: {}", col, r);
            } else if last.is_some() {
                println!("  col {}: (none)", col);
            }
            last = result;
        }
    }
}
