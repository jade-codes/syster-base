//! Debug test for exact position matching in vehicle example

use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_symbols_unified};
use syster::syntax::parser::parse_content;

#[test]
#[ignore = "requires external file that may not exist"]
fn debug_position_line_524() {
    // Read the actual vehicle example file
    let source = std::fs::read_to_string(
        "/home/jade-codes/Work/syster-repos/syster-lsp/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml"
    ).expect("Failed to read vehicle example");

    // Target: Line 524, col 43-47, target='fuel'
    // Line text: "ref item redefines fuel{"
    let target_line: u32 = 524;
    let target_col: u32 = 43;

    // Parse and extract
    let parse = parse_content(&source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    // Build index
    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols.clone());
    index.ensure_visibility_maps();
    index.resolve_all_type_refs();

    // Show the actual line
    let lines: Vec<&str> = source.lines().collect();
    println!(
        "\n=== Line {} (0-indexed: {}) ===",
        target_line,
        target_line - 1
    );
    if let Some(line) = lines.get((target_line - 1) as usize) {
        println!("Content: '{}'", line);
        println!("Length: {} chars", line.len());
        // Show character positions
        if line.len() >= 50 {
            println!("Chars 40-50: '{}'", &line[40..50.min(line.len())]);
        }
    }

    // Show lines around it for context
    println!(
        "\n=== Context (lines {}-{}) ===",
        target_line - 3,
        target_line + 2
    );
    for i in (target_line - 3)..=(target_line + 2) {
        if let Some(line) = lines.get((i - 1) as usize) {
            println!("L{}: {}", i, line);
        }
    }

    // Find ALL type_refs containing 'fuel' on nearby lines
    println!("\n=== ALL 'fuel' type_refs on lines 517-524 (0-indexed) ===");
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if tr.target.as_ref() == "fuel" && tr.start_line >= 517 && tr.start_line <= 524 {
                    println!("\nSymbol: {}", sym.qualified_name);
                    println!(
                        "  line={} col={}-{} kind={:?} resolved={:?}",
                        tr.start_line, tr.start_col, tr.end_col, tr.kind, tr.resolved_target
                    );
                }
            }
        }
    }

    // Find all symbols that have type_refs covering this position
    // Note: The IDE uses 0-indexed lines, but the test reports 1-indexed
    let search_line = target_line - 1; // Convert to 0-indexed

    println!(
        "\n=== Symbols with type_refs at line {} (0-indexed), col {} ===",
        search_line, target_col
    );
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                // Check if this type_ref covers our position
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

    // Also search with 1-indexed line (in case there's inconsistency)
    println!(
        "\n=== Also checking with 1-indexed line {} ===",
        target_line
    );
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if tr.start_line == target_line
                    && tr.start_col <= target_col
                    && tr.end_col >= target_col
                {
                    println!("\nFOUND (1-indexed) in {}", sym.qualified_name);
                    println!("  type_ref: {:?}", tr);
                }
            }
        }
    }

    // Try hover at multiple positions around the target
    use syster::ide::hover;

    println!("\n=== Hover tests around target position ===");
    for line_offset in -1i32..=1 {
        for col_offset in -2i32..=2 {
            let test_line = (target_line as i32 + line_offset) as u32;
            let test_col = (target_col as i32 + col_offset) as u32;

            // hover uses 0-indexed
            let hover_result = hover(&index, FileId::new(0), test_line - 1, test_col);
            if let Some(result) = &hover_result {
                if let Some(qn) = &result.qualified_name {
                    // Only show if it found something
                    let is_target = line_offset == 0 && col_offset == 0;
                    let marker = if is_target { " <-- TARGET" } else { "" };
                    println!("Line {} Col {}: {}{}", test_line, test_col, qn, marker);
                }
            }
        }
    }

    // Direct check: what does find_type_ref_at_position return?
    use syster::ide::find_type_ref_at_position;

    println!(
        "\n=== find_type_ref_at_position at line {} col {} (0-indexed) ===",
        target_line - 1,
        target_col
    );
    if let Some(ctx) =
        find_type_ref_at_position(&index, FileId::new(0), target_line - 1, target_col)
    {
        println!("target_name: {}", ctx.target_name);
        println!("type_ref: {:?}", ctx.type_ref);
        if let Some(sym) = ctx.containing_symbol {
            println!("containing_symbol: {}", sym.qualified_name);
        }
    } else {
        println!("No type_ref found!");
    }
}
