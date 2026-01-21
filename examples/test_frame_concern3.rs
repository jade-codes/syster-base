use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    // Test variations of framed_concern_member
    let tests = vec![
        ("frame vs:VehicleSafety;", Rule::framed_concern_member, "frame without concern keyword"),
        ("frame concern vs:VehicleSafety;", Rule::framed_concern_member, "frame with concern keyword"),
    ];
    
    println!("=== Testing Framed Concern Member ===\n");
    
    for (input, rule, desc) in tests {
        print!("Testing {}: ", desc);
        match SysMLParser::parse(rule, input) {
            Ok(pairs) => {
                println!("✓ PASSED");
                for pair in pairs {
                    print_pair(&pair, 1);
                }
            }
            Err(e) => {
                println!("✗ FAILED");
                println!("  Input: {}", input);
                println!("  Error: {:?}", e);
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
