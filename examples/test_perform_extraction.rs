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
    println!("SOURCE:\n{}", source);

    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    
    // Print elements directly from AST
    println!("\n=== AST Elements ===");
    for element in &sysml_file.elements {
        match element {
            syster::syntax::sysml::ast::Element::Definition(def) => {
                println!("Definition: {:?} kind={:?}", def.name, def.kind);
                for member in &def.body {
                    match member {
                        syster::syntax::sysml::ast::DefinitionMember::Usage(u) => {
                            println!("  Usage: {:?} kind={:?}", u.name, u.kind);
                            println!("    performs: {:?}", u.relationships.performs);
                            println!("    typed_by: {:?}", u.relationships.typed_by);
                            println!("    expression_refs: {:?}", u.expression_refs);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    println!("\n=== REFERENCES ===");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in refs {
        println!("  source={} span={:?}", r.source_qname, r.span);
    }
    
    println!("\n=== REVERSE REFERENCES ===");
    for target in ["transportPassenger", "a", "driverGetInVehicle", "unlockDoor_in"] {
        let refs = workspace.reference_index().get_references(target);
        println!("  '{}' -> {} refs", target, refs.len());
    }
    
    println!("\nDone!");
}
