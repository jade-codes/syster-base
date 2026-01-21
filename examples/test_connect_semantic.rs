use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::SyntaxFile;
use syster::syntax::sysml::ast::{parse_file};
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = include_str!("/tmp/test_connect.sysml");
    println!("=== SOURCE ===");
    println!("{}", source);
    println!("\n=== SEMANTIC LAYER ===");
    
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Grammar parse failed");
    let file = parse_file(&mut pairs).expect("AST parse failed");
    
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    workspace.add_file(PathBuf::from("/test.sysml"), SyntaxFile::SysML(file));
    workspace.populate_all().expect("Should populate");

    println!("\n=== SYMBOLS ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {} at {:?}", sym.qualified_name(), sym.span());
    }

    println!("\n=== REFERENCES ===");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in &refs {
        println!("  source={} span={:?} chain={:?}", r.source_qname, r.span, r.chain_context);
    }
}
