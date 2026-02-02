//! Phase 2 test: Symbol deduplication preserves element IDs on re-parse

#![allow(clippy::unwrap_used)]

#[test]
fn test_reparse_preserves_element_ids() {
    use syster::ide::AnalysisHost;

    // 1. Parse a file initially
    let mut host = AnalysisHost::new();
    let initial_content = r#"
        package TestPackage {
            part def Vehicle;
        }
    "#;

    host.set_file_content("test.sysml", initial_content);
    let analysis1 = host.analysis();

    // Get initial symbols and their element IDs
    let pkg1 = analysis1
        .symbol_index()
        .lookup_qualified("TestPackage")
        .expect("Should find TestPackage");
    let initial_pkg_id = pkg1.element_id.clone();

    let vehicle1 = analysis1
        .symbol_index()
        .lookup_qualified("TestPackage::Vehicle")
        .expect("Should find Vehicle");
    let initial_vehicle_id = vehicle1.element_id.clone();

    // Verify IDs are UUIDs (non-empty)
    assert!(!initial_pkg_id.is_empty(), "Package should have element_id");
    assert!(
        !initial_vehicle_id.is_empty(),
        "Vehicle should have element_id"
    );

    // 2. Re-parse the same file with modifications (add an attribute)
    let modified_content = r#"
        package TestPackage {
            part def Vehicle {
                attribute mass : Real;
            }
        }
    "#;

    host.set_file_content("test.sysml", modified_content);
    let analysis2 = host.analysis();

    // Get symbols after re-parse
    let pkg2 = analysis2
        .symbol_index()
        .lookup_qualified("TestPackage")
        .expect("Should still find TestPackage");
    let vehicle2 = analysis2
        .symbol_index()
        .lookup_qualified("TestPackage::Vehicle")
        .expect("Should still find Vehicle");

    // 3. Verify element IDs were preserved
    assert_eq!(
        pkg2.element_id, initial_pkg_id,
        "TestPackage element_id should be preserved across re-parse"
    );
    assert_eq!(
        vehicle2.element_id, initial_vehicle_id,
        "Vehicle element_id should be preserved across re-parse"
    );

    // New symbol should get a new ID
    let mass = analysis2
        .symbol_index()
        .lookup_qualified("TestPackage::Vehicle::mass")
        .expect("Should find new attribute 'mass'");
    assert!(
        !mass.element_id.is_empty(),
        "New attribute should have element_id"
    );
    assert_ne!(
        mass.element_id, initial_pkg_id,
        "New symbol should have different ID"
    );
}

#[cfg(feature = "interchange")]
#[test]
fn test_imported_symbols_preserve_ids_on_reparse() {
    use syster::ide::AnalysisHost;
    use syster::interchange::{Element, ElementKind, Model};

    // 1. Import XMI with known IDs
    let mut model = Model::new();
    model.add_element(
        Element::new("xmi-imported-001", ElementKind::Package).with_name("ImportedPackage"),
    );
    model.add_element(
        Element::new("xmi-imported-002", ElementKind::PartDefinition).with_name("ImportedPart"),
    );

    let mut host = AnalysisHost::new();
    host.add_model(&model, "imported.sysml");

    // Verify imported IDs
    let analysis1 = host.analysis();
    let all_syms1: Vec<_> = analysis1.symbol_index().all_symbols().collect();
    println!("After import, {} symbols", all_syms1.len());
    for s in &all_syms1 {
        println!("  {} -> {}", s.qualified_name, s.element_id);
    }

    let pkg1 = analysis1
        .symbol_index()
        .lookup_qualified("ImportedPackage")
        .expect("Should find imported package");
    assert_eq!(
        pkg1.element_id.as_ref(),
        "xmi-imported-001",
        "Imported package should have XMI element_id"
    );

    // 2. Add a new file to trigger index rebuild
    host.set_file_content("new.sysml", "package NewPackage;");

    let analysis2 = host.analysis();
    let all_syms2: Vec<_> = analysis2.symbol_index().all_symbols().collect();
    println!("After rebuild, {} symbols", all_syms2.len());
    for s in &all_syms2 {
        println!("  {} -> {}", s.qualified_name, s.element_id);
    }

    // 3. Verify imported symbols still have original XMI IDs after rebuild
    let pkg2_opt = analysis2.symbol_index().lookup_qualified("ImportedPackage");
    if pkg2_opt.is_none() {
        panic!(
            "ImportedPackage not found after rebuild! Available: {:?}",
            all_syms2
                .iter()
                .map(|s| s.qualified_name.as_ref())
                .collect::<Vec<_>>()
        );
    }
    let pkg2 = pkg2_opt.unwrap();
    assert_eq!(
        pkg2.element_id.as_ref(),
        "xmi-imported-001",
        "Imported package element_id should survive index rebuild"
    );

    let part2 = analysis2
        .symbol_index()
        .lookup_qualified("ImportedPart")
        .expect("Should still find imported part after rebuild");
    assert_eq!(
        part2.element_id.as_ref(),
        "xmi-imported-002",
        "Imported part element_id should survive index rebuild"
    );
}

#[test]
fn test_symbol_removed_and_readded() {
    use syster::ide::AnalysisHost;

    // 1. Parse initial file
    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", "package Pkg { part def A; }");

    let analysis1 = host.analysis();
    let a1 = analysis1
        .symbol_index()
        .lookup_qualified("Pkg::A")
        .expect("Should find A");
    let original_id = a1.element_id.clone();

    // 2. Remove the symbol
    host.set_file_content("test.sysml", "package Pkg { }");
    let analysis2 = host.analysis();
    assert!(
        analysis2
            .symbol_index()
            .lookup_qualified("Pkg::A")
            .is_none(),
        "A should be removed"
    );

    // 3. Re-add the symbol - should get the SAME ID back
    host.set_file_content("test.sysml", "package Pkg { part def A; }");
    let analysis3 = host.analysis();
    let a3 = analysis3
        .symbol_index()
        .lookup_qualified("Pkg::A")
        .expect("Should find A again");

    assert_eq!(
        a3.element_id, original_id,
        "Re-added symbol should get its original element_id back"
    );
}
