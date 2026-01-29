use std::fs;
use syster::interchange::{ModelFormat, Xmi};

fn main() {
    let original_path = "/tmp/sysml-v2-release/sysml.library.xmi/Systems Library/Parts.sysmlx";

    // Step 1: Read original XMI
    let original_bytes = fs::read(original_path).expect("Failed to read original");
    println!("Original file: {} bytes", original_bytes.len());

    // Step 2: Import to Model
    let xmi = Xmi;
    let model = xmi.read(&original_bytes).expect("Failed to import");
    println!(
        "Imported: {} elements, {} relationships",
        model.elements.len(),
        model.relationships.len()
    );

    // Step 3: Export back to XMI
    let exported_bytes = xmi.write(&model).expect("Failed to export");
    println!("Exported: {} bytes", exported_bytes.len());

    // Step 4: Re-import the exported XMI
    let model2 = xmi.read(&exported_bytes).expect("Failed to re-import");
    println!(
        "Re-imported: {} elements, {} relationships",
        model2.elements.len(),
        model2.relationships.len()
    );

    // Compare
    println!("\n=== Comparison ===");
    println!(
        "Elements match: {}",
        model.elements.len() == model2.elements.len()
    );
    println!(
        "Relationships match: {}",
        model.relationships.len() == model2.relationships.len()
    );

    // Check if specific elements match
    let mut matching_ids = 0;
    let mut matching_names = 0;
    let mut matching_kinds = 0;

    for (id, elem1) in &model.elements {
        if let Some(elem2) = model2.elements.get(id) {
            matching_ids += 1;
            if elem1.name == elem2.name {
                matching_names += 1;
            }
            if elem1.kind == elem2.kind {
                matching_kinds += 1;
            }
        }
    }

    println!("\nElement-by-element comparison:");
    println!("  Matching IDs: {}/{}", matching_ids, model.elements.len());
    println!(
        "  Matching names: {}/{}",
        matching_names,
        model.elements.len()
    );
    println!(
        "  Matching kinds: {}/{}",
        matching_kinds,
        model.elements.len()
    );

    // Show a sample element
    if let Some((id, elem)) = model.elements.iter().find(|(_, e)| e.name.is_some()) {
        println!("\nSample element from original:");
        println!("  ID: {}", id);
        println!("  Name: {:?}", elem.name);
        println!("  Kind: {:?}", elem.kind);

        if let Some(elem2) = model2.elements.get(id) {
            println!("Same element after roundtrip:");
            println!("  ID: {}", id);
            println!("  Name: {:?}", elem2.name);
            println!("  Kind: {:?}", elem2.kind);
        }
    }

    // Save exported for manual inspection
    fs::write("/tmp/parts_roundtrip.xmi", &exported_bytes).unwrap();
    println!("\nExported file saved to: /tmp/parts_roundtrip.xmi");
}
