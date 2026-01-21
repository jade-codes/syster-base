use syster::semantic::SyntaxWorkspace;
use syster::semantic::symbol_table::Symbol;
use syster::syntax::SyntaxFile;
use std::path::PathBuf;

#[test]
fn test_debug_semantic_refs() {
    let source = r#"package SimpleVehicleModel {
    package SignalDefinitions {
        item def IgnitionCmd;
    }
    
    package Sequence {
        import SignalDefinitions::*;
        
        part part0 {
            perform action startVehicle {
                action turnVehicleOn send ignitionCmd via driver.p1 {
                    in ignitionCmd : IgnitionCmd;
                }
            }
        }
    }
}"#;

    let mut workspace = SyntaxWorkspace::new();
    let path = PathBuf::from("/test.sysml");
    let syntax_file = syster::syntax::sysml::parser::parse_content(source, &path).expect("Should parse");
    workspace.update_file(&path, SyntaxFile::SysML(syntax_file));
    
    println!("\n=== Symbols ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {} (span: {:?})", sym.qualified_name(), sym.span());
        if let Symbol::Usage { usage_type, .. } = sym {
            println!("    usage_type: {:?}", usage_type);
        }
    }
    
    println!("\n=== References ===");
    for r in workspace.reference_index().get_references_in_file("/test.sysml") {
        println!("  {} at {:?}", r.source_qname, r.span);
    }
}
