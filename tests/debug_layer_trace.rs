//! Debug test: Trace `IgnitionOnOff` through all layers
//!
//! Target: Line 59 in SimpleVehicleModel.sysml
//! ```sysml
//! enum def IgnitionOnOff {on;off;}
//! ```
//!
//! And its usage in the transition (line ~58):
//! ```sysml
//! if ignitionCmd.ignitionOnOff==IgnitionOnOff::on
//! ```
//!
//! We need to verify each layer:
//! 1. Parser - does `enum def IgnitionOnOff` parse?
//! 2. AST - what AST node is produced?
//! 3. Extractor - is `IgnitionOnOff` extracted as a symbol?
//! 4. Resolver - can we resolve `IgnitionOnOff::on`?

use syster::ide::AnalysisHost;
use syster::parser::{SyntaxNode, parse_sysml};

fn print_tree(node: &SyntaxNode, indent: usize) {
    let kind = node.kind();
    let text_preview: String = node.text().to_string().chars().take(80).collect();
    let text_preview = text_preview.replace('\n', "\\n").replace('\r', "");
    println!(
        "{:indent$}{:?}: '{}'",
        "",
        kind,
        text_preview,
        indent = indent
    );
    for child in node.children() {
        print_tree(&child, indent + 2);
    }
}

#[test]
fn test_layer_1_parser_enum() {
    println!("\n========== LAYER 1: PARSER - ENUM ==========\n");

    let source = "enum def IgnitionOnOff { on; off; }";
    println!("Source: {}\n", source);

    let parsed = parse_sysml(source);

    // Check for parse errors
    if parsed.errors.is_empty() {
        println!("✓ No parse errors");
    } else {
        println!("✗ Parse errors:");
        for err in &parsed.errors {
            println!("  - {:?}", err);
        }
    }

    // Print full CST
    println!("\nFull CST:");
    print_tree(&parsed.syntax(), 0);
}

#[test]
fn test_layer_1_parser_struct() {
    println!("\n========== LAYER 1: PARSER - STRUCT ==========\n");

    let source = "struct IgnitionCmd { attribute ignitionOnOff: IgnitionOnOff; }";
    println!("Source: {}\n", source);

    let parsed = parse_sysml(source);

    // Check for parse errors
    if parsed.errors.is_empty() {
        println!("✓ No parse errors");
    } else {
        println!("✗ Parse errors:");
        for err in &parsed.errors {
            println!("  - {:?}", err);
        }
    }

    // Print full CST
    println!("\nFull CST:");
    print_tree(&parsed.syntax(), 0);
}

#[test]
fn test_layer_1_parser_item_def() {
    println!("\n========== LAYER 1: PARSER - ITEM DEF ==========\n");

    let source = "item def IgnitionCmd { attribute ignitionOnOff: IgnitionOnOff; }";
    println!("Source: {}\n", source);

    let parsed = parse_sysml(source);

    // Check for parse errors
    if parsed.errors.is_empty() {
        println!("✓ No parse errors");
    } else {
        println!("✗ Parse errors:");
        for err in &parsed.errors {
            println!("  - {:?}", err);
        }
    }

    // Print full CST
    println!("\nFull CST:");
    print_tree(&parsed.syntax(), 0);
}

#[test]
fn test_layer_2_ast_enum_kinds() {
    println!("\n========== LAYER 2: AST - LOOKING FOR ENUM SYNTAX KINDS ==========\n");

    let source = "enum def IgnitionOnOff { on; off; }";
    let parsed = parse_sysml(source);

    // Find all unique SyntaxKinds
    fn collect_kinds(node: &SyntaxNode, kinds: &mut std::collections::HashSet<String>) {
        kinds.insert(format!("{:?}", node.kind()));
        for child in node.children() {
            collect_kinds(&child, kinds);
        }
    }

    let mut kinds = std::collections::HashSet::new();
    collect_kinds(&parsed.syntax(), &mut kinds);

    println!("All SyntaxKinds in enum parse:");
    let mut kinds_vec: Vec<_> = kinds.iter().collect();
    kinds_vec.sort();
    for kind in kinds_vec {
        println!("  {}", kind);
    }

    // Specifically look for ENUM-related kinds
    println!("\nENUM-related kinds:");
    for kind in &kinds {
        if kind.to_lowercase().contains("enum") {
            println!("  FOUND: {}", kind);
        }
    }
}

#[test]
fn test_layer_3_extractor_enum() {
    println!("\n========== LAYER 3: EXTRACTOR - ENUM ==========\n");

    let source = r#"
package Test {
    enum def IgnitionOnOff { on; off; }
    
    item def IgnitionCmd {
        attribute ignitionOnOff: IgnitionOnOff;
    }
}
"#;

    println!("Source:\n{}\n", source);

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let index = analysis.symbol_index();

    println!("Extracted symbols:");
    for sym in index.all_symbols() {
        println!("  {} -> {:?}", sym.qualified_name, sym.kind);
    }

    // Look for IgnitionOnOff
    println!("\n--- Searching for 'IgnitionOnOff' ---");
    let mut found = false;
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("IgnitionOnOff") {
            println!("  ✓ Found: {}", sym.qualified_name);
            found = true;
        }
    }
    if !found {
        println!("  ✗ NOT FOUND");
    }

    // Look for enum variants
    println!("\n--- Searching for enum variants 'on' and 'off' ---");
    for variant in ["on", "off"] {
        let mut found = false;
        for sym in index.all_symbols() {
            if sym.qualified_name.ends_with(&format!("::{}", variant)) {
                println!("  ✓ Found variant '{}': {}", variant, sym.qualified_name);
                found = true;
            }
        }
        if !found {
            println!("  ✗ Variant '{}' NOT FOUND", variant);
        }
    }
}

#[test]
fn test_layer_4_full_file() {
    println!("\n========== LAYER 4: FULL FILE - VEHICLE EXAMPLE ==========\n");

    use std::path::Path;

    let example_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml",
    );

    if !example_path.exists() {
        println!("Example file not found, skipping");
        return;
    }

    let content = std::fs::read_to_string(&example_path).unwrap();
    let path_str = example_path.to_string_lossy().to_string();

    let mut host = AnalysisHost::new();
    let parse_errors = host.set_file_content(&path_str, &content);

    println!("Parse errors: {}", parse_errors.len());
    for err in parse_errors.iter().take(5) {
        println!("  {:?}", err);
    }

    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Search for IgnitionOnOff
    println!("\n--- Symbols containing 'IgnitionOnOff' ---");
    let mut count = 0;
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("IgnitionOnOff") {
            println!("  {} -> {:?}", sym.qualified_name, sym.kind);
            count += 1;
        }
    }
    println!("Total: {}", count);

    // Search for IgnitionCmd
    println!("\n--- Symbols containing 'IgnitionCmd' ---");
    count = 0;
    for sym in index.all_symbols() {
        if sym.qualified_name.contains("IgnitionCmd") {
            println!("  {} -> {:?}", sym.qualified_name, sym.kind);
            count += 1;
        }
    }
    println!("Total: {}", count);

    // Search for all enum-like symbols
    println!("\n--- All EnumDef symbols ---");
    for sym in index.all_symbols() {
        let kind_str = format!("{:?}", sym.kind);
        if kind_str.contains("Enum") {
            println!("  {} -> {:?}", sym.qualified_name, sym.kind);
        }
    }
}
