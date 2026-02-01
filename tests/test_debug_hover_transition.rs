//! Debug test for why hover returns None on transition source/target
//!
//! This test mimics what the LSP server does and shows where the disconnect is.

use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, TypeRefKind, extract_symbols_unified};
use syster::parser::SyntaxNode;
use syster::syntax::parser::parse_content;

fn print_tree(node: &SyntaxNode, indent: usize) {
    let kind = node.kind();
    let text = if node.children().next().is_none() {
        format!(" {:?}", node.text())
    } else {
        String::new()
    };
    println!("{}{:?}{}", "  ".repeat(indent), kind, text);
    for child in node.children() {
        print_tree(&child, indent + 1);
    }
}

#[test]
fn debug_hover_on_transition_initial() {
    // Mimics SimpleVehicleModel.sysml line 54 indentation
    let source = r#"package Test {
                        state def VehicleStates {
                            state off;
                            transition initial then off;
                        }
                    }
"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();

    // Print the AST structure
    println!("\n=== AST Structure ===");
    print_tree(&parse.parse().syntax(), 0);

    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== All symbols with type_refs ===");
    for sym in &symbols {
        println!("\n{} ({:?})", sym.qualified_name, sym.kind);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("  type_ref[{}]: {:?}", i, tr);
        }
    }

    // Now create SymbolIndex and try to find type_ref at position
    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols.clone());

    // Find the line/col of "initial" in the source
    let lines: Vec<&str> = source.lines().collect();
    let mut target_line = 0;
    let mut target_col = 0;
    for (i, line) in lines.iter().enumerate() {
        if let Some(pos) = line.find("initial") {
            target_line = i as u32;
            target_col = pos as u32;
            println!(
                "\nFound 'initial' at line {} col {} in: {}",
                target_line,
                target_col,
                line.trim()
            );
            break;
        }
    }

    // Now try find_type_ref_at_position
    use syster::ide::find_type_ref_at_position;
    let result = find_type_ref_at_position(&index, FileId::new(0), target_line, target_col);

    let found = result.is_some();
    println!(
        "\nfind_type_ref_at_position(line={}, col={}) = {:?}",
        target_line, target_col, found
    );
    if let Some(ctx) = result {
        println!("  target: {}", ctx.target_name);
        println!("  type_ref: {:?}", ctx.type_ref);
        println!(
            "  symbol: {:?}",
            ctx.containing_symbol.map(|s| &s.qualified_name)
        );
    } else {
        // Debug: print all type_refs to see their line/col
        println!("\nNo match found. All type_refs in index:");
        for sym in index.symbols_in_file(FileId::new(0)) {
            for tr in sym.type_refs.iter() {
                match tr {
                    TypeRefKind::Simple(r) => {
                        println!(
                            "  {} has Simple ref '{}' at line {} col {}-{}",
                            sym.qualified_name, r.target, r.start_line, r.start_col, r.end_col
                        );
                    }
                    TypeRefKind::Chain(chain) => {
                        for part in &chain.parts {
                            println!(
                                "  {} has Chain part '{}' at line {} col {}-{}",
                                sym.qualified_name,
                                part.target,
                                part.start_line,
                                part.start_col,
                                part.end_col
                            );
                        }
                    }
                }
            }
        }
    }

    assert!(found, "Should find type_ref for 'initial'");
}
