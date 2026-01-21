use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};
use syster::syntax::sysml::ast::parsers::parse_usage;

fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!("{}[{:?}] = '{}'", indent_str, pair.as_rule(), pair.as_str().chars().take(50).collect::<String>().replace('\n', "\\n"));
    for inner in pair.clone().into_inner() {
        print_tree(&inner, indent + 1);
    }
}

fn main() {
    let input = "join join1;";
    println!("=== 'join join1;' ===");
    let pairs = SysMLParser::parse(Rule::join_node, input).unwrap();
    for pair in pairs.clone() {
        print_tree(&pair, 0);
    }
    
    // Try parsing as usage
    println!("\n=== As Usage ===");
    let pair = SysMLParser::parse(Rule::join_node, input).unwrap().next().unwrap();
    let usage = parse_usage(pair);
    println!("Name: {:?}", usage.name);
    println!("Kind: {:?}", usage.kind);
}
