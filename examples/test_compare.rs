use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
#[allow(unused_imports)]
use syster::syntax::parser::parse_with_result;

fn main() {
    let text = r#"package Test {
    action def ProvidePower {
        action distributeTorque;
    }
    part x {
        perform providePower.distributeTorque;
        action providePower : ProvidePower;
    }
}"#;

    let result = parse_with_result(text, Path::new("test.sysml"));
    let syntax = result.content.unwrap();
    let file_id = FileId::new(0);

    // Unified extraction
    let symbols = extract_symbols_unified(file_id, &syntax);

    println!("\n=== UNIFIED EXTRACTION ===");
    for sym in &symbols {
        println!("Symbol: {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for trk in &sym.type_refs {
                for tr in trk.as_refs() {
                    println!(
                        "    '{}' at {}:{}-{}:{}",
                        tr.target, tr.start_line, tr.start_col, tr.end_line, tr.end_col
                    );
                }
            }
        }
    }
}
