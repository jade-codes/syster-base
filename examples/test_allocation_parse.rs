use syster::parser::parse_sysml;

fn main() {
    let source = r#"package Test {
    allocation vehicleLogicalToPhysicalAllocation : LogicalToPhysical
        allocate vehicleLogical to vehicle_b {
            allocate vehicleLogical.torqueGenerator to vehicle_b.engine;
        }
}"#;
    let parsed = parse_sysml(source);
    println!("Errors: {:?}", parsed.errors);
    println!("\nTree:\n{:#?}", parsed.syntax());
}
