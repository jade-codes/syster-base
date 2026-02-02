//! Tests for CHAIN_MEMBER hover resolution
//!
//! These tests document expected hover behavior for member access chains
//! like `vehicle.engine.power` where we need to follow the type hierarchy
//! to resolve members.
//!
//! Many of these tests are expected to FAIL initially - they document
//! the desired behavior that needs to be implemented.

use std::path::PathBuf;
use syster::hir::TypeRefKind;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

// ============================================================================
// Issue 1: Bind RHS Chain Resolution
// ============================================================================
// Pattern: bind lhs.chain = rhs.chain.member
// The member after the first part of the RHS chain fails to resolve

mod bind_rhs_chain {
    use super::*;

    /// Test: bind with simple RHS chain - member should resolve via type
    /// ```sysml
    /// bind rearAxle.wheelPort = vehiclePort.wheelPort1;
    ///                          ^^^^^^^^^^^  ^^^^^^^^^^
    ///                          First ✅     Member should resolve via type
    /// ```
    #[test]
    fn test_bind_rhs_chain_member_resolves() {
        let source = r#"
package Test {
    port def WheelPort;
    
    port def VehiclePort {
        port wheelPort1 : WheelPort;
        port wheelPort2 : WheelPort;
    }
    
    part def RearAxle {
        port wheelPort : WheelPort;
    }
    
    part def Vehicle {
        part rearAxle : RearAxle;
        port vehiclePort : VehiclePort;
        
        bind rearAxle.wheelPort = vehiclePort.wheelPort1;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Find the bind symbol
        let bind_sym = analysis
            .symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .find(|s| s.name.contains("bind"))
            .expect("bind symbol should exist");

        // Find the wheelPort1 type_ref (RHS chain member)
        let wheel_port1_ref = bind_sym
            .type_refs
            .iter()
            .flat_map(|tr: &TypeRefKind| tr.as_refs())
            .find(|r| r.target.as_ref() == "wheelPort1");

        assert!(
            wheel_port1_ref.is_some(),
            "bind should have wheelPort1 as type_ref"
        );

        let ref_ = wheel_port1_ref.unwrap();
        let hover = analysis.hover(file_id, ref_.start_line, ref_.start_col + 1);

        assert!(
            hover.is_some(),
            "hover on 'wheelPort1' in bind RHS should resolve"
        );

        let qn = hover.unwrap().qualified_name;
        assert!(
            qn.as_ref()
                .map(|s| s.contains("wheelPort1"))
                .unwrap_or(false),
            "hover should resolve to VehiclePort::wheelPort1, got {:?}",
            qn
        );
    }
}

// ============================================================================
// Issue 2: Flow Endpoint Chains
// ============================================================================
// Pattern: flow of Type from source.member to target.member
// The chain members in from/to clauses fail to resolve

mod flow_endpoints {
    use super::*;

    /// Test: flow endpoint chain members should resolve
    /// ```sysml
    /// flow of Signal from sender.outPort to receiver.inPort;
    ///                     ^^^^^^  ^^^^^^^    ^^^^^^^^  ^^^^^^
    ///                     Part    Member     Part      Member
    /// ```
    #[test]
    fn test_flow_from_chain_member_resolves() {
        let source = r#"
package Test {
    item def Signal;
    
    port def OutPort {
        out item signal : Signal;
    }
    
    port def InPort {
        in item signal : Signal;
    }
    
    part def Sender {
        port outPort : OutPort;
    }
    
    part def Receiver {
        port inPort : InPort;
    }
    
    part def System {
        part sender : Sender;
        part receiver : Receiver;
        
        flow of Signal from sender.outPort to receiver.inPort;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Find the flow symbol
        let flow_sym = analysis
            .symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .find(|s| s.qualified_name.contains("flow") || s.qualified_name.contains("Flow"));

        // The flow should have type_refs for the endpoint chains
        if let Some(sym) = &flow_sym {
            let has_outport = sym
                .type_refs
                .iter()
                .flat_map(|tr: &TypeRefKind| tr.as_refs())
                .any(|r| r.target.as_ref() == "outPort");

            assert!(
                has_outport,
                "flow should extract 'outPort' from 'sender.outPort' as type_ref"
            );
        }

        // Test hover on the flow line for 'outPort'
        // Line: flow of Signal from sender.outPort to receiver.inPort;
        // This is line 24 (0-indexed, accounting for empty line 0)
        let line = 24;

        // Find column for 'outPort' - after "sender."
        // We'll search for hover success in the expected range
        let mut found_outport_hover = false;
        for col in 30..50 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("outPort"))
                    .unwrap_or(false)
                {
                    found_outport_hover = true;
                    break;
                }
            }
        }

        assert!(
            found_outport_hover,
            "hover on 'outPort' in flow endpoint should resolve to Sender::outPort"
        );
    }

    /// Test: flow endpoint chains extracted as type_refs
    #[test]
    fn test_flow_endpoints_extracted_as_type_refs() {
        let source = r#"
package Test {
    item def Data;
    
    part def Producer {
        out item output : Data;
    }
    
    part def Consumer {
        in item input : Data;
    }
    
    part def Pipeline {
        part producer : Producer;
        part consumer : Consumer;
        
        flow of Data from producer.output to consumer.input;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Find any flow-related symbol
        let symbols: Vec<_> = analysis
            .symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .filter(|s| s.qualified_name.contains("Pipeline"))
            .collect();

        // Check if flow endpoints are captured somewhere
        let has_flow_refs = symbols.iter().any(|s| {
            s.type_refs
                .iter()
                .flat_map(|tr: &TypeRefKind| tr.as_refs())
                .any(|r| r.target.as_ref() == "output" || r.target.as_ref() == "input")
        });

        assert!(
            has_flow_refs,
            "flow endpoints 'producer.output' and 'consumer.input' should be extracted as type_refs"
        );
    }
}

// ============================================================================
// Issue 3: Message/First-Then Endpoints
// ============================================================================
// Pattern: message from source.port to target.port
// Pattern: first state.event then other.event

mod message_succession_endpoints {
    use super::*;

    /// Test: message endpoint chain members should resolve
    /// ```sysml
    /// message of Cmd from driver.sendCmd to vehicle.receiveCmd;
    /// ```
    #[test]
    fn test_message_from_chain_member_resolves() {
        let source = r#"
package Test {
    item def Command;
    
    part def Driver {
        event occurrence sendCmd;
    }
    
    part def Vehicle {
        event occurrence receiveCmd;
    }
    
    part def Interaction {
        part driver : Driver;
        part vehicle : Vehicle;
        
        message of Command from driver.sendCmd to vehicle.receiveCmd;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // The message endpoints should be extractable
        // Line: message of Command from driver.sendCmd to vehicle.receiveCmd;
        let line = 16;

        // Test hover on 'sendCmd' - should resolve to Driver::sendCmd
        let mut found_sendcmd = false;
        for col in 35..55 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("sendCmd"))
                    .unwrap_or(false)
                {
                    found_sendcmd = true;
                    break;
                }
            }
        }

        assert!(
            found_sendcmd,
            "hover on 'sendCmd' in message endpoint should resolve to Driver::sendCmd"
        );
    }

    /// Test: succession (first/then) chain members should resolve
    /// ```sysml
    /// first vehicle.started then driver.acknowledged;
    /// ```
    #[test]
    fn test_succession_chain_member_resolves() {
        let source = r#"
package Test {
    part def Vehicle {
        event occurrence started;
        event occurrence stopped;
    }
    
    part def Driver {
        event occurrence acknowledged;
    }
    
    action def StartSequence {
        part vehicle : Vehicle;
        part driver : Driver;
        
        first vehicle.started then driver.acknowledged;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: first vehicle.started then driver.acknowledged;
        let line = 15;

        // Test hover on 'started' - should resolve to Vehicle::started
        let mut found_started = false;
        for col in 14..30 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("started"))
                    .unwrap_or(false)
                {
                    found_started = true;
                    break;
                }
            }
        }

        assert!(
            found_started,
            "hover on 'started' in first/then should resolve to Vehicle::started"
        );

        // Test hover on 'acknowledged' - should resolve to Driver::acknowledged
        let mut found_ack = false;
        for col in 35..55 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("acknowledged"))
                    .unwrap_or(false)
                {
                    found_ack = true;
                    break;
                }
            }
        }

        assert!(
            found_ack,
            "hover on 'acknowledged' in first/then should resolve to Driver::acknowledged"
        );
    }
}

