//! Test hover for perform_action_usage with complete definitions

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use syster::core::Position;
use pest::Parser;

fn main() {
    let source = r#"
use case def TransportPassengerDef {
    action a {
        action driverGetInVehicle {
            action unlockDoor_in;
            action openDoor_in;
        }
    }
}

part def Test {
    use case transportPassenger : TransportPassengerDef;
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;

    println!("=== Testing perform with definitions ===");

    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    println!("\n=== SYMBOLS ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {}", sym.qualified_name());
    }

    let ref_index = workspace.reference_index();
    
    println!("\n=== Testing hover at positions ===");
    
    // Line 12 (0-indexed: 11): "    perform transportPassenger;"
    // Line 13 (0-indexed: 12): "    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;"
    
    let test_positions = [
        (12, 12, "transportPassenger on perform line 1"),
        (13, 12, "transportPassenger on perform line 2"),
        (13, 31, "a"),
        (13, 33, "driverGetInVehicle"),
        (13, 52, "unlockDoor_in"),
    ];
    
    for (line, col, desc) in test_positions {
        let pos = Position::new(line, col);
        let result = ref_index.get_full_reference_at_position("/test.sysml", pos);
        match result {
            Some((target, info)) => {
                println!("  ({},{}) {}: target='{}' span={:?}", line, col, desc, target, info.span);
                // Try to find symbol directly
                let mut found = false;
                for sym in workspace.symbol_table().iter_symbols() {
                    if sym.qualified_name() == target || sym.name() == target {
                        println!("    -> RESOLVED to '{}'", sym.qualified_name());
                        found = true;
                        break;
                    }
                }
                if !found {
                    println!("    -> NOT RESOLVED (no symbol for '{}')", target);
                }
            }
            None => {
                println!("  ({},{}) {}: NOT FOUND in reference index", line, col, desc);
            }
        }
    }
    
    println!("\nDone!");
}
