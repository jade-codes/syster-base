//! Test extraction of perform_action_usage references

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"
part def Test {
    perform transportPassenger;
    perform transportPassenger.a.driverGetInVehicle.unlockDoor_in;
}
"#;

    println!("=== Testing perform_action_usage extraction ===");

    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    println!("\n=== ALL INDEXED TARGETS ===");
    // Print all targets that have references
    let ref_index = workspace.reference_index();
    // Need to iterate all refs and collect unique targets
    let refs = ref_index.get_references_in_file("/test.sysml");
    let mut seen_targets = std::collections::HashSet::new();
    for r in refs {
        // The ref_index stores references by target, but we can only query by target
        // So let's check various combinations
        if seen_targets.insert(r.source_qname.clone()) {
            println!("  ref from: {}", r.source_qname);
        }
    }
    
    // Check specific targets
    println!("\n=== SPECIFIC TARGET CHECKS ===");
    for target in ["transportPassenger", "Test::transportPassenger", "a", "driverGetInVehicle", "unlockDoor_in", "transportPassenger.a", "transportPassenger::a"] {
        let refs = ref_index.get_references(target);
        if !refs.is_empty() {
            println!("  '{}' -> {} refs", target, refs.len());
            for r in refs {
                println!("    from {} at {:?}", r.source_qname, r.span);
            }
        }
    }
}
