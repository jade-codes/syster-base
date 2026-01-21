//! Test hover for perform_action_usage references

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use syster::core::Position;
use pest::Parser;

fn main() {
    let source = r#"
part def Test {
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;

    println!("=== Testing perform hover ===");

    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    // Simulate hover at different positions
    // Line 2: "    perform transportPassenger;"
    //          0123456789012345678901234567890
    //          ^- col 4 = perform
    //                     ^- col 12 = transportPassenger
    
    // Line 3: "    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;"
    //          0123456789012345678901234567890123456789012345678901234567890123456
    //                     ^- col 12 = transportPassenger
    //                                ^- col 31 = a
    //                                  ^- col 33 = driverGetInVehicle
    //                                                    ^- col 52 = unlockDoor_in

    let ref_index = workspace.reference_index();
    
    println!("\n=== Testing hover at positions ===");
    
    let test_positions = [
        (2, 12, "transportPassenger on line 2"),
        (2, 20, "transportPassenger on line 2 (middle)"),
        (3, 12, "transportPassenger on line 3"),
        (3, 31, "a"),
        (3, 33, "driverGetInVehicle"),
        (3, 52, "unlockDoor_in"),
    ];
    
    for (line, col, desc) in test_positions {
        // Note: positions are 0-indexed in syster::core::Position
        let pos = Position::new(line, col);
        let result = ref_index.get_full_reference_at_position("/test.sysml", pos);
        match result {
            Some((target, info)) => {
                println!("  ({},{}) {}: FOUND target='{}' span={:?}", line, col, desc, target, info.span);
            }
            None => {
                println!("  ({},{}) {}: NOT FOUND", line, col, desc);
            }
        }
    }
    
    println!("\nDone!");
}
