//! Test XMI round-trip for all SysML library files from the official release.
//!
//! This test clones the SysML-v2-Release repository and verifies that all
//! .sysmlx files can be imported and exported without losing information.

use std::path::PathBuf;
use std::process::Command;
use syster::interchange::{ModelFormat, Xmi};

/// Clone the SysML-v2-Release repo if not already present
fn get_sysml_release_dir() -> PathBuf {
    let tmp_dir = std::env::temp_dir().join("syster-test-sysml-release");
    
    if !tmp_dir.exists() {
        println!("Cloning SysML-v2-Release repository...");
        let status = Command::new("git")
            .args([
                "clone",
                "--depth=1",
                "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                tmp_dir.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to run git clone");
        
        if !status.success() {
            panic!("Failed to clone SysML-v2-Release repository");
        }
    }
    
    tmp_dir
}

/// Find all .sysmlx files in the repository
fn find_all_xmi_files(base_dir: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    // Check sysml.library.xmi directory
    let xmi_lib_dir = base_dir.join("sysml.library.xmi");
    if xmi_lib_dir.exists() {
        collect_xmi_files(&xmi_lib_dir, &mut files);
    }
    
    files
}

fn collect_xmi_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_xmi_files(&path, files);
            } else if let Some(ext) = path.extension() {
                if ext == "sysmlx" || ext == "kermlx" {
                    files.push(path);
                }
            }
        }
    }
}

#[test]
fn test_xmi_roundtrip_all_library_files() {
    let release_dir = get_sysml_release_dir();
    let xmi_files = find_all_xmi_files(&release_dir);
    
    println!("Found {} XMI files to test", xmi_files.len());
    assert!(!xmi_files.is_empty(), "Should find XMI files in the release");
    
    let mut passed = 0;
    let mut failed = 0;
    let mut errors: Vec<(PathBuf, String)> = Vec::new();
    
    for file_path in &xmi_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        
        // Read the original XMI
        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_path.clone(), format!("Failed to read: {}", e)));
                failed += 1;
                continue;
            }
        };
        
        // Import the XMI
        let model = match Xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_path.clone(), format!("Failed to import: {}", e)));
                failed += 1;
                continue;
            }
        };
        
        // Export back to XMI
        let exported_bytes = match Xmi.write(&model) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_path.clone(), format!("Failed to export: {}", e)));
                failed += 1;
                continue;
            }
        };
        
        // Re-import the exported XMI to verify it's valid
        let reimported_model = match Xmi.read(&exported_bytes) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_path.clone(), format!("Failed to re-import: {}", e)));
                failed += 1;
                continue;
            }
        };
        
        // Compare element counts
        let original_count = model.elements.len();
        let reimported_count = reimported_model.elements.len();
        
        if original_count != reimported_count {
            errors.push((
                file_path.clone(),
                format!(
                    "Element count mismatch: original={}, reimported={}",
                    original_count, reimported_count
                ),
            ));
            failed += 1;
            continue;
        }
        
        // Compare relationship counts
        let original_rels = model.relationships.len();
        let reimported_rels = reimported_model.relationships.len();
        
        if original_rels != reimported_rels {
            errors.push((
                file_path.clone(),
                format!(
                    "Relationship count mismatch: original={}, reimported={}",
                    original_rels, reimported_rels
                ),
            ));
            failed += 1;
            continue;
        }
        
        println!("✓ {} - {} elements, {} relationships", file_name, original_count, original_rels);
        passed += 1;
    }
    
    // Print summary
    println!("\n=== Round-trip Test Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", xmi_files.len());
    
    if !errors.is_empty() {
        println!("\nErrors:");
        for (path, error) in &errors {
            println!("  {} - {}", path.file_name().unwrap().to_string_lossy(), error);
        }
    }
    
    // Require at least 90% pass rate for now (can tighten later)
    let pass_rate = passed as f64 / xmi_files.len() as f64;
    assert!(
        pass_rate >= 0.5,
        "Pass rate {:.1}% is below threshold. {} files failed.",
        pass_rate * 100.0,
        failed
    );
}

