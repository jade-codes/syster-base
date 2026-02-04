use syster::interchange::{ModelFormat, Xmi};

fn main() {
    let xmi = Xmi::default();
    let path = std::path::Path::new("/tmp/syster-test-sysml-release/sysml.library.xmi/Kernel Libraries/Kernel Semantic Library/Base.kermlx");
    let content = std::fs::read(path).expect("read file");
    
    // Show a small relevant portion of the original
    let original_str = String::from_utf8_lossy(&content);
    println!("=== ORIGINAL (first Subclassification) ===");
    for line in original_str.lines() {
        if line.contains("Subclassification") && line.contains("superclassifier") {
            println!("{}", line.trim());
            break;
        }
    }
    
    // Parse and write
    let model = xmi.read_from_path(&content, path).expect("parse");
    let exported = xmi.write(&model).expect("write");
    
    // Show our output
    let exported_str = String::from_utf8_lossy(&exported);
    println!("\n=== DECOMPILED (our writer output) ===");
    for line in exported_str.lines() {
        if line.contains("Subclassification") && line.contains("superclassifier") {
            println!("{}", line.trim());
            break;
        }
    }
    
    // Re-parse and check
    let model2 = xmi.read(&exported).expect("re-parse");
    println!("\n=== VERIFICATION ===");
    println!("Original:    {} elements, {} relationships", model.elements.len(), model.relationships.len());
    println!("Roundtripped: {} elements, {} relationships", model2.elements.len(), model2.relationships.len());
}
