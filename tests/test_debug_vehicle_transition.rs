//! Debug test for vehicle example transition refs

use std::fs;
use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, TypeRefKind, extract_symbols_unified};
use syster::syntax::parser::parse_content;

#[test]
#[ignore = "requires external file that may not exist"]
fn debug_vehicle_example_transition_refs() {
    let file_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../syster-lsp/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml");

    let source = fs::read_to_string(&file_path).expect("Should read file");

    let parse = parse_content(&source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    // Find type_refs for 'initial'
    println!("\n=== Type refs targeting 'initial' ===");
    for sym in &symbols {
        for tr in sym.type_refs.iter() {
            match tr {
                TypeRefKind::Simple(r) if r.target.as_ref() == "initial" => {
                    println!(
                        "Symbol '{}' has ref to 'initial' at line {} col {}-{}",
                        sym.qualified_name, r.start_line, r.start_col, r.end_col
                    );
                }
                TypeRefKind::Chain(chain) => {
                    for part in &chain.parts {
                        if part.target.as_ref() == "initial" {
                            println!(
                                "Symbol '{}' has chain ref to 'initial' at line {} col {}-{}",
                                sym.qualified_name, part.start_line, part.start_col, part.end_col
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Build index and test find_type_ref_at_position at line 53 (0-indexed), col 35
    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols);

    // Line 54 (1-indexed) = line 53 (0-indexed)
    // Col 35 (0-indexed)
    use syster::ide::find_type_ref_at_position;
    let result = find_type_ref_at_position(&index, FileId::new(0), 53, 35);

    let found = result.is_some();
    println!("\nfind_type_ref_at_position(line=53, col=35) = {:?}", found);
    if let Some(ctx) = &result {
        println!(
            "  Found: target='{}', symbol='{:?}'",
            ctx.target_name,
            ctx.containing_symbol.map(|s| &s.qualified_name)
        );
    } else {
        println!("  NOT FOUND - dumping all type_refs around line 53:");
        for sym in index.symbols_in_file(FileId::new(0)) {
            for tr in sym.type_refs.iter() {
                match tr {
                    TypeRefKind::Simple(r) if r.start_line >= 50 && r.start_line <= 56 => {
                        println!(
                            "    {} has '{}' at line {} col {}-{}",
                            sym.qualified_name, r.target, r.start_line, r.start_col, r.end_col
                        );
                    }
                    TypeRefKind::Chain(chain) => {
                        for part in &chain.parts {
                            if part.start_line >= 50 && part.start_line <= 56 {
                                println!(
                                    "    {} has chain '{}' at line {} col {}-{}",
                                    sym.qualified_name,
                                    part.target,
                                    part.start_line,
                                    part.start_col,
                                    part.end_col
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    assert!(
        found,
        "Should find type_ref for 'initial' at line 53 col 35"
    );
}
