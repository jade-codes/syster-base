//! Detailed failing tests for each LSP hover failure pattern.
//! These are minimal reproducible cases extracted from the vehicle example failures.
//!
//! Run with: cargo test --test test_lsp_failures_detailed -- --nocapture

use syster::ide::AnalysisHost;

fn create_analysis(source: &str) -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);
    host
}

fn has_hover_at(host: &mut AnalysisHost, line: u32, col: u32) -> Option<String> {
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    analysis.hover(file_id, line, col).map(|h| h.contents)
}

fn print_symbols(host: &mut AnalysisHost) {
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- SYMBOLS ---");
    for sym in index.symbols_in_file(file_id) {
        println!("  {} ({:?}) line {}-{}", sym.qualified_name, sym.kind, sym.start_line, sym.end_line);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("    type_ref[{}]: {:?}", i, tr);
        }
    }
}

// =============================================================================
// PATTERN 1: "transition initial then off" - state machine transitions
// Line 54: transition initial then off;
// Target: 'initial' at col 35-42, 'off' at col 48-51
// =============================================================================

#[test]
fn test_transition_initial_then_off() {
    let source = r#"
package TestPkg {
    state def VehicleStates {
        entry state initial;
        state off;
        state on;
        
        transition initial then off;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 7: transition initial then off;
    // 'initial' should have hover
    let hover = has_hover_at(&mut host, 7, 20);
    println!("\nHover on 'initial': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on transition source 'initial'");
    
    // 'off' should have hover
    let hover = has_hover_at(&mut host, 7, 32);
    println!("Hover on 'off': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on transition target 'off'");
}

// =============================================================================
// PATTERN 2: "if ignitionCmd.ignitionOnOff" - expression chains in conditions
// Line 59: if ignitionCmd.ignitionOnOff==IgnitionOnOff::on and brakePedalDepressed
// Target: 'ignitionCmd' at col 35-46
// =============================================================================

#[test]
fn test_expression_in_if_condition() {
    let source = r#"
package TestPkg {
    item def IgnitionCmd {
        attribute ignitionOnOff;
    }
    
    state def VehicleStates {
        in item ignitionCmd : IgnitionCmd;
        in attribute brakePedalDepressed;
        
        state off;
        state starting;
        
        transition off_To_starting
            first off
            if ignitionCmd.ignitionOnOff == 1 and brakePedalDepressed
            then starting;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 15: if ignitionCmd.ignitionOnOff == 1
    // 'ignitionCmd' should have hover (first part of chain)
    let hover = has_hover_at(&mut host, 15, 16);
    println!("\nHover on 'ignitionCmd': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on expression 'ignitionCmd'");
    
    // 'ignitionOnOff' should have hover (second part of chain)  
    let hover = has_hover_at(&mut host, 15, 28);
    println!("Hover on 'ignitionOnOff': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on 'ignitionOnOff' chain member");
}

// =============================================================================
// PATTERN 3: "bind shaftPort_d=differential.shaftPort_d" - binding connectors
// Line 635: bind shaftPort_d=differential.shaftPort_d;
// Target: 'shaftPort_d' at col 29-40, 'differential' at col 41-53
// =============================================================================

#[test]
fn test_bind_connector_chain() {
    let source = r#"
package TestPkg {
    port def ShaftPort;
    
    part def Differential {
        port shaftPort_d : ShaftPort;
    }
    
    part def RearAxle {
        port shaftPort_d : ShaftPort;
    }
    
    part def Assembly {
        part differential : Differential;
        part rearAxle : RearAxle;
        
        bind rearAxle.shaftPort_d = differential.shaftPort_d;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 16: bind rearAxle.shaftPort_d = differential.shaftPort_d;
    // 'differential' should have hover
    let hover = has_hover_at(&mut host, 16, 38);
    println!("\nHover on 'differential': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on bind RHS 'differential'");
    
    // 'shaftPort_d' (second part of chain) should have hover
    let hover = has_hover_at(&mut host, 16, 51);
    println!("Hover on 'shaftPort_d' (RHS chain): {:?}", hover);
    assert!(hover.is_some(), "Expected hover on RHS chain member 'shaftPort_d'");
}

// =============================================================================
// PATTERN 4: "redefines Vehicle::mass" - qualified redefines target
// Line 518: attribute mass redefines Vehicle::mass=dryMass+...
// Target: 'Vehicle::mass' at col 45-58
// =============================================================================

#[test]
fn test_qualified_redefines() {
    let source = r#"
package TestPkg {
    part def Vehicle {
        attribute mass;
        attribute dryMass;
    }
    
    part vehicle_a : Vehicle {
        attribute mass redefines Vehicle::mass = 100;
        attribute dryMass redefines Vehicle::dryMass = 50;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 8: attribute mass redefines Vehicle::mass = 100;
    // 'Vehicle::mass' should resolve and have hover
    // Note: Both Vehicle and Vehicle::mass have the same span, so hover could return either
    let hover = has_hover_at(&mut host, 8, 38);
    println!("\nHover at col 38 (within Vehicle::mass): {:?}", hover);
    assert!(hover.is_some(), "Expected hover within qualified redefines 'Vehicle::mass'");
    
    // The hover should be about Vehicle::mass (the redefines target) or Vehicle (namespace)
    let hover_text = hover.unwrap();
    assert!(hover_text.contains("Vehicle") || hover_text.contains("mass"), 
        "Hover should mention Vehicle or mass, got: {}", hover_text);
}

// =============================================================================
// PATTERN 5: ":>> mRefs" - shorthand redefines (specializes)
// Line 479: { :>> mRefs = (m, m, m); }
// Target: 'mRefs' at col 80-85
// =============================================================================

#[test]
fn test_shorthand_redefines_mRefs() {
    let source = r#"
package TestPkg {
    attribute def CoordinateFrame {
        attribute mRefs;
    }
    
    part def Context {
        attribute spatialCF : CoordinateFrame { :>> mRefs = 1; }
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 7: attribute spatialCF : CoordinateFrame { :>> mRefs = 1; }
    // 'mRefs' after :>> should have hover
    let hover = has_hover_at(&mut host, 7, 52);
    println!("\nHover on 'mRefs': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on :>> target 'mRefs'");
    assert!(hover.as_ref().unwrap().contains("mRefs"), 
        "Hover should mention mRefs, got: {}", hover.unwrap());
}

// =============================================================================
// PATTERN 6: ":>> torqueGenerationRequirement" - requirement redefines
// Line 653: requirement torqueGenerationRequirement :>> torqueGenerationRequirement{
// =============================================================================

#[test]
fn test_requirement_shorthand_redefines() {
    let source = r#"
package TestPkg {
    requirement def TorqueRequirement {
        doc /* Torque generation requirement */
    }
    
    part def Engine {
        requirement torqueGenerationRequirement : TorqueRequirement;
    }
    
    part engine : Engine {
        requirement torqueGenerationRequirement :>> torqueGenerationRequirement {
            doc /* Engine torque requirement */
        }
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 11: requirement torqueGenerationRequirement :>> torqueGenerationRequirement
    // The :>> target 'torqueGenerationRequirement' should resolve to Engine::torqueGenerationRequirement
    let hover = has_hover_at(&mut host, 11, 56);
    println!("\nHover on :>> torqueGenerationRequirement: {:?}", hover);
    assert!(hover.is_some(), "Expected hover on :>> requirement target");
}

// =============================================================================
// PATTERN 7: "message of IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1"
// Line 781: message of IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
// Targets: 'turnVehicleOn' at col 71-84, 'trigger1' at col 96-104
// =============================================================================

#[test]
fn test_message_chain_members() {
    let source = r#"
package TestPkg {
    item def IgnitionCmd;
    
    part def Driver {
        action turnVehicleOn;
    }
    
    part def Vehicle {
        action trigger1;
    }
    
    part def Sequence {
        part driver : Driver;
        part vehicle : Vehicle;
        
        message of IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 16: message of IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    // 'turnVehicleOn' should have hover (chain member)
    let hover = has_hover_at(&mut host, 16, 47);
    println!("\nHover on 'turnVehicleOn': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on message chain member 'turnVehicleOn'");
    
    // 'trigger1' should have hover (chain member)
    let hover = has_hover_at(&mut host, 16, 68);
    println!("Hover on 'trigger1': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on message chain member 'trigger1'");
}

// =============================================================================
// PATTERN 8: "first driverGetInVehicle then join1"
// Line 1423: first driverGetInVehicle then join1;
// Target: 'join1' at col 46-51
// =============================================================================

#[test]
fn test_first_then_succession() {
    let source = r#"
package TestPkg {
    action def TransportPassenger {
        action driverGetInVehicle;
        merge join1;
        
        first driverGetInVehicle then join1;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 6: first driverGetInVehicle then join1;
    // 'join1' should have hover
    let hover = has_hover_at(&mut host, 6, 43);
    println!("\nHover on 'join1': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on 'join1' after then");
    
    // 'driverGetInVehicle' should have hover  
    let hover = has_hover_at(&mut host, 6, 15);
    println!("Hover on 'driverGetInVehicle': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on 'driverGetInVehicle' after first");
}

// =============================================================================
// PATTERN 9: "action driverGetInVehicle subsets getInVehicle_a[1]"
// Line 1387: action driverGetInVehicle subsets getInVehicle_a[1];
// Target: 'getInVehicle_a' at col 54-68
// =============================================================================

#[test]
fn test_subsets_action() {
    let source = r#"
package TestPkg {
    action def GetInVehicle;
    
    action def TransportPassenger {
        action getInVehicle_a : GetInVehicle;
        
        action driverGetInVehicle subsets getInVehicle_a;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 7: action driverGetInVehicle subsets getInVehicle_a;
    // 'getInVehicle_a' should have hover
    let hover = has_hover_at(&mut host, 7, 44);
    println!("\nHover on 'getInVehicle_a': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on subsets target 'getInVehicle_a'");
}

// =============================================================================
// PATTERN 10: "assume constraint {assumedCargoMass<=500 [kg]}"
// Line 884: assume constraint {assumedCargoMass<=500 [kg]}
// Target: 'assumedCargoMass' at col 47-63
// =============================================================================

#[test]
fn test_assume_constraint_expression() {
    let source = r#"
package TestPkg {
    requirement def FuelEconomyRequirement {
        attribute assumedCargoMass;
        
        assume constraint { assumedCargoMass <= 500; }
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 5: assume constraint { assumedCargoMass <= 500; }
    // 'assumedCargoMass' should have hover
    let hover = has_hover_at(&mut host, 5, 30);
    println!("\nHover on 'assumedCargoMass': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on constraint expression 'assumedCargoMass'");
}

// =============================================================================
// PATTERN 11: "accept when senseTemperature.temp > Tmax"
// Line 94: accept when senseTemperature.temp > Tmax
// Target: 'temp' at col 57-61
// =============================================================================

#[test]
fn test_accept_when_chain() {
    let source = r#"
package TestPkg {
    item def Temperature {
        attribute temp;
    }
    
    state def HealthStates {
        in item senseTemperature : Temperature;
        attribute Tmax;
        
        state normal;
        state degraded;
        
        transition normal_To_degraded
            first normal
            accept when senseTemperature.temp > Tmax
            then degraded;
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 15: accept when senseTemperature.temp > Tmax
    // 'temp' chain member should have hover
    let hover = has_hover_at(&mut host, 15, 41);
    println!("\nHover on 'temp' in chain: {:?}", hover);
    assert!(hover.is_some(), "Expected hover on chain member 'temp'");
}

// =============================================================================
// PATTERN 12: "connect [5] lugNutPort ::> wheel1.lugNutCompositePort"
// Line 970: connect [5] lugNutPort ::> lugNutCompositePort.lugNutPort to ...
// Target: 'lugNutPort' at col 75-85 (chain member)
// =============================================================================

#[test]
fn test_connect_featured_by_chain() {
    let source = r#"
package TestPkg {
    port def LugNutPort;
    
    part def CompositePort {
        port lugNutPort : LugNutPort;
    }
    
    interface def WheelHubInterface {
        end lugNutCompositePort : CompositePort;
    }
    
    part def WheelHubAssembly {
        interface wheelHubInterface : WheelHubInterface {
            connect lugNutPort ::> lugNutCompositePort.lugNutPort;
        }
    }
}
"#;
    
    let mut host = create_analysis(source);
    print_symbols(&mut host);
    
    // Line 14: connect lugNutPort ::> lugNutCompositePort.lugNutPort;
    // 'lugNutPort' (chain member) should have hover
    let hover = has_hover_at(&mut host, 14, 52);
    println!("\nHover on chain member 'lugNutPort': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on ::> chain member 'lugNutPort'");
}
