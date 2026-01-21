use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;
    
    println!("=== SEMANTIC TEST: #derivation connection with end ::> ===\n");
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
    
    println!("\nREFERENCES:");
    let refs = workspace.reference_index().get_references_in_file("/test.sysml");
    for r in &refs {
        println!("  source={} span={:?} chain_context={:?}", r.source_qname, r.span, r.chain_context);
    }
}
