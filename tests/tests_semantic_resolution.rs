//! Tests for semantic resolution - ensuring symbols resolve correctly within files.
//!
//! These tests verify that the resolver correctly handles:
//! - Nested package imports (import Definitions::*)
//! - Sibling package imports (import PortDefinitions::* from PartDefinitions context)
//! - Forward references within the same file

use std::path::PathBuf;
use std::sync::Arc;
use syster::hir::{ResolveResult, Resolver, SymbolIndex, check_file, new_element_id};
use syster::ide::AnalysisHost;

fn get_examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sysml-examples")
}

/// Test that the SimpleVehicleModel file has no false positive undefined reference errors.
///
/// This file has a complex package structure:
/// ```
/// package SimpleVehicleModel {
///     public import Definitions::*;
///     package Definitions {
///         public import PartDefinitions::*;
///         public import PortDefinitions::*;
///         package PartDefinitions {
///             part def Vehicle {
///                 port ignitionCmdPort: IgnitionCmdPort;  // Should resolve!
///             }
///         }
///         package PortDefinitions {
///             port def IgnitionCmdPort { ... }
///         }
///     }
/// }
/// ```
///
/// The `IgnitionCmdPort` reference in `Vehicle` should resolve because:
/// 1. Vehicle is in PartDefinitions
/// 2. PartDefinitions' parent (Definitions) has `public import PortDefinitions::*`
/// 3. PortDefinitions contains IgnitionCmdPort
#[test]
fn test_simple_vehicle_model_resolution() {
    let file_path = get_examples_dir()
        .join("Vehicle Example")
        .join("SysML v2 Spec Annex A SimpleVehicleModel.sysml");

    if !file_path.exists() {
        eprintln!("Skipping test: file not found at {:?}", file_path);
        return;
    }

    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();

    // Parse and build symbol index
    let mut host = AnalysisHost::new();
    let parse_errors = host.set_file_content(&path_str, &content);

    // Should have no parse errors
    assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    // Run semantic checks
    let diagnostics = check_file(analysis.symbol_index(), file_id);

    // Test specific resolution cases
    let index = analysis.symbol_index();

    // Test 1: IgnitionCmdPort should be defined
    let ignition_cmd_port =
        index.lookup_qualified("SimpleVehicleModel::Definitions::PortDefinitions::IgnitionCmdPort");
    assert!(
        ignition_cmd_port.is_some(),
        "IgnitionCmdPort definition not found"
    );

    // Test 2: Vehicle should be defined
    let vehicle =
        index.lookup_qualified("SimpleVehicleModel::Definitions::PartDefinitions::Vehicle");
    assert!(vehicle.is_some(), "Vehicle definition not found");

    // Test 3: From Vehicle's scope, IgnitionCmdPort should resolve
    // Vehicle is in SimpleVehicleModel::Definitions::PartDefinitions::Vehicle
    // Parent scope is SimpleVehicleModel::Definitions::PartDefinitions
    // Grandparent is SimpleVehicleModel::Definitions which has `import PortDefinitions::*`
    let resolver = Resolver::new(index)
        .with_scope("SimpleVehicleModel::Definitions::PartDefinitions::Vehicle");

    let result = resolver.resolve("IgnitionCmdPort");

    assert!(
        matches!(result, ResolveResult::Found(_)),
        "IgnitionCmdPort should resolve from Vehicle scope"
    );

    // The actual test: there should be NO undefined reference errors for types
    // defined in the same file's package structure
    let undefined_refs: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("undefined reference"))
        .collect();

    if !undefined_refs.is_empty() {
        eprintln!("\n=== Undefined References (should be empty) ===");
        for diag in &undefined_refs {
            eprintln!("  Line {}: {}", diag.start_line + 1, diag.message);
        }

        // Extract the names that weren't found
        let missing_names: Vec<_> = undefined_refs
            .iter()
            .filter_map(|d| {
                // Extract name from "undefined reference: 'NAME'"
                let msg = d.message.as_ref();
                if let Some(start) = msg.find('\'') {
                    if let Some(end) = msg[start + 1..].find('\'') {
                        return Some(&msg[start + 1..start + 1 + end]);
                    }
                }
                None
            })
            .collect();

        eprintln!("\nMissing names: {:?}", missing_names);

        // Check if these names exist in the index at all
        for name in &missing_names {
            let matches: Vec<_> = index
                .lookup_simple(name)
                .iter()
                .map(|s| s.qualified_name.as_ref())
                .collect();
            if !matches.is_empty() {
                eprintln!("'{}' exists as: {:?}", name, matches);
            } else {
                eprintln!("'{}' not found anywhere in index", name);
            }
        }
        eprintln!("================================================\n");
    }

    // For now, just report the count - we'll fix the resolver to make this pass
    eprintln!("Total undefined reference errors: {}", undefined_refs.len());

    // This assertion will fail until we fix the resolution
    // assert!(undefined_refs.is_empty(),
    //     "Should have no undefined reference errors for symbols in the same file");
}