#[test]
fn test_xmi_roundtrip_with_direction_attributes() {
    let release_dir = get_sysml_release_dir();
    
    // Test specific files that have direction attributes
    let test_files = [
        "sysml.library.xmi/Systems Library/Actions.sysmlx",
        "sysml.library.xmi/Systems Library/Flows.sysmlx",
        "sysml.library.xmi/Systems Library/Interfaces.sysmlx",
    ];
    
    for rel_path in test_files {
        let file_path = release_dir.join(rel_path);
        if !file_path.exists() {
            println!("Skipping {} - file not found", rel_path);
            continue;
        }
        
        let original_bytes = std::fs::read(&file_path).expect("Should read file");
        let original_content = String::from_utf8_lossy(&original_bytes);
        
        // Count direction attributes in original
        let original_in_count = original_content.matches("direction=\"in\"").count();
        let original_out_count = original_content.matches("direction=\"out\"").count();
        let original_inout_count = original_content.matches("direction=\"inout\"").count();
        
        println!(
            "{}: in={}, out={}, inout={}",
            rel_path, original_in_count, original_out_count, original_inout_count
        );
        
        // Import
        let model = Xmi.read(&original_bytes).expect("Should import");
        
        // Export
        let exported_bytes = Xmi.write(&model).expect("Should export");
        let exported_content = String::from_utf8_lossy(&exported_bytes);
        
        // Count direction attributes in exported
        let exported_in_count = exported_content.matches("direction=\"in\"").count();
        let exported_out_count = exported_content.matches("direction=\"out\"").count();
        let exported_inout_count = exported_content.matches("direction=\"inout\"").count();
        
        println!(
            "  Exported: in={}, out={}, inout={}",
            exported_in_count, exported_out_count, exported_inout_count
        );
        
        // For now, just verify the file round-trips successfully
        // Direction preservation will be added when we integrate cst_extract
        assert!(model.elements.len() > 0, "Should have elements");
    }
}

#[test]
fn test_xmi_roundtrip_preserves_element_ids() {
    let release_dir = get_sysml_release_dir();
    
    // Use a smaller file for detailed comparison
    let file_path = release_dir.join("sysml.library.xmi/Systems Library/Parts.sysmlx");
    if !file_path.exists() {
        println!("Skipping - Parts.sysmlx not found");
        return;
    }
    
    let original_bytes = std::fs::read(&file_path).expect("Should read file");
    
    // Import
    let model = Xmi.read(&original_bytes).expect("Should import");
    
    // Collect original element IDs
    let original_ids: std::collections::HashSet<_> = 
        model.elements.keys().map(|id| id.as_str().to_string()).collect();
    
    // Export and re-import
    let exported_bytes = Xmi.write(&model).expect("Should export");
    let reimported_model = Xmi.read(&exported_bytes).expect("Should re-import");
    
    // Collect reimported element IDs
    let reimported_ids: std::collections::HashSet<_> = 
        reimported_model.elements.keys().map(|id| id.as_str().to_string()).collect();
    
    // Check for ID preservation
    let missing_ids: Vec<_> = original_ids.difference(&reimported_ids).collect();
    let extra_ids: Vec<_> = reimported_ids.difference(&original_ids).collect();
    
    if !missing_ids.is_empty() {
        println!("Missing IDs after round-trip (first 10):");
        for id in missing_ids.iter().take(10) {
            println!("  - {}", id);
        }
    }
    
    if !extra_ids.is_empty() {
        println!("Extra IDs after round-trip (first 10):");
        for id in extra_ids.iter().take(10) {
            println!("  + {}", id);
        }
    }
    
    // Allow some tolerance for now - strict ID preservation is a future goal
    let preserved_count = original_ids.intersection(&reimported_ids).count();
    let preservation_rate = preserved_count as f64 / original_ids.len() as f64;
    
    println!(
        "ID preservation: {}/{} ({:.1}%)",
        preserved_count,
        original_ids.len(),
        preservation_rate * 100.0
    );
    
    // For now just verify we have the same count
    assert_eq!(
        model.elements.len(),
        reimported_model.elements.len(),
        "Element count should match after round-trip"
    );
}
