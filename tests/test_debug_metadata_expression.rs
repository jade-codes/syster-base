//! Debug test to understand how expression refs are extracted from metadata bodies

use syster::ide::AnalysisHost;

#[test]
fn test_debug_metadata_expression_hover() {
    // File 1: Define enum and metadata def
    let lib_source = r#"
package RiskLib {
    enum def Priority {
        low = 1;
        medium = 2;
        high = 3;
    }
    
    metadata def Risk {
        attribute level : Priority;
    }
}
"#;

    // File 2: Use them via import (like the real RiskMetadataExample)
    // This tests the SysML spec pattern: after importing RiskLib::*, 
    // Priority should be visible as just "Priority" for subsequent imports
    let usage_source = r#"
package UsageExample {
    import RiskLib::*;
    import Priority::*;
    
    part engine {
        @Risk {
            level = medium;
        }
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("lib.sysml", lib_source);
    host.set_file_content("usage.sysml", usage_source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("usage.sysml").expect("file should exist");
    
    // In usage.sysml:
    // Line 6: "        @Risk {"
    // Line 7: "            level = medium;"
    // "medium" starts at col 20
    let line = 7u32;
    let col = 22u32;
    
    println!("Testing hover at line {}, col {}", line, col);
    
    let hover = analysis.hover(file_id, line, col);
    
    println!("Hover result: {:?}", hover);
    
    // Also print resolved_target status
    println!("\n=== Checking resolved_target ===");
    let index = analysis.symbol_index();
    for sym in index.symbols_in_file(file_id) {
        if sym.name.as_ref() == "level" {
            for trk in &sym.type_refs {
                for tr in trk.as_refs() {
                    if tr.target.as_ref() == "medium" {
                        println!("level's type_ref 'medium': resolved_target = {:?}", tr.resolved_target);
                    }
                }
            }
        }
    }
    
    if let Some(h) = &hover {
        println!("\nHover contents:\n{}", h.contents);
    } else {
        println!("No hover found!");
        
        // Let's debug - check if any type_ref contains this position
        let index = analysis.symbol_index();
        
        println!("\n=== All symbols in usage.sysml ===");
        for sym in index.symbols_in_file(file_id) {
            println!("Symbol: '{}' (kind: {:?}) lines {}-{}", sym.name, sym.kind, sym.start_line, sym.end_line);
            for trk in &sym.type_refs {
                for tr in trk.as_refs() {
                    println!("  TypeRef: target='{}' kind={:?} lines={}-{} cols={}-{}", 
                             tr.target, tr.kind, tr.start_line, tr.end_line, tr.start_col, tr.end_col);
                    println!("    resolved_target: {:?}", tr.resolved_target);
                    let contains = tr.contains(line, col);
                    println!("    contains({}, {}) = {}", line, col, contains);
                }
            }
        }
        
        // Check if the 'medium' symbol exists in the library file
        println!("\n=== Looking for 'medium' symbol in lib.sysml ===");
        let lib_file_id = analysis.get_file_id("lib.sysml").expect("lib file should exist");
        for sym in index.symbols_in_file(lib_file_id) {
            if sym.name.as_ref() == "medium" {
                println!("Found: '{}' qualified='{}' kind={:?}", sym.name, sym.qualified_name, sym.kind);
            }
        }
        
        // Check what's visible in UsageExample scope
        println!("\n=== Visibility map for UsageExample ===");
        let resolver = index.resolver_for_scope("UsageExample");
        match resolver.resolve("medium") {
            syster::hir::ResolveResult::Found(sym) => {
                println!("Resolved 'medium' to: {}", sym.qualified_name);
            }
            syster::hir::ResolveResult::NotFound => {
                println!("Could not resolve 'medium'");
            }
            syster::hir::ResolveResult::Ambiguous(syms) => {
                println!("Ambiguous: {:?}", syms.iter().map(|s| s.qualified_name.as_ref()).collect::<Vec<_>>());
            }
        }
        
        // Check what Priority::* imports
        println!("\n=== Checking visibility at different scopes ===");
        let resolver2 = index.resolver_for_scope("UsageExample::engine");
        match resolver2.resolve("medium") {
            syster::hir::ResolveResult::Found(sym) => {
                println!("In engine scope, resolved 'medium' to: {}", sym.qualified_name);
            }
            syster::hir::ResolveResult::NotFound => {
                println!("In engine scope, could not resolve 'medium'");
            }
            syster::hir::ResolveResult::Ambiguous(syms) => {
                println!("In engine scope, ambiguous: {:?}", syms.iter().map(|s| s.qualified_name.as_ref()).collect::<Vec<_>>());
            }
        }

        // Print the source lines to verify positions
        println!("\n=== Source lines ===");
        for (i, line_str) in usage_source.lines().enumerate() {
            if i >= 6 && i <= 10 {
                println!("Line {}: '{}'", i, line_str);
            }
        }
    }
}
