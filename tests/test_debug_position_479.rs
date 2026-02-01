//! Debug test for specializes pattern at line 479

use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_symbols_unified};
use syster::syntax::parser::parse_content;

#[test]
#[ignore = "requires external file that may not exist"]
fn debug_position_line_479() {
    let source = std::fs::read_to_string(
        "/home/jade-codes/Work/syster-repos/syster-lsp/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml"
    ).expect("Failed to read vehicle example");

    // Target: Line 479, col 80-85, target='mRefs'
    let target_line: u32 = 479;
    let target_col: u32 = 80;

    let parse = parse_content(&source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols.clone());
    index.ensure_visibility_maps();
    index.resolve_all_type_refs();

    let lines: Vec<&str> = source.lines().collect();
    println!(
        "\n=== Line {} (0-indexed: {}) ===",
        target_line,
        target_line - 1
    );
    if let Some(line) = lines.get((target_line - 1) as usize) {
        println!("Content: '{}'", line);
    }

    // Find ALL type_refs containing 'mRefs'
    println!("\n=== ALL 'mRefs' type_refs ===");
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if tr.target.as_ref() == "mRefs" {
                    println!("\nSymbol: {}", sym.qualified_name);
                    println!(
                        "  line={} col={}-{} kind={:?} resolved={:?}",
                        tr.start_line, tr.start_col, tr.end_col, tr.kind, tr.resolved_target
                    );
                }
            }
        }
    }

    let search_line = target_line - 1;
    println!(
        "\n=== Symbols with type_refs at line {} col {} ===",
        search_line, target_col
    );
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if tr.start_line == search_line
                    && tr.start_col <= target_col
                    && tr.end_col >= target_col
                {
                    println!("\nFOUND in {}", sym.qualified_name);
                    println!("  type_ref: {:?}", tr);
                }
            }
        }
    }

    use syster::ide::find_type_ref_at_position;

    println!(
        "\n=== find_type_ref_at_position at line {} col {} ===",
        search_line, target_col
    );
    if let Some(ctx) = find_type_ref_at_position(&index, FileId::new(0), search_line, target_col) {
        println!("target_name: {}", ctx.target_name);
        println!("type_ref: {:?}", ctx.type_ref);
        if let Some(sym) = ctx.containing_symbol {
            println!("containing_symbol: {}", sym.qualified_name);
        }
    } else {
        println!("No type_ref found!");
    }

    use syster::ide::hover;
    let hover_result = hover(&index, FileId::new(0), search_line, target_col);
    println!(
        "\nHover result: {:?}",
        hover_result.as_ref().map(|r| &r.qualified_name)
    );
}
