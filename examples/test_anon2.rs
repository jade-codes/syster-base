use std::path::Path;
use syster::base::FileId;
use syster::hir::extract_symbols_unified;
use syster::syntax::parser::parse_content;

fn main() {
    let source = r#"package Test {
    part def Hub;
    part def ShankCompositePort {
        port shankPort;
    }
    part hub1 : Hub {
        port :>> shankCompositePort : ShankCompositePort {
            port shankPort :>> shankPort [5];
        }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let symbols = extract_symbols_unified(FileId::new(0), &parse);

    for sym in &symbols {
        println!("\n=== Symbol: {} ===", sym.name);
        println!("Qualified: {}", sym.qualified_name);
        println!(
            "Symbol span: L{}:{} - L{}:{}",
            sym.start_line, sym.start_col, sym.end_line, sym.end_col
        );
        for tr in &sym.type_refs {
            println!("  Type ref: {:?}", tr);
        }
    }

    // Print the source with line/col markers
    println!("\n=== Source with positions ===");
    for (line_num, line) in source.lines().enumerate() {
        println!("L{}: {}", line_num, line);
    }
}
