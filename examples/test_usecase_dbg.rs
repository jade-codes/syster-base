use syster::parser::parse_sysml;

fn main() {
    let source = r#"use case def MyUseCase {
    item def Request;
    first start;
    then action doSomething;
    then done;
}"#;
    let parsed = parse_sysml(source);
    println!("Errors: {:?}", parsed.errors);
    println!("\nTree:\n{:#?}", parsed.syntax());
}
