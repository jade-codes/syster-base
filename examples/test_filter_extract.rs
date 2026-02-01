use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_with_filters;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    filter @Safety and Safety::isMandatory;
}"#;
    let syntax = parse_content(source, Path::new("test.sysml")).unwrap();
    let result = extract_with_filters(FileId(1), &syntax);

    println!("=== Symbols ===");
    for sym in &result.symbols {
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

    println!("\n=== Scope filters ===");
    for (scope, filters) in &result.scope_filters {
        println!("  {}: {:?}", scope, filters);
    }
}
