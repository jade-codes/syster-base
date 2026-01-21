use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};

fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}[{:?}] = '{}'", indent_str, pair.as_rule(), pair.as_str().chars().take(50).collect::<String>().replace('\n', "\\n"));
    for inner in pair.clone().into_inner() {
        print_tree(&inner, indent + 1);
    }
}

fn main() {
    let input = "first driverGetInVehicle then join1;";
    println!("=== 'first driverGetInVehicle then join1;' ===");
    let pairs = SysMLParser::parse(Rule::succession_as_usage, input).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
}
