use syster::syntax::parser::parse_content;
use syster::parser::AstNode;
use std::path::Path;

fn main() {
    let source = r#"package Test {
    analysis engineTradeOffAnalysis : TradeStudy {
        subject vehicleAlternatives [2] :> vehicle_b;
    }
}"#;
    
    let parse = parse_content(source, Path::new("test.sysml")).unwrap();
    
    // Print the syntax tree
    println!("=== Syntax Tree ===");
    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let prefix = "  ".repeat(indent);
        println!("{}{:?} {:?}", prefix, node.kind(), node.text_range());
        for child in node.children() {
            print_tree(&child, indent + 1);
        }
    }
    print_tree(&parse.parse().syntax(), 0);
}
