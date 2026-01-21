//! Parser test for metadata annotation references
//!
//! Tests that @StatusInfo { status = StatusKind::closed; } parses correctly

use syster::parser::sysml::Rule;
use pest::Parser;
use syster::parser::SysMLParser;

fn main() {
    let source = r#"package ModelingMetadata {
    enum def StatusKind {
        enum open;
        enum closed;
    }
    
    metadata def StatusInfo {
        attribute status : StatusKind;
    }
}

package Test {
    import ModelingMetadata::*;
    
    part myPart {
        @StatusInfo {
            status = StatusKind::closed;
        }
    }
}"#;

    println!("=== PARSER TEST: metadata annotation ===\n");
    println!("SOURCE:\n{}\n", source);

    let result = SysMLParser::parse(Rule::root_namespace, source);
    match result {
        Ok(pairs) => {
            println!("PARSE SUCCESSFUL!\n");
            println!("Parse tree:");
            for pair in pairs {
                print_tree(pair, 0);
            }
        }
        Err(e) => {
            println!("PARSE FAILED: {:?}", e);
        }
    }
}

fn print_tree(pair: pest::iterators::Pair<Rule>, indent: usize) {
    let rule = pair.as_rule();
    let span = pair.as_span();
    let text = pair.as_str();
    
    // Only print rules related to metadata, qualified_name, or annotations
    let rule_name = format!("{:?}", rule);
    if rule_name.contains("metadata") 
        || rule_name.contains("annotation")
        || rule_name.contains("qualified_name")
        || rule_name.contains("feature_value")
        || rule_name.contains("StatusKind")
        || rule_name.contains("status")
        || text.contains("StatusKind::closed")
        || text.contains("status")
    {
        let indent_str = "  ".repeat(indent);
        let short_text = if text.len() > 60 {
            format!("{}...", &text[..60])
        } else {
            text.to_string()
        };
        println!("{}{:?} ({},{}): '{}'", indent_str, rule, span.start(), span.end(), short_text);
    }
    
    for inner in pair.into_inner() {
        print_tree(inner, indent + 1);
    }
}
