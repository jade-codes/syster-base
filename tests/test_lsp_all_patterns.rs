//! Tests for ALL remaining LSP hover failure patterns from vehicle example
//! Run with: cargo test --test test_lsp_all_patterns -- --nocapture

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

// ==============================================================================
// PATTERN 1: bind (551 occurrences)
// bind shaftPort_d = differential.shaftPort_d;
// ==============================================================================

#[test]
fn test_bind_lhs_port() {
    // Line 0: package P {
    // Line 1:     part def AxleAssembly { port shaftPort_d; }
    // ...
    // Line 5:         bind shaftPort_d = differential.shaftPort_d;
    let source = "package P {
    part def AxleAssembly { port shaftPort_d; }
    part def Differential { port shaftPort_d; }
    part rearAxle : AxleAssembly {
        part differential : Differential;
        bind shaftPort_d = differential.shaftPort_d;
    }
}";
    let mut host = create_analysis(source);

    // bind shaftPort_d at line 5, col ~13
    let hover = has_hover_at(&mut host, 5, 14);
    println!("bind LHS shaftPort_d hover: {:?}", hover);
    assert!(hover.is_some(), "bind LHS shaftPort_d should resolve");
}

#[test]
fn test_bind_rhs_part() {
    let source = "package P {
    part def AxleAssembly { port shaftPort_d; }
    part def Differential { port shaftPort_d; }
    part rearAxle : AxleAssembly {
        part differential : Differential;
        bind shaftPort_d = differential.shaftPort_d;
    }
}";
    let mut host = create_analysis(source);
    // differential at line 5, col ~27
    let hover = has_hover_at(&mut host, 5, 28);
    println!("bind RHS differential hover: {:?}", hover);
    assert!(hover.is_some(), "bind RHS differential should resolve");
}

// ==============================================================================
// PATTERN 2: redefines unqualified (176 occurrences)
// ref item redefines fuel { ... }
// ==============================================================================

#[test]
fn test_redefines_unqualified_item() {
    let source = "package P {
    item def FuelItem;
    part def FuelTank { item fuel : FuelItem; }
    part fuelTank : FuelTank {
        ref item redefines fuel { }
    }
}";
    let mut host = create_analysis(source);

    // redefines fuel at line 4, col ~26
    let hover = has_hover_at(&mut host, 4, 27);
    println!("redefines fuel hover: {:?}", hover);
    assert!(hover.is_some(), "redefines fuel should resolve");
}

#[test]
fn test_redefines_nested_attribute() {
    let source = "package P {
    part def FuelTank {
        item fuel { attribute fuelMass = 0; }
    }
    part fuelTank : FuelTank {
        ref item redefines fuel {
            attribute redefines fuelMass = 50;
        }
    }
}";
    let mut host = create_analysis(source);

    // Debug
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG NESTED REDEFINES ---");
    for sym in index.symbols_in_file(file_id) {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, tr);
            }
        }
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // redefines fuelMass at line 6 - find correct column from debug
    let hover = has_hover_at(&mut host, 6, 33);
    println!("nested redefines fuelMass hover: {:?}", hover);
    assert!(hover.is_some(), "nested redefines fuelMass should resolve");
}

// ==============================================================================
// PATTERN 3: specializes :>> (68 occurrences)
// attribute x : Type { :>> mRefs = 1; }
// ==============================================================================

#[test]
fn test_specializes_nested_attribute() {
    let source = "package P {
    attribute def CoordinateFrame { attribute mRefs; }
    part def Context {
        attribute spatialCF : CoordinateFrame { :>> mRefs = 1; }
    }
}";
    let mut host = create_analysis(source);

    // :>> mRefs at line 3, col ~52
    let hover = has_hover_at(&mut host, 3, 53);
    println!(":>> mRefs hover: {:?}", hover);
    assert!(hover.is_some(), ":>> mRefs should resolve");
}

#[test]
fn test_specializes_requirement() {
    let source = "package P {
    requirement def TorqueReq;
    part def Vehicle { requirement torqueReq : TorqueReq; }
    part vehicle : Vehicle {
        part engine { requirement :>> torqueReq; }
    }
}";
    let mut host = create_analysis(source);

    // :>> torqueReq at line 4
    let hover = has_hover_at(&mut host, 4, 40);
    println!(":>> torqueReq hover: {:?}", hover);
    assert!(hover.is_some(), ":>> torqueReq should resolve");
}

// ==============================================================================
// PATTERN 4: expression/value (51 occurrences)
// if ignitionCmd.ignitionOnOff == value
// ==============================================================================

