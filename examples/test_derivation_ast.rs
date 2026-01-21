use syster::syntax::sysml::ast::{parse_file, DefinitionMember, UsageMember, Element};
use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"package Test {
    #derivation connection {
        end #original ::> vehicleSpecification.vehicleMassRequirement;
        end #derive ::> engineSpecification.engineMassRequirement;
    }
}"#;
    
    println!("=== AST TEST: #derivation connection with end ::> ===\n");
    println!("SOURCE:\n{}\n", source);
    
    let mut pairs = SysMLParser::parse(Rule::file, source).expect("Parse should succeed");
    let file = parse_file(&mut pairs).expect("AST parse should succeed");
    
    println!("AST ELEMENTS:");
    for elem in &file.elements {
        print_element(elem, 0);
    }
}

fn print_element(elem: &Element, indent: usize) {
    let prefix = "  ".repeat(indent);
    match elem {
        Element::Package(pkg) => {
            println!("{}Package: {}", prefix, pkg.name.as_ref().unwrap_or(&"<unnamed>".to_string()));
            for e in &pkg.elements {
                print_element(e, indent + 1);
            }
        }
        Element::Definition(def) => {
            println!("{}Definition: {} (kind: {:?})", prefix, def.name.as_ref().unwrap_or(&"<unnamed>".to_string()), def.kind);
            for m in &def.body {
                print_def_member(m, indent + 1);
            }
        }
        Element::Usage(usage) => {
            print_usage(usage, indent);
        }
        _ => {}
    }
}

fn print_def_member(member: &DefinitionMember, indent: usize) {
    let prefix = "  ".repeat(indent);
    match member {
        DefinitionMember::Usage(usage) => {
            print_usage(usage, indent);
        }
        DefinitionMember::Comment(c) => {
            println!("{}Comment", prefix);
        }
        DefinitionMember::Import(i) => {
            println!("{}Import: {}", prefix, i.path);
        }
    }
}

fn print_usage(usage: &syster::syntax::sysml::ast::Usage, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!("{}Usage: {} (kind: {:?})", prefix, usage.name.as_ref().unwrap_or(&"<unnamed>".to_string()), usage.kind);
    println!("{}  subsets: {:?}", prefix, usage.relationships.subsets);
    println!("{}  references: {:?}", prefix, usage.relationships.references);
    println!("{}  typed_by: {:?}", prefix, usage.relationships.typed_by);
    println!("{}  expression_refs: {:?}", prefix, usage.expression_refs);
    for m in &usage.body {
        print_usage_member(m, indent + 1);
    }
}

fn print_usage_member(member: &UsageMember, indent: usize) {
    match member {
        UsageMember::Usage(usage) => {
            print_usage(usage, indent);
        }
        UsageMember::Comment(_) => {}
    }
}