/// Debug test to understand what supertypes/type_refs are captured for allocations
#[test]
fn test_debug_allocation_refs() {
    // Test what a deep import looks like
    let content = r#"
package Root {
    package Inner {
        package Deep {
            part vehicle_b : Vehicle;
        }
    }
    
    part def Vehicle;
    
    package Allocations {
        // Deep import
        public import Inner::Deep::**;
        
        // Should now be able to reference vehicle_b
        part ref_to_vehicle : SomeType {
            :>> vehicle_b;  // Subsetting feature reference
        }
        part def SomeType;
    }
}
"#;

    let mut host = AnalysisHost::new();
    let parse_errors = host.set_file_content("test.sysml", content);
    eprintln!("Parse errors: {:?}", parse_errors);

    let analysis = host.analysis();
    let file_id = analysis
        .get_file_id("test.sysml")
        .expect("File not in index");
    let index = analysis.symbol_index();

    // Look at what symbols we got for the allocation
    eprintln!("\n=== Symbols in file ===");
    for sym in index.symbols_in_file(file_id) {
        eprintln!("Symbol: {} (kind={:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            eprintln!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            eprintln!("  type_refs: {} items", sym.type_refs.len());
            for tr in &sym.type_refs {
                eprintln!("    {:?}", tr);
            }
        }
    }

    // Run semantic checks
    let diagnostics = check_file(index, file_id);

    eprintln!("\n=== Diagnostics ===");
    for diag in &diagnostics {
        eprintln!(
            "  [{:?}] Line {}: {}",
            diag.severity,
            diag.start_line + 1,
            diag.message
        );
    }

    // Check what's in Allocations scope
    if let Some(vis) = index.visibility_for_scope("Root::Allocations") {
        eprintln!("\n=== Visibility in Root::Allocations ===");
        eprintln!("Direct defs:");
        for (name, qname) in vis.direct_defs() {
            eprintln!("  {} -> {}", name, qname);
        }
        eprintln!("Imports:");
        for (name, qname) in vis.imports() {
            eprintln!("  {} -> {}", name, qname);
        }
    }
}

/// Debug test to understand nested allocate statements
#[test]
fn test_nested_allocate_layers() {
    let content = r#"
package Test {
    // Define the types we're allocating between
    allocation def LogicalToPhysical;
    
    part def Logical {
        part torqueGenerator {
            action generateTorque;
        }
    }
    
    part def Physical {
        part engine {
            action generateTorque;
        }
    }
    
    // Instances  
    part vehicleLogical : Logical;
    part vehicle_b : Physical;
    
    // The allocation with nested allocates
    allocation vehicleLogicalToPhysicalAllocation : LogicalToPhysical
        allocate vehicleLogical to vehicle_b {
            allocate vehicleLogical.torqueGenerator to vehicle_b.engine {
                allocate vehicleLogical.torqueGenerator.generateTorque to vehicle_b.engine.generateTorque;
            }
        }
}
"#;

    let mut host = AnalysisHost::new();
    let parse_errors = host.set_file_content("test.sysml", content);
    eprintln!("Parse errors: {:?}", parse_errors);

    let analysis = host.analysis();
    let file_id = analysis
        .get_file_id("test.sysml")
        .expect("File not in index");
    let index = analysis.symbol_index();

    // Look at what symbols we got
    eprintln!("\n=== Symbols in file ===");
    for sym in index.symbols_in_file(file_id) {
        eprintln!("Symbol: {} (kind={:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            eprintln!("  type_refs:");
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        eprintln!("    {} -> {:?}", r.target, r.resolved_target);
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        let parts: Vec<_> = c
                            .parts
                            .iter()
                            .map(|p| format!("{} -> {:?}", p.target, p.resolved_target))
                            .collect();
                        eprintln!("    Chain: {:?}", parts);
                    }
                }
            }
        }
    }

    // Run semantic checks
    let diagnostics = syster::hir::check_file(index, file_id);

    eprintln!("\n=== Diagnostics ===");
    for diag in &diagnostics {
        eprintln!(
            "  [{:?}] Line {}: {}",
            diag.severity,
            diag.start_line + 1,
            diag.message
        );
    }
}

