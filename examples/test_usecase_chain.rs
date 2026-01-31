use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_with_filters};
use syster::syntax::parser::parse_content;
use std::path::Path;

fn main() {
    // Simplified version of stdlib inheritance chain
    let source = r#"package Performances {
    abstract action def Performance;
}
package Actions {
    action def Action :> Performances::Performance {
        action start;
        action done;
    }
}
package Calculations {
    calc def Calculation :> Actions::Action;
}
package Cases {
    case def Case :> Calculations::Calculation;
}
package UseCases {
    use case def UseCase :> Cases::Case;
}
use case def MyUseCase {
    first start;
    then done;
}"#;
    let syntax = parse_content(source, Path::new("test.sysml")).unwrap();
    
    let mut index = SymbolIndex::new();
    index.add_extraction_result(FileId(1), extract_with_filters(FileId(1), &syntax));
    
    index.ensure_visibility_maps();
    index.resolve_all_type_refs();
    
    println!("=== MyUseCase symbols ===");
    for sym in index.all_symbols() {
        if sym.qualified_name.starts_with("MyUseCase") {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            println!("    supertypes: {:?}", sym.supertypes);
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        println!("    TypeRef: {} -> {:?}", r.target, r.resolved_target);
                    }
                    _ => {}
                }
            }
        }
    }
}
