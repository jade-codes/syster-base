use syster::syntax::sysml::ast::{DefinitionMember, UsageMember, Element, parse_file};
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

#[test]
fn test_debug_typed_by_span() {
    let source = r#"package Test {
    item def IgnitionCmd;
    part def V {
        action turnVehicleOn send ignitionCmd via p1 {
            in ignitionCmd : IgnitionCmd;
        }
    }
}"#;

    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let file = parse_file(&mut pairs).expect("AST parse should succeed");
    
    for elem in &file.elements {
        if let Element::Definition(def) = elem {
            println!("Definition: {} (kind: {:?})", def.name.as_ref().unwrap_or(&"<unnamed>".to_string()), def.kind);
            for member in &def.body {
                if let DefinitionMember::Usage(usage) = member {
                    println!("  Usage: {} (kind: {:?})", usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()), usage.kind);
                    println!("    typed_by: {:?}", usage.relationships.typed_by);
                    println!("    typed_by_span: {:?}", usage.relationships.typed_by_span);
                    for nested in &usage.body {
                        if let UsageMember::Usage(nested_usage) = nested {
                            println!("    Nested Usage: {} (kind: {:?})", nested_usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()), nested_usage.kind);
                            println!("      typed_by: {:?}", nested_usage.relationships.typed_by);
                            println!("      typed_by_span: {:?}", nested_usage.relationships.typed_by_span);
                        }
                    }
                }
            }
        }
    }
}
