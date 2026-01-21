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
    part def Engine4Cyl;
    part engine4Cyl : Engine4Cyl;
    
    #refinement dependency engine4Cyl to Target::path::element;
}"#;
    
    println!("=== PARSER TEST: #refinement dependency with 'to' ===\n");
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
