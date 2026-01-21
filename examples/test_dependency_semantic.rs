//! Semantic test for dependency references
//!
//! Tests that all parts of `#refinement dependency source to target` are indexed

use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package VehicleConfiguration_b {
    package PartsTree {
        part vehicle_b {
            part engine;
        }
    }
}

package Test {
    part def Engine4Cyl;
    part engine4Cyl : Engine4Cyl;
    
    #refinement dependency engine4Cyl to VehicleConfiguration_b::PartsTree::vehicle_b::engine;
}"#;
    
    println!("=== SEMANTIC TEST: dependency references ===\n");
    println!("SOURCE:\n{}\n", source);
    
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");
    
    println!("SYMBOLS:");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {} at {:?}", sym.qualified_name(), sym.span());
    }
    
    println!("\nALL REFERENCES:");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in &refs {
        println!("  line={} col={}-{} source={}", 
            r.span.start.line, r.span.start.column, r.span.end.column, r.source_qname);
    }
    
    // Check for references on line 12 (the dependency line)
    println!("\nReferences on line 12 (dependency line):");
    let line12_refs: Vec<_> = refs.iter().filter(|r| r.span.start.line == 12).collect();
    for r in &line12_refs {
        println!("  col={}-{} source={}", r.span.start.column, r.span.end.column, r.source_qname);
    }
    
    println!("\nExpected references on line 12:");
    println!("  - engine4Cyl (around col 26-36)");
    println!("  - VehicleConfiguration_b::PartsTree::vehicle_b::engine (around col 41-87)");
}
