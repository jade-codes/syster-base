//! Hover triage tests - tracking remaining hover issues.
//!
//! These tests document patterns where hover doesn't work yet.

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let mut stdlib_loader = StdLibLoader::with_path(stdlib_path());
    stdlib_loader
        .ensure_loaded_into_host(&mut host)
        .expect("Failed to load stdlib");
    host
}

// =============================================================================
// EXPRESSION REFS - References inside constraints/calculations
// =============================================================================

/// Test: Expression refs inside `accept ... if` condition
/// Pattern: `accept ignitionCmd:IgnitionCmd ... if ignitionCmd.ignitionOnOff==...`
/// The `ignitionCmd` and `ignitionOnOff` inside the `if` expression should hover.
#[test]
#[ignore = "Expression refs not yet implemented"]
fn test_hover_expression_ref_in_accept_if() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    enum IgnitionOnOff { on; off; }
    struct IgnitionCmd {
        attribute ignitionOnOff : IgnitionOnOff;
    }
    
    part def Vehicle {
        port ignitionCmdPort;
        
        state def VehicleStates {
            state off;
            state on;
            
            transition off_To_on
                first off
                accept ignitionCmd : IgnitionCmd via ignitionCmdPort
                    if ignitionCmd.ignitionOnOff == IgnitionOnOff::on
                then on;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `ignitionCmd` in the if condition (line 17, around col 23)
    let hover = analysis.hover(file_id, 17, 23);
    assert!(
        hover.is_some(),
        "Should hover on 'ignitionCmd' in if condition"
    );

    // Hover on `ignitionOnOff` in the if condition (line 17, around col 35)
    let hover = analysis.hover(file_id, 17, 40);
    assert!(
        hover.is_some(),
        "Should hover on 'ignitionOnOff' in if condition"
    );
}

/// Test: Expression refs in `send ... to` target
/// Pattern: `do send new Signal() to controller`
/// The `controller` should hover and resolve to a part.
#[test]
#[ignore = "Expression refs not yet implemented"]
fn test_hover_expression_ref_in_send_to() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    struct Signal;
    
    part def Vehicle {
        part controller;
        
        state def VehicleStates {
            state off;
            state on;
            
            transition off_To_on
                first off
                do send new Signal() to controller
                then on;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `controller` in the send statement (line 14, around col 40)
    let hover = analysis.hover(file_id, 14, 40);
    assert!(
        hover.is_some(),
        "Should hover on 'controller' in send target"
    );
}

/// Test: Expression refs in `accept when` condition
/// Pattern: `accept when senseTemperature.temp > Tmax`
/// The `temp` should hover and resolve.
#[test]
#[ignore = "Expression refs not yet implemented"]
fn test_hover_expression_ref_in_accept_when() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    attribute def TemperatureValue;
    attribute Tmax : TemperatureValue;
    
    action def SenseTemperature {
        out temp : TemperatureValue;
    }
    
    part def Vehicle {
        state def HealthStates {
            do senseTemperature : SenseTemperature {
                out temp;
            }
            
            state normal;
            state degraded;
            
            transition normal_To_degraded
                first normal
                accept when senseTemperature.temp > Tmax
                then degraded;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `temp` in the when condition
    let hover = analysis.hover(file_id, 21, 50);
    assert!(
        hover.is_some(),
        "Should hover on 'temp' in accept when condition"
    );
}

// =============================================================================
// BINDING/CONNECTION ENDPOINT REFS - Deep path references
// =============================================================================

/// Test: Connection endpoint paths
/// Pattern: `connect speedSensor.speedSensorPort to vehicleSoftware.vehicleController...`
/// Each segment of the path should hover.
#[test]
#[ignore = "Connection endpoint paths not yet implemented"]
fn test_hover_connection_endpoint_path() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    port def SensorPort;
    port def ControllerPort;
    
    part def Sensor {
        port speedSensorPort : SensorPort;
    }
    
    part def Controller {
        port sensorPort : ControllerPort;
    }
    
    part def Software {
        part controller : Controller;
    }
    
    part def Vehicle {
        part speedSensor : Sensor;
        part vehicleSoftware : Software;
        
        connect speedSensor.speedSensorPort to vehicleSoftware.controller.sensorPort;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `speedSensor` (first segment)
    let hover = analysis.hover(file_id, 22, 18);
    assert!(hover.is_some(), "Should hover on 'speedSensor' in connect");

    // Hover on `speedSensorPort` (second segment)
    let hover = analysis.hover(file_id, 22, 35);
    assert!(
        hover.is_some(),
        "Should hover on 'speedSensorPort' in connect"
    );

    // Hover on `vehicleSoftware` (target first segment)
    let hover = analysis.hover(file_id, 22, 58);
    assert!(
        hover.is_some(),
        "Should hover on 'vehicleSoftware' in connect"
    );
}

/// Test: Bind endpoint paths
/// Pattern: `bind rearAxleAssembly.rearWheel1.wheelToRoadPort = vehicleToRoadPort.wheelToRoadPort1`
#[test]
#[ignore = "Bind endpoint paths not yet implemented"]
fn test_hover_bind_endpoint_path() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    port def WheelPort;
    port def RoadPort {
        port wheelPort1 : WheelPort;
        port wheelPort2 : WheelPort;
    }
    
    part def Wheel {
        port wheelToRoadPort : WheelPort;
    }
    
    part def Axle {
        part wheel1 : Wheel;
        part wheel2 : Wheel;
    }
    
    part def Vehicle {
        part axle : Axle;
        port roadPort : RoadPort;
        
        bind axle.wheel1.wheelToRoadPort = roadPort.wheelPort1;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `wheelPort1` at end of path
    let hover = analysis.hover(file_id, 22, 58);
    assert!(hover.is_some(), "Should hover on 'wheelPort1' in bind");
}

/// Test: Message from/to endpoint paths
/// Pattern: `message sendSensedSpeed from speedSensor.speedSensorPort.sensedSpeedSent to ...`
#[test]
#[ignore = "Message endpoint paths not yet implemented"]
fn test_hover_message_endpoint_path() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    struct SensedSpeed;
    
    port def SensorPort {
        event occurrence sensedSpeedSent;
    }
    
    port def ControllerPort {
        event occurrence sensedSpeedReceived;
    }
    
    part def Sensor {
        port sensorPort : SensorPort;
    }
    
    part def Controller {
        port controllerPort : ControllerPort;
    }
    
    part def Vehicle {
        part sensor : Sensor;
        part controller : Controller;
        
        message sendSpeed of SensedSpeed
            from sensor.sensorPort.sensedSpeedSent 
            to controller.controllerPort.sensedSpeedReceived;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `sensedSpeedSent` (last segment of from path)
    let hover = analysis.hover(file_id, 26, 50);
    assert!(
        hover.is_some(),
        "Should hover on 'sensedSpeedSent' in message from"
    );

    // Hover on `sensedSpeedReceived` (last segment of to path)
    let hover = analysis.hover(file_id, 27, 55);
    assert!(
        hover.is_some(),
        "Should hover on 'sensedSpeedReceived' in message to"
    );
}

// =============================================================================
// SUBSETS REFS - References in subset clauses
// =============================================================================

/// Test: Subset target reference
/// Pattern: `part cylinder1 subsets cylinders[1]`
/// The `cylinders` should hover and resolve to the parent's feature.
#[test]
fn test_hover_subset_target() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    part def Cylinder;
    
    part def Engine {
        part cylinders : Cylinder[4..8] ordered;
    }
    
    part engine4Cyl : Engine {
        part redefines cylinders[4];
        part cylinder1 subsets cylinders[1];
        part cylinder2 subsets cylinders[1];
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `cylinders` in subset clause (line 11, around col 35)
    let hover = analysis.hover(file_id, 11, 35);
    assert!(
        hover.is_some(),
        "Should hover on 'cylinders' in subset target"
    );

    let hover = hover.unwrap();
    assert!(hover.qualified_name.is_some(), "Should have qualified name");
}

/// Test: Subset target reference in action context
/// Pattern: `in item getInVehicle_a subsets getInVehicle_a`
#[test]
#[ignore = "Subset target refs not yet implemented"]
fn test_hover_subset_target_in_action() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    action def GetIn {
        in item vehicle;
    }
    
    action def DriveScenario {
        action getIn : GetIn {
            in item getInVehicle_a subsets vehicle;
        }
        
        action drive : GetIn {
            in item driverVehicle subsets getIn.getInVehicle_a;
        }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `getInVehicle_a` in subset (line 13)
    let hover = analysis.hover(file_id, 13, 55);
    assert!(
        hover.is_some(),
        "Should hover on 'getInVehicle_a' in subset"
    );
}

// =============================================================================
// SUCCESSION/TRANSITION REFS - References in then/first clauses
// =============================================================================

/// Test: Transition `first` state reference
/// Pattern: `transition ... first off`
/// The `off` should hover and resolve to the state.
#[test]
fn test_hover_transition_first_state() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    state def VehicleStates {
        state off;
        state on;
        
        transition off_To_on
            first off
            then on;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `off` in first clause (line 8, around col 18)
    let hover = analysis.hover(file_id, 8, 18);
    assert!(hover.is_some(), "Should hover on 'off' in first clause");
}

/// Test: Transition `then` state reference  
/// Pattern: `transition ... then on;`
/// The `on` should hover and resolve to the state.
#[test]
fn test_hover_transition_then_state() {
    let mut host = create_host_with_stdlib();
    // Line numbers (0-indexed):
    // 0: (empty)
    // 1: package Test {
    // 2:     state def VehicleStates {
    // 3:         state off;
    // 4:         state on;
    // 5:
    // 6:         transition off_To_on
    // 7:             first off
    // 8:             then on;  <- `on` at cols 17-18
    // 9:     }
    // 10: }
    let source = r#"
package Test {
    state def VehicleStates {
        state off;
        state on;
        
        transition off_To_on
            first off
            then on;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `on` in then clause (line 8, col 17-18)
    let hover = analysis.hover(file_id, 8, 17);
    assert!(hover.is_some(), "Should hover on 'on' in then clause");
    let qn = hover.unwrap().qualified_name;
    assert_eq!(qn.as_deref(), Some("Test::VehicleStates::on"));
}

// =============================================================================
// TRIGGER/DONE REFS - References in trigger/done accept clauses
// =============================================================================

/// Test: Trigger accept reference
/// Pattern: `accept trigger1:Trigger...` then later `trigger1`
#[test]
#[ignore = "Trigger refs not yet implemented"]
fn test_hover_trigger_ref() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    struct Trigger;
    
    action def MyAction {
        accept trigger1 : Trigger;
        
        action step1;
        action step2;
        
        first start then step1;
        then step2 after trigger1;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `trigger1` in `after trigger1` (line 12)
    let hover = analysis.hover(file_id, 12, 30);
    assert!(hover.is_some(), "Should hover on 'trigger1' reference");
}

/// Test: Done reference
/// Pattern: `then x after done;`
#[test]
#[ignore = "Done refs not yet implemented"]
fn test_hover_done_ref() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    action def MyAction {
        action step1;
        action step2;
        
        first start then step1;
        then step2 after step1.done;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `done` in `after step1.done` (line 8)
    let hover = analysis.hover(file_id, 8, 35);
    assert!(hover.is_some(), "Should hover on 'done' reference");
}

// =============================================================================
// CALCULATION/CONSTRAINT EXPRESSION REFS
// =============================================================================

/// Test: Attribute reference in calculation
/// Pattern: `attribute totalMass = engine.mass + body.mass`
#[test]
#[ignore = "Calculation expression refs not yet implemented"]
fn test_hover_calculation_expression_ref() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    attribute def MassValue;
    
    part def Engine {
        attribute mass : MassValue;
    }
    
    part def Body {
        attribute mass : MassValue;
    }
    
    part def Vehicle {
        part engine : Engine;
        part body : Body;
        
        attribute totalMass : MassValue = engine.mass + body.mass;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `engine` in calculation (line 17)
    let hover = analysis.hover(file_id, 17, 48);
    assert!(hover.is_some(), "Should hover on 'engine' in calculation");

    // Hover on `mass` after `engine.` (line 17)
    let hover = analysis.hover(file_id, 17, 55);
    assert!(hover.is_some(), "Should hover on 'mass' in calculation");
}

/// Test: Enum member reference in constraint
/// Pattern: `constraint { status == StatusKind::closed }`
#[test]
#[ignore = "Enum member refs not yet implemented"]
fn test_hover_enum_member_in_constraint() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    enum StatusKind { open; closed; }
    
    part def Container {
        attribute status : StatusKind;
        
        constraint { status == StatusKind::closed }
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `closed` (line 8)
    let hover = analysis.hover(file_id, 8, 50);
    assert!(hover.is_some(), "Should hover on 'closed' enum member");
}

// =============================================================================
// PORT NESTED REF - References to nested port features
// =============================================================================

/// Test: Nested port reference in redefines
/// Pattern: `port redefines setSpeedPort { event occurrence setSpeedReceived; }`
/// Later: reference to `setSpeedPort.setSpeedReceived`
#[test]
#[ignore = "Nested port refs not yet implemented"]
fn test_hover_nested_port_event() {
    let mut host = create_host_with_stdlib();
    let source = r#"
package Test {
    port def SpeedPort {
        event occurrence speedReceived;
    }
    
    part def Controller {
        port speedPort : SpeedPort {
            event occurrence speedReceived;
        }
    }
    
    part def Vehicle {
        part controller : Controller;
        
        // Reference the nested event
        message msg from controller.speedPort.speedReceived to controller.speedPort.speedReceived;
    }
}
"#;
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Hover on `speedReceived` (line 17)
    let hover = analysis.hover(file_id, 17, 60);
    assert!(
        hover.is_some(),
        "Should hover on 'speedReceived' nested event"
    );
}
