//! Test for flow chain member hover - fixing extraction gap for send actions

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

/// Debug test to see AST structure of standalone accept action
#[test]
fn test_send_action_ast_structure() {
    let source = r#"
package Test {
    action def StartEngine {
        accept ignitionCmd : IgnitionCmd via port1;
    }
}
"#;
    
    let tree = parse_sysml(source);
    println!("\nAST for standalone accept action:");
    fn print_tree(node: &syster::parser::SyntaxNode, indent: usize) {
        let prefix = "  ".repeat(indent);
        let text_snippet = node.text().to_string().chars().take(40).collect::<String>();
        println!("{}{:?} {:?} '{}'", prefix, node.kind(), node.text_range(), text_snippet);
        for child in node.children() {
            print_tree(&child, indent + 1);
        }
    }
    print_tree(&tree.syntax(), 0);
}

/// Test that send actions are indexed
#[test]
fn test_flow_to_endpoint_chain_member() {
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../language-server/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml");
    
    let source = std::fs::read_to_string(&file_path)
        .expect("Failed to read SimpleVehicleModel.sysml");
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", &source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Check if sendStatus, trigger1, and startEngine exist in the index
    println!("\nLooking for relevant symbols:");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        let name = sym.name.as_ref();
        if name == "sendStatus" || name == "startEngine" || name == "trigger1" || name == "trigger2" {
            println!("  Found: {} (line {})", sym.qualified_name, sym.start_line);
        }
    }
    
    // Line 764 (1-indexed) = line 763 (0-indexed)
    let line = 763_u32;
    
    // sendStatus should resolve
    let hover_72 = analysis.hover(file_id, line, 72);
    assert!(
        hover_72.as_ref()
            .and_then(|h| h.qualified_name.as_ref())
            .map(|qn| qn.contains("sendStatus"))
            .unwrap_or(false),
        "hover on 'sendStatus' at line 764 col 72 should resolve"
    );
}
