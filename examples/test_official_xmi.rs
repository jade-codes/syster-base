//! Test importing official SysML v2 XMI files from the Systems-Modeling repository.
//!
//! Usage:
//!   1. Clone the official repo:
//!      git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release
//!   
//!   2. Run this example:
//!      cargo run --features interchange --example test_official_xmi
//!
//!   3. Or test a specific file:
//!      cargo run --features interchange --example test_official_xmi -- /path/to/file.sysmlx

use std::fs;
use std::path::Path;
use syster::interchange::{ModelFormat, Xmi};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Test specific file
        test_file(&args[1]);
    } else {
        // Test all files in the official release
        test_official_library();
    }
}

fn test_file(path: &str) {
    println!("Testing file: {}", path);
    let content = fs::read(path).expect("Failed to read file");
    println!("  File size: {} bytes", content.len());

    let xmi = Xmi;
    match xmi.read(&content) {
        Ok(model) => {
            println!(
                "  ✓ Success! Elements: {}, Relationships: {}",
                model.elements.len(),
                model.relationships.len()
            );

            // Count elements by kind
            let mut kind_counts: std::collections::HashMap<_, usize> =
                std::collections::HashMap::new();
            for elem in model.elements.values() {
                *kind_counts.entry(elem.kind).or_insert(0) += 1;
            }

            println!("  Element kinds:");
            let mut kinds: Vec<_> = kind_counts.into_iter().collect();
            kinds.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
            for (kind, count) in kinds.iter().take(10) {
                println!("    {:?}: {}", kind, count);
            }

            // Show named elements
            let named: Vec<_> = model
                .elements
                .values()
                .filter(|e| e.name.is_some())
                .take(10)
                .collect();
            if !named.is_empty() {
                println!("  Named elements (first 10):");
                for elem in named {
                    println!("    - {} ({:?})", elem.name.as_ref().unwrap(), elem.kind);
                }
            }
        }
        Err(e) => {
            println!("  ✗ Error: {:?}", e);
        }
    }
    println!();
}

fn test_official_library() {
    let base_path = "/tmp/sysml-v2-release/sysml.library.xmi";

    if !Path::new(base_path).exists() {
        eprintln!("Official SysML v2 XMI library not found at: {}", base_path);
        eprintln!();
        eprintln!("Please clone it first:");
        eprintln!(
            "  git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release"
        );
        std::process::exit(1);
    }

    println!("Testing official SysML v2 XMI library from: {}", base_path);
    println!("{}", "=".repeat(60));

    let mut total_files = 0;
    let mut successful = 0;
    let mut total_elements = 0;
    let mut failed_files: Vec<String> = Vec::new();

    // Walk through all .sysmlx and .kermlx files
    for entry in walkdir::WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sysmlx" || ext == "kermlx" {
                total_files += 1;
                let content = match fs::read(path) {
                    Ok(c) => c,
                    Err(e) => {
                        println!("✗ {} - Read error: {}", path.display(), e);
                        failed_files.push(path.display().to_string());
                        continue;
                    }
                };

                let xmi = Xmi;
                match xmi.read(&content) {
                    Ok(model) => {
                        successful += 1;
                        total_elements += model.elements.len();
                        println!(
                            "✓ {} - {} elements",
                            path.file_name().unwrap().to_string_lossy(),
                            model.elements.len()
                        );
                    }
                    Err(e) => {
                        println!(
                            "✗ {} - Error: {:?}",
                            path.file_name().unwrap().to_string_lossy(),
                            e
                        );
                        failed_files.push(path.display().to_string());
                    }
                }
            }
        }
    }

    println!();
    println!("{}", "=".repeat(60));
    println!("Summary:");
    println!("  Total files: {}", total_files);
    println!(
        "  Successful:  {} ({:.1}%)",
        successful,
        (successful as f64 / total_files as f64) * 100.0
    );
    println!("  Failed:      {}", failed_files.len());
    println!("  Total elements imported: {}", total_elements);

    if !failed_files.is_empty() {
        println!();
        println!("Failed files:");
        for f in &failed_files {
            println!("  - {}", f);
        }
    }
}
