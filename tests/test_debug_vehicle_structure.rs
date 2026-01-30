//! Debug test for the vehicle example structure

use syster::base::FileId;
use syster::hir::{extract_symbols_unified, SymbolIndex};
use syster::syntax::parser::parse_content;
use syster::syntax::SyntaxFile;
use std::path::Path;

#[test]
fn debug_vehicle_structure() {
    // Exact structure from SimpleVehicleModel
    let source = r#"package SimpleVehicleModel {
    public import Definitions::*;  
    
    package Definitions {
        public import PartDefinitions::*;
        public import ActionDefinitions::*;
        
        package ActionDefinitions {
            action def ProvidePower {
                action distributeTorque;
            }
        }
        
        package PartDefinitions {
            part def Vehicle {
                perform action providePower;
            }
        }
    }
    
    package VehicleConfigurations {
        package VehicleConfiguration_b {
            package ActionTree {  
            }
            
            package PartsTree {
                part vehicle_b : Vehicle {
                    perform ActionTree::providePower redefines providePower;
                    
                    part rearAxleAssembly {
                        perform providePower.distributeTorque;
                    }
                }
            }
            
            package ActionTree {
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
    index.add_file(FileId::new(0), symbols);
    index.ensure_visibility_maps();
    index.resolve_all_type_refs();
    
    // Check if providePower in Vehicle got implicit typing
    println!("\n=== Checking Vehicle::providePower ===");
    if let Some(sym) = index.lookup_qualified("SimpleVehicleModel::Definitions::PartDefinitions::Vehicle::providePower") {
        println!("Found: supertypes = {:?}", sym.supertypes);
    } else {
        println!("NOT FOUND!");
    }
    
    // Check visibility for vehicle_b
    println!("\n=== Visibility for vehicle_b ===");
    let vb_scope = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b";
    if let Some(vis) = index.visibility_for_scope(vb_scope) {
        println!("  Direct:");
        for (name, qname) in vis.direct_defs() {
            if name.contains("providePower") || name.contains("rearAxle") {
                println!("    {} -> {}", name, qname);
            }
        }
    } else {
        println!("  NO visibility map");
    }
    
    // Check visibility for rearAxleAssembly
    let ra_scope = "SimpleVehicleModel::VehicleConfigurations::VehicleConfiguration_b::PartsTree::vehicle_b::rearAxleAssembly";
    
    // Try to resolve providePower from rearAxleAssembly
    let resolver = index.resolver_for_scope(ra_scope);
    let pp_result = resolver.resolve("providePower");
    println!("\n=== Resolving 'providePower' from rearAxleAssembly ===");
    match &pp_result {
        syster::hir::ResolveResult::Found(sym) => {
            println!("Found: {}", sym.qualified_name);
            println!("  supertypes: {:?}", sym.supertypes);
        }
        other => println!("Result: {:?}", other),
    }
    
    // Try chain resolution
    let chain_parts: Vec<std::sync::Arc<str>> = vec!["providePower".into(), "distributeTorque".into()];
    let chain_result = index.resolve_feature_chain_member(ra_scope, &chain_parts, 1);
    println!("\n=== resolve_feature_chain_member for 'providePower.distributeTorque' ===");
    println!("Result: {:?}", chain_result);
    
    // Try hover
    use syster::ide::hover;
    
    // Find the line
    let lines: Vec<&str> = source.lines().collect();
    let mut target_line = 0;
    let mut target_col = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("perform providePower.distributeTorque") {
            if let Some(pos) = line.find("distributeTorque") {
                target_line = i as u32;
                target_col = pos as u32;
                println!("\nFound 'distributeTorque' at line {} col {} in: {}", target_line, target_col, line.trim());
                break;
            }
        }
    }
    
    let hover_result = hover(&index, FileId::new(0), target_line, target_col);
    println!("\n=== hover result ===");
    println!("Result: {:?}", hover_result.is_some());
    if let Some(h) = &hover_result {
        println!("  qualified_name: {:?}", h.qualified_name);
    }
    
    assert!(hover_result.is_some(), "Should get hover for distributeTorque");
}
