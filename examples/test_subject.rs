use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    analysis engineTradeOffAnalysis : TradeStudy {
        subject vehicleAlternatives [2] :> vehicle_b;
        
        part vehicle_b_engine4cyl :> vehicleAlternatives {
        }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &parse);

    println!("=== All symbols ===");
    for sym in &symbols {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("    supertypes: {:?}", sym.supertypes);
        }
    }
}
