use syster::parser::parse_sysml;

fn main() {
    let source = r#"package Comments {
    comment cmt /* Named Comment */
    comment cmt_cmt about cmt /* Comment about Comment */
}"#;
    let parsed = parse_sysml(source);
    println!("Errors: {:?}", parsed.errors);
    println!("\nTree:\n{:#?}", parsed.syntax());
}