// ============================================================================
// Issue 4: Connect with Redefinition Chains
// ============================================================================
// Pattern: connect port ::> compositePort.subPort to other ::> otherComposite.subPort
// Members after ::> redefinition chains fail to resolve

mod connect_redefines_chain {
    use super::*;

    /// Test: connect with ::> chain member should resolve
    /// ```sysml
    /// connect lugNutPort ::> compositePort.subPort1 to shankPort ::> otherComposite.subPort1;
    /// ```
    #[test]
    fn test_connect_redefines_chain_member_resolves() {
        let source = r#"
package Test {
    port def SubPort;
    
    port def CompositePort {
        port subPort1 : SubPort;
        port subPort2 : SubPort;
    }
    
    part def Assembly {
        port compositeA : CompositePort;
        port compositeB : CompositePort;
        
        connect portA ::> compositeA.subPort1 to portB ::> compositeB.subPort1;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: connect portA ::> compositeA.subPort1 to portB ::> compositeB.subPort1;
        let line = 13;

        // Test hover on first 'subPort1' after compositeA
        let mut found_subport1 = false;
        for col in 30..45 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("subPort1"))
                    .unwrap_or(false)
                {
                    found_subport1 = true;
                    break;
                }
            }
        }

        assert!(
            found_subport1,
            "hover on 'subPort1' after ::> chain should resolve to CompositePort::subPort1"
        );
    }
}

