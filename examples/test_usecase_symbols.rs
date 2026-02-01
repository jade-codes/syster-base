use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"use case def MyUseCase {
    item def Request;
    first start;
    then action doSomething;
    then done;
}"#;
    let syntax = parse_content(source, Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId(1), &syntax);

    println!("=== Symbols ===");
    for sym in &symbols {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            println!("    type_refs:");
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        println!("      Simple: {} ({:?})", r.target, r.kind);
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        println!(
                            "      Chain: {:?}",
                            c.parts
                                .iter()
                                .map(|p| p.target.as_ref())
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
        if !sym.supertypes.is_empty() {
            println!("    supertypes: {:?}", sym.supertypes);
        }
    }
}
