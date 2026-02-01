//! Focused tests for hover resolution issues.
//!
//! These tests isolate specific patterns that fail in the LSP hover test.

use syster::ide::AnalysisHost;

fn create_analysis(source: &str) -> AnalysisHost {
    println!("\n========== CREATING ANALYSIS ==========");
    println!("Source code:\n{}", source);
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", source);
    println!("Parse errors: {:?}", errors);
    host
}

/// Helper to check if hover at a position returns something
fn has_hover_at(host: &mut AnalysisHost, line: u32, col: u32) -> Option<String> {
    println!("\n---------- HOVER REQUEST ----------");
    println!("Position: line {}, col {}", line, col);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    println!("File ID: {:?}", file_id);
    let result = analysis.hover(file_id, line, col).map(|h| h.contents);
    println!("Hover result: {:?}", result);
    result
}

// =============================================================================
// REDEFINES PATTERN
// =============================================================================

#[test]
fn test_hover_on_redefines_target() {
    // Pattern: `perform ActionTree::providePower redefines providePower;`
    // Hover on `providePower` (the redefines target) should resolve
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

    // Line 9: `perform ActionTree::providePower redefines providePower;`
    // The `providePower` after `redefines` should hover to Vehicle::providePower
    let hover = has_hover_at(&mut host, 9, 60); // "providePower" after redefines
    assert!(
        hover.is_some(),
        "Expected hover on redefines target 'providePower'"
    );
    println!("Hover result: {:?}", hover);
}

#[test]
fn test_hover_on_qualified_redefines_source() {
    // Pattern: `perform ActionTree::providePower redefines providePower;`
    // Hover on `ActionTree::providePower` should resolve to the action in ActionTree
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

    // Line 9: hover on "providePower" part of "ActionTree::providePower"
    let hover = has_hover_at(&mut host, 9, 36); // "providePower" in ActionTree::providePower
    assert!(
        hover.is_some(),
        "Expected hover on qualified ref 'ActionTree::providePower'"
    );
    println!("Hover result: {:?}", hover);
}

// =============================================================================
// SPECIALIZES PATTERN
// =============================================================================

#[test]
fn test_hover_on_specializes_target() {
    // Pattern: `part engine : Engine;`
    // Hover on `Engine` should resolve to the part def
    let source = r#"
package TestPkg {
    part def Engine;
    
    part def Vehicle {
        part engine : Engine;
    }
}
"#;

    let mut host = create_analysis(source);

    // Line 5: `part engine : Engine;`
    // Hover on `Engine` should work
    let hover = has_hover_at(&mut host, 5, 22); // "Engine"
    assert!(hover.is_some(), "Expected hover on type 'Engine'");
    println!("Hover result: {:?}", hover);
}

// =============================================================================
// FEATURE CHAIN PATTERN
// =============================================================================

#[test]
fn test_hover_on_feature_chain_first_part() {
    // Pattern: `fuelTank.mass`
    // Hover on `fuelTank` should resolve to the part
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

    // Line 8: `attribute totalMass = fuelTank.mass;`
    // Hover on `fuelTank` should work
    let hover = has_hover_at(&mut host, 8, 32); // "fuelTank" in expression
    assert!(
        hover.is_some(),
        "Expected hover on feature chain first part 'fuelTank'"
    );
    println!("Hover result: {:?}", hover);
}

#[test]
fn test_hover_on_feature_chain_second_part() {
    // Pattern: `fuelTank.mass`
    // Hover on `mass` should resolve to FuelTank::mass
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

    // Line 8: `attribute totalMass = fuelTank.mass;`
    // Hover on `mass` should work
    let hover = has_hover_at(&mut host, 8, 41); // "mass" in expression
    assert!(
        hover.is_some(),
        "Expected hover on feature chain second part 'mass'"
    );
    println!("Hover result: {:?}", hover);
}

// =============================================================================
// SUBSETS PATTERN
// =============================================================================

#[test]
fn test_hover_on_subsets_target() {
    // Pattern: `action doSomething subsets parentAction;`
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

    // Line 7: `action doSomething subsets parentAction;`
    // Hover on `parentAction` should work
    let hover = has_hover_at(&mut host, 7, 40); // "parentAction" after subsets
    assert!(
        hover.is_some(),
        "Expected hover on subsets target 'parentAction'"
    );
    println!("Hover result: {:?}", hover);
}

// =============================================================================
// TRANSITION PATTERN (then)
// =============================================================================