// ============================================================================
// Issue 5: Deep Assignment Chains
// ============================================================================
// Pattern: x = a.b.c.d where depth > 2 fails
// Three-level chains where the third member fails

mod deep_chain_resolution {
    use super::*;

    /// Test: three-level chain should resolve all members
    /// ```sysml
    /// attribute x = vehicle.software.controller.value;
    ///               ^^^^^^^  ^^^^^^^^  ^^^^^^^^^^  ^^^^^
    ///               Level 1  Level 2   Level 3     Level 4 - all should resolve
    /// ```
    #[test]
    fn test_three_level_chain_resolves() {
        let source = r#"
package Test {
    part def Controller {
        attribute value : Real;
    }
    
    part def Software {
        part controller : Controller;
    }
    
    part def Vehicle {
        part software : Software;
    }
    
    part def System {
        part vehicle : Vehicle;
        
        attribute x = vehicle.software.controller.value;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: attribute x = vehicle.software.controller.value;
        let line = 17;

        // Test each part of the chain resolves
        let expected = [
            ("vehicle", "System::vehicle"),
            ("software", "Vehicle::software"),
            ("controller", "Software::controller"),
            ("value", "Controller::value"),
        ];

        for (name, expected_contains) in expected {
            let mut found = false;
            for col in 0..70 {
                if let Some(hover) = analysis.hover(file_id, line, col) {
                    if let Some(qn) = &hover.qualified_name {
                        if qn.contains(expected_contains) {
                            found = true;
                            break;
                        }
                    }
                }
            }

            assert!(
                found,
                "hover on '{}' should resolve to something containing '{}', but it didn't",
                name, expected_contains
            );
        }
    }

    /// Test: event occurrence with deep chain
    /// ```sysml
    /// event occurrence x = part.port.event;
    /// ```
    #[test]
    fn test_event_occurrence_deep_chain() {
        let source = r#"
package Test {
    port def EventPort {
        event occurrence trigger;
    }
    
    part def Component {
        port eventPort : EventPort;
    }
    
    part def System {
        part component : Component;
        
        event occurrence myTrigger = component.eventPort.trigger;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: event occurrence myTrigger = component.eventPort.trigger;
        let line = 13;

        // Test hover on 'trigger' - the deepest member
        let mut found_trigger = false;
        for col in 50..70 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("trigger"))
                    .unwrap_or(false)
                {
                    found_trigger = true;
                    break;
                }
            }
        }

        assert!(
            found_trigger,
            "hover on 'trigger' in deep chain should resolve to EventPort::trigger"
        );
    }
}

// ============================================================================
// Issue 6: Type Resolution Through Supertypes
// ============================================================================
// The core issue: when resolving `port.member`, we need to follow the port's
// type to find the member definition

mod type_hierarchy_resolution {
    use super::*;