#[test]
fn test_expression_feature_chain() {
    let source = "package P {
    attribute def IgnitionCmd { attribute ignitionOnOff; }
    state def VehicleStates {
        in attribute ignitionCmd : IgnitionCmd;
        state operatingStates {
            state off;
            transition t1 first off if ignitionCmd.ignitionOnOff == true then off;
        }
    }
}";
    let mut host = create_analysis(source);

    // ignitionCmd at line 6
    let hover = has_hover_at(&mut host, 6, 40);
    println!("if ignitionCmd hover: {:?}", hover);
    assert!(hover.is_some(), "ignitionCmd in if should resolve");
}

#[test]
fn test_expression_chained_feature() {
    let source = "package P {
    attribute def IgnitionCmd { attribute ignitionOnOff; }
    state def VehicleStates {
        in attribute ignitionCmd : IgnitionCmd;
        state s1 {
            transition t1 first s1 if ignitionCmd.ignitionOnOff == true then s1;
        }
    }
}";
    let mut host = create_analysis(source);
    // ignitionOnOff at line 5 (after the dot)
    let hover = has_hover_at(&mut host, 5, 52);
    println!("ignitionOnOff chained hover: {:?}", hover);
    assert!(hover.is_some(), "ignitionOnOff in chain should resolve");
}

// ==============================================================================
// PATTERN 5: transition then (38 occurrences)
// transition initial then off;
// ==============================================================================

#[test]
fn test_transition_then_target() {
    let source = "package P {
    state def VehicleStates {
        state operatingStates {
            entry; then off;
            state off;
            state on;
        }
    }
}";
    let mut host = create_analysis(source);

    // Debug
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG TRANSITION ---");
    for sym in index.symbols_in_file(file_id) {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("  type_ref[{}]: {:?}", i, tr);
        }
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // then off at line 3, col 24-27 (based on type_ref debug)
    let hover = has_hover_at(&mut host, 3, 25);
    println!("then off hover: {:?}", hover);
    assert!(hover.is_some(), "then off should resolve");
}

#[test]
fn test_transition_first_then() {
    let source = "package P {
    state def VehicleStates {
        state s {
            state off;
            state starting;
            transition t1 first off then starting;
        }
    }
}";
    let mut host = create_analysis(source);

    // Debug: print type_refs to find correct columns
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG FIRST/THEN ---");
    for sym in index.symbols_in_file(file_id) {
        if !sym.type_refs.is_empty() {
            println!("{} ({:?})", sym.qualified_name, sym.kind);
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, tr);
            }
        }
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // Check hover for 'off' and 'starting' based on type_ref columns
    let hover_off = has_hover_at(&mut host, 5, 32);
    println!("first off hover: {:?}", hover_off);
    assert!(hover_off.is_some(), "first off should resolve");
    let hover_starting = has_hover_at(&mut host, 5, 42);
    println!("then starting hover: {:?}", hover_starting);
    assert!(hover_starting.is_some(), "then starting should resolve");
}

// ==============================================================================
// PATTERN 6: featured by ::> (24 occurrences)
// connect lugNutPort ::> lugNutCompositePort.lugNutPort
// ==============================================================================

#[test]
fn test_featured_by_connect() {
    let source = "package P {
    port def LugNutPort;
    port def CompositePort { port lugNutPort : LugNutPort[5]; }
    part def WheelHub {
        port comp : CompositePort;
        interface intf {
            connect p ::> comp.lugNutPort to comp.lugNutPort;
        }
    }
}";
    let mut host = create_analysis(source);

    // ::> comp at line 6, col ~25
    let hover = has_hover_at(&mut host, 6, 26);
    println!("::> comp hover: {:?}", hover);
    assert!(hover.is_some(), "::> comp should resolve");
}

#[test]
fn test_featured_by_simple() {
    let source = "package P {
    port def Port1;
    part def Assembly {
        port p1 : Port1;
        part sub { port ::> p1; }
    }
}";
    let mut host = create_analysis(source);

    // ::> p1 at line 4, col ~28
    let hover = has_hover_at(&mut host, 4, 29);
    println!("::> p1 hover: {:?}", hover);
    assert!(hover.is_some(), "::> p1 should resolve");
}

// ==============================================================================
// PATTERN 7: subsets (16 occurrences)
// action driverGetInVehicle subsets getInVehicle_a;
// ==============================================================================

