use syster::ide::AnalysisHost;

fn main() {
    let mut host = AnalysisHost::new();
    let code = r#"package Test {
    part def Controller;
    part def MySignal;
    part def OtherSignal;
    
    state def VehicleState {
        entry; then on;
        
        state on;
        state off;
        
        transition on_To_off
            first on
            accept MySignal
            do send new OtherSignal() to controller
            then off;
            
        ref part controller : Controller;
    }
}"#;
    host.set_file_content("/test.sysml", code);

    let analysis = host.analysis();

    // Check for specific type refs we're looking for
    let mut found_my_signal = false;
    let mut found_other_signal = false;

    for sym in analysis.symbol_index().all_symbols() {
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                if &*tr.target == "MySignal"
                    && tr.resolved_target.as_deref() == Some("Test::MySignal")
                {
                    found_my_signal = true;
                }
                if &*tr.target == "OtherSignal"
                    && tr.resolved_target.as_deref() == Some("Test::OtherSignal")
                {
                    found_other_signal = true;
                }
            }
        }
    }

    println!(
        "MySignal from 'accept MySignal' resolved: {}",
        found_my_signal
    );
    println!(
        "OtherSignal from 'send new OtherSignal()' resolved: {}",
        found_other_signal
    );

    if found_my_signal && found_other_signal {
        println!("\n✓ Both accept and send type refs are now working!");
    } else {
        println!("\n✗ Some type refs are still missing");
    }
}
