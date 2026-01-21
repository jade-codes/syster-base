use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};

fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}[{:?}] = '{}'", indent_str, pair.as_rule(), pair.as_str().replace('\n', "\\n"));
    for inner in pair.clone().into_inner() {
        print_tree(&inner, indent + 1);
    }
}

fn main() {
    // This is the SHORT form - just a reference
    let input = "perform providePower;";
    println!("=== SHORT FORM: 'perform providePower;' ===");
    let pairs = SysMLParser::parse(Rule::perform_action_usage, input).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
    
    println!("\n=== LONG FORM: 'perform action providePower;' ===");
    let input2 = "perform action providePower;";
    let pairs2 = SysMLParser::parse(Rule::perform_action_usage, input2).unwrap();
    for pair in pairs2 {
        print_tree(&pair, 0);
    }
}
