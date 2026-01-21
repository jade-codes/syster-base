use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};

fn print_tree(pair: &pest::iterators::Pair<Rule>, indent: usize) {
    let indent_str = "  ".repeat(indent);
    let text = pair.as_str().chars().take(50).collect::<String>().replace('\n', "\\n");
    println!("{}[{:?}] = '{}'", indent_str, pair.as_rule(), text);
    for inner in pair.clone().into_inner() {
        print_tree(&inner, indent + 1);
    }
}

fn main() {
    // Anonymous return with just type
    let input1 = "return : Real;";
    println!("=== 'return : Real;' ===");
    let pairs = SysMLParser::parse(Rule::return_parameter_member, input1).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
    
    println!("\n=== 'return dpv :> distancePerVolume = 1/f;' ===");
    let input2 = "return dpv :> distancePerVolume = 1/f;";
    let pairs = SysMLParser::parse(Rule::return_parameter_member, input2).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
    
    println!("\n=== 'in bestFuelConsumption: Real;' ===");
    let input3 = "in bestFuelConsumption: Real;";
    let pairs = SysMLParser::parse(Rule::parameter_binding, input3).unwrap();
    for pair in pairs {
        print_tree(&pair, 0);
    }
}