    /// Test: member lookup should follow type hierarchy
    /// When we have `myPort : PortDef` and look for `myPort.member`,
    /// we should find `PortDef::member`
    #[test]
    fn test_member_resolves_through_type() {
        let source = r#"
package Test {
    port def DataPort {
        attribute dataValue : Real;
        in item dataIn;
        out item dataOut;
    }
    
    part def Component {
        port myPort : DataPort;
    }
    
    part def System {
        part comp : Component;
        
        // Reference to member through type
        attribute x = comp.myPort.dataValue;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: attribute x = comp.myPort.dataValue;
        let line = 16;

        // Test hover on 'dataValue' - should resolve via DataPort
        let mut found = false;
        for col in 30..50 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("dataValue"))
                    .unwrap_or(false)
                {
                    found = true;
                    break;
                }
            }
        }

        assert!(
            found,
            "hover on 'dataValue' should resolve to DataPort::dataValue through type hierarchy"
        );
    }

    /// Test: nested type resolution (type of type's member)
    #[test]
    fn test_nested_type_resolution() {
        let source = r#"
package Test {
    port def InnerPort {
        attribute innerValue : Real;
    }
    
    port def OuterPort {
        port inner : InnerPort;
    }
    
    part def Device {
        port outer : OuterPort;
    }
    
    part def System {
        part device : Device;
        
        // Two levels of type resolution needed
        attribute x = device.outer.inner.innerValue;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: attribute x = device.outer.inner.innerValue;
        let line = 18;

        // Each level should resolve through types:
        // device -> Device
        // outer -> Device::outer -> OuterPort
        // inner -> OuterPort::inner -> InnerPort
        // innerValue -> InnerPort::innerValue

        let mut found_inner_value = false;
        for col in 40..60 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("innerValue"))
                    .unwrap_or(false)
                {
                    found_inner_value = true;
                    break;
                }
            }
        }

        assert!(
            found_inner_value,
            "hover on 'innerValue' should resolve through nested type hierarchy"
        );
    }
}

// ============================================================================
// Issue 5: Three-Level Bind Chains (From Triage)
// ============================================================================
// Pattern: bind part.subpart.port = otherPart.subpart.port
// Third-level chain members fail to resolve

mod three_level_bind_chains {
    use super::*;

    /// Test: bind with 3-level chain on both sides
    /// ```sysml
    /// bind rearAxleAssembly.rearWheel1.wheelToRoadPort = vehicleToRoadPort.wheelToRoadPort1;
    ///                      ^^^^^^^^^^  ^^^^^^^^^^^^^^^
    ///                      Part        Port (FAILS - depth 3)
    /// ```
    #[test]
    fn test_bind_three_level_chain_resolves() {
        let source = r#"
package Test {
    port def WheelToRoadPort;
    
    port def VehicleToRoadPort {
        port wheelToRoadPort1 : WheelToRoadPort;
        port wheelToRoadPort2 : WheelToRoadPort;
    }
    
    part def Wheel {
        port wheelToRoadPort : WheelToRoadPort;
    }
    
    part def RearAxleAssembly {
        part rearWheel1 : Wheel;
        part rearWheel2 : Wheel;
    }
    
    part def Vehicle {
        part rearAxleAssembly : RearAxleAssembly;
        port vehicleToRoadPort : VehicleToRoadPort;
        
        bind rearAxleAssembly.rearWheel1.wheelToRoadPort = vehicleToRoadPort.wheelToRoadPort1;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: bind rearAxleAssembly.rearWheel1.wheelToRoadPort = ...
        // The bind is at line 22 (0-indexed, line 0 is empty)
        let line = 22;

        // Test hover on 'wheelToRoadPort' (3rd level) - should resolve to Wheel::wheelToRoadPort
        let mut found_wheel_port = false;
        for col in 40..60 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("wheelToRoadPort") && s.contains("Wheel"))
                    .unwrap_or(false)
                {
                    found_wheel_port = true;
                    break;
                }
            }
        }