#[test]
fn test_hover_on_transition_target() {
    use syster::parser::{
        ast::{AstNode, NamespaceMember, SourceFile},
        parse_sysml,
    };

    println!("\n\n========== TEST: TRANSITION TARGET ==========");
    // Pattern: `transition initial then running;`
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

    // Debug: print the AST structure first
    println!("\n--- AST structure ---");
    let parse = parse_sysml(source);
    let root = SourceFile::cast(parse.syntax()).unwrap();
    for member in root.members() {
        if let NamespaceMember::Package(pkg) = member {
            if let Some(body) = pkg.body() {
                for inner in body.members() {
                    println!("Inner: {:?}", inner.syntax().kind());
                    if let NamespaceMember::Definition(def) = inner {
                        println!(
                            "  Definition body: {:?}",
                            def.body().map(|b| b.syntax().kind())
                        );
                        println!(
                            "  Definition children count: {}",
                            def.body().map(|b| b.members().count()).unwrap_or(0)
                        );
                        println!("  Raw syntax children:");
                        for child in def.syntax().children() {
                            let text: String = child.text().to_string().chars().take(50).collect();
                            println!("    {:?}: '{}'", child.kind(), text);
                        }
                    }
                }
            }
        }
    }

    let mut host = create_analysis(source);

    // Print what's at each position on line 6
    println!("\n--- Scanning line 6 for hover positions ---");
    for col in 0..50 {
        let hover = has_hover_at(&mut host, 6, col);
        if hover.is_some() {
            println!("  Col {}: FOUND hover", col);
        }
    }

    // Line 6: `transition initial then running;`
    // Hover on `running` should work
    println!("\n--- Testing specific position ---");
    let hover = has_hover_at(&mut host, 6, 36); // "running" after then

    // Also try nearby positions
    println!("\n--- Trying nearby positions for 'running' ---");
    for col in 30..45 {
        let h = has_hover_at(&mut host, 6, col);
        println!("  Col {}: {:?}", col, h.is_some());
    }

    assert!(
        hover.is_some(),
        "Expected hover on transition target 'running'"
    );
}

// =============================================================================
// EXPRESSION VALUE PATTERN
// =============================================================================

#[test]
fn test_hover_on_expression_reference() {
    // Pattern: `attribute x = y + 1;`
    let source = r#"
package TestPkg {
    part def Calculator {
        attribute y : Real = 10;
        attribute x : Real = y + 1;
    }
}
"#;

    let mut host = create_analysis(source);

    // Line 4: `attribute x : Real = y + 1;`
    // Hover on `y` should work
    let hover = has_hover_at(&mut host, 4, 30); // "y" in expression
    assert!(
        hover.is_some(),
        "Expected hover on expression reference 'y'"
    );
    println!("Hover result: {:?}", hover);
}
// =============================================================================
// MESSAGE CHAIN PATTERN (from LSP test failure)
// =============================================================================

#[test]
fn test_hover_on_message_chain_second_part() {
    // Pattern: `message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;`
    // This is the exact pattern failing in the LSP test - hover on `turnVehicleOn` returns None
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

    // Print all symbols for debugging
    println!("\n--- ALL SYMBOLS ---");
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").expect("file not found");
    let index = analysis.symbol_index();
    for sym in index.symbols_in_file(file_id) {
        println!("  {} ({:?})", sym.qualified_name, sym.kind);
        if !sym.type_refs.is_empty() {
            for (i, tr) in sym.type_refs.iter().enumerate() {
                println!("    type_ref[{}]: {:?}", i, tr);
            }
        }
    }

    // Line 14: `message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;`
    // Count: 0         1         2         3         4         5         6         7         8
    //        0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890
    //                message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    //                                                        ^     ^             ^  ^       ^
    //                                                        52    59            75 79      87

    println!("\n--- Testing hover positions on line 14 ---");

    // First, hover on `driver` - should work
    let hover_driver = has_hover_at(&mut host, 14, 56);
    println!("Hover on 'driver' (col 56): {:?}", hover_driver.is_some());

    // Now, hover on `turnVehicleOn` - this is the failing case
    let hover_turn = has_hover_at(&mut host, 14, 63);
    println!(
        "Hover on 'turnVehicleOn' (col 63): {:?}",
        hover_turn.is_some()
    );

    // Also test `trigger1`
    let hover_trigger = has_hover_at(&mut host, 14, 87);
    println!(
        "Hover on 'trigger1' (col 87): {:?}",
        hover_trigger.is_some()
    );

    assert!(hover_driver.is_some(), "Expected hover on 'driver'");
    assert!(
        hover_turn.is_some(),
        "Expected hover on 'turnVehicleOn' - this is the chain member"
    );
    assert!(
        hover_trigger.is_some(),
        "Expected hover on 'trigger1' - this is the chain member"
    );
}

// =============================================================================
// BIND CHAIN PATTERN (from LSP test failure - "other" category)
// =============================================================================

