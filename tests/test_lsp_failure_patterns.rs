//! Tests reproducing LSP hover failure patterns from SimpleVehicleModel.sysml
//!
//! Each test recreates the exact failing scenario from test-failure-report.md

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
        println!(
            "  {} ({:?}) line {}-{}",
            sym.qualified_name, sym.kind, sym.start_line, sym.end_line
        );
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("    type_ref[{}]: {:?}", i, tr);
        }
    }
}

// =============================================================================
// PATTERN 1: "other" - bind with feature chain (555 occurrences)
// Line 635: bind shaftPort_d=differential.shaftPort_d;
// Target: 'shaftPort_d' at col 29-40
// =============================================================================

#[test]
fn test_pattern_other_bind_chain() {
    let source = r#"
package TestPkg {
    port def ShaftPort;
    
    part def Differential {
        port shaftPort_d : ShaftPort;
    }
    
    part def RearAxleAssembly {
        part differential : Differential;
        port shaftPort_d : ShaftPort;
        
        bind shaftPort_d = differential.shaftPort_d;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 12: bind shaftPort_d = differential.shaftPort_d;
    // Test hover on first shaftPort_d (bind target)
    let hover = has_hover_at(&mut host, 12, 17);
    println!("\nHover on first 'shaftPort_d': {:?}", hover);
    assert!(
        hover.is_some(),
        "Expected hover on bind target 'shaftPort_d'"
    );
    assert!(
        hover.as_ref().unwrap().contains("shaftPort_d"),
        "Hover should mention shaftPort_d, got: {}",
        hover.unwrap()
    );
}

// =============================================================================
// PATTERN 2: "redefines" - item redefines in port (209 occurrences)
// Line 234: in item fuelCmd:FuelCmd redefines pwrCmd;
// Target: 'pwrCmd' at col 50-56
// =============================================================================

#[test]
fn test_pattern_redefines_in_port() {
    let source = r#"
package TestPkg {
    item def PwrCmd;
    item def FuelCmd :> PwrCmd;
    
    port def PwrCmdPort {
        in item pwrCmd : PwrCmd;
    }
    
    port def FuelCmdPort :> PwrCmdPort {
        in item fuelCmd : FuelCmd redefines pwrCmd;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 10: in item fuelCmd : FuelCmd redefines pwrCmd;
    // Test hover on 'pwrCmd' after redefines
    let hover = has_hover_at(&mut host, 10, 48);
    println!("\nHover on 'pwrCmd': {:?}", hover);
    assert!(
        hover.is_some(),
        "Expected hover on redefines target 'pwrCmd'"
    );
    // Should resolve to PwrCmdPort::pwrCmd, not FuelCmd
    assert!(
        hover.as_ref().unwrap().contains("pwrCmd"),
        "Hover should mention pwrCmd, got: {}",
        hover.unwrap()
    );
}

// =============================================================================
// PATTERN 3: "specializes (:>>)" - shorthand redefines (113 occurrences)
// Line 479: attribute spatialCF: CartesianSpatial3dCoordinateFrame[1] { :>> mRefs = (m, m, m); }
// Target: 'mRefs' at col 80-85
// =============================================================================

#[test]
fn test_pattern_specializes_shorthand_redefines() {
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
    // Test hover on 'mRefs' after :>>
    let hover = has_hover_at(&mut host, 7, 52);
    println!("\nHover on 'mRefs': {:?}", hover);
    assert!(hover.is_some(), "Expected hover on :>> target 'mRefs'");
    assert!(
        hover.as_ref().unwrap().contains("mRefs"),
        "Hover should mention mRefs, got: {}",
        hover.unwrap()
    );
}

// =============================================================================
// PATTERN 4: "then (transition)" - transition source/target (39 occurrences)
// Line 54: transition initial then off;
// Target: 'initial' at col 35-42
// =============================================================================

#[test]
fn test_pattern_transition_then() {
    let source = r#"
package TestPkg {
    state def VehicleStates {
        state initial;
        state off;
        state on;
        
        transition initial then off;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 7: transition initial then off;
    // Test hover on 'initial'
    let hover1 = has_hover_at(&mut host, 7, 21);
    println!("\nHover on 'initial': {:?}", hover1);
    assert!(
        hover1.is_some(),
        "Expected hover on transition source 'initial'"
    );
    assert!(
        hover1.as_ref().unwrap().contains("initial"),
        "Hover should mention initial, got: {}",
        hover1.unwrap()
    );

    // Test hover on 'off'
    let hover2 = has_hover_at(&mut host, 7, 33);
    println!("\nHover on 'off': {:?}", hover2);
    assert!(
        hover2.is_some(),
        "Expected hover on transition target 'off'"
    );
    assert!(
        hover2.as_ref().unwrap().contains("off"),
        "Hover should mention off, got: {}",
        hover2.unwrap()
    );
}

// =============================================================================
// PATTERN 5: "expression/value" - if expression chain (35 occurrences)
// Line 59: if ignitionCmd.ignitionOnOff==IgnitionOnOff::on and brakePedalDepressed
// Target: 'ignitionCmd' at col 35-46, 'ignitionOnOff' at col 47-60
// =============================================================================

#[test]
fn test_pattern_expression_chain_in_if() {
    let source = r#"
package TestPkg {
    item def IgnitionCmd {
        attribute ignitionOnOff;
    }
    
    state def OperatingStates {
        in item ignitionCmd : IgnitionCmd;
        
        transition off_To_starting
            first off
            accept ignitionCmd
            if ignitionCmd.ignitionOnOff == 1
            then starting;
        
        state off;
        state starting;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 12: if ignitionCmd.ignitionOnOff == 1
    // Test hover on 'ignitionCmd'
    let hover1 = has_hover_at(&mut host, 12, 18);
    println!("\nHover on 'ignitionCmd': {:?}", hover1);
    assert!(
        hover1.is_some(),
        "Expected hover on expression 'ignitionCmd'"
    );

    // Test hover on 'ignitionOnOff' (chain member)
    let hover2 = has_hover_at(&mut host, 12, 32);
    println!("\nHover on 'ignitionOnOff': {:?}", hover2);
    assert!(
        hover2.is_some(),
        "Expected hover on chain member 'ignitionOnOff'"
    );
}

// =============================================================================
// PATTERN 6: "featured by (::>)" - connect with feature reference (32 occurrences)
// Line 970: connect [5] lugNutPort ::> lugNutCompositePort.lugNutPort to ...
// Target: 'lugNutPort' at col 40-50
// =============================================================================

#[test]
fn test_pattern_featured_by_connect() {
    let source = r#"
package TestPkg {
    port def LugNutPort;
    
    part def LugNutCompositePort {
        port lugNutPort : LugNutPort[5];
    }
    
    part def WheelFastenerInterface {
        part lugNutCompositePort : LugNutCompositePort;
        
        connect lugNutPort ::> lugNutCompositePort.lugNutPort to shankPort;
        port shankPort;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 11: connect lugNutPort ::> lugNutCompositePort.lugNutPort to shankPort;
    // Test hover on 'lugNutPort' after ::>
    let hover = has_hover_at(&mut host, 11, 36);
    println!("\nHover on 'lugNutPort' (chain first): {:?}", hover);
    // Note: This tests the ::> (featured by) pattern
}

// =============================================================================
// PATTERN 7: "subsets" - action subsets (8 occurrences)
// Line 1387: action driverGetInVehicle subsets getInVehicle_a[1];
// Target: 'getInVehicle_a' at col 54-68
// =============================================================================

#[test]
fn test_pattern_subsets_action() {
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
    // Test hover on 'getInVehicle_a'
    let hover = has_hover_at(&mut host, 7, 46);
    println!("\nHover on 'getInVehicle_a': {:?}", hover);
    assert!(
        hover.is_some(),
        "Expected hover on subsets target 'getInVehicle_a'"
    );
    assert!(
        hover.as_ref().unwrap().contains("getInVehicle_a"),
        "Hover should mention getInVehicle_a, got: {}",
        hover.unwrap()
    );
}

// =============================================================================
// PATTERN 8: "first (transition)" - first action reference (7 occurrences)
// Line 1385: first start;
// Target: 'start' at col 22-27
// =============================================================================

#[test]
fn test_pattern_first_action() {
    let source = r#"
package TestPkg {
    action def TransportPassenger {
        action start;
        action middle;
        action end;
        
        first start then middle;
        first middle then end;
    }
}
"#;

    let mut host = create_analysis(source);
    print_symbols(&mut host);

    // Line 7: first start then middle;
    // Test hover on 'start'
    let hover1 = has_hover_at(&mut host, 7, 17);
    println!("\nHover on 'start': {:?}", hover1);
    assert!(hover1.is_some(), "Expected hover on first source 'start'");

    // Test hover on 'middle'
    let hover2 = has_hover_at(&mut host, 7, 28);
    println!("\nHover on 'middle': {:?}", hover2);
    assert!(hover2.is_some(), "Expected hover on first target 'middle'");
}

// =============================================================================
// PATTERN 9: "constraint" - assume constraint expression (7 occurrences)
// Line 884: assume constraint {assumedCargoMass<=500 [kg]}
// Target: 'assumedCargoMass' at col 47-63
// =============================================================================

#[test]
fn test_pattern_assume_constraint_expression() {
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
    // Test hover on 'assumedCargoMass'
    let hover = has_hover_at(&mut host, 5, 32);
    println!("\nHover on 'assumedCargoMass': {:?}", hover);
    assert!(
        hover.is_some(),
        "Expected hover on constraint expression 'assumedCargoMass'"
    );
}

// =============================================================================
// PATTERN 10: "accept (state machine)" - accept when expression chain (3 occurrences)
// Line 94: accept when senseTemperature.temp > Tmax
// Target: 'temp' at col 57-61
// =============================================================================

#[test]
fn test_pattern_accept_when_chain() {
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
    // Test hover on 'senseTemperature'
    let hover1 = has_hover_at(&mut host, 15, 27);
    println!("\nHover on 'senseTemperature': {:?}", hover1);

    // Test hover on 'temp' (chain member)
    let hover2 = has_hover_at(&mut host, 15, 44);
    println!("\nHover on 'temp': {:?}", hover2);
    assert!(hover2.is_some(), "Expected hover on chain member 'temp'");
}