        assert!(
            found_wheel_port,
            "hover on 'wheelToRoadPort' (depth 3) should resolve to Wheel::wheelToRoadPort"
        );
    }
}

// ============================================================================
// Issue 6: Connect Endpoint Chains (From Triage)
// ============================================================================
// Pattern: connect portA ::> part.subport to portB ::> otherPart.subport
// Chain members after ::> fail to resolve

mod connect_endpoint_chains {
    use super::*;

    /// Test: connect with chain after redefinition
    /// ```sysml
    /// connect lugNutPort ::> wheel.lugNutPort to shankPort ::> lugNut.shankPort;
    ///                        ^^^^^  ^^^^^^^^^^    ^^^^^^  ^^^^^^^^^
    ///                        Part   Port (FAILS)  Part    Port (FAILS)
    /// ```
    #[test]
    fn test_connect_endpoint_chain_resolves() {
        let source = r#"
package Test {
    port def LugNutPort;
    port def ShankPort;
    
    part def Wheel {
        port lugNutPort : LugNutPort;
    }
    
    part def LugNut {
        port shankPort : ShankPort;
    }
    
    part def WheelAssembly {
        part wheel : Wheel;
        part lugNut : LugNut;
        
        connect lugNutConnection ::> wheel.lugNutPort to shankConnection ::> lugNut.shankPort;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: connect lugNutConnection ::> wheel.lugNutPort to shankConnection ::> lugNut.shankPort;
        // Connect is at line 17 (0-indexed, line 0 is empty)
        let line = 17;

        // Test hover on 'lugNutPort' after ::> - should resolve to Wheel::lugNutPort
        let mut found_lug_port = false;
        for col in 35..55 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("lugNutPort") && s.contains("Wheel"))
                    .unwrap_or(false)
                {
                    found_lug_port = true;
                    break;
                }
            }
        }

        assert!(
            found_lug_port,
            "hover on 'lugNutPort' in connect endpoint should resolve to Wheel::lugNutPort"
        );

        // Test hover on 'shankPort' after ::> - should resolve to LugNut::shankPort
        let mut found_shank_port = false;
        for col in 75..95 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("shankPort") && s.contains("LugNut"))
                    .unwrap_or(false)
                {
                    found_shank_port = true;
                    break;
                }
            }
        }

        assert!(
            found_shank_port,
            "hover on 'shankPort' in connect endpoint should resolve to LugNut::shankPort"
        );
    }
}

// ============================================================================
// Issue 7: Message Endpoint Chains (From Triage)
// ============================================================================
// Pattern: message of Type from source.port to target.port
// Second chain members fail to resolve

mod message_endpoint_chains {
    use super::*;

    /// Test: message endpoint chain members
    /// ```sysml
    /// message of TurnVehicleOn from turnVehicleOn.start to trigger.receive;
    ///                              ^^^^^^^^^^^^^^^  ^^^^^    ^^^^^^^  ^^^^^^^
    ///                              Part            Port     Part     Port
    /// ```
    #[test]
    fn test_message_endpoint_chain_resolves() {
        let source = r#"
package Test {
    item def TurnVehicleOn;
    
    part def Starter {
        event occurrence start;
    }
    
    part def Trigger {
        event occurrence receive;
    }
    
    part def Interaction {
        part starter : Starter;
        part trigger : Trigger;
        
        message of TurnVehicleOn from starter.start to trigger.receive;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: message of TurnVehicleOn from starter.start to trigger.receive;
        // Message is at line 16 (0-indexed, line 0 is empty)
        let line = 16;

        // Test hover on 'start' - should resolve to Starter::start
        let mut found_start = false;
        for col in 45..60 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("start") && s.contains("Starter"))
                    .unwrap_or(false)
                {
                    found_start = true;
                    break;
                }
            }
        }