#[test]
fn test_hover_on_bind_chain_second_part() {
    // Pattern: `bind shaftPort_d=differential.shaftPort_d;`
    // This is failing as "other" in LSP test
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

    // Print all symbols for debugging
    println!("\n--- ALL SYMBOLS ---");
    {
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").expect("file not found");
        let index = analysis.symbol_index();
        for sym in index.symbols_in_file(file_id) {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            if !sym.type_refs.is_empty() {
                for (i, tr) in sym.type_refs.iter().enumerate() {
                    println!("    type_ref[{}]: {:?}", i, tr);
                }
            }
        }
    }

    println!("\n--- Testing bind chain positions ---");

    // Line 11: `bind shaftPort_d = differential.shaftPort_d;`
    //          0         1         2         3         4         5
    //          0123456789012345678901234567890123456789012345678901234
    //                  bind shaftPort_d = differential.shaftPort_d;
    //                       ^            ^            ^
    //                       13           27           40

    // Hover on first `shaftPort_d`
    let hover1 = has_hover_at(&mut host, 11, 17);
    println!("Hover on first 'shaftPort_d': {:?}", hover1.is_some());

    // Hover on `differential`
    let hover2 = has_hover_at(&mut host, 11, 31);
    println!("Hover on 'differential': {:?}", hover2.is_some());

    // Hover on second `shaftPort_d` (the chain member)
    let hover3 = has_hover_at(&mut host, 11, 44);
    println!(
        "Hover on second 'shaftPort_d' (chain): {:?}",
        hover3.is_some()
    );

    assert!(
        hover3.is_some(),
        "Expected hover on 'differential.shaftPort_d' chain member"
    );
}

// =============================================================================
// REDEFINES IN ITEM PATTERN (from LSP test failure)
// =============================================================================

/// Test for: `in item fuelCmd:FuelCmd redefines pwrCmd;`
/// This is a nested item that redefines a feature from a parent port def.
#[test]
fn test_hover_on_item_redefines_in_port_def() {
    // This pattern is from SimpleVehicleModel.sysml line 234
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

    // Print all symbols for debugging
    println!("\n--- ALL SYMBOLS ---");
    {
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").expect("file not found");
        let index = analysis.symbol_index();
        for sym in index.symbols_in_file(file_id) {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            if !sym.type_refs.is_empty() {
                for (i, tr) in sym.type_refs.iter().enumerate() {
                    println!("    type_ref[{}]: {:?}", i, tr);
                }
            }
        }
    }

    // Line 7: `in item fuelCmd : FuelCmd redefines pwrCmd;`
    // The `pwrCmd` after `redefines` should hover to PwrCmdPort::pwrCmd
    let hover = has_hover_at(&mut host, 7, 48); // "pwrCmd" after redefines
    println!("Hover result: {:?}", hover);

    assert!(
        hover.is_some(),
        "Expected hover on redefines target 'pwrCmd'"
    );
}

// =============================================================================
// BIND PATTERN (from LSP test failure - "other" category)
// =============================================================================

/// Test for: `bind shaftPort_d=differential.shaftPort_d;`
/// This tests the scenario from LSP test failure report line 635.
/// The test checks hover on:
/// 1. First `shaftPort_d` (the bind target)
/// 2. `differential` (first part of chain)
/// 3. Second `shaftPort_d` (chain member)
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

    // Print all symbols for debugging
    println!("\n--- ALL SYMBOLS ---");
    {
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").expect("file not found");
        let index = analysis.symbol_index();
        for sym in index.symbols_in_file(file_id) {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "    span: line {}-{}, col {}-{}",
                sym.start_line, sym.end_line, sym.start_col, sym.end_col
            );
            if !sym.type_refs.is_empty() {
                for (i, tr) in sym.type_refs.iter().enumerate() {
                    println!("    type_ref[{}]: {:?}", i, tr);
                }
            }
        }
    }

    // Line 12: `bind shaftPort_d = differential.shaftPort_d;`
    //          0         1         2         3         4         5
    //          0123456789012345678901234567890123456789012345678901234
    //                  bind shaftPort_d = differential.shaftPort_d;
    //                       ^            ^             ^
    //                       13           28            41

    // Hover on first `shaftPort_d` (the bind target)
    println!("\n--- Testing hover on first shaftPort_d (bind target) ---");
    let hover1 = has_hover_at(&mut host, 12, 17);
    println!("Hover on first 'shaftPort_d': {:?}", hover1);

    // Hover on `differential`
    println!("\n--- Testing hover on differential ---");
    let hover2 = has_hover_at(&mut host, 12, 32);
    println!("Hover on 'differential': {:?}", hover2);

    // Hover on second `shaftPort_d` (chain member)
    println!("\n--- Testing hover on second shaftPort_d (chain) ---");
    let hover3 = has_hover_at(&mut host, 12, 47);
    println!("Hover on second 'shaftPort_d': {:?}", hover3);

    // All should resolve
    assert!(
        hover1.is_some(),
        "Expected hover on bind target 'shaftPort_d'"
    );
    assert!(hover2.is_some(), "Expected hover on 'differential'");
    assert!(
        hover3.is_some(),
        "Expected hover on chain member 'shaftPort_d'"
    );
}

