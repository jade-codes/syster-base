use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    attribute def MassValue;
    attribute assumedCargoMass : MassValue;
    
    requirement def FuelEconomyRequirement {
        attribute requiredFuelEconomy;
    }
    
    requirement highwayFuelEconomyRequirement : FuelEconomyRequirement {
        assume constraint { assumedCargoMass <= 500 }
    }
}"#;
    
    println!("=== SEMANTIC TEST: assume constraint ===\n");
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
    
    // Check if assumedCargoMass reference is indexed
    println!("\nLooking for 'assumedCargoMass' reference...");
    let found = refs.iter().any(|r| r.source_qname.contains("assumedCargoMass") || 
        r.chain_context.as_ref().map(|c| c.chain_parts.contains(&"assumedCargoMass".to_string())).unwrap_or(false));
    println!("Found assumedCargoMass reference: {}", found);
}
