//! Focused tests for hover resolution issues.
//!
//! These tests verify that hover works correctly for various SysML patterns.

use syster::ide::AnalysisHost;

fn create_analysis(source: &str) -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let _errors = host.set_file_content("test.sysml", source);
    host
}

/// Helper to check if hover at a position returns something
fn hover_at(host: &mut AnalysisHost, line: u32, col: u32) -> Option<String> {
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    analysis.hover(file_id, line, col).map(|h| h.contents)
}

// =============================================================================
// REDEFINES PATTERN
// =============================================================================

#[test]
fn test_hover_on_redefines_target() {
    let source = r#"
package TestPkg {
    action def ProvidePower;
    
    part def Vehicle {
        perform providePower : ProvidePower;
    }
    
    part vehicle_b : Vehicle {
        perform ActionTree::providePower redefines providePower;
    }
    
    package ActionTree {
        action providePower : ProvidePower;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 9, 60);
    assert!(
        hover.is_some(),
        "Expected hover on redefines target 'providePower'"
    );
    assert!(hover.unwrap().contains("providePower"));
}

#[test]
fn test_hover_on_qualified_redefines_source() {
    let source = r#"
package TestPkg {
    action def ProvidePower;
    
    part def Vehicle {
        perform providePower : ProvidePower;
    }
    
    part vehicle_b : Vehicle {
        perform ActionTree::providePower redefines providePower;
    }
    
    package ActionTree {
        action providePower : ProvidePower;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 9, 36);
    assert!(
        hover.is_some(),
        "Expected hover on qualified ref 'ActionTree::providePower'"
    );
    assert!(hover.unwrap().contains("ActionTree::providePower"));
}

// =============================================================================
// SPECIALIZES PATTERN
// =============================================================================

#[test]
fn test_hover_on_specializes_target() {
    let source = r#"
package TestPkg {
    part def Engine;
    
    part def Vehicle {
        part engine : Engine;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 5, 22);
    assert!(hover.is_some(), "Expected hover on type 'Engine'");
    assert!(hover.unwrap().contains("Engine"));
}

// =============================================================================
// FEATURE CHAIN PATTERN
// =============================================================================

#[test]
fn test_hover_on_feature_chain_first_part() {
    let source = r#"
package TestPkg {
    part def FuelTank {
        attribute mass : Real;
    }
    
    part def Vehicle {
        part fuelTank : FuelTank;
        attribute totalMass = fuelTank.mass;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 8, 32);
    assert!(
        hover.is_some(),
        "Expected hover on feature chain first part 'fuelTank'"
    );
    assert!(hover.unwrap().contains("fuelTank"));
}

#[test]
fn test_hover_on_feature_chain_second_part() {
    let source = r#"
package TestPkg {
    part def FuelTank {
        attribute mass : Real;
    }
    
    part def Vehicle {
        part fuelTank : FuelTank;
        attribute totalMass = fuelTank.mass;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 8, 41);
    assert!(
        hover.is_some(),
        "Expected hover on feature chain second part 'mass'"
    );
    assert!(hover.unwrap().contains("mass"));
}

// =============================================================================
// SUBSETS PATTERN
// =============================================================================

#[test]
fn test_hover_on_subsets_target() {
    let source = r#"
package TestPkg {
    action def ParentAction {
        action parentAction;
    }
    
    action def ChildAction :> ParentAction {
        action doSomething subsets parentAction;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 7, 40);
    assert!(
        hover.is_some(),
        "Expected hover on subsets target 'parentAction'"
    );
    assert!(hover.unwrap().contains("parentAction"));
}

// =============================================================================
// TRANSITION PATTERN
// =============================================================================

#[test]
fn test_hover_on_transition_target() {
    let source = r#"
package TestPkg {
    state def VehicleStates {
        entry; then initial;
        state initial;
        state running;
        transition initial then running;
    }
}
"#;

    let mut host = create_analysis(source);

    // Hover on 'initial' (source)
    let hover_initial = hover_at(&mut host, 6, 21);
    assert!(
        hover_initial.is_some(),
        "Expected hover on transition source 'initial'"
    );
    assert!(hover_initial.unwrap().contains("initial"));

    // Hover on 'running' (target)
    let hover_running = hover_at(&mut host, 6, 36);
    assert!(
        hover_running.is_some(),
        "Expected hover on transition target 'running'"
    );
    assert!(hover_running.unwrap().contains("running"));
}

// =============================================================================
// EXPRESSION VALUE PATTERN
// =============================================================================

#[test]
fn test_hover_on_expression_reference() {
    let source = r#"
package TestPkg {
    part def Calculator {
        attribute y : Real = 10;
        attribute x : Real = y + 1;
    }
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 4, 30);
    assert!(
        hover.is_some(),
        "Expected hover on expression reference 'y'"
    );
    assert!(hover.unwrap().contains("y"));
}

// =============================================================================
// MESSAGE CHAIN PATTERN
// =============================================================================

#[test]
fn test_hover_on_message_chain_second_part() {
    let source = r#"
package TestPkg {
    part def Sequence;
    part def Driver {
        action turnVehicleOn;
    }
    part def Vehicle {
        action trigger1;
    }
    part def IgnitionCmd;
    
    part sequence : Sequence {
        part driver : Driver;
        part vehicle : Vehicle;
        message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    }
}
"#;

    let mut host = create_analysis(source);

    // Hover on 'driver'
    let hover_driver = hover_at(&mut host, 14, 56);
    assert!(hover_driver.is_some(), "Expected hover on 'driver'");

    // Hover on 'turnVehicleOn'
    let hover_turn = hover_at(&mut host, 14, 63);
    assert!(
        hover_turn.is_some(),
        "Expected hover on 'turnVehicleOn' - chain member"
    );
    assert!(hover_turn.unwrap().contains("turnVehicleOn"));

    // Hover on 'trigger1'
    let hover_trigger = hover_at(&mut host, 14, 87);
    assert!(
        hover_trigger.is_some(),
        "Expected hover on 'trigger1' - chain member"
    );
    assert!(hover_trigger.unwrap().contains("trigger1"));
}

// =============================================================================
// BIND CHAIN PATTERN
// =============================================================================

#[test]
fn test_hover_on_bind_chain_second_part() {
    let source = r#"
package TestPkg {
    port def ShaftPort;
    
    part def Differential {
        port shaftPort_d : ShaftPort;
    }
    
    part def Axle {
        part differential : Differential;
        port shaftPort_d : ShaftPort;
        bind shaftPort_d = differential.shaftPort_d;
    }
}
"#;

    let mut host = create_analysis(source);

    // Hover on first 'shaftPort_d'
    let hover1 = hover_at(&mut host, 11, 17);
    assert!(hover1.is_some(), "Expected hover on first 'shaftPort_d'");

    // Hover on 'differential'
    let hover2 = hover_at(&mut host, 11, 31);
    assert!(hover2.is_some(), "Expected hover on 'differential'");

    // Hover on second 'shaftPort_d' (chain member)
    let hover3 = hover_at(&mut host, 11, 44);
    assert!(
        hover3.is_some(),
        "Expected hover on 'differential.shaftPort_d' chain member"
    );
}

// =============================================================================
// ITEM REDEFINES IN PORT DEF
// =============================================================================

#[test]
fn test_hover_on_item_redefines_in_port_def() {
    let source = r#"
package TestPkg {
    port def PwrCmdPort {
        in item pwrCmd : PwrCmd;
    }
    
    port def FuelCmdPort :> PwrCmdPort {
        in item fuelCmd : FuelCmd redefines pwrCmd;
    }
    
    item def PwrCmd;
    item def FuelCmd :> PwrCmd;
}
"#;

    let mut host = create_analysis(source);
    let hover = hover_at(&mut host, 7, 48);
    assert!(
        hover.is_some(),
        "Expected hover on redefines target 'pwrCmd'"
    );
    assert!(hover.unwrap().contains("pwrCmd"));
}

// =============================================================================
// BIND EQUALS CHAIN
// =============================================================================

#[test]
fn test_hover_on_bind_equals_chain() {
    let source = r#"
package TestPkg {
    port def ShaftPort;
    
    part def Differential {
        port shaftPort_d : ShaftPort;
    }
    
    part def Axle {
        part differential : Differential;
        port shaftPort_d : ShaftPort;
        
        bind shaftPort_d = differential.shaftPort_d;
    }
}
"#;

    let mut host = create_analysis(source);

    // Hover on first 'shaftPort_d' (bind target)
    let hover1 = hover_at(&mut host, 12, 17);
    assert!(
        hover1.is_some(),
        "Expected hover on bind target 'shaftPort_d'"
    );

    // Hover on 'differential'
    let hover2 = hover_at(&mut host, 12, 32);
    assert!(hover2.is_some(), "Expected hover on 'differential'");

    // Hover on second 'shaftPort_d' (chain member)
    let hover3 = hover_at(&mut host, 12, 47);
    assert!(
        hover3.is_some(),
        "Expected hover on chain member 'shaftPort_d'"
    );
}

// =============================================================================
// TRANSITION NAMES
// =============================================================================

#[test]
fn test_hover_on_transition_names() {
    let source = r#"
package TestPkg {
    state def VehicleStates {
        entry; do; exit;
        
        state off;
        state on;
        state initial;
        
        transition initial then off;
        transition off then on;
    }
}
"#;

    let mut host = create_analysis(source);

    // Hover on 'initial'
    let hover1 = hover_at(&mut host, 9, 21);
    assert!(
        hover1.is_some(),
        "Expected hover on transition source 'initial'"
    );
    let h1 = hover1.unwrap();
    assert!(
        h1.contains("initial"),
        "Hover should mention 'initial', got: {}",
        h1
    );

    // Hover on 'off'
    let hover2 = hover_at(&mut host, 9, 33);
    assert!(
        hover2.is_some(),
        "Expected hover on transition target 'off'"
    );
    let h2 = hover2.unwrap();
    assert!(
        h2.contains("off"),
        "Hover should mention 'off', got: {}",
        h2
    );
}
