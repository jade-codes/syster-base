//! Debug specific line 556 failure

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

#[test]
fn test_debug_line_556() {
    let file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    
    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();

    let mut host = create_host_with_stdlib();
    let _parse_errors = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    // Line 556 is 0-indexed as line 555
    let target_line = 555u32;
    
    let lines: Vec<&str> = content.lines().collect();
    println!("Line 556: {}", lines[target_line as usize]);
    
    // Check what the scope is for this line
    let scope = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b";
    println!("\nChecking resolution of 'mop' from scope {}:", scope);
    let resolver = analysis.symbol_index().resolver_for_scope(scope);
    let result = resolver.resolve("mop");
    println!("  resolve('mop') = {:?}", result);
    
    // Also check from VehicleConfiguration_b where the import is
    let parent_scope = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b";
    println!("\nChecking resolution of 'mop' from parent scope {}:", parent_scope);
    let parent_resolver = analysis.symbol_index().resolver_for_scope(parent_scope);
    let parent_result = parent_resolver.resolve("mop");
    println!("  resolve('mop') = {:?}", parent_result);
    
    // Check the import symbol details
    println!("\nImport symbol details:");
    let import_qn = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::import:ParametersOfInterestMetadata::mop";
    if let Some(sym) = analysis.symbol_index().lookup_qualified(import_qn) {
        println!("  name: {}", sym.name);
        println!("  qualified_name: {}", sym.qualified_name);
        println!("  kind: {:?}", sym.kind);
        println!("  is_public: {}", sym.is_public);
    }
    
    // Check if ParametersOfInterestMetadata resolves
    println!("\nChecking resolution of 'ParametersOfInterestMetadata' from parent scope:");
    let pom_result = parent_resolver.resolve("ParametersOfInterestMetadata");
    println!("  resolve('ParametersOfInterestMetadata') = {:?}", pom_result);
    
    // Check qualified lookup
    println!("\nQualified lookup 'ParametersOfInterestMetadata::MeasureOfPerformance':");
    if let Some(sym) = analysis.symbol_index().lookup_qualified("ParametersOfInterestMetadata::MeasureOfPerformance") {
        println!("  Found: {} (short_name={:?})", sym.qualified_name, sym.short_name);
    } else {
        println!("  Not found");
    }
    
    // Check if mop symbol exists in the index
    println!("\nSearching for 'mop' in symbol index:");
    for sym in analysis.symbol_index().all_symbols() {
        if sym.name.contains("mop") || sym.qualified_name.contains("mop") {
            println!("  Found: {} (short_name={:?})", sym.qualified_name, sym.short_name);
        }
        if sym.short_name.as_ref().map(|s| s.as_ref()).unwrap_or("") == "mop" {
            println!("  Found by short_name: {} (short_name={:?})", sym.qualified_name, sym.short_name);
        }
    }
    
    println!("\nHover scan for #mop (cols 20-30):");
    for col in 20..30 {
        if let Some(hover) = analysis.hover(file_id, target_line, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            } else {
                println!("  col {}: hover but no qn", col);
            }
        } else {
            println!("  col {}: NO HOVER", col);
        }
    }
    
    println!("\nFull hover scan:");
    for col in 0..100 {
        if let Some(hover) = analysis.hover(file_id, target_line, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // Find what character is at what column
    println!("\nCharacter positions:");
    for (i, c) in lines[target_line as usize].char_indices() {
        if c.is_alphabetic() || c == '#' {
            print!("{}@{} ", c, i);
        }
    }
    println!();
}
