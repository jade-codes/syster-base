//! Debug test for AST inspection of event chain pattern

use syster::parser::{SyntaxKind, SyntaxNode, parse_sysml};

fn print_tree(node: &SyntaxNode, indent: usize) {
    let kind = node.kind();
    let text_preview: String = node.text().to_string().chars().take(60).collect();
    let text_preview = text_preview.replace('\n', "\\n").replace('\r', "");
    println!(
        "{:indent$}{:?}: '{}'",
        "",
        kind,
        text_preview,
        indent = indent
    );
    for child in node.children() {
        print_tree(&child, indent + 2);
    }
}

/// Test parsing of `event sendSpeed.sourceEvent;`
#[test]
fn test_ast_event_chain() {
    let source = r#"
port speedSensorPort {
    event sendSpeed.sourceEvent;
}
"#;

    let tree = parse_sysml(source);
    println!("\n=== Full AST ===");
    print_tree(&tree.syntax(), 0);

    // Look for specific nodes
    println!("\n=== Looking for FEATURE_CHAIN nodes ===");
    fn find_kinds(node: &SyntaxNode, target: SyntaxKind, depth: usize) {
        if node.kind() == target {
            println!("Found {:?} at depth {}: '{}'", target, depth, node.text());
        }
        for child in node.children() {
            find_kinds(&child, target, depth + 1);
        }
    }
    find_kinds(&tree.syntax(), SyntaxKind::FEATURE_CHAIN, 0);
    find_kinds(&tree.syntax(), SyntaxKind::QUALIFIED_NAME, 0);
    find_kinds(&tree.syntax(), SyntaxKind::OCCURRENCE_USAGE, 0);
}

/// Test parsing of `then event occurrence sendData;`
#[test]
fn test_ast_then_succession() {
    let source = r#"
port speedSensorPort {
    event occurrence setSpeedReceived;
    then event occurrence sendData;
}
"#;

    let tree = parse_sysml(source);
    println!("\n=== Full AST for then succession ===");
    print_tree(&tree.syntax(), 0);

    println!("\n=== Looking for SUCCESSION* nodes ===");
    fn find_succession(node: &SyntaxNode, depth: usize) {
        let kind = node.kind();
        let kind_str = format!("{:?}", kind);
        if kind_str.contains("SUCC") || kind_str.contains("THEN") {
            println!("Found {:?} at depth {}: '{}'", kind, depth, node.text());
        }
        for child in node.children() {
            find_succession(&child, depth + 1);
        }
    }
    find_succession(&tree.syntax(), 0);
}

/// Test parsing of `perform providePower : ProvidePower;`
#[test]
fn test_ast_perform_typed() {
    let source = r#"
part def Vehicle {
    perform providePower : ProvidePower;
}
"#;

    let tree = parse_sysml(source);
    println!("\n=== Full AST for perform typed ===");
    print_tree(&tree.syntax(), 0);
}
