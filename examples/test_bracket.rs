use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let input = r#"attribute :>> position3dVector = (0,0,0) [spatialCF];"#;
    
    match SysMLParser::parse(Rule::attribute_usage, input) {
        Ok(pairs) => {
            for pair in pairs {
                print_pairs(pair, 0);
            }
        }
        Err(e) => println!("Parse error: {}", e),
    }
}

fn print_pairs(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let pad = "  ".repeat(indent);
    println!("{}Rule::{:?} = {:?}", pad, pair.as_rule(), pair.as_str());
    for inner in pair.into_inner() {
        print_pairs(inner, indent + 1);
    }
}
