//! Debug test to inspect visibility maps for the vehicle example structure

use syster::base::FileId;
use syster::hir::{extract_symbols_unified, SymbolIndex};
use syster::syntax::parser::parse_content;
use syster::syntax::SyntaxFile;
use std::path::Path;

#[test]
fn debug_visibility_maps_for_redefines() {
    // Exact structure from SimpleVehicleModel that we're struggling with
    let source = r#"package SimpleVehicleModel {
    public import Definitions::*;  
    
    package Definitions {
        public import PartDefinitions::*;
        public import ActionDefinitions::*;
        
        package ActionDefinitions {
            action def ProvidePower {
                // Note: ProvidePower definition does NOT have distributeTorque
                in item pwrCmd;
            }
        }
        
        package PartDefinitions {
            part def Vehicle {
                // Untyped perform - should get implicit typing to ProvidePower
                perform action providePower;
            }
        }
    }
    
    package VehicleConfigurations {
        package VehicleConfiguration_b {
            package PartsTree {
                part vehicle_b : Vehicle {
                    // This is the key line: redefines providePower with ActionTree::providePower
                    perform ActionTree::providePower redefines providePower;
                    
                    part rearAxleAssembly {
                        // This should resolve providePower to ActionTree::providePower
                        // then find distributeTorque in it
                        perform providePower.distributeTorque;
                    }
                }
            }
            
            package ActionTree {
                // This is the specialized usage that HAS distributeTorque
                action providePower: ProvidePower {
                    action distributeTorque;
                }
            }
        }
    }
}"#;
    
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);
    
    // Build index
    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols.clone());
    
    println!("\n{}", "=".repeat(80));
    println!("BEFORE ensure_visibility_maps()");
    println!("{}", "=".repeat(80));
    
    // Check symbols with redefines
    println!("\n=== Symbols with Redefines type_refs ===");
    for sym in &symbols {
        let has_redefines = sym.type_refs.iter().any(|trk| {
            trk.as_refs().iter().any(|tr| tr.kind == syster::hir::RefKind::Redefines)
        });
        if has_redefines {
            println!("\n{} ({:?})", sym.qualified_name, sym.kind);
            for (i, trk) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, trk);
            }
        }
    }
    
    // Build visibility maps
    index.ensure_visibility_maps();
    
    println!("\n{}", "=".repeat(80));
    println!("AFTER ensure_visibility_maps()");
    println!("{}", "=".repeat(80));
    
    // Check Vehicle::providePower for implicit typing
    println!("\n=== Vehicle::providePower ===");
    let vehicle_pp = "SimpleVehicleModel::Definitions::PartDefinitions::Vehicle::providePower";
    if let Some(sym) = index.lookup_qualified(vehicle_pp) {
        println!("Found: {}", sym.qualified_name);
        println!("  supertypes: {:?}", sym.supertypes);
        println!("  type_refs: {:?}", sym.type_refs);
    } else {
        println!("NOT FOUND!");
    }
    
    // Check ActionTree::providePower
    println!("\n=== ActionTree::providePower ===");
    let actiontree_pp = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::ActionTree::providePower";
    if let Some(sym) = index.lookup_qualified(actiontree_pp) {
        println!("Found: {}", sym.qualified_name);
        println!("  supertypes: {:?}", sym.supertypes);
        println!("  type_refs: {:?}", sym.type_refs);
    } else {
        println!("NOT FOUND!");
    }
    
    // Check visibility map for vehicle_b
    println!("\n=== Visibility map for vehicle_b ===");
    let vehicle_b = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b";
    if let Some(vis) = index.visibility_for_scope(vehicle_b) {
        println!("Direct definitions:");
        for (name, qname) in vis.direct_defs() {
            if name.contains("providePower") || name.contains("rearAxle") {
                println!("  {} -> {}", name, qname);
            }
        }
        println!("\nAll entries with 'provide':");
        for (name, qname) in vis.direct_defs() {
            if name.to_lowercase().contains("provide") {
                println!("  {} -> {}", name, qname);
            }
        }
    } else {
        println!("NO visibility map!");
    }
    
    // Lookup the anonymous symbol to see its full structure
    println!("\n=== Anonymous providePower symbol ===");
    let anon_pp = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b::<:>>providePower#1@L0>";
    if let Some(sym) = index.lookup_qualified(anon_pp) {
        println!("Found: {}", sym.qualified_name);
        println!("  name: {}", sym.name);
        println!("  kind: {:?}", sym.kind);
        println!("  supertypes: {:?}", sym.supertypes);
        println!("  type_refs:");
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("    [{}]: {:?}", i, tr);
        }
    } else {
        println!("NOT FOUND!");
    }
    
    // Check visibility map for rearAxleAssembly
    println!("\n=== Visibility map for rearAxleAssembly ===");
    let rear_axle = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b::rearAxleAssembly";
    if let Some(vis) = index.visibility_for_scope(rear_axle) {
        println!("Direct definitions:");
        for (name, qname) in vis.direct_defs() {
            println!("  {} -> {}", name, qname);
        }
    } else {
        println!("NO visibility map!");
    }
    
    // Try resolving providePower from rearAxleAssembly
    println!("\n=== Resolving 'providePower' from rearAxleAssembly ===");
    let resolver = index.resolver_for_scope(rear_axle);
    let result = resolver.resolve("providePower");
    println!("Result: {:?}", result);
    
    // Now try resolve_all_type_refs and check chain resolution
    index.resolve_all_type_refs();
    
    println!("\n=== After resolve_all_type_refs ===");
    
    // Try feature chain resolution
    let chain_parts: Vec<std::sync::Arc<str>> = vec!["providePower".into(), "distributeTorque".into()];
    let chain_result = index.resolve_feature_chain_member(rear_axle, &chain_parts, 1);
    println!("\nresolve_feature_chain_member('providePower.distributeTorque') from rearAxleAssembly:");
    println!("  Result: {:?}", chain_result);
    
    // Try hover
    use syster::ide::hover;
    
    // Find the distributeTorque line
    let lines: Vec<&str> = source.lines().collect();
    let mut target_line = 0;
    let mut target_col = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("perform providePower.distributeTorque") {
            if let Some(pos) = line.find("distributeTorque") {
                target_line = i as u32;
                target_col = pos as u32;
                println!("\nFound target at line {} col {}: {}", target_line, target_col, line.trim());
            }
        }
    }
    
    let hover_result = hover(&index, FileId::new(0), target_line, target_col);
    println!("\nHover result: {:?}", hover_result.as_ref().map(|h| &h.qualified_name));
}
