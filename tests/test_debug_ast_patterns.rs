//! Debug test to inspect AST symbol extraction for failing patterns

use std::path::Path;
use syster::base::FileId;
use syster::hir::{SymbolIndex, extract_symbols_unified};
use syster::syntax::parser::parse_content;

/// Test redefines pattern: `ref item redefines fuel` - nested in a part
#[test]
fn debug_ast_redefines_pattern() {
    // More accurate to the actual vehicle example - fuel is in fuelTank, not directly in vehicle
    let source = r#"package Test {
    part def FuelTank {
        item fuel;
    }
    
    part def Vehicle {
        part fuelTank : FuelTank;
    }
    
    part vehicle_a : Vehicle {
        part redefines fuelTank {
            ref item redefines fuel;
        }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== REDEFINES PATTERN (NESTED): ref item redefines fuel ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Now test resolution
    let mut index = SymbolIndex::new();
    index.add_file(FileId::new(0), symbols.clone());
    index.ensure_visibility_maps();
    index.resolve_all_type_refs(); // This should populate resolved_target

    println!("\n=== RESOLUTION TEST ===");

    // Check what symbols have type_refs covering line 11 col 31
    println!("\n=== Symbols with type_refs at line 11, col 31 (after resolve_all_type_refs) ===");
    for sym in index.symbols_in_file(FileId::new(0)) {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if tr.start_line == 11 && tr.start_col <= 31 && tr.end_col >= 31 {
                    println!("{}: {:?}", sym.qualified_name, tr);
                }
            }
        }
    }

    // Check visibility map for the redefined fuelTank
    let fuel_tank_redef = "Test::vehicle_a::<:>>fuelTank#1@L0>";
    if let Some(vis) = index.visibility_for_scope(fuel_tank_redef) {
        println!("\nVisibility map for redefined fuelTank:");
        for (name, qname) in vis.direct_defs() {
            println!("  {} -> {}", name, qname);
        }
    } else {
        println!("\nNO visibility map for {}", fuel_tank_redef);
    }

    // Try to resolve 'fuel' from the redefined fuelTank
    let resolver = index.resolver_for_scope(fuel_tank_redef);
    let result = resolver.resolve("fuel");
    println!("\nResolving 'fuel' from redefined fuelTank: {:?}", result);

    // Try hover at the 'fuel' reference
    use syster::ide::hover;
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("ref item redefines fuel") {
            if let Some(pos) = line.rfind("fuel") {
                println!("\nTrying hover at line {} col {} (redefines fuel)", i, pos);
                let hover_result = hover(&index, FileId::new(0), i as u32, pos as u32);
                println!(
                    "Hover result: {:?}",
                    hover_result.as_ref().map(|h| &h.qualified_name)
                );
            }
        }
    }
}

