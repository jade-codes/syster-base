//! Debug test for expression references

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

#[test]
fn test_expr_ref_hover() {
    let mut host = AnalysisHost::new();
    let source = r#"
package Test {
    attribute def MassValue;
    
    part def Engine {
        attribute mass : MassValue;
    }
    
    part def Vehicle {
        part engine : Engine;
        attribute totalMass : MassValue = engine.mass;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let index = analysis.symbol_index();

    // Print lines with numbers
    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{:2}: {}", i, line);
    }

    // Print all symbols and their type_refs
    println!("\n=== Symbols with type_refs ===");
    for sym in index.symbols_in_file(file_id) {
        if !sym.type_refs.is_empty() {
            println!("\nSymbol: {} ({})", sym.qualified_name, sym.kind.display());
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        println!(
                            "  - Simple: {} ({:?}) at line {} cols {}-{}",
                            r.target, r.kind, r.start_line, r.start_col, r.end_col
                        );
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        println!("  - Chain: {} parts", c.parts.len());
                        for p in &c.parts {
                            println!(
                                "      {} ({:?}) at line {} cols {}-{}",
                                p.target, p.kind, p.start_line, p.start_col, p.end_col
                            );
                        }
                    }
                }
            }
        }
    }

    // Test hover at position of `engine` in `engine.mass`
    // Line 10 (0-indexed): `        attribute totalMass : MassValue = engine.mass;`
    println!("\n=== Testing hover on line 10 ===");
    for col in 40..58 {
        let hover = analysis.hover(file_id, 10, col);
        let c = source
            .lines()
            .nth(10)
            .and_then(|l| l.chars().nth(col as usize));
        println!(
            "Hover at (10, {:2}) [char={:?}]: {:?}",
            col,
            c,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }
}

