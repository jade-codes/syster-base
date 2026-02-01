use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_with_filters};
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    #Safety {
        attribute isMandatory : Boolean;
    }
    filter @Safety and Safety::isMandatory;
}"#;
    let syntax = parse_content(source, Path::new("test.sysml")).unwrap();

    let mut index = SymbolIndex::new();
    index.add_extraction_result(FileId(1), extract_with_filters(FileId(1), &syntax));

    index.ensure_visibility_maps();
    index.resolve_all_type_refs();

    println!("=== Symbols ===");
    for sym in index.all_symbols() {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            println!("    type_refs:");
            for tr in &sym.type_refs {
                if let syster::hir::TypeRefKind::Simple(r) = tr {
                    println!(
                        "      {} @ ({},{}) -> {:?}",
                        r.target, r.start_line, r.start_col, r.resolved_target
                    );
                }
            }
        }
    }
}