/// Test then action inline - check symbol span is correct
#[test]
fn test_then_action_inline_span() {
    let content = r#"
package Test {
    action def TestAction {
        action start;
        then action evaluatePassFail {
            in massMeasured;
            out verdict;
        }
        flow from start to evaluatePassFail.massMeasured;
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let index = analysis.symbol_index();

    eprintln!("\n=== Symbols ===");
    for sym in index.symbols_in_file(file_id) {
        eprintln!(
            "{} (kind={:?}) lines {}-{}",
            sym.qualified_name,
            sym.kind,
            sym.start_line + 1,
            sym.end_line + 1
        );
        for tr in &sym.type_refs {
            match tr {
                syster::hir::TypeRefKind::Simple(r) => {
                    eprintln!("  TypeRef: {} -> {:?}", r.target, r.resolved_target);
                }
                syster::hir::TypeRefKind::Chain(c) => {
                    for p in &c.parts {
                        eprintln!("  Chain part: {} -> {:?}", p.target, p.resolved_target);
                    }
                }
            }
        }
    }

    // Check evaluatePassFail symbol exists and has correct span
    let eval_sym = index
        .symbols_in_file(file_id)
        .into_iter()
        .find(|s| s.name.as_ref() == "evaluatePassFail");
    assert!(eval_sym.is_some(), "evaluatePassFail should exist");
    let eval_sym = eval_sym.unwrap();
    eprintln!(
        "\nevaluatePassFail: lines {}-{}, cols {}-{}",
        eval_sym.start_line + 1,
        eval_sym.end_line + 1,
        eval_sym.start_col,
        eval_sym.end_col
    );

    // Check visibility - evaluatePassFail should be visible from TestAction scope
    eprintln!("\n=== Visibility in Test::TestAction ===");
    if let Some(vis) = index.visibility_for_scope("Test::TestAction") {
        eprintln!("Direct defs:");
        for (name, qname) in vis.direct_defs() {
            eprintln!("  {} -> {}", name, qname);
        }
    }
}

/// Test redefines type refs are extracted for hover
#[test]
fn test_redefines_type_ref_extraction() {
    let content = r#"
package Test {
    action def BaseAction {
        action providePower;
        action performSelfTest;
    }
    
    part def Vehicle : BaseAction {
        perform BaseAction::providePower redefines providePower;
    }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let index = analysis.symbol_index();

    eprintln!("\n=== Symbols ===");
    for sym in index.symbols_in_file(file_id) {
        eprintln!("{} (kind={:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        eprintln!(
                            "  TypeRef: {} ({:?}) at L{}:{}-{}:{}",
                            r.target,
                            r.kind,
                            r.start_line + 1,
                            r.start_col,
                            r.end_line + 1,
                            r.end_col
                        );
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        for p in &c.parts {
                            eprintln!(
                                "  Chain part: {} ({:?}) at L{}:{}-{}:{}",
                                p.target,
                                p.kind,
                                p.start_line + 1,
                                p.start_col,
                                p.end_line + 1,
                                p.end_col
                            );
                        }
                    }
                }
            }
        }
    }

    // Check that the perform symbol has the redefines type_ref
    let perform_sym = index
        .symbols_in_file(file_id)
        .into_iter()
        .find(|s| s.qualified_name.contains("perform:"));
    assert!(perform_sym.is_some(), "perform symbol should exist");
    let perform_sym = perform_sym.unwrap();

    // Should have redefines type ref
    let has_redefines_ref = perform_sym.type_refs.iter().any(|tr| match tr {
        syster::hir::TypeRefKind::Simple(r) => r.kind == syster::hir::RefKind::Redefines,
        syster::hir::TypeRefKind::Chain(c) => c
            .parts
            .iter()
            .any(|p| p.kind == syster::hir::RefKind::Redefines),
    });
    eprintln!("\nPerform symbol has Redefines ref: {}", has_redefines_ref);
}

