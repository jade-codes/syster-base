use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let tests = vec![
        ("fork fork1;", Rule::fork_node, "fork_node rule"),
        ("join join1;", Rule::join_node, "join_node rule"),
        ("fork fork1;", Rule::control_node, "control_node rule"),
        ("fork fork1;", Rule::action_node, "action_node rule"),
    ];
    
    println!("=== Testing Control Nodes ===\n");
    
    for (input, rule, desc) in tests {
        print!("Testing {}: ", desc);
        match SysMLParser::parse(rule, input) {
            Ok(_) => println!("✓ PASSED"),
            Err(e) => {
                println!("✗ FAILED");
                println!("  Input: {}", input);
                println!("  Error: {}", e);
            }
        }
    }
}
