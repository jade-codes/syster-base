use syster::analyzer::sysml::SysmlAnalyzer;

fn main() {
    let input = r#"
part def Vehicle {
    port ignitionCmdPort;
    part engine {
        port ignitionCmdPort;
    }
}
part vehicle : Vehicle {
    bind engine.ignitionCmdPort=ignitionCmdPort;
}
    "#;
    
    let analyzer = SysmlAnalyzer::default();
    let ast = analyzer.parse_str(input).unwrap();
    
    // Print all references from expression_refs
    println!("=== References in AST ===");
    for elem in ast.elements.iter() {
        println!("\n{:?}:", elem);
        fn print_refs(elem: &syster::syntax::sysml::ast::Element, indent: usize) {
            let indent_str = "  ".repeat(indent);
            match elem {
                syster::syntax::sysml::ast::Element::Usage(u) => {
                    println!("{}Usage: {} (kind: {:?})", indent_str, u.name.as_ref().unwrap_or(&"<anon>".to_string()), u.kind);
                    println!("{}  expression_refs: {:?}", indent_str, u.expression_refs);
                    println!("{}  rels: {:?}", indent_str, u.relationships);
                    for child in &u.children {
                        print_refs(child, indent + 1);
                    }
                }
                syster::syntax::sysml::ast::Element::Definition(d) => {
                    println!("{}Definition: {} (kind: {:?})", indent_str, d.name.as_ref().unwrap_or(&"<anon>".to_string()), d.kind);
                    for child in &d.children {
                        print_refs(child, indent + 1);
                    }
                }
                _ => {}
            }
        }
        print_refs(elem, 0);
    }
}
