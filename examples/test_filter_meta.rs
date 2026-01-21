use syster::parser::sysml::{SysMLParser, Rule};
use pest::Parser;

fn main() {
    let source = r#"
view def PartsTreeView {
    filter @SysML::PartUsage;
}
"#;
    
    let pairs = SysMLParser::parse(Rule::view_definition, source.trim()).unwrap();
    let pair = pairs.into_iter().next().unwrap();
    
    let def = syster::syntax::sysml::ast::parse_definition(pair).unwrap();
    
    println!("Definition name: {:?}", def.name);
    println!("Meta relationships: {:?}", def.relationships.meta);
    println!("Body members: {:?}", def.body);
}