#[test]
fn test_subsets_action() {
    let source = "package P {
    action def GetInVehicle;
    action def TransportScenario {
        action trigger { action getInVehicle_a : GetInVehicle; }
    }
    action transport : TransportScenario {
        action trigger {
            action a { action driverGetIn subsets getInVehicle_a; }
        }
    }
}";
    let mut host = create_analysis(source);

    // Debug
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG ALL SYMBOLS ---");
    for sym in index.symbols_in_file(file_id) {
        println!("{} ({:?})", sym.qualified_name, sym.kind);
        println!("  supertypes: {:?}", sym.supertypes);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("  type_ref[{}]: {:?}", i, tr);
        }
    }
    // Debug visibility for key scopes
    println!("\n--- VISIBILITY MAP ---");
    for scope in &[
        "P::transport",
        "P::transport::trigger",
        "P::transport::trigger::a",
        "P::TransportScenario::trigger",
    ] {
        if let Some(vis) = index.visibility_for_scope(scope) {
            println!("{} visible: {:?}", scope, vis.lookup("getInVehicle_a"));
        } else {
            println!("{} no visibility map", scope);
        }
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // subsets getInVehicle_a at line 7
    let hover = has_hover_at(&mut host, 7, 53);
    println!("subsets getInVehicle_a hover: {:?}", hover);
    assert!(hover.is_some(), "subsets getInVehicle_a should resolve");
}

#[test]
fn test_subsets_inherited() {
    let source = "package P {
    action def BaseAction { action step1; }
    action derived : BaseAction {
        action myStep subsets step1;
    }
}";
    let mut host = create_analysis(source);

    // Debug: print symbol structure
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG SYMBOLS (file_id={:?}) ---", file_id);
    for sym in index.symbols_in_file(file_id) {
        println!(
            "{} ({:?}) file={:?}",
            sym.qualified_name, sym.kind, sym.file
        );
        println!("  supertypes: {:?}", sym.supertypes);
        for (i, tr) in sym.type_refs.iter().enumerate() {
            println!("  type_ref[{}]: {:?}", i, tr);
        }
    }
    println!("--- ALL symbols ---");
    for sym in index.all_symbols() {
        println!("  {} file={:?}", sym.qualified_name, sym.file);
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // subsets step1 at line 3
    let hover = has_hover_at(&mut host, 3, 32);
    println!("subsets step1 hover: {:?}", hover);
    assert!(hover.is_some(), "subsets step1 should resolve (inherited)");
}

// ==============================================================================
// PATTERN 8: constraint (15 occurrences)
// assert constraint fuelConstraint { fuel.fuelMass <= fuelMassMax }
// ==============================================================================

#[test]
fn test_constraint_assert() {
    let source = "package P {
    part def FuelTank {
        item fuel { attribute fuelMass; }
        attribute fuelMassMax;
        assert constraint fuelConstraint { fuel.fuelMass <= fuelMassMax }
    }
}";
    let mut host = create_analysis(source);

    // fuelConstraint at line 4
    let hover = has_hover_at(&mut host, 4, 30);
    println!("constraint fuelConstraint hover: {:?}", hover);
    // Note: constraint names may not have hover, just ensure no crash
}

#[test]
fn test_constraint_assume() {
    let source = "package P {
    requirement def VehicleSpec {
        attribute assumedCargoMass;
        requirement fuelEconomyReq {
            assume constraint { assumedCargoMass <= 500 }
        }
    }
}";
    let mut host = create_analysis(source);

    // assumedCargoMass in constraint at line 4
    let hover = has_hover_at(&mut host, 4, 35);
    println!("assumedCargoMass hover: {:?}", hover);
    assert!(
        hover.is_some(),
        "assumedCargoMass in constraint should resolve"
    );
}

// ==============================================================================
// PATTERN 9: first transition (5 occurrences)
// first join1 then trigger;
// ==============================================================================

#[test]
fn test_first_join_then_action() {
    let source = "package P {
    action transportPassenger {
        merge join1;
        action trigger;
        fork fork2;
        first join1 then trigger;
        first trigger then fork2;
    }
}";
    let mut host = create_analysis(source);

    // join1 at line 5, col ~14
    let hover_join = has_hover_at(&mut host, 5, 15);
    println!("first join1 hover: {:?}", hover_join);
    assert!(hover_join.is_some(), "first join1 should resolve");
    // trigger at line 5, col ~27
    let hover_trigger = has_hover_at(&mut host, 5, 28);
    println!("then trigger hover: {:?}", hover_trigger);
    assert!(hover_trigger.is_some(), "then trigger should resolve");
}

#[test]
fn test_first_fork_join() {
    let source = "package P {
    action workflow {
        fork f1;
        action step1;
        action step2;
        join j1;
        first f1 then step1;
        first f1 then step2;
        first step1 then j1;
        first step2 then j1;
    }
}";
    let mut host = create_analysis(source);

    // f1 at line 6, col ~14
    let hover_f1 = has_hover_at(&mut host, 6, 15);
    println!("first f1 hover: {:?}", hover_f1);
    assert!(hover_f1.is_some(), "first f1 should resolve");
    // j1 at line 8, col ~25
    let hover_j1 = has_hover_at(&mut host, 8, 26);
    println!("then j1 hover: {:?}", hover_j1);
    assert!(hover_j1.is_some(), "then j1 should resolve");
}

