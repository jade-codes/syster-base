use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    // Test various use case constructs
    let tests = vec![
        // Test 1: first start then action
        ("first start;", Rule::empty_succession_member, "first start"),
        
        // Test 2: then action with body
        ("then action a { action driverGetInVehicle; }", Rule::transition_target, "then action with body"),
        
        // Test 3: action with subsets and array index
        ("action driverGetInVehicle subsets getInVehicle_a[1];", Rule::action_usage, "action with subsets array index"),
        
        // Test 4: accept with typed payload
        ("accept ignitionCmd:IgnitionCmd", Rule::accept_node_declaration, "accept with typed payload"),
        
        // Test 5: fork declaration
        ("fork fork1;", Rule::action_usage, "fork declaration"),
        
        // Test 6: join declaration  
        ("join join1;", Rule::action_usage, "join declaration"),
        
        // Test 7: then fork
        ("then fork fork1;", Rule::transition_target, "then fork"),
        
        // Test 8: first X then Y succession
        ("first driverGetInVehicle then join1;", Rule::succession_as_usage, "first then succession"),
    ];
    
    println!("=== Testing Use Case Constructs ===\n");
    
    for (input, rule, desc) in tests {
        print!("Testing {}: ", desc);
        match SysMLParser::parse(rule, input) {
            Ok(pairs) => {
                println!("✓ PASSED");
                // Print the parse tree for debugging
                for pair in pairs {
                    print_pair(&pair, 1);
                }
            }
            Err(e) => {
                println!("✗ FAILED");
                println!("  Input: {}", input);
                println!("  Error: {}", e);
            }
        }
        println!();
    }
}

fn print_pair(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let text = pair.as_str();
    let short_text: String = text.chars().take(50).collect();
    println!("{}{:?}: {:?}", indent_str, pair.as_rule(), short_text);
    for inner in pair.clone().into_inner() {
        print_pair(&inner, indent + 1);
    }
}
