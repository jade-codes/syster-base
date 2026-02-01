use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_with_filters};
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Actions {
    action def Action {
        action start;
        action done;
    }
}
package UseCases {
    use case def UseCase :> Actions::Action;
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

    // Check if start/done are in Actions::Action
    println!("=== All symbols ===");
    for sym in index.all_symbols() {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("    supertypes: {:?}", sym.supertypes);
        }
        for tr in &sym.type_refs {
            if let syster::hir::TypeRefKind::Simple(r) = tr {
                println!("    TypeRef: {} -> {:?}", r.target, r.resolved_target);
            }
        }
    }
}
