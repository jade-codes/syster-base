#[test]
fn debug_expression_qualified_names() {
    use syster::ide::AnalysisHost;

    let input = r#"
package Test {
    part def Vehicle {
        part fuelTank { attribute mass; }
        attribute partMasses = (fuelTank.mass);
    }
}
"#;

    // Set up analysis host
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", input);
    println!("Parse errors: {:?}", errors);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");

    println!("\n=== All symbols ===");
    for symbol in analysis.symbol_index().all_symbols() {
        println!(
            "  {} ({}) at {}:{}-{}:{}",
            symbol.qualified_name,
            symbol.kind.display(),
            symbol.start_line,
            symbol.start_col,
            symbol.end_line,
            symbol.end_col
        );

        if !symbol.type_refs.is_empty() {
            println!("    type_refs:");
            for tr in &symbol.type_refs {
                println!("      {:?}", tr);
            }
        }
    }

    // Find position of fuelTank in the expression
    let lines: Vec<&str> = input.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        println!("Line {}: {}", i, line);
    }

    // Line 4 (0-indexed): "        attribute partMasses = (fuelTank.mass);"
    // fuelTank starts around col 33
    // The type_ref says start_col: 32, end_col: 45
    let line = 4u32;
    let col = 33u32;
    println!("\n=== Hover at line {} col {} ===", line, col);

    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("Hover contents:\n{}", hover.contents);
        println!("Qualified name: {:?}", hover.qualified_name);
    } else {
        println!("No hover result!");
    }
}

#[test]
fn debug_redefines_vehicle_indentation() {
    use syster::ide::AnalysisHost;

    // Test with vehicle file indentation (24 spaces)
    let input = r#"package Test {
    item def Fuel;
    
    part def FuelTank {
        item fuel : Fuel;
    }
    
    part def SpecialTank :> FuelTank {
                        ref item redefines fuel{
                        }
    }
}
"#;

    // Set up analysis host
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", input);
    println!("Parse errors: {:?}", errors);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");

    println!("\n=== All symbols with type_refs ===");
    for symbol in analysis.symbol_index().all_symbols() {
        if !symbol.type_refs.is_empty() && symbol.qualified_name.contains("fuel") {
            println!(
                "  {} ({}) at {}:{}-{}:{}",
                symbol.qualified_name,
                symbol.kind.display(),
                symbol.start_line,
                symbol.start_col,
                symbol.end_line,
                symbol.end_col
            );
            println!("    type_refs:");
            for tr in &symbol.type_refs {
                println!("      {:?}", tr);
            }
        }
    }

    // Find position of fuel
    let lines: Vec<&str> = input.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("redefines") {
            println!("Line {}: {}", i, line);
        }
    }

    // Line 8: "                        ref item redefines fuel{"
    // "fuel" should start at col 43
    let line = 8u32;
    let col = 43u32;
    println!("\n=== Hover at line {} col {} ===", line, col);

    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("Hover contents:\n{}", hover.contents);
        println!("Qualified name: {:?}", hover.qualified_name);
    } else {
        println!("No hover result!");
    }
}

#[test]
fn debug_perform_chain() {
    use syster::ide::AnalysisHost;

    // Simplified version based on actual SimpleVehicleModel.sysml structure
    let input = r#"
package Test {
    action def ProvidePower {
        action distributeTorque;
    }
    
    part def Vehicle {
        action providePower : ProvidePower;
        
        part def AxleAssembly {
            perform providePower.distributeTorque;
        }
    }
}
"#;

    // Set up analysis host
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", input);
    println!("Parse errors: {:?}", errors);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");

    println!("\n=== All symbols ===");
    for symbol in analysis.symbol_index().all_symbols() {
        println!(
            "  {} ({}) at {}:{}-{}:{}",
            symbol.qualified_name,
            symbol.kind.display(),
            symbol.start_line,
            symbol.start_col,
            symbol.end_line,
            symbol.end_col
        );

        if !symbol.type_refs.is_empty() {
            println!("    type_refs:");
            for tr in &symbol.type_refs {
                println!("      {:?}", tr);
            }
        }
    }

    // Find position of distributeTorque in the perform
    let lines: Vec<&str> = input.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        println!("Line {}: {}", i, line);
    }

    // Line 10 (0-indexed): "            perform providePower.distributeTorque;"
    // Let me manually count the columns:
    //             perform providePower.distributeTorque;
    // 0         1         2         3         4         5
    // 0123456789012345678901234567890123456789012345678901
    //             perform providePower.distributeTorque;
    // 'p' at 12, 'r' at 20, '.' at 32, 'd' at 33, 'e' at 49

    // Test exact col of 'd' in distributeTorque
    let line = 10u32;
    let col = 33u32; // start of distributeTorque
    println!("\n=== Hover at line {} col {} ===", line, col);

    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("Hover contents:\n{}", hover.contents);
        println!("Qualified name: {:?}", hover.qualified_name);
    } else {
        println!("No hover result!");
    }

    // Also test col 35 (middle of distributeTorque)
    let col = 35u32;
    println!("\n=== Hover at line {} col {} ===", line, col);

    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("Hover contents:\n{}", hover.contents);
        println!("Qualified name: {:?}", hover.qualified_name);
    } else {
        println!("No hover result!");
    }
}

#[test]
fn debug_perform_chain_vehicle_indentation() {
    use syster::ide::AnalysisHost;

    // This matches the actual indentation in SimpleVehicleModel.sysml line 600
    // 24 spaces before perform
    let input = r#"package SimpleVehicleModel {
    package VehicleConfigurations {
        package VehicleConfiguration_b {
            package PartsTree {
                part vehicle_b : Vehicle {
                    part rearAxleAssembly : RearAxleAssembly {
                        perform providePower.distributeTorque;
                    }
                }
            }
        }
    }
    
    action def ProvidePower {
        action distributeTorque;
    }
    
    part def Vehicle {
        action providePower : ProvidePower;
    }
    
    part def RearAxleAssembly;
}
"#;

    // Set up analysis host
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", input);
    println!("Parse errors: {:?}", errors);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");

    // Find all type_refs for symbols containing "rearAxleAssembly"
    println!("\n=== Looking for rearAxleAssembly symbols ===");
    for symbol in analysis.symbol_index().all_symbols() {
        if symbol.qualified_name.contains("rearAxleAssembly")
            || symbol.qualified_name.contains("RearAxleAssembly")
        {
            println!(
                "  {} ({}) at {}:{}-{}:{}",
                symbol.qualified_name,
                symbol.kind.display(),
                symbol.start_line,
                symbol.start_col,
                symbol.end_line,
                symbol.end_col
            );

            if !symbol.type_refs.is_empty() {
                println!("    type_refs:");
                for tr in &symbol.type_refs {
                    println!("      {:?}", tr);
                }
            }
        }
    }

    // Find position of distributeTorque in the perform
    let lines: Vec<&str> = input.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("perform") || line.contains("providePower") {
            println!("Line {}: {}", i, line);
        }
    }

    // Line 6 (0-indexed): "                        perform providePower.distributeTorque;"
    // 24 spaces + "perform " = 32, "providePower" = 32-44, "." = 44, "distributeTorque" = 45-61
    let line = 6u32;

    // Test col 45 (start of distributeTorque after deep indentation)
    let col = 45u32;
    println!("\n=== Hover at line {} col {} ===", line, col);

    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("Hover contents:\n{}", hover.contents);
        println!("Qualified name: {:?}", hover.qualified_name);
    } else {
        println!("No hover result!");
    }
}
