use syster::parser::sysml::{SysMLParser, Rule};
use syster::syntax::sysml::ast::parse_usage;
use pest::Parser;

fn main() {
    let tests = vec![
        ("fork fork1;", Rule::fork_node, "fork_node AST"),
        ("join join1;", Rule::join_node, "join_node AST"),
        ("merge merge1;", Rule::merge_node, "merge_node AST"),
        ("decide decide1;", Rule::decision_node, "decision_node AST"),
    ];
    
    println!("=== Testing Control Node AST ===\n");
    
    for (input, rule, desc) in tests {
        print!("Testing {}: ", desc);
        match SysMLParser::parse(rule, input) {
            Ok(pairs) => {
                let pair = pairs.into_iter().next().unwrap();
                let usage = parse_usage(pair);
                println!("✓ PASSED");
                println!("  name: {:?}", usage.name);
                println!("  kind: {:?}", usage.kind);
            }
            Err(e) => {
                println!("✗ FAILED");
                println!("  Error: {}", e);
            }
        }
        println!();
    }
}
