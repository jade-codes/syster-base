use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_with_filters;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    #Safety {
        attribute isMandatory : Boolean;
    }
    filter @Safety and Safety::isMandatory;
}"#;
    let syntax = parse_content(source, Path::new("test.sysml")).unwrap();
    let result = extract_with_filters(FileId(1), &syntax);

    println!("=== All Symbols ===");
    for sym in &result.symbols {
        println!("  qname='{}' kind={:?}", sym.qualified_name, sym.kind);
    }
}
