//! Tests for CHAIN_MEMBER hover failures
//! 
//! These are cases where `foo.bar` chain member resolution fails.
//! The pattern is: first element resolves, but the `.member` part doesn't.

use std::path::PathBuf;
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

/// Helper to test hover at a specific position
fn test_hover_resolves(source: &str, line: u32, col: u32, expected_name: &str) -> bool {
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    let hover = analysis.hover(file_id, line, col);
    hover.as_ref()
        .and_then(|h| h.qualified_name.as_ref())
        .map(|qn| qn.contains(expected_name))
        .unwrap_or(false)
}

/// Test flow chain member: `flow from trigger1.ignitionCmd to startEngine.ignitionCmd`
/// Line 759: trigger1.ignitionCmd - the .ignitionCmd part fails
#[test]
fn test_flow_chain_member_ignition_cmd() {
    let source = r#"
package Test {
    item def IgnitionCmd;
    action def StartVehicle {
        action trigger1 accept ignitionCmd : IgnitionCmd;
        action startEngine {
            in item ignitionCmd : IgnitionCmd;
        }
        flow of IgnitionCmd from trigger1.ignitionCmd to startEngine.ignitionCmd;
    }
}
"#;

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Debug: print what symbols exist
    println!("\nSymbols containing 'ignition' or 'trigger':");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        let name = sym.name.as_ref();
        if name.contains("ignition") || name.contains("trigger") || name.contains("startEngine") {
            println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
        }
    }
    
    // Debug: scan the flow line for hover results
    println!("\nHover scan on flow line (line 8):");
    for col in 35..85 {
        if let Some(hover) = analysis.hover(file_id, 8, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }

    // trigger1 should resolve (line 9, ~col 41)
    assert!(
        test_hover_resolves(source, 8, 41, "trigger1"),
        "trigger1 should resolve in flow"
    );
    
    // trigger1.ignitionCmd should resolve (the .ignitionCmd part at ~col 50)
    assert!(
        test_hover_resolves(source, 8, 50, "ignitionCmd"),
        "trigger1.ignitionCmd chain member should resolve"
    );
    
    // startEngine.ignitionCmd should resolve 
    assert!(
        test_hover_resolves(source, 8, 75, "ignitionCmd"),
        "startEngine.ignitionCmd chain member should resolve"
    );
}

/// Test message chain member: `message from driver.turnVehicleOn to vehicle.trigger1`
/// Line 781: driver.turnVehicleOn and vehicle.trigger1
#[test]
fn test_message_chain_member() {
    let source = r#"
package Test {
    item def IgnitionCmd;
    part def Driver {
        action turnVehicleOn;
    }
    part def Vehicle {
        action trigger1;
    }
    part driver : Driver;
    part vehicle : Vehicle;
    message of IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
}
"#;
    // driver.turnVehicleOn chain member
    assert!(
        test_hover_resolves(source, 11, 45, "turnVehicleOn"),
        "driver.turnVehicleOn chain member should resolve"
    );
    
    // vehicle.trigger1 chain member
    assert!(
        test_hover_resolves(source, 11, 65, "trigger1"),
        "vehicle.trigger1 chain member should resolve"
    );
}

/// Test allocation chain member: `allocate foo.bar to baz.qux`
#[test]
fn test_allocation_chain_member() {
    let source = r#"
package Test {
    part def System {
        part logical {
            action computeResult;
        }
        part physical {
            action runComputation;
        }
        allocate logical.computeResult to physical.runComputation;
    }
}
"#;
    // logical.computeResult
    assert!(
        test_hover_resolves(source, 9, 27, "computeResult"),
        "logical.computeResult chain member should resolve"
    );
    
    // physical.runComputation
    assert!(
        test_hover_resolves(source, 9, 52, "runComputation"),
        "physical.runComputation chain member should resolve"
    );
}

