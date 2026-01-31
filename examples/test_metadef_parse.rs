use syster::parser::parse_sysml;

fn main() {
    let source = r#"package Test {
    #Safety {
        attribute isMandatory : Boolean;
    }
}"#;
    let parsed = parse_sysml(source);
    println!("Errors: {:?}", parsed.errors);
    println!("\nTree:\n{:#?}", parsed.syntax());
}
