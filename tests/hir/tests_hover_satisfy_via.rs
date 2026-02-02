//! Tests for hover improvements - satisfy by target and accept via extraction

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::hir::TypeRefKind;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

mod satisfy_by_target {
    use super::*;

    /// Test: hover on 'by' target in satisfy statement
    #[test]
    fn test_satisfy_by_target_hover() {
        let source = r#"
package Test {
    part def Vehicle {
        attribute mass;
    }
    
    requirement def VehicleSpec {
        attribute massActual;
    }
    
    package VehicleConfig {
        part vehicle_b : Vehicle;
        
        satisfy VehicleSpec by vehicle_b {
            attribute massActual redefines massActual = vehicle_b.mass;
        }
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();
        
        // Find the satisfy symbol
        let satisfy_sym = analysis.symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .find(|s| s.name.contains("satisfy"))
            .expect("satisfy symbol should exist");
        
        // Verify the 'by' target type_ref exists
        let has_by_target = satisfy_sym.type_refs.iter().any(|tr: &TypeRefKind| {
            tr.as_refs().iter().any(|r| r.target.as_ref() == "vehicle_b")
        });
        assert!(has_by_target, "satisfy symbol should have vehicle_b as type_ref");
        
        // Line: satisfy VehicleSpec by vehicle_b {
        // Line 13 (0-indexed), vehicle_b starts around col 31
        let line = 13;
        
        // Test hover at a position that should hit vehicle_b
        let vehicle_b_ref = satisfy_sym.type_refs.iter()
            .flat_map(|tr: &TypeRefKind| tr.as_refs())
            .find(|r| r.target.as_ref() == "vehicle_b" && r.start_line == line as u32);
        
        if let Some(ref_) = vehicle_b_ref {
            let hover = analysis.hover(file_id, ref_.start_line, ref_.start_col + 1);
            assert!(hover.is_some(), "hover on 'vehicle_b' by-target should resolve");
            let qn = hover.unwrap().qualified_name;
            assert!(qn.as_ref().map(|s| s.contains("vehicle_b")).unwrap_or(false),
                "hover should resolve to vehicle_b, got {:?}", qn);
        } else {
            panic!("Could not find vehicle_b type_ref on line 13");
        }
    }
}

mod accept_via_target {
    use super::*;

    /// Test: hover on 'via' target in standalone accept action
    #[test]
    fn test_accept_via_standalone() {
        let source = r#"
package Test {
    port def CmdPort;
    item def IgnitionCmd;
    
    action def StartEngine {
        in item ignitionPort : CmdPort;
        
        accept ignitionCmd : IgnitionCmd via ignitionPort;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();
        
        // Find the accept action symbol
        let accept_sym = analysis.symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .find(|s| s.name.contains("ignitionCmd"))
            .expect("accept action symbol should exist");
        
        // Verify the 'via' target type_ref exists
        let has_via_target = accept_sym.type_refs.iter().any(|tr: &TypeRefKind| {
            tr.as_refs().iter().any(|r| r.target.as_ref() == "ignitionPort")
        });
        assert!(has_via_target, "accept symbol should have ignitionPort as type_ref");
        
        // Find the via ref and test hover
        let via_ref = accept_sym.type_refs.iter()
            .flat_map(|tr: &TypeRefKind| tr.as_refs())
            .find(|r| r.target.as_ref() == "ignitionPort");
        
        if let Some(ref_) = via_ref {
            let hover = analysis.hover(file_id, ref_.start_line, ref_.start_col + 1);
            assert!(hover.is_some(), "hover on via target should resolve");
            let qn = hover.unwrap().qualified_name;
            assert!(qn.as_ref().map(|s| s.contains("ignitionPort")).unwrap_or(false),
                "hover should resolve to ignitionPort, got {:?}", qn);
        }
    }

    /// Test: hover on 'via' target in transition accept
    #[test]
    fn test_accept_via_in_transition() {
        let source = r#"
package Test {
    port def CmdPort;
    item def IgnitionCmd;
    
    state def EngineStates {
        in item ignitionCmdPort : CmdPort;
        
        state off;
        state starting;
        
        transition off_To_starting
            first off
            accept ignitionCmd : IgnitionCmd via ignitionCmdPort
            then starting;
    }
}
"#;

        let mut host = create_host_with_stdlib();
        host.set_file_content("test.sysml", source);
        let analysis = host.analysis();
        let file_id = analysis.get_file_id("test.sysml").unwrap();
        
        // Find the accept payload symbol (inside transition)
        let accept_sym = analysis.symbol_index()
            .symbols_in_file(file_id)
            .into_iter()
            .find(|s| s.name.as_ref() == "ignitionCmd")
            .expect("accept payload symbol should exist");
        
        // Verify the 'via' target type_ref exists
        let has_via_target = accept_sym.type_refs.iter().any(|tr: &TypeRefKind| {
            tr.as_refs().iter().any(|r| r.target.as_ref() == "ignitionCmdPort")
        });
        assert!(has_via_target, "accept symbol should have ignitionCmdPort as type_ref from via clause");
        
        // Find the via ref and test hover
        let via_ref = accept_sym.type_refs.iter()
            .flat_map(|tr: &TypeRefKind| tr.as_refs())
            .find(|r| r.target.as_ref() == "ignitionCmdPort");
        
        if let Some(ref_) = via_ref {
            let hover = analysis.hover(file_id, ref_.start_line, ref_.start_col + 1);
            assert!(hover.is_some(), "hover on transition via target should resolve");
            let qn = hover.unwrap().qualified_name;
            assert!(qn.as_ref().map(|s| s.contains("ignitionCmdPort")).unwrap_or(false),
                "hover should resolve to ignitionCmdPort, got {:?}", qn);
        }
    }
}
