// Test file for investigating vehicle chain resolution issues in perform statements without typing

use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_symbols_unified};
use syster::syntax::parser::parse_content;

#[test]
fn debug_vehicle_chain_resolution() {
    // Test case: vehicle_b inherits from Vehicle but the perform statement doesn't
    // have its own typing - it needs to find providePower from Vehicle's members
    let content = r#"package VehicleConfig {
    part def Vehicle {
        perform action providePower : ProvidePower {
            action distributeTorque : DistributeTorque;
        }
    }
    
    action def ProvidePower {
        action distributeTorque : DistributeTorque;
    }
    
    action def DistributeTorque;
    
    part vehicle_b : Vehicle {
        // This is a reference to providePower.distributeTorque from the supertype Vehicle
        perform providePower.distributeTorque;
    }
}"#;

    let syntax_file = parse_content(content, Path::new("test.sysml")).unwrap();

    let mut index = SymbolIndex::new();
    let symbols = extract_symbols_unified(FileId::new(0), &syntax_file);
    for symbol in symbols {
        index.insert(symbol);
    }
    index.ensure_visibility_maps();

    println!("\n=== Symbols with providePower ===");
    for sym in index.all_symbols() {
        if sym.name.contains("providePower") || sym.qualified_name.contains("providePower") {
            println!("{} ({:?})", sym.qualified_name, sym.kind);
            println!("  supertypes: {:?}", sym.supertypes);
            for tr in &sym.type_refs {
                println!("  type_ref: {:?}", tr);
            }
        }
    }

    println!("\n=== Symbols with distributeTorque ===");
    for sym in index.all_symbols() {
        if sym.name.contains("distributeTorque") || sym.qualified_name.contains("distributeTorque")
        {
            println!("{} ({:?})", sym.qualified_name, sym.kind);
            println!("  supertypes: {:?}", sym.supertypes);
        }
    }

    println!("\n=== Vehicle visibility map ===");
    if let Some(vis_map) = index.visibility_maps().get("VehicleConfig::Vehicle") {
        for (name, qname) in vis_map.direct_defs().take(15) {
            println!("  {} -> {}", name, qname);
        }
    }

    println!("\n=== vehicle_b visibility map ===");
    if let Some(vis_map) = index.visibility_maps().get("VehicleConfig::vehicle_b") {
        for (name, qname) in vis_map.direct_defs().take(15) {
            println!("  {} -> {}", name, qname);
        }
    }

    // Find the line with "perform providePower.distributeTorque;"
    // Line 15 (0-based) contains "        perform providePower.distributeTorque;"
    println!("\n=== Finding the target line ===");
    for (i, line) in content.lines().enumerate() {
        if line.contains("perform providePower.distributeTorque") {
            println!("Line {} (0-based): {}", i, line);
            // Find positions within the line
            if let Some(pos) = line.find("providePower") {
                println!("  providePower starts at col {}", pos);
            }
            if let Some(pos) = line.find("distributeTorque") {
                println!("  distributeTorque starts at col {}", pos);
            }
        }
    }

    let line = 15; // 0-based - the line with perform providePower.distributeTorque
    let col_provide = 16; // position of 'providePower' 
    let col_distribute = 29; // position of 'distributeTorque'

    use syster::ide::find_type_ref_at_position;

    println!(
        "\n=== Testing providePower resolution (line={}, col={}) ===",
        line, col_provide
    );
    let result1 = find_type_ref_at_position(&index, FileId::new(0), line, col_provide);
    println!("Result: {:?}", result1.is_some());

    println!(
        "\n=== Testing distributeTorque resolution (line={}, col={}) ===",
        line, col_distribute
    );
    let result2 = find_type_ref_at_position(&index, FileId::new(0), line, col_distribute);
    println!("Result: {:?}", result2.is_some());
}
