//! Debug test to understand hover failures

#[test]
fn test_hover_at_specific_position() {
    use syster::ide::AnalysisHost;
    use std::path::PathBuf;
    
    // Load the actual vehicle file
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("syster-lsp/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml");
    
    let source = std::fs::read_to_string(&file_path)
        .expect("Should be able to read SimpleVehicleModel.sysml");
    
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", &source);
    println!("Parse errors: {}", errors.len());
    
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    
    // Line 600 (1-indexed) = line 599 (0-indexed)
    // perform providePower.distributeTorque;
    //                      ^ col 45 = distributeTorque
    let line = 599u32;
    let col = 45u32;
    
    println!("\n=== Hover at line {} col {} (distributeTorque) ===", line + 1, col);
    if let Some(hover) = analysis.hover(file_id, line, col) {
        println!("SUCCESS! Hover result:\n{}", hover.contents);
    } else {
        println!("FAILURE - NO HOVER RESULT");
        
        // Debug: check what providePower.distributeTorque resolves to now
        println!("\n=== Checking chain resolution for providePower.distributeTorque ===");
        for symbol in analysis.symbol_index().all_symbols() {
            if symbol.qualified_name.contains("rearAxleAssembly") && !symbol.qualified_name.contains("<ref:") {
                for trk in &symbol.type_refs {
                    if let syster::hir::TypeRefKind::Chain(chain) = trk {
                        if chain.parts.first().map(|p| p.target.as_ref()) == Some("providePower") {
                            println!("Chain in {}: {:?}", symbol.qualified_name, chain);
                        }
                    }
                }
            }
        }
    }
}
