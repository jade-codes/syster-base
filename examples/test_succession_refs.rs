use syster::syntax::sysml::ast::parsers::parse_usage;
use syster::parser::sysml::{Rule, SysMLParser};
use pest::Parser;

fn main() {
    let input = "first driverGetInVehicle then join1;";
    println!("=== 'first driverGetInVehicle then join1;' ===");
    let pair = SysMLParser::parse(Rule::succession_as_usage, input).unwrap().next().unwrap();
    let usage = parse_usage(pair);
    
    println!("Usage name: {:?}", usage.name);
    println!("Usage kind: {:?}", usage.kind);
    println!("Expression refs: {:?}", usage.expression_refs);
}
