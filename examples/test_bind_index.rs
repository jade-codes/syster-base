use std::sync::Arc;
use syster::semantic::SymbolIndex;
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let input = r#"
part def Vehicle {
    port ignitionCmdPort;
    part engine {
        port ignitionCmdPort;
    }
}
part vehicle : Vehicle {
    bind engine.ignitionCmdPort=ignitionCmdPort;
}
    "#;
    
    let index = SymbolIndex::default();
    let file = Arc::from("test.sysml");
    index.index_file(&file, input).unwrap();
    
    println!("=== References ===");
    for entry in index.refs.lock().unwrap().iter() {
        let (key, reflist) = entry.pair();
        println!("File: {}", key);
        for r in reflist {
            println!("  {:?}", r);
        }
    }
    
    println!("\n=== Symbols ===");
    for entry in index.symbols.lock().unwrap().iter() {
        let (fqn, sym) = entry.pair();
        println!("{} -> {:?}", fqn, sym.kind);
    }
}
