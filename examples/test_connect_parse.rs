use syster::parser::sysml::Rule;
use pest::Parser;
use syster::parser::SysMLParser;

fn main() {
    let source = include_str!("/tmp/test_connect.sysml");
    println!("=== SOURCE ===");
    println!("{}", source);
    println!("\n=== PARSING ===");
    
    match SysMLParser::parse(Rule::file, source) {
        Ok(pairs) => {
            println!("PARSE SUCCESS!");
            for pair in pairs {
                print_pair(pair, 0);
            }
        }
        Err(e) => {
            println!("PARSE FAILED!");
            println!("{}", e);
        }
    }
}

fn print_pair(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let rule = pair.as_rule();
    let text = pair.as_str();
    let short = if text.len() > 60 { &text[..60] } else { text };
    println!("{}{:?}: {:?}", "  ".repeat(indent), rule, short.replace('\n', "\\n"));
    for inner in pair.into_inner() {
        print_pair(inner, indent + 1);
    }
}