// =============================================================================
// TRANSITION PATTERN (from LSP test failure - "then (transition)" category)
// =============================================================================

/// Test for: `transition initial then off;`
/// This tests the scenario from LSP test failure report line 54.
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

    // Print all symbols for debugging
    println!("\n--- ALL SYMBOLS ---");
    {
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").expect("file not found");
        let index = analysis.symbol_index();
        for sym in index.symbols_in_file(file_id) {
            println!("  {} ({:?})", sym.qualified_name, sym.kind);
            println!(
                "    span: line {}-{}, col {}-{}",
                sym.start_line, sym.end_line, sym.start_col, sym.end_col
            );
            if !sym.type_refs.is_empty() {
                for (i, tr) in sym.type_refs.iter().enumerate() {
                    println!("    type_ref[{}]: {:?}", i, tr);
                }
            }
        }
    }

    // Line 9: `transition initial then off;`
    //         0         1         2         3         4
    //         01234567890123456789012345678901234567890
    //                 transition initial then off;
    //                            ^       ^
    //                            19      31

    // Hover on `initial`
    println!("\n--- Testing hover on initial ---");
    let hover1 = has_hover_at(&mut host, 9, 21);
    println!("Hover on 'initial': {:?}", hover1);

    // Hover on `off`
    println!("\n--- Testing hover on off ---");
    let hover2 = has_hover_at(&mut host, 9, 33);
    println!("Hover on 'off': {:?}", hover2);

    // Check that hover contains the state name (not just any hover)
    assert!(
        hover1.is_some(),
        "Expected hover on transition source 'initial'"
    );
    let h1 = hover1.unwrap();
    assert!(
        h1.contains("initial") || h1.contains("::initial"),
        "Hover should mention 'initial', got: {}",
        h1
    );

    assert!(
        hover2.is_some(),
        "Expected hover on transition target 'off'"
    );
    let h2 = hover2.unwrap();
    assert!(
        h2.contains("off") || h2.contains("::off"),
        "Hover should mention 'off', got: {}",
        h2
    );
}

/// Debug test to see what the parse tree looks like for a transition
#[test]
fn test_debug_transition_parse_tree() {
    use syster::parser::parse_sysml;

    let source = "state def T { transition initial then off; }";
    let parsed = parse_sysml(source);

    println!("\n=== PARSE TREE ===");

    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let indent_str = "  ".repeat(indent);
        println!("{}{:?} [{:?}]", indent_str, node.kind(), node.text_range());
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => print_tree(&n, indent + 1),
                rowan::NodeOrToken::Token(t) => {
                    println!(
                        "{}  {:?} {:?} [{:?}]",
                        indent_str,
                        t.kind(),
                        t.text(),
                        t.text_range()
                    );
                }
            }
        }
    }

    print_tree(&parsed.syntax(), 0);
}

/// Debug test for first X then Y parse tree
#[test]
fn test_debug_first_action_parse_tree() {
    use syster::parser::parse_sysml;

    let source = "action def T { action start; action middle; first start then middle; }";
    let parsed = parse_sysml(source);

    println!("\n=== FIRST ACTION PARSE TREE ===");

    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let indent_str = "  ".repeat(indent);
        println!("{}{:?} [{:?}]", indent_str, node.kind(), node.text_range());
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => print_tree(&n, indent + 1),
                rowan::NodeOrToken::Token(t) => {
                    println!(
                        "{}  {:?} {:?} [{:?}]",
                        indent_str,
                        t.kind(),
                        t.text(),
                        t.text_range()
                    );
                }
            }
        }
    }

    print_tree(&parsed.syntax(), 0);
}

/// Debug test for assume constraint parse tree
#[test]
fn test_debug_assume_constraint_parse_tree() {
    use syster::parser::parse_sysml;

    let source = "requirement def R { attribute x; assume constraint { x <= 500; } }";
    let parsed = parse_sysml(source);

    println!("\n=== ASSUME CONSTRAINT PARSE TREE ===");

    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let indent_str = "  ".repeat(indent);
        println!("{}{:?} [{:?}]", indent_str, node.kind(), node.text_range());
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(n) => print_tree(&n, indent + 1),
                rowan::NodeOrToken::Token(t) => {
                    println!(
                        "{}  {:?} {:?} [{:?}]",
                        indent_str,
                        t.kind(),
                        t.text(),
                        t.text_range()
                    );
                }
            }
        }
    }

    print_tree(&parsed.syntax(), 0);
}
