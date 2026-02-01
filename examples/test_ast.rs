use syster::parser::parse_sysml;

fn main() {
    let source = r#"package P {
    port p {
        event occurrence a;
        then event b.sourceEvent;
    }
}"#;
    let parsed = parse_sysml(source);
    println!("{:#?}", parsed.syntax());
}