#[test]
fn test_expr_ref_in_transition_if() {
    let mut host = AnalysisHost::new();
    let mut stdlib_loader = StdLibLoader::with_path(stdlib_path());
    stdlib_loader
        .ensure_loaded_into_host(&mut host)
        .expect("Failed to load stdlib");

    // Simplified version of the Vehicle Example transition
    let source = r#"
package Test {
    enum IgnitionOnOff { on; off; }
    
    struct IgnitionCmd {
        attribute ignitionOnOff : IgnitionOnOff;
    }
    
    part def VehicleBody {
        port ignitionCmdPort;
        part controller;
        
        state def VehicleStates {
            state off;
            state starting;
            
            transition off_To_starting
                first off
                accept ignitionCmd : IgnitionCmd via ignitionCmdPort
                    if ignitionCmd.ignitionOnOff == IgnitionOnOff::on
                do send new IgnitionCmd() to controller
                then starting;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let index = analysis.symbol_index();

    // Print lines with numbers
    println!("\n=== Source with line numbers ===");
    for (i, line) in source.lines().enumerate() {
        println!("{:2}: {}", i, line);
    }

    // Print all symbols
    println!("\n=== All symbols ===");
    for sym in index.symbols_in_file(file_id) {
        println!(
            "{} ({}) at line {} cols {}-{}",
            sym.qualified_name,
            sym.kind.display(),
            sym.start_line,
            sym.start_col,
            sym.end_col
        );
        if !sym.type_refs.is_empty() {
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        println!(
                            "    TypeRef: {} ({:?}) at line {} cols {}-{}",
                            r.target, r.kind, r.start_line, r.start_col, r.end_col
                        );
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        println!("    Chain: {} parts", c.parts.len());
                        for p in &c.parts {
                            println!(
                                "        {} ({:?}) at line {} cols {}-{}",
                                p.target, p.kind, p.start_line, p.start_col, p.end_col
                            );
                        }
                    }
                }
            }
        }
    }

    // The `if ignitionCmd.ignitionOnOff` is on line 19 (0-indexed)
    // Let's check hover
    println!("\n=== Testing hover on line 19 (if condition) ===");
    let line = 19;

    // First, manually check what type_refs contain line 19
    println!("\nSymbols with type_refs containing line {}:", line);
    for sym in index.symbols_in_file(file_id) {
        for tr_kind in &sym.type_refs {
            if tr_kind.contains(line, 30) {
                println!(
                    "  Symbol {} has type_ref containing (19, 30)",
                    sym.qualified_name
                );
            }
        }
    }

    // Debug: manually call find_type_ref_at_position for cols 35-48 (ignitionOnOff)
    use syster::ide::find_type_ref_at_position;
    println!("\n=== Debug: find_type_ref_at_position for ignitionOnOff ===");
    if let Some(ctx) = find_type_ref_at_position(index, file_id, 19, 40) {
        println!("Found type_ref context:");
        println!("  target_name: {}", ctx.target_name);
        println!(
            "  containing_symbol: {:?}",
            ctx.containing_symbol.map(|s| &s.qualified_name)
        );
        println!("  chain_prefix len: {}", ctx.chain_prefix.len());
        for (i, p) in ctx.chain_prefix.iter().enumerate() {
            println!(
                "    prefix[{}]: {} (resolved: {:?})",
                i, p.target, p.resolved_target
            );
        }

        // Manually debug chain resolution
        let scope = ctx
            .containing_symbol
            .map(|s| s.qualified_name.as_ref())
            .unwrap_or("");
        println!("  Base scope: {}", scope);

        let resolver = index.resolver_for_scope(scope);
        let first_part = ctx.chain_prefix.first().unwrap();
        println!("  Resolving first part '{}' in scope...", first_part.target);
        match resolver.resolve(&first_part.target) {
            syster::hir::ResolveResult::Found(sym) => {
                println!(
                    "    Found: {} (kind: {})",
                    sym.qualified_name,
                    sym.kind.display()
                );
                println!("    supertypes: {:?}", sym.supertypes);

                // Get the type name
                let type_name = sym.supertypes.first().map(|s| s.as_ref()).unwrap_or("None");
                println!("    type_name from supertypes: {}", type_name);

                // Try to look up the type
                println!("\n  Looking up type '{}' ...", type_name);
                if let Some(type_sym) = index.lookup_qualified(type_name) {
                    println!("    lookup_qualified found: {}", type_sym.qualified_name);
                } else {
                    println!("    lookup_qualified failed");
                }
                if let Some(type_sym) = index.lookup_definition(type_name) {
                    println!("    lookup_definition found: {}", type_sym.qualified_name);
                } else {
                    println!("    lookup_definition failed");
                }

                // Check if IgnitionCmd exists somewhere
                println!("\n  Looking for any symbol containing 'IgnitionCmd' or 'Ignition'...");
                for sym2 in index.all_symbols() {
                    if sym2.qualified_name.contains("Ignition") || sym2.name.contains("Ignition") {
                        println!(
                            "    {} (kind: {})",
                            sym2.qualified_name,
                            sym2.kind.display()
                        );
                    }
                }

                println!("\n  All symbols:");
                for sym2 in index.all_symbols() {
                    println!(
                        "    {} (kind: {})",
                        sym2.qualified_name,
                        sym2.kind.display()
                    );
                }
            }
            syster::hir::ResolveResult::Ambiguous(syms) => {
                println!("    Ambiguous: {} matches", syms.len());
            }
            syster::hir::ResolveResult::NotFound => {
                println!("    NOT FOUND");
            }
        }
    } else {
        println!("No type_ref found at (19, 40)");
    }

    for col in 20..70 {
        let hover = analysis.hover(file_id, line, col);
        let c = source
            .lines()
            .nth(line as usize)
            .and_then(|l| l.chars().nth(col as usize));
        println!(
            "Hover at ({}, {:2}) [char={:?}]: {:?}",
            line,
            col,
            c,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }

    println!("\n=== Testing hover on line 20 (send statement) ===");
    let line = 20;
    for col in 30..60 {
        let hover = analysis.hover(file_id, line, col);
        let c = source
            .lines()
            .nth(line as usize)
            .and_then(|l| l.chars().nth(col as usize));
        println!(
            "Hover at ({}, {:2}) [char={:?}]: {:?}",
            line,
            col,
            c,
            hover.as_ref().map(|h| &h.qualified_name)
        );
    }
}
