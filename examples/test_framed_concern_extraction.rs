//! Test extraction of framed_concern_member references

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"
viewpoint def SafetyViewpoint {
    frame concern vs:VehicleSafety;
}
"#;

    println!("=== Testing framed_concern_member extraction ===");
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
                println!("  relationships.meta: {:?}", def.relationships.meta);
                println!("  body members: {}", def.body.len());
            }
            _ => {}
        }
    }
    
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");

    println!("\n=== SYMBOLS ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {} at {:?}", sym.qualified_name(), sym.span());
    }

    println!("\n=== REFERENCES ===");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in refs {
        println!("  source={} span={:?}", r.source_qname, r.span);
    }
    
    println!("\n=== ALL REVERSE REFERENCES ===");
    for target in ["vs", "VehicleSafety", "vs::VehicleSafety"] {
        let refs = workspace.reference_index().get_references(target);
        println!("  '{}' -> {} refs", target, refs.len());
        for r in refs {
            println!("    source={} span={:?}", r.source_qname, r.span);
        }
    }
    
    println!("\nDone!");
}
