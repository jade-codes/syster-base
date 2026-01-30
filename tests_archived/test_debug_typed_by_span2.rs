use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};
use syster::syntax::sysml::ast::{Element, UsageMember, parse_file};

#[test]
fn test_debug_typed_by_span2() {
    // Test case with perform action containing feature chains
    let source = r#"package VehicleActions {
    action def ProvidePower {
        action distributeTorque;
    }
    
    state def VehicleStates {
        entry; then idle;
        
        state idle;
        state on {
            // Test the action reference via typing
            do action performAction : ProvidePower;
        }
        
        action providePower : ProvidePower;
    }
}"#;

    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let file = parse_file(&mut pairs).expect("AST parse should succeed");

    fn print_usage(usage: &syster::syntax::sysml::ast::Usage, indent: &str) {
        let name = usage
            .name
            .as_ref()
            .unwrap_or(&"<unnamed>".to_string())
            .clone();
        println!("{}Usage: {} (kind: {:?})", indent, name, usage.kind);

        // Print typed_by
        if let Some(typed_by) = &usage.relationships.typed_by {
            println!("{}  typed_by: {}", indent, typed_by);
        }

        // Print subsets
        for (i, subset) in usage.relationships.subsets.iter().enumerate() {
            println!(
                "{}  subsets[{}]: target='{}', chain_ctx={:?}",
                indent,
                i,
                subset.target(),
                subset.extracted.chain_context()
            );
        }

        // Print specializes
        for (i, spec) in usage.relationships.specializes.iter().enumerate() {
            println!(
                "{}  specializes[{}]: target='{}', chain_ctx={:?}",
                indent,
                i,
                spec.target(),
                spec.extracted.chain_context()
            );
        }

        // Print performs
        for (i, perf) in usage.relationships.performs.iter().enumerate() {
            println!(
                "{}  performs[{}]: target='{}', chain_ctx={:?}",
                indent,
                i,
                perf.target(),
                perf.extracted.chain_context()
            );
        }

        for member in &usage.body {
            if let UsageMember::Usage(nested) = member {
                print_usage(nested, &format!("{}    ", indent));
            }
        }
    }

    fn print_element(elem: &Element, indent: &str) {
        match elem {
            Element::Package(pkg) => {
                println!(
                    "{}Package: {}",
                    indent,
                    pkg.name.as_ref().unwrap_or(&"<anon>".to_string())
                );
                for e in &pkg.elements {
                    print_element(e, &format!("{}  ", indent));
                }
            }
            Element::Usage(usage) => {
                print_usage(usage, indent);
            }
            Element::Definition(def) => {
                let name = def.name.as_ref().unwrap_or(&"<anon>".to_string()).clone();
                println!("{}Definition: {} ({:?})", indent, name, def.kind);
                // Print definition's inner usages
                for member in &def.body {
                    if let syster::syntax::sysml::ast::enums::DefinitionMember::Usage(u) = member {
                        print_usage(u, &format!("{}  ", indent));
                    }
                }
            }
            _ => {}
        }
    }

    for elem in &file.elements {
        print_element(elem, "");
    }
}