        assert!(
            found_start,
            "hover on 'start' in message endpoint should resolve to Starter::start"
        );

        // Test hover on 'receive' - should resolve to Trigger::receive
        let mut found_receive = false;
        for col in 65..85 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("receive") && s.contains("Trigger"))
                    .unwrap_or(false)
                {
                    found_receive = true;
                    break;
                }
            }
        }

        assert!(
            found_receive,
            "hover on 'receive' in message endpoint should resolve to Trigger::receive"
        );
    }
}

// ============================================================================
// Issue 8: Expression Assignment Chains (From Triage)
// ============================================================================
// Pattern: attribute x = part.subpart.value;
// Third-level chain in expression RHS fails

mod expression_assignment_chains {
    use super::*;

    /// Test: 3-level chain in expression assignment
    /// ```sysml
    /// event occurrence setSpeedReceived = vehicle.setSpeedPort.setSpeedReceived;
    ///                                     ^^^^^^^  ^^^^^^^^^^^^  ^^^^^^^^^^^^^^^
    ///                                     Part     Port          Event (FAILS depth 3)
    /// ```
    #[test]
    fn test_expression_three_level_chain_resolves() {
        let source = r#"
package Test {
    part def SetSpeedPort {
        event occurrence setSpeedReceived;
    }
    
    part def Vehicle {
        port setSpeedPort : SetSpeedPort;
    }
    
    part def Controller {
        part vehicle : Vehicle;
        
        event occurrence localSetSpeed = vehicle.setSpeedPort.setSpeedReceived;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: event occurrence localSetSpeed = vehicle.setSpeedPort.setSpeedReceived;
        // Event occurrence is at line 13 (0-indexed, line 0 is empty)
        let line = 13;

        // Test hover on 'setSpeedReceived' (depth 3) - should resolve to SetSpeedPort::setSpeedReceived
        let mut found_set_speed = false;
        for col in 55..75 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("setSpeedReceived") && s.contains("SetSpeedPort"))
                    .unwrap_or(false)
                {
                    found_set_speed = true;
                    break;
                }
            }
        }

        assert!(
            found_set_speed,
            "hover on 'setSpeedReceived' (depth 3) should resolve to SetSpeedPort::setSpeedReceived"
        );
    }

    /// Test: attribute chain in flow from clause
    /// ```sysml
    /// from speedSensor.speedSensorPort.sensedSpeedSent to ...
    /// ```
    #[test]
    fn test_flow_three_level_chain_resolves() {
        let source = r#"
package Test {
    item def Speed;
    
    port def SpeedSensorPort {
        out item sensedSpeed : Speed;
        event occurrence sensedSpeedSent;
    }
    
    part def SpeedSensor {
        port speedSensorPort : SpeedSensorPort;
    }
    
    part def VehicleSoftware {
        in item speedInput : Speed;
    }
    
    part def System {
        part speedSensor : SpeedSensor;
        part vehicleSoftware : VehicleSoftware;
        
        flow of Speed from speedSensor.speedSensorPort.sensedSpeed to vehicleSoftware.speedInput;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();

        // Line: flow of Speed from speedSensor.speedSensorPort.sensedSpeed to ...
        // Flow is at line 21 (0-indexed, line 0 is empty)
        let line = 21;

        // Test hover on 'sensedSpeed' (depth 3) - should resolve to SpeedSensorPort::sensedSpeed
        let mut found_sensed_speed = false;
        for col in 50..75 {
            if let Some(hover) = analysis.hover(file_id, line, col) {
                if hover
                    .qualified_name
                    .as_ref()
                    .map(|s| s.contains("sensedSpeed") && s.contains("SpeedSensorPort"))
                    .unwrap_or(false)
                {
                    found_sensed_speed = true;
                    break;
                }
            }
        }

        assert!(
            found_sensed_speed,
            "hover on 'sensedSpeed' (depth 3) in flow should resolve to SpeedSensorPort::sensedSpeed"
        );
    }
}
