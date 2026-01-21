use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};

fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let text = pair.as_str().chars().take(60).collect::<String>().replace('\n', "\\n");
    println!("{}[{:?}] = '{}'", indent_str, pair.as_rule(), text);
    for inner in pair.clone().into_inner() {
        print_tree(&inner, indent + 1);
    }
}

fn main() {
    let input = r#"calc def BestFuel {
        in mass: MassValue;
        return f_b : Real = bsfc * mass;
    }"#;
    
    println!("=== calc def with return ===");
    let pairs = SysMLParser::parse(Rule::calculation_definition, input).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
}
