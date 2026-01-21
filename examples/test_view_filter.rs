use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let code = r#"
package Test {
    view def PartsTreeView {
        filter @SysML::PartUsage;
    }
}
"#;
    
    println!("=== Parsing ===");
    let pairs = SysMLParser::parse(Rule::file, code).unwrap();
    
    fn print_tree(pair: pest::iterators::Pair<Rule>, indent: usize) {
        let rule = pair.as_rule();
        let text = pair.as_str();
        let text_preview = if text.len() > 40 { &text[..40] } else { text };
        println!("{:indent$}{:?}: {:?}", "", rule, text_preview.replace("\n", "\\n"), indent = indent);
        for inner in pair.into_inner() {
            print_tree(inner, indent + 2);
        }
    }
    
    for pair in pairs {
        print_tree(pair, 0);
    }
}
