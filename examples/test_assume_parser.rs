use syster::parser::sysml::Rule;
use pest::Parser;
use syster::parser::SysMLParser;

fn print_tree(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let rule = pair.as_rule();
    let text = pair.as_str();
    let short = if text.len() > 60 { &text[..60] } else { text };
    println!("{}{:?}: {:?}", "  ".repeat(indent), rule, short.replace('\n', "\\n"));
    for inner in pair.into_inner() {
        print_tree(inner, indent + 1);
    }
}

fn main() {
    let source = r#"package Test {
    attribute def MassValue;
    attribute assumedCargoMass : MassValue;
    
    requirement def FuelEconomyRequirement {
        attribute requiredFuelEconomy;
    }
    
    requirement highwayFuelEconomyRequirement : FuelEconomyRequirement {
        assume constraint { assumedCargoMass <= 500 }
    }
}"#;
    
    println!("=== PARSER TEST: assume constraint ===\n");
    println!("SOURCE:\n{}\n", source);
    
    let result = SysMLParser::parse(Rule::file, source);
    match result {
        Ok(pairs) => {
            println!("PARSE TREE:");
            for pair in pairs {
                print_tree(pair, 0);
            }
        }
        Err(e) => {
            println!("PARSE ERROR: {:?}", e);
        }
    }
}