/// Test featured by pattern: `:>> mRefs`
#[test]
fn debug_ast_featured_by_pattern() {
    let source = r#"package Test {
    attribute def CartesianSpatial3dCoordinateFrame {
        attribute mRefs;
    }
    
    part def Vehicle {
        attribute spatialCF: CartesianSpatial3dCoordinateFrame[1] { 
            :>> mRefs = (1, 2, 3); 
        }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== FEATURED BY PATTERN: :>> mRefs ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check if 'mRefs' appears anywhere
    println!("\n=== Symbols with 'mRefs' ===");
    for sym in &symbols {
        if sym.name.contains("mRefs") || sym.qualified_name.contains("mRefs") {
            println!("{}: {:?}", sym.qualified_name, sym.type_refs);
        }
        let has_mrefs = sym
            .type_refs
            .iter()
            .any(|trk| trk.as_refs().iter().any(|tr| tr.target.contains("mRefs")));
        if has_mrefs {
            println!(
                "{} has mRefs in type_refs: {:?}",
                sym.qualified_name, sym.type_refs
            );
        }
    }
}

/// Test subsets pattern: `action X subsets Y`
#[test]
fn debug_ast_subsets_pattern() {
    let source = r#"package Test {
    action def GetInVehicle;
    
    action getInVehicle_a : GetInVehicle[1];
    
    action scenario {
        action driverGetIn subsets getInVehicle_a[1];
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== SUBSETS PATTERN: action X subsets Y ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check if 'getInVehicle_a' appears anywhere
    println!("\n=== Symbols with 'getInVehicle_a' in refs ===");
    for sym in &symbols {
        let has_target = sym.type_refs.iter().any(|trk| {
            trk.as_refs()
                .iter()
                .any(|tr| tr.target.contains("getInVehicle"))
        });
        if has_target {
            println!("{}: {:?}", sym.qualified_name, sym.type_refs);
        }
    }
}

/// Test transition pattern: `transition initial then off`
#[test]
fn debug_ast_transition_pattern() {
    let source = r#"package Test {
    state def OperatingStates {
        entry state off;
        state on;
        
        transition initial then off;
        transition off_to_on
            first off
            accept ignitionCmd
            then on;
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== TRANSITION PATTERN: transition initial then off ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check for initial/off in refs
    println!("\n=== Symbols referencing 'initial', 'off', 'on' ===");
    for sym in &symbols {
        let has_target = sym.type_refs.iter().any(|trk| {
            trk.as_refs().iter().any(|tr| {
                tr.target.as_ref() == "initial"
                    || tr.target.as_ref() == "off"
                    || tr.target.as_ref() == "on"
            })
        });
        if has_target {
            println!("{}: {:?}", sym.qualified_name, sym.type_refs);
        }
    }
}

/// Test specializes pattern: `requirement X :> Y`
#[test]
fn debug_ast_specializes_pattern() {
    let source = r#"package Test {
    requirement def MassRequirement {
        attribute massActual;
        attribute massLimit;
    }
    
    requirement vehicleMassRequirement : MassRequirement {
        attribute :>> massLimit = 100;
    }
    
    part vehicle_b {
        requirement myMassReq :> vehicleMassRequirement {
            attribute :>> massActual = 50;
        }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== SPECIALIZES PATTERN: requirement X :> Y ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check for vehicleMassRequirement refs
    println!("\n=== Symbols referencing 'vehicleMassRequirement' ===");
    for sym in &symbols {
        let has_target = sym.type_refs.iter().any(|trk| {
            trk.as_refs()
                .iter()
                .any(|tr| tr.target.contains("vehicleMassRequirement"))
        });
        if has_target {
            println!("{}: {:?}", sym.qualified_name, sym.type_refs);
        }
        // Also check supertypes
        if sym
            .supertypes
            .iter()
            .any(|s| s.contains("vehicleMassRequirement"))
        {
            println!(
                "{} has supertype vehicleMassRequirement",
                sym.qualified_name
            );
        }
    }
}

/// Test bind feature chain: `bind a.b.c = d.e.f`
#[test]
fn debug_ast_bind_chain_pattern() {
    let source = r#"package Test {
    port def RoadPort {
        attribute friction;
    }
    
    part def Wheel {
        port wheelToRoadPort : RoadPort;
    }
    
    part def Vehicle {
        port vehicleToRoadPort {
            port wheelToRoadPort1 : RoadPort;
        }
        part rearWheel1 : Wheel;
        
        bind rearWheel1.wheelToRoadPort = vehicleToRoadPort.wheelToRoadPort1;
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== BIND CHAIN PATTERN: bind a.b = c.d ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check for chain refs
    println!("\n=== Symbols with chain type_refs ===");
    for sym in &symbols {
        for trk in &sym.type_refs {
            if let syster::hir::TypeRefKind::Chain(_) = trk {
                println!("{}: {:?}", sym.qualified_name, trk);
            }
        }
    }
}

/// Test constraint pattern: `assert constraint X`
#[test]
fn debug_ast_constraint_pattern() {
    let source = r#"package Test {
    part def FuelTank {
        attribute fuel;
        attribute fuelMassMax;
        
        assert constraint fuelConstraint { fuel <= fuelMassMax }
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    let syntax = parse;
    let symbols = extract_symbols_unified(FileId::new(0), &syntax);

    println!("\n=== CONSTRAINT PATTERN: assert constraint X ===\n");

    for sym in &symbols {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.supertypes.is_empty() {
            println!("  supertypes: {:?}", sym.supertypes);
        }
        if !sym.type_refs.is_empty() {
            println!("  type_refs:");
            for tr in &sym.type_refs {
                println!("    {:?}", tr);
            }
        }
    }

    // Check if fuelConstraint appears
    println!("\n=== Looking for 'fuelConstraint' ===");
    for sym in &symbols {
        if sym.name.contains("fuelConstraint") {
            println!("Found symbol: {} ({:?})", sym.qualified_name, sym.kind);
        }
        let has_target = sym.type_refs.iter().any(|trk| {
            trk.as_refs()
                .iter()
                .any(|tr| tr.target.contains("fuelConstraint"))
        });
        if has_target {
            println!(
                "{} references fuelConstraint: {:?}",
                sym.qualified_name, sym.type_refs
            );
        }
    }
}
