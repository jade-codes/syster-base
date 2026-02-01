//! Debug test for complex redefines pattern

use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;

#[test]
fn test_debug_redefines_qualified() {
    let mut host = AnalysisHost::new();
    
    let source = r#"
package TestPkg {
    action def ProvidePower;
    
    part def Vehicle {
        perform providePower : ProvidePower;
    }
    
    part vehicle_b : Vehicle {
        perform ActionTree::providePower redefines providePower;
    }
    
    package ActionTree {
        action providePower : ProvidePower;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{}: {}", i, line);
    }

    // Look for all symbols and their type refs
    println!("\n=== Looking for symbols with redefines type refs ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if matches!(tr.kind, syster::hir::RefKind::Redefines) {
                    println!("Symbol '{}' has redefines typeref to '{}'", sym.qualified_name, tr.target);
                    println!("  at line {} cols {}-{}", tr.start_line, tr.start_col, tr.end_col);
                    println!("  resolved: {:?}", tr.resolved_target);
                }
            }
        }
    }

    // Look for Vehicle::providePower
    println!("\n=== Looking for providePower symbols ===");
    for sym in index.all_symbols() {
        if sym.name.as_ref() == "providePower" || sym.qualified_name.contains("providePower") {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
        }
    }

    // Look for Vehicle scope
    println!("\n=== Symbols in Vehicle scope ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.starts_with("TestPkg::Vehicle::") {
            println!("Symbol: {} (name={}, {:?})", sym.qualified_name, sym.name, sym.kind);
        }
    }

    // Check visibility map for Vehicle by looking up some names
    println!("\n=== Visibility lookups for TestPkg::Vehicle ===");
    if let Some(vis) = index.visibility_for_scope("TestPkg::Vehicle") {
        for name in ["providePower", "ProvidePower", "<:ProvidePower#1@L5>"] {
            println!("  lookup '{}' -> {:?}", name, vis.lookup(name));
        }
    } else {
        println!("No visibility map found");
    }

    // Look for symbols containing vehicle_b - show ALL type refs including resolved
    println!("\n=== Symbols in vehicle_b scope (with resolved) ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("vehicle_b") {
            println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
            println!("  Location: line {} cols {}-{}", sym.start_line, sym.start_col, sym.end_col);
            for trk in &sym.type_refs {
                match trk {
                    TypeRefKind::Simple(tr) => {
                        println!("  TypeRef Simple: '{}' kind={:?} resolved={:?}",
                            tr.target, tr.kind, tr.resolved_target);
                    }
                    TypeRefKind::Chain(chain) => {
                        println!("  TypeRef Chain:");
                        for (i, tr) in chain.parts.iter().enumerate() {
                            println!("    Part {}: '{}' resolved={:?}",
                                i, tr.target, tr.resolved_target);
                        }
                    }
                }
            }
        }
    }

    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 9 (0-indexed): "        perform ActionTree::providePower redefines providePower;"
    // Let's try various columns
    println!("\n=== Testing hover on line 9 ===");
    for col in [8u32, 16, 28, 41, 50, 51, 55, 60, 62] {
        let hover = analysis.hover(file_id, 9, col);
        println!("Hover at line 9, col {}: {:?}", col, hover.as_ref().map(|h| &h.qualified_name));
    }
}
