fn main() {
    // Full state example from user
    let source = r#"package TestStates {
    state def Vehicle {
        state healthStates {
            entry action initial;
            do senseTemperature{
                out temp;
            }

            state normal;
            state maintenance;
            state degraded;                    

            transition initial then normal;

            transition normal_To_maintenance
                first normal
                accept at maintenanceTime
                then maintenance;

            transition normal_To_degraded
                first normal
                accept when senseTemperature.temp > Tmax 
                do send new OverTemp() to controller
                then degraded;

            transition maintenance_To_normal
                first maintenance
                accept ReturnToNormal
                then normal;

            transition degraded_To_normal
                first degraded
                accept ReturnToNormal
                then normal;
        }
    }
}"#;
    use syster::syntax::file::FileExtension;
    let syntax_file = syster::syntax::SyntaxFile::new(source, FileExtension::SysML);

    // Extract symbols
    let symbols = syster::hir::extract_symbols_unified(syster::FileId(0), &syntax_file);

    println!("=== EXTRACTED SYMBOLS ===");
    for sym in &symbols {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
    }

    // Also print AST to see transition structure
    println!("\n=== TRANSITION AST ===");
    let parse = syster::parser::parse_sysml(source);
    fn find_transitions(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>) {
        if node.kind() == syster::parser::SyntaxKind::TRANSITION_USAGE {
            println!("TRANSITION_USAGE found:");
            for child in node.children() {
                println!("  child: {:?} - {:?}", child.kind(), child.text());
            }
        }
        for child in node.children() {
            find_transitions(&child);
        }
    }
    find_transitions(&parse.syntax());
}
