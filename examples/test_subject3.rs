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
    
    // Print the syntax tree with all tokens
    println!("=== Syntax Tree with tokens ===");
    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let prefix = "  ".repeat(indent);
        println!("{}{:?} {:?}", prefix, node.kind(), node.text_range());
        for elem in node.children_with_tokens() {
            match elem {
                rowan::NodeOrToken::Node(child) => print_tree(&child, indent + 1),
                rowan::NodeOrToken::Token(tok) => {
                    println!("{}  TOKEN {:?} {:?} {:?}", prefix, tok.kind(), tok.text_range(), tok.text());
                }
            }
        }
    }
    print_tree(&parse.parse().syntax(), 0);
}
