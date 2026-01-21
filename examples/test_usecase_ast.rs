use std::path::PathBuf;
use syster::semantic::Workspace;
use syster::syntax::file::SyntaxFile;
use syster::syntax::sysml::ast::parse_file;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    use case def TransportPassenger;
    
    use case transportPassenger_1:TransportPassenger{
        action driverGetInVehicle subsets getInVehicle_a[1];
        action driveVehicleToDestination;
        action providePower;
        item def VehicleOnSignal;
        join join1;
        first start;
        then fork fork1;
        first join1 then trigger;
    }
}"#;

    println!("=== PARSING USE CASE ===\n");
    println!("SOURCE:\n{}\n", source);
    
    let mut workspace: Workspace<SyntaxFile> = Workspace::new();
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let sysml_file = parse_file(&mut pairs).expect("AST parse should succeed");
    let syntax_file = SyntaxFile::SysML(sysml_file);
    workspace.add_file(PathBuf::from("/test.sysml"), syntax_file);
    workspace.populate_all().expect("Should populate");
    
    println!("=== ALL SYMBOLS ===");
    for sym in workspace.symbol_table().iter_symbols() {
        println!("  {}", sym.qualified_name());
    }
    
    println!("\n=== ALL REFERENCES ===");
    let ref_index = workspace.reference_index();
    for target in ref_index.targets() {
        let refs = ref_index.get_references(target);
        for r in refs {
            println!("  line={} col={}-{} target='{}' source='{}'", 
                r.span.start.line, r.span.start.column, r.span.end.column,
                target, r.source_qname);
        }
    }
}
