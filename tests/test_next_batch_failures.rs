//! Tests for the next batch of hover failures
//! 
//! Categories:
//! - CHAIN_MEMBER: message chain endpoints (driver.turnVehicleOn, vehicle.trigger1)
//! - CHAIN_FIRST: metadata prefix (#mop), redefines with chain (= vehicle_b.engine.x)
//! - SPECIALIZATION: subsets with multiplicity (subsets foo[1])
//! - TYPING: transition accept (then action trigger accept)

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;
use syster::parser::parse_sysml;

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

fn print_ast(source: &str) {
    let tree = parse_sysml(source);
    fn print_tree(node: &syster::parser::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text: String = node.text().to_string().chars().take(50).collect();
        println!("{}{:?} {:?} '{}'", prefix, node.kind(), node.text_range(), text);
        for child in node.children() {
            print_tree(&child, indent + 1);
        }
    }
    print_tree(&tree.syntax(), 0);
}

// ============================================================================
// CHAIN_MEMBER: Message chain endpoints
// ============================================================================

/// Test message chain: `message from driver.turnVehicleOn to vehicle.trigger1`
/// Line 781: driver.turnVehicleOn and vehicle.trigger1 fail to resolve
#[test]
fn test_message_chain_endpoints() {
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
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on message line (line 11):");
    for col in 30..75 {
        if let Some(hover) = analysis.hover(file_id, 11, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // driver.turnVehicleOn should resolve
    assert!(
        test_hover_resolves(source, 11, 48, "turnVehicleOn"),
        "driver.turnVehicleOn chain member should resolve"
    );
    
    // vehicle.trigger1 should resolve
    assert!(
        test_hover_resolves(source, 11, 68, "trigger1"),
        "vehicle.trigger1 chain member should resolve"
    );
}

// ============================================================================
// CHAIN_FIRST: Metadata prefix and redefines with chain
// ============================================================================

/// Test metadata prefix: `#mop attribute mass`
/// Line 556: #mop fails - the metadata usage reference
#[test]
fn test_metadata_prefix_hover() {
    let source = r#"
package Test {
    metadata def mop;
    part def Vehicle {
        #mop attribute mass : Real;
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on attribute line (line 4):");
    for col in 8..30 {
        if let Some(hover) = analysis.hover(file_id, 4, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // #mop should resolve to Test::mop
    assert!(
        test_hover_resolves(source, 4, 10, "mop"),
        "#mop metadata reference should resolve"
    );
}

/// Test redefines with chain: `subject x redefines x = vehicle_b.engine.generateTorque`
/// Line 654: the chain vehicle_b.engine.generateTorque fails
#[test]
fn test_redefines_chain_value() {
    let source = r#"
package Test {
    part def Engine {
        attribute generateTorque : Real;
    }
    part def Vehicle {
        part engine : Engine;
    }
    part vehicle_b : Vehicle;
    
    requirement def TorqueReq {
        subject generateTorque redefines generateTorque = vehicle_b.engine.generateTorque;
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on subject line (line 11):");
    for col in 55..95 {
        if let Some(hover) = analysis.hover(file_id, 11, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // vehicle_b should resolve
    assert!(
        test_hover_resolves(source, 11, 60, "vehicle_b"),
        "vehicle_b in redefines chain should resolve"
    );
    
    // vehicle_b.engine should resolve
    assert!(
        test_hover_resolves(source, 11, 70, "engine"),
        "vehicle_b.engine chain member should resolve"
    );
    
    // vehicle_b.engine.generateTorque should resolve
    assert!(
        test_hover_resolves(source, 11, 80, "generateTorque"),
        "vehicle_b.engine.generateTorque chain member should resolve"
    );
}

// ============================================================================
// SPECIALIZATION: Subsets with multiplicity
// ============================================================================

/// Test subsets with multiplicity: `action x subsets foo[1]`
/// Line 1387: getInVehicle_a[1] fails
#[test]
fn test_subsets_with_multiplicity() {
    let source = r#"
package Test {
    action def GetInVehicle;
    action getInVehicle_a : GetInVehicle[0..*];
    
    action def Scenario {
        action driverGetInVehicle subsets getInVehicle_a[1];
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on action line (line 6):");
    for col in 35..60 {
        if let Some(hover) = analysis.hover(file_id, 6, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // getInVehicle_a should resolve (before the [1])
    assert!(
        test_hover_resolves(source, 6, 45, "getInVehicle_a"),
        "getInVehicle_a in subsets should resolve"
    );
}

// ============================================================================
// TYPING: Transition with accept
// ============================================================================

/// Test transition accept: `then action trigger accept ignitionCmd:IgnitionCmd`
/// Line 1390: trigger and ignitionCmd fail
#[test]
fn test_transition_accept_action() {
    let source = r#"
package Test {
    item def IgnitionCmd;
    
    state def VehicleState {
        entry;
        then action trigger accept ignitionCmd : IgnitionCmd;
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on transition line (line 6):");
    for col in 13..60 {
        if let Some(hover) = analysis.hover(file_id, 6, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // trigger should resolve
    assert!(
        test_hover_resolves(source, 6, 22, "trigger"),
        "trigger in transition accept should resolve"
    );
    
    // ignitionCmd should resolve (the payload)
    assert!(
        test_hover_resolves(source, 6, 37, "ignitionCmd"),
        "ignitionCmd payload in transition accept should resolve"
    );
}

// ============================================================================
// MESSAGE: Named argument in constructor
// ============================================================================

/// Test named argument: `send new IgnitionCmd (ignitionOnOff=...)`
/// Line 1328: ignitionOnOff fails
#[test]
fn test_named_argument_in_constructor() {
    let source = r#"
package Test {
    enum def OnOff { on; off; }
    item def IgnitionCmd {
        attribute ignitionOnOff : OnOff;
    }
    port def CmdPort;
    
    state def VehicleState {
        state on {
            do send new IgnitionCmd (ignitionOnOff = OnOff::on) via handPort;
        }
        port handPort : CmdPort;
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on send line (line 10):");
    for col in 30..70 {
        if let Some(hover) = analysis.hover(file_id, 10, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // ignitionOnOff in the constructor call should resolve
    assert!(
        test_hover_resolves(source, 10, 43, "ignitionOnOff"),
        "ignitionOnOff named argument should resolve"
    );
}

// ============================================================================
// OTHER: Enum member access
// ============================================================================

/// Test enum member: `status = StatusKind::closed`
/// Line 893: closed fails
#[test]
fn test_enum_member_access() {
    let source = r#"
package Test {
    enum def StatusKind { open; closed; }
    
    part def Door {
        attribute status : StatusKind = StatusKind::closed;
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on attribute line (line 5):");
    for col in 40..60 {
        if let Some(hover) = analysis.hover(file_id, 5, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // StatusKind should resolve
    assert!(
        test_hover_resolves(source, 5, 45, "StatusKind"),
        "StatusKind in default value should resolve"
    );
    
    // StatusKind::closed should resolve
    assert!(
        test_hover_resolves(source, 5, 57, "closed"),
        "StatusKind::closed enum member should resolve"
    );
}

// ============================================================================
// BINDING: Metadata on connection
// ============================================================================

/// Test metadata on connection: `#derivation connection { }`
/// Line 918: derivation fails
#[test]
fn test_metadata_on_connection() {
    let source = r#"
package Test {
    metadata def derivation;
    port def P;
    
    part def System {
        port p1 : P;
        port p2 : P;
        #derivation connection {
            end p1;
            end p2;
        }
    }
}
"#;
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    println!("\nAST:");
    print_ast(source);
    
    println!("\nSymbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!("  {} (line {}) kind={:?}", sym.qualified_name, sym.start_line, sym.kind);
    }
    
    println!("\nHover scan on connection line (line 8):");
    for col in 8..30 {
        if let Some(hover) = analysis.hover(file_id, 8, col) {
            if let Some(qn) = &hover.qualified_name {
                println!("  col {}: {}", col, qn);
            }
        }
    }
    
    // #derivation should resolve
    assert!(
        test_hover_resolves(source, 8, 11, "derivation"),
        "#derivation metadata reference should resolve"
    );
}
