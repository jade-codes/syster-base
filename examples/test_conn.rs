use syster::parser::sysml::Rule;
use pest::Parser;
use syster::parser::SysMLParser;

fn print_tree(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let rule = pair.as_rule();
    let text = pair.as_str();
    let short_text = if text.len() > 50 { &text[..50] } else { text };
    println!("{}{:?}: {:?}", " ".repeat(indent), rule, short_text.replace('\n', "\\n"));
    for inner in pair.into_inner() {
        print_tree(inner, indent + 2);
    }
}

fn main() {
    let source = r#"#derivation connection {
    end #original ::> vehicleSpecification.vehicleMassRequirement;
}"#;
    
    println!("Parsing: {:?}\n", source);
    let result = SysMLParser::parse(Rule::connection_usage, source);
    match result {
        Ok(pairs) => {
            for pair in pairs {
                print_tree(pair, 0);
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}