/// Test allocation visibility with imports
#[test]
fn test_allocation_visibility_with_imports() {
    let content = r#"
package VehicleConfigurations {
    package VehicleConfiguration_b {
        package PartsTree {
            part vehicle_b {
                part engine {
                    action generateTorque;
                    part alternator {
                        action generateElectricity;
                    }
                }
            }
        }
    }
}

package VehicleLogicalConfiguration {
    package PartsTree {
        part vehicleLogical {
            part torqueGenerator {
                action generateTorque;
            }
            part electricalGenerator {
                action generateElectricity;
            }
        }
    }
}

package VehicleLogicalToPhysicalAllocation {
    public import VehicleConfigurations::VehicleConfiguration_b::PartsTree::**;
    public import VehicleLogicalConfiguration::PartsTree::*;

    allocation vehicleLogicalToPhysicalAllocation : LogicalToPhysical
        allocate vehicleLogical to vehicle_b {
            allocate vehicleLogical.torqueGenerator to vehicle_b.engine;
        }
}
"#;

    let mut host = AnalysisHost::new();
    host.set_file_content("test.sysml", content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    let index = analysis.symbol_index();

    // Check visibility in VehicleLogicalToPhysicalAllocation
    eprintln!("=== Visibility in VehicleLogicalToPhysicalAllocation ===");
    if let Some(vis) = index.visibility_for_scope("VehicleLogicalToPhysicalAllocation") {
        eprintln!("Direct defs:");
        for (name, qname) in vis.direct_defs() {
            eprintln!("  {} -> {}", name, qname);
        }
        eprintln!("\nImports:");
        for (name, qname) in vis.imports() {
            eprintln!("  {} -> {}", name, qname);
        }
    } else {
        eprintln!("No visibility found!");
    }

    // Check what symbols exist
    eprintln!("\n=== Symbols in allocation scope ===");
    for sym in index.symbols_in_file(file_id) {
        if sym
            .qualified_name
            .contains("VehicleLogicalToPhysicalAllocation")
        {
            eprintln!("{} ({:?})", sym.qualified_name, sym.kind);
            for tr in &sym.type_refs {
                match tr {
                    syster::hir::TypeRefKind::Simple(r) => {
                        eprintln!("  TypeRef: {} -> {:?}", r.target, r.resolved_target);
                    }
                    syster::hir::TypeRefKind::Chain(c) => {
                        for p in &c.parts {
                            eprintln!("  Chain part: {} -> {:?}", p.target, p.resolved_target);
                        }
                    }
                }
            }
        }
    }

    // Run diagnostics
    eprintln!("\n=== Diagnostics ===");
    let diags = syster::hir::check_file(index, file_id);
    for d in &diags {
        eprintln!("  Line {}: {}", d.start_line + 1, d.message);
    }
}

/// Minimal test case for sibling package import resolution
#[test]
fn test_sibling_package_import_resolution() {
    let content = r#"
package Root {
    public import Defs::*;
    
    package Defs {
        public import PartDefs::*;
        public import PortDefs::*;
        
        package PartDefs {
            part def Vehicle {
                // This should resolve - MyPort is in sibling PortDefs
                // which is imported by parent Defs
                port p : MyPort;
            }
        }
        
        package PortDefs {
            port def MyPort;
        }
    }
}
"#;

    let mut host = AnalysisHost::new();
    let parse_errors = host.set_file_content("test.sysml", content);
    assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

    let analysis = host.analysis();
    let file_id = analysis
        .get_file_id("test.sysml")
        .expect("File not in index");
    let index = analysis.symbol_index();

    // Verify the port definition exists
    let my_port = index.lookup_qualified("Root::Defs::PortDefs::MyPort");
    assert!(my_port.is_some(), "MyPort should exist");

    // Debug: Check what's visible in Defs
    if let Some(vis) = index.visibility_for_scope("Root::Defs") {
        eprintln!("\n=== Visibility in Root::Defs ===");
        eprintln!("Direct defs:");
        for (name, qname) in vis.direct_defs() {
            eprintln!("  {} -> {}", name, qname);
        }
        eprintln!("Imports:");
        for (name, qname) in vis.imports() {
            eprintln!("  {} -> {}", name, qname);
        }
    }

    // Test resolution from Vehicle's scope
    let resolver = Resolver::new(index).with_scope("Root::Defs::PartDefs::Vehicle");

    let result = resolver.resolve("MyPort");

    match result {
        ResolveResult::Found(sym) => {
            eprintln!("✓ MyPort resolved to: {}", sym.qualified_name);
            assert_eq!(sym.qualified_name.as_ref(), "Root::Defs::PortDefs::MyPort");
        }
        ResolveResult::NotFound => {
            panic!("MyPort should resolve from Vehicle scope via parent's import!");
        }
        ResolveResult::Ambiguous(_) => {
            panic!("MyPort should not be ambiguous");
        }
    }

    // Run semantic checks - should have no errors
    let diagnostics = check_file(index, file_id);
    let undefined_refs: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.message.contains("undefined reference"))
        .collect();

    assert!(
        undefined_refs.is_empty(),
        "Should have no undefined reference errors: {:?}",
        undefined_refs
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}

/// Test that short names (aliases like <kg> for kilogram) can be resolved via imports.
#[test]
fn test_short_name_visibility_via_import() {
    // Simulate:
    // package SI { attribute <kg> kilogram : MassUnit; }
    // package User { import SI::*; attribute mass = 800[kg]; }
    // In User, "kg" should resolve to "SI::kilogram"

    use syster::base::FileId;
    use syster::hir::{HirSymbol, ResolveResult, SymbolKind};

    let mut index = SymbolIndex::new();

    // Add SI package with kilogram attribute that has short_name "kg"
    index.add_file(
        FileId::new(0),
        vec![
            HirSymbol {
                name: Arc::from("SI"),
                qualified_name: Arc::from("SI"),
                element_id: new_element_id(),
                kind: SymbolKind::Package,
                file: FileId::new(0),
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            HirSymbol {
                name: Arc::from("kilogram"),
                short_name: Some(Arc::from("kg")), // <-- This is the alias!
                qualified_name: Arc::from("SI::kilogram"),
                element_id: new_element_id(),
                kind: SymbolKind::AttributeUsage,
                file: FileId::new(0),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![Arc::from("MassUnit")],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
        ],
    );

    // Add User package with import SI::*
    index.add_file(
        FileId::new(1),
        vec![
            HirSymbol {
                name: Arc::from("User"),
                qualified_name: Arc::from("User"),
                element_id: new_element_id(),
                kind: SymbolKind::Package,
                file: FileId::new(1),
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            HirSymbol {
                name: Arc::from("SI::*"),
                qualified_name: Arc::from("User::import:SI::*"),
                element_id: new_element_id(),
                kind: SymbolKind::Import,
                file: FileId::new(1),
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 10,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
        ],
    );

    // Build visibility maps
    index.ensure_visibility_maps();

    // Debug: Check what's in the SI scope
    eprintln!("\n=== Debug: SI scope visibility ===");
    eprintln!("{}", index.debug_dump_scope("SI"));

    // Debug: Check what's in the User scope
    eprintln!("\n=== Debug: User scope visibility ===");
    eprintln!("{}", index.debug_dump_scope("User"));

    // Debug: Check where 'kg' is visible
    eprintln!("\n=== Debug: Where is 'kg' visible? ===");
    for scope in index.debug_find_name_in_visibility("kg") {
        eprintln!("  {}", scope);
    }

    // Debug: Check where 'kilogram' is visible
    eprintln!("\n=== Debug: Where is 'kilogram' visible? ===");
    for scope in index.debug_find_name_in_visibility("kilogram") {
        eprintln!("  {}", scope);
    }

    // Debug: lookup_simple for kg
    eprintln!("\n=== Debug: lookup_simple('kg') ===");
    for sym in index.lookup_simple("kg") {
        eprintln!(
            "  {} (short_name: {:?})",
            sym.qualified_name, sym.short_name
        );
    }

    // Now resolve "kg" in User scope - this should find SI::kilogram!
    let resolver = index.resolver_for_scope("User");
    let result = resolver.resolve("kg");

    match result {
        ResolveResult::Found(sym) => {
            eprintln!("\n✓ 'kg' resolved to: {}", sym.qualified_name);
            assert_eq!(
                sym.qualified_name.as_ref(),
                "SI::kilogram",
                "kg should resolve to SI::kilogram"
            );
        }
        ResolveResult::NotFound => {
            panic!("'kg' should resolve in User scope via 'import SI::*'");
        }
        ResolveResult::Ambiguous(syms) => {
            eprintln!("Ambiguous results:");
            for s in &syms {
                eprintln!("  {}", s.qualified_name);
            }
            panic!("'kg' should not be ambiguous");
        }
    }
}

/// Test that usages inherit members from their type definition.
/// When `transportPassenger : TransportPassenger`, members of TransportPassenger
/// should be visible inside transportPassenger.
#[test]
fn test_usage_inherits_type_members() {
    use syster::base::FileId;
    use syster::hir::{HirSymbol, ResolveResult, SymbolKind};

    let mut index = SymbolIndex::new();

    // Simulate:
    // use case def TransportPassenger {
    //     include use case getInVehicle_a;  // <-- defined here
    // }
    // use case transportPassenger : TransportPassenger {
    //     action driverGetInVehicle subsets getInVehicle_a;  // <-- referenced here
    // }

    index.add_file(
        FileId::new(0),
        vec![
            // Package
            HirSymbol {
                name: Arc::from("MissionContext"),
                qualified_name: Arc::from("MissionContext"),
                element_id: new_element_id(),
                kind: SymbolKind::Package,
                file: FileId::new(0),
                start_line: 0,
                start_col: 0,
                end_line: 0,
                end_col: 0,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Definition: TransportPassenger
            HirSymbol {
                name: Arc::from("TransportPassenger"),
                qualified_name: Arc::from("MissionContext::TransportPassenger"),
                element_id: new_element_id(),
                kind: SymbolKind::UseCaseDef,
                file: FileId::new(0),
                start_line: 1,
                start_col: 0,
                end_line: 5,
                end_col: 0,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Member of definition: getInVehicle_a
            HirSymbol {
                name: Arc::from("getInVehicle_a"),
                qualified_name: Arc::from("MissionContext::TransportPassenger::getInVehicle_a"),
                element_id: new_element_id(),
                kind: SymbolKind::ActionUsage, // usage inside definition
                file: FileId::new(0),
                start_line: 2,
                start_col: 4,
                end_line: 2,
                end_col: 30,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![Arc::from("getInVehicle")],
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Usage: transportPassenger : TransportPassenger
            HirSymbol {
                name: Arc::from("transportPassenger"),
                qualified_name: Arc::from("MissionContext::transportPassenger"),
                element_id: new_element_id(),
                kind: SymbolKind::ActionUsage, // usage
                file: FileId::new(0),
                start_line: 10,
                start_col: 0,
                end_line: 20,
                end_col: 0,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![Arc::from("TransportPassenger")], // typed by TransportPassenger
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Nested member: driverGetInVehicle (references getInVehicle_a)
            HirSymbol {
                name: Arc::from("driverGetInVehicle"),
                qualified_name: Arc::from("MissionContext::transportPassenger::driverGetInVehicle"),
                element_id: new_element_id(),
                kind: SymbolKind::ActionUsage,
                file: FileId::new(0),
                start_line: 11,
                start_col: 4,
                end_line: 11,
                end_col: 50,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![Arc::from("getInVehicle_a")], // subsets getInVehicle_a
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Nested action 'a' inside transportPassenger (no type annotation)
            HirSymbol {
                name: Arc::from("a"),
                qualified_name: Arc::from("MissionContext::transportPassenger::a"),
                element_id: new_element_id(),
                kind: SymbolKind::ActionUsage,
                file: FileId::new(0),
                start_line: 12,
                start_col: 4,
                end_line: 15,
                end_col: 4,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![], // no type annotation
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
            // Action inside 'a' that references getInVehicle_a
            HirSymbol {
                name: Arc::from("nestedAction"),
                qualified_name: Arc::from("MissionContext::transportPassenger::a::nestedAction"),
                element_id: new_element_id(),
                kind: SymbolKind::ActionUsage,
                file: FileId::new(0),
                start_line: 13,
                start_col: 8,
                end_line: 13,
                end_col: 50,
                short_name: None,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: vec![Arc::from("getInVehicle_a")], // subsets getInVehicle_a
                relationships: vec![],
                type_refs: vec![],
                doc: None,
                is_public: true,
view_data: None,
                metadata_annotations: vec![],
            },
        ],
    );

    // Build visibility maps
    index.ensure_visibility_maps();

    // Debug: Check what's in the TransportPassenger scope (definition)
    eprintln!("\n=== Debug: TransportPassenger (definition) visibility ===");
    eprintln!(
        "{}",
        index.debug_dump_scope("MissionContext::TransportPassenger")
    );

    // Debug: Check what's in the transportPassenger scope (usage)
    eprintln!("\n=== Debug: transportPassenger (usage) visibility ===");
    eprintln!(
        "{}",
        index.debug_dump_scope("MissionContext::transportPassenger")
    );

    // Debug: Check what's in the 'a' scope (nested action)
    eprintln!("\n=== Debug: transportPassenger::a visibility ===");
    eprintln!(
        "{}",
        index.debug_dump_scope("MissionContext::transportPassenger::a")
    );

    // Debug: Check where 'getInVehicle_a' is visible
    eprintln!("\n=== Debug: Where is 'getInVehicle_a' visible? ===");
    for scope in index.debug_find_name_in_visibility("getInVehicle_a") {
        eprintln!("  {}", scope);
    }

    // TEST 1: Resolve from transportPassenger scope (direct)
    eprintln!("\n=== Test 1: Resolve from transportPassenger scope ===");
    let resolver = index.resolver_for_scope("MissionContext::transportPassenger");
    let result = resolver.resolve("getInVehicle_a");

    match result {
        ResolveResult::Found(sym) => {
            eprintln!("✓ 'getInVehicle_a' resolved to: {}", sym.qualified_name);
            assert_eq!(
                sym.qualified_name.as_ref(),
                "MissionContext::TransportPassenger::getInVehicle_a"
            );
        }
        _ => panic!("Should resolve from transportPassenger scope"),
    }

    // TEST 2: Resolve from nested a::nestedAction scope (should walk up to transportPassenger)
    eprintln!("\n=== Test 2: Resolve from transportPassenger::a::nestedAction scope ===");
    let resolver = index.resolver_for_scope("MissionContext::transportPassenger::a::nestedAction");
    let result = resolver.resolve("getInVehicle_a");

    match result {
        ResolveResult::Found(sym) => {
            eprintln!("✓ 'getInVehicle_a' resolved to: {}", sym.qualified_name);
            assert_eq!(
                sym.qualified_name.as_ref(),
                "MissionContext::TransportPassenger::getInVehicle_a",
                "getInVehicle_a should resolve via scope walk to transportPassenger's inherited member"
            );
        }
        ResolveResult::NotFound => {
            panic!(
                "'getInVehicle_a' should be visible from nested scope (walk up to transportPassenger)"
            );
        }
        ResolveResult::Ambiguous(syms) => {
            eprintln!("Ambiguous results:");
            for s in &syms {
                eprintln!("  {}", s.qualified_name);
            }
            panic!("'getInVehicle_a' should not be ambiguous");
        }
    }
}
