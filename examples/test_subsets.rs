use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    part def Cylinder;
    part def Engine {
        part cylinders: Cylinder[4..8];
    }
    part engine4Cyl :> Engine {
        part cylinder1 subsets cylinders[1];
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &parse);

    for sym in &symbols {
        if sym.name.contains("cylinder") {
            println!("\n=== Symbol: {} ===", sym.name);
            println!("Qualified: {}", sym.qualified_name);
            for tr in &sym.type_refs {
                println!("Type ref: {:?}", tr);
            }
        }
    }

    // Print the source with line/col markers
    println!("\n=== Source with positions ===");
    for (line_num, line) in source.lines().enumerate() {
        println!("L{}: {}", line_num, line);
    }
}