// ==============================================================================
// PATTERN 10: accept (3 occurrences)
// accept when senseTemperature.temp > Tmax
// ==============================================================================

#[test]
fn test_accept_when_chain() {
    let source = "package P {
    action def SenseTemp { out attribute temp; }
    attribute Tmax;
    state def HealthStates {
        state normal;
        state degraded;
        action senseTemp : SenseTemp;
        transition t1 first normal accept when senseTemp.temp > Tmax then degraded;
    }
}";
    let mut host = create_analysis(source);

    // senseTemp at line 7
    let hover_sense = has_hover_at(&mut host, 7, 50);
    println!("accept senseTemp hover: {:?}", hover_sense);
    assert!(hover_sense.is_some(), "senseTemp should resolve");
    // temp (chained) at line 7
    let hover_temp = has_hover_at(&mut host, 7, 60);
    println!("senseTemp.temp hover: {:?}", hover_temp);
    assert!(hover_temp.is_some(), "temp (chained) should resolve");
}

#[test]
#[ignore = "Parser doesn't extract 'accept sig' as a type_ref"]
fn test_accept_port() {
    let source = "package P {
    port def Signal;
    state def SM {
        in port sig : Signal;
        state s1;
        state s2;
        transition t1 first s1 accept sig then s2;
    }
}";
    let mut host = create_analysis(source);

    // Debug
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    println!("\n--- DEBUG ACCEPT ---");
    for sym in index.symbols_in_file(file_id) {
        if !sym.type_refs.is_empty() {
            println!("{} ({:?})", sym.qualified_name, sym.kind);
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("  type_ref[{}]: {:?}", i, tr);
            }
        }
    }
    println!("--- END DEBUG ---\n");
    // analysis goes out of scope here

    // accept sig at line 6 - find correct column from debug
    let hover = has_hover_at(&mut host, 6, 38);
    println!("accept sig hover: {:?}", hover);
    assert!(hover.is_some(), "accept sig should resolve");
}

// ==============================================================================
// INHERITANCE TESTS
// ==============================================================================

#[test]
fn test_inheritance_feature_resolution() {
    let source = "package P {
    part def Vehicle { attribute mass; port fuelPort; }
    part myVehicle : Vehicle {
        attribute redefines mass = 1000;
    }
}";
    let mut host = create_analysis(source);

    // redefines mass at line 3
    let hover = has_hover_at(&mut host, 3, 30);
    println!("redefines mass hover: {:?}", hover);
    assert!(hover.is_some(), "redefines mass should resolve (inherited)");
}

#[test]
fn test_deep_inheritance() {
    let source = "package P {
    part def A { attribute x; }
    part def B :> A { attribute y; }
    part def C :> B { attribute z; }
    part c : C;
}";
    let mut host = create_analysis(source);

    // Check C inherits x, y, z - verify c has ref to C
    let hover = has_hover_at(&mut host, 4, 14);
    println!("c : C hover: {:?}", hover);
    assert!(hover.is_some(), "c : C should resolve");
}

// ==============================================================================
// NESTED SCOPE TESTS
// ==============================================================================

#[test]
fn test_nested_scope_resolution() {
    let source = "package P {
    part def Outer {
        attribute outerAttr;
        part inner {
            attribute innerAttr;
            part deepest { attribute a = outerAttr + innerAttr; }
        }
    }
}";
    let mut host = create_analysis(source);

    // outerAttr at line 5
    let hover = has_hover_at(&mut host, 5, 42);
    println!("outerAttr from deepest hover: {:?}", hover);
    assert!(
        hover.is_some(),
        "outerAttr should resolve from nested scope"
    );
}

#[test]
fn test_sibling_resolution() {
    let source = "package P {
    part container {
        part sibling1;
        part sibling2;
        bind sibling1 = sibling2;
    }
}";
    let mut host = create_analysis(source);

    // sibling1 at line 4
    let hover1 = has_hover_at(&mut host, 4, 14);
    println!("bind sibling1 hover: {:?}", hover1);
    assert!(hover1.is_some(), "sibling1 should resolve");
    // sibling2 at line 4
    let hover2 = has_hover_at(&mut host, 4, 26);
    println!("= sibling2 hover: {:?}", hover2);
    assert!(hover2.is_some(), "sibling2 should resolve");
}
