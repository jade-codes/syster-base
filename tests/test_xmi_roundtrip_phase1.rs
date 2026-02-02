//! Phase 1 test: XMI → symbols_from_model → export preserves IDs

#![allow(clippy::unwrap_used)]

#[cfg(feature = "interchange")]
#[test]
fn test_xmi_to_symbols_preserves_element_ids() {
    use syster::hir::SymbolIndex;
    use syster::interchange::{
        Element, ElementKind, Model, ModelFormat, Xmi, model_from_symbols,
        restore_ids_from_symbols, symbols_from_model,
    };

    // 1. Create a test XMI model
    let mut model = Model::new();
    let pkg = Element::new("test-pkg-123", ElementKind::Package).with_name("TestPackage");
    model.add_element(pkg);

    let part_def = Element::new("test-part-456", ElementKind::PartDefinition).with_name("Vehicle");
    model.add_element(part_def);

    // 2. Convert to XMI
    let xmi_bytes = Xmi.write(&model).expect("Failed to write XMI");

    // 3. Read XMI back
    let model2 = Xmi.read(&xmi_bytes).expect("Failed to read XMI");

    // 4. Convert to symbols
    let symbols = symbols_from_model(&model2);

    // Verify symbols have the original element IDs
    assert_eq!(symbols.len(), 2, "Should have 2 symbols");

    let pkg_symbol = symbols
        .iter()
        .find(|s| s.name.as_ref() == "TestPackage")
        .expect("Should find TestPackage symbol");
    assert_eq!(
        pkg_symbol.element_id.as_ref(),
        "test-pkg-123",
        "Package element_id should be preserved"
    );

    let part_symbol = symbols
        .iter()
        .find(|s| s.name.as_ref() == "Vehicle")
        .expect("Should find Vehicle symbol");
    assert_eq!(
        part_symbol.element_id.as_ref(),
        "test-part-456",
        "PartDef element_id should be preserved"
    );

    // 5. Export back via restore_ids_from_symbols
    let mut symbol_index = SymbolIndex::new();
    symbol_index.add_file(syster::base::FileId::new(0), symbols);

    let symbols_vec: Vec<_> = symbol_index.all_symbols().cloned().collect();
    let model3 = model_from_symbols(&symbols_vec);
    let model3 = restore_ids_from_symbols(model3, &symbol_index);

    // 6. Verify exported model has original IDs
    assert!(
        model3
            .elements
            .iter()
            .any(|(_, e)| e.id.as_str() == "test-pkg-123"
                && e.name.as_deref() == Some("TestPackage")),
        "Exported model should have original package ID"
    );

    assert!(
        model3
            .elements
            .iter()
            .any(|(_, e)| e.id.as_str() == "test-part-456" && e.name.as_deref() == Some("Vehicle")),
        "Exported model should have original part def ID"
    );
}

#[cfg(feature = "interchange")]
#[test]
fn test_import_into_host_preserves_ids() {
    use syster::ide::AnalysisHost;
    use syster::interchange::{Element, ElementKind, Model};

    // 1. Create test model with known IDs
    let mut model = Model::new();
    model.add_element(Element::new("xmi-id-001", ElementKind::Package).with_name("RootPackage"));
    model.add_element(
        Element::new("xmi-id-002", ElementKind::PartDefinition).with_name("Component"),
    );

    // 2. Add model to analysis host (decompiles to SysML and parses)
    let mut host = AnalysisHost::new();
    host.add_model(&model, "imported.sysml");

    // 3. Verify symbols are queryable with preserved IDs
    let analysis = host.analysis();
    let all_symbols: Vec<_> = analysis.symbol_index().all_symbols().collect();

    assert_eq!(all_symbols.len(), 2, "Should have 2 symbols in host");

    let root_pkg = all_symbols
        .iter()
        .find(|s| s.name.as_ref() == "RootPackage")
        .expect("Should find RootPackage");
    assert_eq!(
        root_pkg.element_id.as_ref(),
        "xmi-id-001",
        "RootPackage element_id should match XMI"
    );

    let component = all_symbols
        .iter()
        .find(|s| s.name.as_ref() == "Component")
        .expect("Should find Component");
    assert_eq!(
        component.element_id.as_ref(),
        "xmi-id-002",
        "Component element_id should match XMI"
    );
}
