use pest::Parser;
use syster::parser::sysml::{Rule, SysMLParser};
use syster::syntax::sysml::ast::{DefinitionMember, Element, UsageMember, parse_file};

#[test]
fn test_debug_extended_usage_parsing() {
    let source = r#"package Requirements {
    requirement def MassRequirement;
    
    requirement vehicleSpecification {
        requirement vehicleMassRequirement : MassRequirement;
    }
    
    requirement engineSpecification {
        requirement engineMassRequirement : MassRequirement;
    }
    
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;

    println!("\n=== SOURCE ===");
    println!("{}", source);

    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let file = parse_file(&mut pairs).expect("AST parse should succeed");

    println!("\n=== ELEMENTS ===");
    for elem in &file.elements {
        match elem {
            Element::Package(pkg) => {
                println!(
                    "Package: {}",
                    pkg.name.as_ref().unwrap_or(&"<unnamed>".to_string())
                );
                for member in &pkg.elements {
                    print_element(member, 1);
                }
            }
            Element::Definition(def) => {
                println!(
                    "Definition: {} (kind: {:?})",
                    def.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                    def.kind
                );
            }
            Element::Usage(usage) => {
                println!(
                    "Usage: {} (kind: {:?})",
                    usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                    usage.kind
                );
            }
            _ => {}
        }
    }
}

fn print_element(elem: &Element, indent: usize) {
    let prefix = "  ".repeat(indent);
    match elem {
        Element::Definition(def) => {
            println!(
                "{}Definition: {} (kind: {:?})",
                prefix,
                def.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                def.kind
            );
            for m in &def.body {
                print_definition_member(m, indent + 1);
            }
        }
        Element::Usage(usage) => {
            print_usage(usage, indent);
        }
        Element::Package(pkg) => {
            println!(
                "{}Package: {}",
                prefix,
                pkg.name.as_ref().unwrap_or(&"<unnamed>".to_string())
            );
        }
        _ => {}
    }
}

fn print_definition_member(member: &DefinitionMember, indent: usize) {
    let prefix = "  ".repeat(indent);
    match member {
        DefinitionMember::Usage(usage) => {
            print_usage(usage, indent);
        }
        DefinitionMember::Comment(c) => {
            println!("{}Comment: {:?}", prefix, c.content);
        }
        DefinitionMember::Import(i) => {
            println!("{}Import: {}", prefix, i.path);
        }
    }
}

fn print_usage(usage: &syster::syntax::sysml::ast::Usage, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!(
        "{}Usage: {} (kind: {:?})",
        prefix,
        usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
        usage.kind
    );
    println!(
        "{}  relationships.subsets: {:?}",
        prefix, usage.relationships.subsets
    );
    println!(
        "{}  relationships.typed_by: {:?}",
        prefix, usage.relationships.typed_by
    );
    println!(
        "{}  relationships.redefines: {:?}",
        prefix, usage.relationships.redefines
    );
    println!(
        "{}  relationships.references: {:?}",
        prefix, usage.relationships.references
    );
    println!("{}  expression_refs: {:?}", prefix, usage.expression_refs);
    for m in &usage.body {
        print_usage_member(m, indent + 1);
    }
}

fn print_usage_member(member: &UsageMember, indent: usize) {
    let prefix = "  ".repeat(indent);
    match member {
        UsageMember::Usage(usage) => {
            print_usage(usage, indent);
        }
        UsageMember::Comment(c) => {
            println!("{}Comment: {:?}", prefix, c.content);
        }
    }
}