/// Test interface end chain: `interface myInterface connect foo.port1 to bar.port2`
/// Uses correct SysML v2 syntax: interface <name> [: Type] connect ...
#[test]
fn test_interface_chain_member() {
    use syster::parser::parse_sysml;
    
    let source = r#"
package Test {
    port def DataPort;
    part def ComponentA {
        port dataOut : DataPort;
    }
    part def ComponentB {
        port dataIn : DataPort;
    }
    part compA : ComponentA;
    part compB : ComponentB;
    interface myInterface connect compA.dataOut to compB.dataIn;
}
"#;

    // First show AST
    let tree = parse_sysml(source);
    println!("\nAST for interface connects:");
    fn print_tree(node: &syster::parser::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text_snippet: String = node.text().to_string().chars().take(50).collect();
        println!("{}{:?} {:?} '{}'", prefix, node.kind(), node.text_range(), text_snippet);
        for child in node.children() {
            print_tree(&child, indent + 1);
        }
    }
    print_tree(&tree.syntax(), 0);

    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Debug: print symbols and their type_refs
    println!("\nSymbols with type_refs:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        if sym.name.as_ref() == "compA" || sym.name.as_ref() == "compB" || sym.name.as_ref() == "myInterface" {
            println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
            for tr in &sym.type_refs {
                println!("    type_ref: {:?}", tr);
            }
        }
    }
    
    // Debug: scan interface line with wider range
    println!("\nHover scan on interface line (line 11):");
    for col in 25..65 {
        if let Some(hover) = analysis.hover(file_id, 11, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        } else {
            println!("  col {}: NONE", col);
        }
    }

    // compA.dataOut - first find the correct column
    // line 11: "    interface myInterface connect compA.dataOut to compB.dataIn;"
    // col:      0123456789...
    assert!(
        test_hover_resolves(source, 11, 45, "dataOut"),
        "compA.dataOut chain member should resolve"
    );
    
    // compB.dataIn
    assert!(
        test_hover_resolves(source, 11, 58, "dataIn"),
        "compB.dataIn chain member should resolve"
    );
}

/// Test bind chain: `bind foo.x = bar.y`
#[test]
fn test_bind_chain_member() {
    let source = r#"
package Test {
    part def Container {
        attribute x : Integer;
        attribute y : Integer;
    }
    part a : Container;
    part b : Container;
    bind a.x = b.y;
}
"#;
    // a.x
    assert!(
        test_hover_resolves(source, 8, 12, "x"),
        "a.x chain member should resolve"
    );
    
    // b.y
    assert!(
        test_hover_resolves(source, 8, 18, "y"),
        "b.y chain member should resolve"
    );
}

/// Test connect chain: `connect a.p1 to b.p2`
#[test]
fn test_connect_chain_member() {
    let source = r#"
package Test {
    port def P;
    part def Box {
        port p1 : P;
        port p2 : P;
    }
    part boxA : Box;
    part boxB : Box;
    connect boxA.p1 to boxB.p2;
}
"#;
    // boxA.p1
    assert!(
        test_hover_resolves(source, 9, 18, "p1"),
        "boxA.p1 chain member should resolve"
    );
    
    // boxB.p2
    assert!(
        test_hover_resolves(source, 9, 30, "p2"),
        "boxB.p2 chain member should resolve"
    );
}

/// Test succession chain: `first a.x then b.y`
#[test]
fn test_succession_chain_member() {
    let source = r#"
package Test {
    action def Process {
        action step1 {
            action substep;
        }
        action step2 {
            action substep;
        }
        first step1.substep then step2.substep;
    }
}
"#;
    // step1.substep
    assert!(
        test_hover_resolves(source, 9, 21, "substep"),
        "step1.substep chain member should resolve"
    );
    
    // step2.substep
    assert!(
        test_hover_resolves(source, 9, 40, "substep"),
        "step2.substep chain member should resolve"
    );
}

/// Test deep chain: `a.b.c.d` - multiple levels
#[test]
fn test_deep_chain_member() {
    let source = r#"
package Test {
    part def Level1 {
        part level2 {
            part level3 {
                attribute value : Integer;
            }
        }
    }
    part root : Level1;
    attribute copy = root.level2.level3.value;
}
"#;
    // root.level2
    assert!(
        test_hover_resolves(source, 10, 27, "level2"),
        "root.level2 chain member should resolve"
    );
    
    // root.level2.level3
    assert!(
        test_hover_resolves(source, 10, 34, "level3"),
        "root.level2.level3 chain member should resolve"
    );
    
    // root.level2.level3.value
    assert!(
        test_hover_resolves(source, 10, 41, "value"),
        "root.level2.level3.value chain member should resolve"
    );
}
