use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let source = include_str!("../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Find the satisfy symbol and check its type_refs
    println!("=== Type refs for satisfy sv:SafetyViewpoint ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("satisfy:sv") {
            println!("Symbol: {}", sym.qualified_name);
            println!("  Location: line {} cols {}-{}", sym.start_line, sym.start_col, sym.end_col);
            println!("  Type refs:");
            for trk in &sym.type_refs {
                match trk {
                    TypeRefKind::Simple(tr) => {
                        println!("    Simple: '{}' at line {} cols {}-{}", tr.target, tr.start_line, tr.start_col, tr.end_col);
                    }
                    TypeRefKind::Chain(c) => {
                        println!("    Chain: {:?}", c.parts.iter().map(|p| p.target.as_ref()).collect::<Vec<_>>());
                    }
                }
            }
            println!("  Relationships:");
            for rel in &sym.relationships {
                println!("    {:?}: {}", rel.kind, rel.target);
            }
        }
    }

    // Check what the actual line looks like at those positions
    let line = source.lines().nth(1573).unwrap();
    println!("\nLine 1573: '{}'", line);
    println!("Chars with indices:");
    for (i, c) in line.chars().enumerate() {
        if i >= 30 && i <= 60 {
            print!("{}:{} ", i, c);
        }
    }
    println!();
}
