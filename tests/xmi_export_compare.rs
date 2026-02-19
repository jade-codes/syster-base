//! Quick test to export XMI for comparison
#![cfg(feature = "interchange")]

use std::collections::HashSet;
use std::path::Path;
use syster::interchange::{ModelFormat, Xmi, decompile};

#[test]
#[ignore] // Requires local /tmp/SysML-v2-Release - run manually
fn export_for_comparison() {
    let file_path = "/tmp/SysML-v2-Release/sysml.library.xmi/Kernel Libraries/Kernel Semantic Library/Base.kermlx";

    let original = std::fs::read(file_path).expect("read");
    let model = Xmi
        .read_from_path(&original, Path::new(file_path))
        .expect("import");

    // Count elements by kind
    let mut kinds = std::collections::HashMap::new();
    for el in model.elements.values() {
        *kinds.entry(format!("{:?}", el.kind)).or_insert(0usize) += 1;
    }

    eprintln!(
        "\n=== Element kinds in Base.kermlx ({} total) ===",
        model.elements.len()
    );
    let mut kinds_vec: Vec<_> = kinds.iter().collect();
    kinds_vec.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (kind, count) in kinds_vec {
        eprintln!("  {}: {}", kind, count);
    }

    // Show what "Other" elements are
    eprintln!("\n=== 'Other' element IDs ===");
    for el in model.elements.values() {
        if format!("{:?}", el.kind) == "Other" {
            eprintln!("  {} name={:?}", el.id, el.name);
        }
    }

    // Debug: show LiteralInteger properties
    eprintln!("\n=== LiteralInteger elements ===");
    for el in model.elements.values() {
        if format!("{:?}", el.kind) == "LiteralInteger" {
            eprintln!("  {}: props={:?}", el.id, el.properties);
        }
    }

    // Debug: show FeatureTyping relationships
    eprintln!("\n=== FeatureTyping relationships ===");
    for rel in model.iter_relationship_elements() {
        if format!("{:?}", rel.kind) == "FeatureTyping" {
            let src_id = rel.source().unwrap();
            let tgt_id = rel.target().unwrap();
            let src_name = model.elements.get(src_id).and_then(|e| e.name.as_ref());
            let tgt_name = model.elements.get(tgt_id).and_then(|e| e.name.as_ref());
            eprintln!(
                "  {} ({:?}) -> {} ({:?})",
                src_id, src_name, tgt_id, tgt_name
            );
        }
    }

    // Debug: show Subsetting relationships
    eprintln!("\n=== Subsetting relationships ===");
    for rel in model.iter_relationship_elements() {
        if format!("{:?}", rel.kind) == "Subsetting" {
            let src_id = rel.source().unwrap();
            let tgt_id = rel.target().unwrap();
            let src_name = model.elements.get(src_id).and_then(|e| e.name.as_ref());
            let tgt_name = model.elements.get(tgt_id).and_then(|e| e.name.as_ref());
            eprintln!(
                "  {} ({:?}) subsets {} ({:?})",
                src_id, src_name, tgt_id, tgt_name
            );
        }
    }

    // Debug: show things element properties
    eprintln!("\n=== 'things' element properties ===");
    for el in model.elements.values() {
        if el.name.as_deref() == Some("things") {
            eprintln!("  props: {:?}", el.properties);
        }
    }
    // Debug: show FeatureChaining relationships
    eprintln!("\n=== FeatureChaining relationships ===");
    for rel in model.iter_relationship_elements() {
        if format!("{:?}", rel.kind) == "FeatureChaining" {
            let src_id = rel.source().unwrap();
            let tgt_id = rel.target().unwrap();
            let src_name = model.elements.get(src_id).and_then(|e| e.name.as_ref());
            let tgt_name = model.elements.get(tgt_id).and_then(|e| e.name.as_ref());
            eprintln!(
                "  {} ({:?}) chains {} ({:?})",
                src_id, src_name, tgt_id, tgt_name
            );
        }
    }
    let exported = Xmi.write(&model).expect("export");
    std::fs::write("/tmp/exported_base.kermlx", &exported).expect("write");

    eprintln!("\nOriginal: {} bytes", original.len());
    eprintln!("Exported: {} bytes", exported.len());
    eprintln!("Written to /tmp/exported_base.kermlx");

    // Decompile to SysML text
    let result = decompile::decompile(&model);
    std::fs::write("/tmp/decompiled_base.kerml", &result.text).expect("write sysml");
    eprintln!("Decompiled to /tmp/decompiled_base.kerml");
}

#[test]
#[ignore] // Requires local /tmp/SysML-v2-Release - run manually
fn debug_ports_roundtrip() {
    let file_path = "/tmp/SysML-v2-Release/sysml.library.xmi/Kernel Libraries/Kernel Data Type Library/ScalarValues.kermlx";

    let original = std::fs::read(file_path).expect("read");
    let model = Xmi.read(&original).expect("import");
    eprintln!(
        "\nOriginal: {} elements, {} relationships",
        model.elements.len(),
        model.relationship_count()
    );

    // Count Subclassification elements
    let subclass_count = model
        .elements
        .values()
        .filter(|e| format!("{:?}", e.kind) == "Specialization")
        .count();
    eprintln!(
        "Subclassification/Specialization elements: {}",
        subclass_count
    );

    // Show element kinds
    let mut kinds = std::collections::HashMap::new();
    for el in model.elements.values() {
        *kinds.entry(format!("{:?}", el.kind)).or_insert(0usize) += 1;
    }
    eprintln!("\nElement kinds:");
    let mut kinds_vec: Vec<_> = kinds.iter().collect();
    kinds_vec.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (kind, count) in kinds_vec.iter().take(10) {
        eprintln!("  {}: {}", kind, count);
    }

    let exported = Xmi.write(&model).expect("export");
    let model2 = Xmi.read(&exported).expect("reimport");
    eprintln!(
        "\nReimported: {} elements, {} relationships",
        model2.elements.len(),
        model2.relationship_count()
    );

    // Find missing elements
    let orig_ids: HashSet<_> = model.elements.keys().collect();
    let new_ids: HashSet<_> = model2.elements.keys().collect();

    let missing: Vec<_> = orig_ids.difference(&new_ids).collect();
    eprintln!("\nMissing elements ({}):", missing.len());
    for id in missing.iter().take(5) {
        if let Some(el) = model.elements.get(&(**id).clone()) {
            eprintln!("  {:?} {:?} (name={:?})", el.kind, id, el.name);
        }
    }

    // Show relationships
    if model.relationship_count() > 0 {
        eprintln!("\nOriginal relationships (first 5):");
        for rel in model.iter_relationship_elements().take(5) {
            let src = rel.source().map(|s| s.as_str()).unwrap_or("?");
            let tgt = rel.target().map(|t| t.as_str()).unwrap_or("?");
            eprintln!("  {:?}: {} -> {}", rel.kind, src, tgt);
        }
    }
}
