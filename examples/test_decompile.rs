//! Test decompiling XMI files to SysML text and verifying they parse.
//!
//! Usage:
//!   1. Clone the official repo:
//!      git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release
//!   
//!   2. Run this example:
//!      cargo run --features interchange --example test_decompile
//!
//!   3. Or test a specific file:
//!      cargo run --features interchange --example test_decompile -- /path/to/file.xmi

use std::fs;
use std::path::Path;
use syster::interchange::{ModelFormat, Xmi, decompile};
use syster::syntax::sysml::parser::parse_content;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Test specific file
        test_file(&args[1]);
    } else {
        // Test a simple built-in example first
        test_simple_example();

        // Then test official library if available
        test_official_library();
    }
}

fn test_simple_example() {
    println!("=== Testing Simple Example ===\n");

    // Create a simple model programmatically
    use syster::interchange::{
        Element, ElementId, ElementKind, Model, Relationship, RelationshipKind,
    };

    let mut model = Model::new();

    // Package
    let pkg_id = ElementId::from("pkg-001");
    let mut pkg = Element::new(pkg_id.clone(), ElementKind::Package).with_name("VehicleModel");

    // Part definition
    let vehicle_id = ElementId::from("def-001");
    let vehicle = Element::new(vehicle_id.clone(), ElementKind::PartDefinition)
        .with_name("Vehicle")
        .with_owner(pkg_id.clone());

    // Another part definition that specializes Vehicle
    let car_id = ElementId::from("def-002");
    let car = Element::new(car_id.clone(), ElementKind::PartDefinition)
        .with_name("Car")
        .with_owner(pkg_id.clone());

    // Part usage
    let engine_id = ElementId::from("usage-001");
    let engine = Element::new(engine_id.clone(), ElementKind::PartUsage)
        .with_name("engine")
        .with_owner(car_id.clone());

    // Set up owned elements
    pkg.owned_elements = vec![vehicle_id.clone(), car_id.clone()];

    // Add a child to car for engine
    let mut car = Element::new(car_id.clone(), ElementKind::PartDefinition)
        .with_name("Car")
        .with_owner(pkg_id.clone());
    car.owned_elements = vec![engine_id.clone()];

    model.elements.insert(pkg_id.clone(), pkg);
    model.elements.insert(vehicle_id.clone(), vehicle);
    model.elements.insert(car_id.clone(), car);
    model.elements.insert(engine_id.clone(), engine);
    model.roots.push(pkg_id.clone());

    // Add specialization relationship
    model.relationships.push(Relationship::new(
        "rel-001",
        RelationshipKind::Specialization,
        "def-002",
        "def-001",
    ));

    // Decompile to SysML text
    let result = decompile(&model);

    println!("Generated SysML text:");
    println!("---");
    println!("{}", result.text);
    println!("---\n");

    println!("Metadata:");
    println!("  Elements tracked: {}", result.metadata.elements.len());
    for (qn, meta) in &result.metadata.elements {
        println!("    {} -> {:?}", qn, meta.element_id());
    }
    println!();

    // Try to parse the generated text
    match parse_content(&result.text, Path::new("generated.sysml")) {
        Ok(syntax_file) => {
            println!("✓ Generated SysML parses successfully!");
            println!("  Parsed {} top-level elements", syntax_file.elements.len());
        }
        Err(err) => {
            println!("✗ Parse error in generated SysML: {}", err);
        }
    }
    println!();
}

fn test_file(path: &str) {
    println!("Testing file: {}", path);
    let content = fs::read(path).expect("Failed to read file");
    println!("  File size: {} bytes", content.len());

    let xmi = Xmi;
    match xmi.read(&content) {
        Ok(model) => {
            println!(
                "  Loaded model: {} elements, {} relationships",
                model.elements.len(),
                model.relationships.len()
            );

            // Decompile to SysML
            let result = decompile(&model);
            println!("  Generated {} chars of SysML", result.text.len());
            println!(
                "  Tracking {} elements in metadata",
                result.metadata.elements.len()
            );

            // Show first 500 chars of output
            if result.text.len() > 0 {
                let preview: String = result.text.chars().take(500).collect();
                println!("\n  Preview:");
                println!("  ---");
                for line in preview.lines().take(20) {
                    println!("  {}", line);
                }
                println!("  ---\n");
            }

            // Try to parse
            match parse_content(&result.text, Path::new(path)) {
                Ok(syntax_file) => {
                    println!(
                        "  ✓ Parse successful! {} elements",
                        syntax_file.elements.len()
                    );
                }
                Err(err) => {
                    println!("  ✗ Parse failed: {}", err);
                }
            }
        }
        Err(e) => {
            println!("  ✗ XMI load error: {:?}", e);
        }
    }
    println!();
}

fn test_official_library() {
    let release_dir = Path::new("/tmp/sysml-v2-release");

    if !release_dir.exists() {
        println!("Official SysML v2 release not found at /tmp/sysml-v2-release");
        println!("To test with official files, run:");
        println!(
            "  git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release"
        );
        return;
    }

    println!("\n=== Testing Official XMI Files ===\n");

    // Test a few key files
    let test_files = [
        "sysml.library.xmi.implied/Systems Library/Parts.sysmlx",
        "sysml.library.xmi.implied/Domain Libraries/Quantities and Units/ISQ.sysmlx",
        "sysml.library.xmi.implied/Kernel Libraries/Kernel Data Types/ScalarValues.sysmlx",
    ];

    let mut passed = 0;
    let mut failed = 0;

    for rel_path in &test_files {
        let full_path = release_dir.join(rel_path);
        if full_path.exists() {
            let content = fs::read(&full_path).expect("Failed to read");
            match Xmi.read(&content) {
                Ok(model) => {
                    let result = decompile(&model);
                    match parse_content(&result.text, &full_path) {
                        Ok(_) => {
                            println!("✓ {}", rel_path);
                            passed += 1;
                        }
                        Err(err) => {
                            println!("✗ {} - parse error: {}", rel_path, err);
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    println!("✗ {} - XMI error: {:?}", rel_path, e);
                    failed += 1;
                }
            }
        } else {
            println!("⊘ {} - not found", rel_path);
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);
}
