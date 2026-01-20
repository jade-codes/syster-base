use syster::syntax::sysml::ast::{UsageMember, Element, parse_file};
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

#[test]
fn test_debug_typed_by_span2() {
    let source = r#"package SimpleVehicleModel {
    package SignalDefinitions {
        item def IgnitionCmd;
    }
    
    package Sequence {
        import SignalDefinitions::*;
        
        part part0 {
            perform action startVehicle {
                action turnVehicleOn send ignitionCmd via driver.p1 {
                    in ignitionCmd : IgnitionCmd;
                }
            }
        }
    }
}"#;

    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let file = parse_file(&mut pairs).expect("AST parse should succeed");
    
    fn print_usage(usage: &syster::syntax::sysml::ast::Usage, indent: &str) {
        println!("{}Usage: {} (kind: {:?})", indent, usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()), usage.kind);
        println!("{}  typed_by: {:?}", indent, usage.relationships.typed_by);
        println!("{}  typed_by_span: {:?}", indent, usage.relationships.typed_by_span);
        for member in &usage.body {
            if let UsageMember::Usage(nested) = member {
                print_usage(nested, &format!("{}    ", indent));
            }
        }
    }
    
    fn print_element(elem: &Element, indent: &str) {
        match elem {
            Element::Package(pkg) => {
                println!("{}Package: {}", indent, pkg.name.as_ref().unwrap_or(&"<anon>".to_string()));
                for e in &pkg.elements {
                    print_element(e, &format!("{}  ", indent));
                }
            }
            Element::Usage(usage) => {
                print_usage(usage, indent);
            }
            Element::Definition(def) => {
                println!("{}Definition: {} ({:?})", indent, def.name.as_ref().unwrap_or(&"<anon>".to_string()), def.kind);
            }
            _ => {}
        }
    }
    
    for elem in &file.elements {
        print_element(elem, "");
    }
}
