use std::path::Path;
use syster::syntax::parser::parse_content;

fn main() {
    // Simpler case - just subject without specialization
    let source = r#"package Test {
    analysis a : TradeStudy {
        subject vehicleAlternatives;
    }
}"#;

    let parse = parse_content(source, Path::new("test.sysml")).unwrap();

    // Print the syntax tree with all tokens
    println!("=== Simple subject ===");
    fn print_tree(node: &rowan::SyntaxNode<syster::parser::SysMLLanguage>, indent: usize) {
        let prefix = "  ".repeat(indent);
        println!("{}{:?} {:?}", prefix, node.kind(), node.text_range());
        for elem in node.children_with_tokens() {
            match elem {
                rowan::NodeOrToken::Node(child) => print_tree(&child, indent + 1),
                rowan::NodeOrToken::Token(tok) => {
                    if tok.kind() != syster::parser::SyntaxKind::WHITESPACE {
                        println!("{}  TOKEN {:?} {:?}", prefix, tok.kind(), tok.text());
                    }
                }
            }
        }
    }
    print_tree(&parse.parse().syntax(), 0);
}
