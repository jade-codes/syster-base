//! Test for interface definition end port qualified names
//!
//! When multiple interface definitions have children with the same name
//! (e.g., `supplierPort`, `consumerPort`), they should get different qualified names.

use syster::ide::AnalysisHost;

#[test]
fn test_interface_def_end_ports_have_unique_qualified_names() {
    let source = r#"
package TestPkg {
    interface def fillStatePorts {
        end supplierPort : FillState;
        end consumerPort : FillState;
    }
    
    interface def dirtyAirPorts {
        end supplierPort : DirtyAirFlow;
        end consumerPort : DirtyAirFlow;
    }
    
    interface def cleanAirPorts {
        end supplierPort : CleanAirFlow;
        end consumerPort : CleanAirFlow;
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let index = analysis.symbol_index();
    let symbols: Vec<_> = index.all_symbols().collect();

    println!("\n=== All Symbols ===");
    for sym in &symbols {
        println!("  {} (kind: {:?})", sym.qualified_name, sym.kind);
    }

    // Find all supplierPort symbols
    let supplier_ports: Vec<_> = symbols
        .iter()
        .filter(|s| s.name.as_ref() == "supplierPort")
        .collect();

    println!("\n=== supplierPort symbols ===");
    for sym in &supplier_ports {
        println!("  {}", sym.qualified_name);
    }

    // Each should have a unique qualified name
    assert_eq!(
        supplier_ports.len(),
        3,
        "Expected 3 supplierPort symbols (one per interface def)"
    );

    // Check they have different qualified names
    let qnames: std::collections::HashSet<_> = supplier_ports
        .iter()
        .map(|s| s.qualified_name.as_ref())
        .collect();
    assert_eq!(
        qnames.len(),
        3,
        "All supplierPort symbols should have unique qualified names"
    );

    // Verify expected qualified names
    assert!(
        qnames.contains("TestPkg::fillStatePorts::supplierPort"),
        "Missing fillStatePorts::supplierPort"
    );
    assert!(
        qnames.contains("TestPkg::dirtyAirPorts::supplierPort"),
        "Missing dirtyAirPorts::supplierPort"
    );
    assert!(
        qnames.contains("TestPkg::cleanAirPorts::supplierPort"),
        "Missing cleanAirPorts::supplierPort"
    );
}

#[test]
fn test_interface_def_end_ports_no_duplicate_errors() {
    let source = r#"
package TestPkg {
    interface def fillStatePorts {
        end supplierPort : FillState;
        end consumerPort : FillState;
    }
    
    interface def dirtyAirPorts {
        end supplierPort : DirtyAirFlow;
        end consumerPort : DirtyAirFlow;
    }
    
    interface def cleanAirPorts {
        end supplierPort : CleanAirFlow;
        end consumerPort : CleanAirFlow;
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", source);

    let analysis = host.analysis();
    let index = analysis.symbol_index();
    
    // Get diagnostics using the check_file function
    let file_id = analysis.get_file_id("test.sysml").expect("Should have file");
    let diagnostics = syster::hir::check_file(index, file_id);

    println!("\n=== Diagnostics ===");
    for diag in &diagnostics {
        println!("  [{:?}] {} (line {})", diag.severity, diag.message, diag.start_line);
    }

    // Check for any "duplicate" errors
    let duplicate_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.to_lowercase().contains("duplicate"))
        .collect();

    assert!(
        duplicate_errors.is_empty(),
        "Should have no duplicate definition errors, but found: {:?}",
        duplicate_errors.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}
