//! Test for bind RHS second chain member hover resolution
//! Line 714: bind rearAxleAssembly.rearWheel1.wheelToRoadPort=vehicleToRoadPort.wheelToRoadPort1;
//! This test validates that the second part of the RHS chain (wheelToRoadPort1) resolves.

use std::path::PathBuf;
use syster::ide::AnalysisHost;
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

/// Test hover on wheelToRoadPort1 at line 714 of SimpleVehicleModel.sysml
/// This is a regression test for the fix to use parent scope in chain resolution.
#[test]
fn test_simple_vehicle_model_line_714_wheel_to_road_port1() {
    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../language-server/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml");
    
    let source = std::fs::read_to_string(&file_path)
        .expect("Failed to read SimpleVehicleModel.sysml");
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", &source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    // Line 714 (1-indexed) = line 713 (0-indexed)
    // wheelToRoadPort1 starts at column 87
    let hover = analysis.hover(file_id, 713, 87);
    
    assert!(
        hover.as_ref()
            .and_then(|h| h.qualified_name.as_ref())
            .map(|qn| qn.contains("wheelToRoadPort1"))
            .unwrap_or(false),
        "hover on 'wheelToRoadPort1' at line 714 col 87 should resolve to a symbol containing 'wheelToRoadPort1'"
    );
}
