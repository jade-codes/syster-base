use std::fs;
use syster::interchange::{ModelFormat, Xmi};
use walkdir::WalkDir;

fn main() {
    let base_path = "/tmp/sysml-v2-release/sysml.library.xmi";
    let xmi = Xmi;

    let mut total = 0;
    let mut success = 0;
    let mut element_match = 0;
    let mut name_match = 0;
    let mut kind_match = 0;

    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sysmlx" || ext == "kermlx" {
                total += 1;
                let content = fs::read(path).unwrap();

                // Import
                let model1 = match xmi.read(&content) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                // Export
                let exported = match xmi.write(&model1) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Re-import
                let model2 = match xmi.read(&exported) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                success += 1;

                // Compare
                if model1.elements.len() == model2.elements.len() {
                    element_match += 1;
                }

                let names1: std::collections::HashSet<_> = model1
                    .elements
                    .values()
                    .filter_map(|e| e.name.clone())
                    .collect();
                let names2: std::collections::HashSet<_> = model2
                    .elements
                    .values()
                    .filter_map(|e| e.name.clone())
                    .collect();
                if names1 == names2 {
                    name_match += 1;
                }

                let kinds1: std::collections::HashSet<_> = model1
                    .elements
                    .values()
                    .map(|e| (e.id.clone(), e.kind))
                    .collect();
                let kinds2: std::collections::HashSet<_> = model2
                    .elements
                    .values()
                    .map(|e| (e.id.clone(), e.kind))
                    .collect();
                if kinds1 == kinds2 {
                    kind_match += 1;
                }
            }
        }
    }

    println!("Roundtrip Test Results:");
    println!("  Total files: {}", total);
    println!("  Successful roundtrips: {}/{}", success, total);
    println!("  Element count match: {}/{}", element_match, success);
    println!("  Names preserved: {}/{}", name_match, success);
    println!("  Kinds preserved: {}/{}", kind_match, success);
}
