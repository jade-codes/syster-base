use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();

    let flows_source = include_str!("../sysml.library/Systems Library/Flows.sysml");
    host.set_file_content("stdlib/Flows.sysml", flows_source);

    let source = include_str!("../tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    host.set_file_content("test.sysml", source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();

    // Line 840: then event sendSensedSpeed.targetEvent;
    println!("Line 840: then event sendSensedSpeed.targetEvent;");
    let line = source.lines().nth(840).unwrap();
    println!("Actual: {}", line);
    
    // Test hover at various columns
    for col in [20u32, 30, 40, 50, 55, 60, 65, 70, 75] {
        let hover = analysis.hover(file_id, 840, col);
        println!("  col {:2}: {:?}", col, hover.as_ref().map(|h| h.qualified_name.as_ref()));
    }
}
