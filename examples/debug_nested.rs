use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;
use syster::base::FileId;
use std::path::Path;

fn main() {
    let source = r#"
package Definitions {
    public import PartDefinitions::*;
    package PartDefinitions {
        part def Vehicle;
    }
}
package Usage {
    import Definitions::*;
    part car : Vehicle;
}
"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &parse);
    
    println!("=== ALL SYMBOLS ({}) ===", symbols.len());
    for sym in &symbols {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        for tr in &sym.type_refs {
            println!("    type_ref: {:?}", tr);
        }
    }
}
