use syster::parser::parse_sysml;

fn main() {
    let source = r#"package P {
    view v {
        satisfy requirement sv:SafetyViewpoint;
    }
}"#;
    let parsed = parse_sysml(source);
    println!("{:#?}", parsed.syntax());
}
