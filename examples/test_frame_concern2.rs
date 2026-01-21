use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    // The grammar is:
    // framed_concern_member = { member_prefix ~ framed_concern_kind ~ framed_concern_usage }
    // framed_concern_kind = { "frame" }
    // framed_concern_usage = { owned_reference_subsetting ~ ... | (concern_usage_keyword | ...) ~ ... }
    
    let tests = vec![
        ("frame", Rule::framed_concern_kind, "framed_concern_kind"),
        ("concern vs:VehicleSafety;", Rule::framed_concern_usage, "framed_concern_usage with keyword"),
        ("vs:VehicleSafety;", Rule::framed_concern_usage, "framed_concern_usage without keyword"),
    ];
    
    println!("=== Testing Frame Concern Parts ===\n");
    
    for (input, rule, desc) in tests {
        print!("Testing {}: ", desc);
        match SysMLParser::parse(rule, input) {
            Ok(pairs) => {
                println!("✓ PASSED");
            }
            Err(e) => {
                println!("✗ FAILED");
                println!("  Input: {}", input);
                println!("  Error: {}", e);
            }
        }
    }
}
