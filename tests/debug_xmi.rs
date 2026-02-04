//! Debug XMI to see what's stored

use syster::interchange::{ModelFormat, Xmi};

#[test]
fn debug_xmi_elements() {
    let file_path = "/tmp/SysML-v2-Release/sysml.library.xmi/Kernel Libraries/Kernel Data Type Library/ScalarValues.kermlx";
    
    let original = std::fs::read(file_path).expect("read");
    let model = Xmi.read(&original).expect("import");
    
    // Find elements with Specialization kind
    for elem in model.iter_elements() {
        if elem.kind == syster::interchange::model::ElementKind::Specialization {
            eprintln!("=== Specialization element ===");
            eprintln!("  ID: {}", elem.id.as_str());
            eprintln!("  Name: {:?}", elem.name);
            eprintln!("  Owner: {:?}", elem.owner);
            eprintln!("  Properties:");
            for (k, v) in &elem.properties {
                eprintln!("    {}: {:?}", k, v);
            }
            eprintln!("  Owned elements: {:?}", elem.owned_elements);
        }
    }
    
    // Also check relationships
    eprintln!("\n=== Relationships ===");
    for rel in model.relationships.iter().take(5) {
        eprintln!("  {:?}: {} -> {}", rel.kind, rel.source.as_str(), rel.target.as_str());
    }
}
