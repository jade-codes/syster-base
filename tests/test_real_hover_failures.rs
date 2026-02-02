//! Tests for REAL hover failures from SimpleVehicleModel.sysml
//!
//! These tests use the actual file and actual positions that fail.
//! Each test MUST fail initially, then we fix and it passes.

#![allow(non_snake_case)]
#![allow(clippy::drop_non_drop)]

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn vehicle_model_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml")
}

fn create_host_with_vehicle_model() -> (AnalysisHost, syster::FileId, String) {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }

    let file_path = vehicle_model_path();
    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();
    let _ = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");
    drop(analysis);

    (host, file_id, content)
}

/// Find column of identifier in a line (0-indexed)
fn find_col(line_content: &str, ident: &str, occurrence: usize) -> u32 {
    let mut start = 0;
    for i in 0..occurrence {
        if let Some(pos) = line_content[start..].find(ident) {
            start += pos + if i < occurrence - 1 { ident.len() } else { 0 };
        } else {
            panic!(
                "Could not find occurrence {} of '{}' in '{}'",
                occurrence, ident, line_content
            );
        }
    }
    start as u32
}

// =============================================================================
// CHAIN_MEMBER failures (20 total)
// =============================================================================

/// Line 781: message from driver.turnVehicleOn to vehicle.trigger1
/// The chain members `turnVehicleOn` and `trigger1` fail to resolve
#[test]
fn test_line_781_message_chain_member_turnVehicleOn() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    // Line 781 (0-indexed: 780): message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;
    let line = 780; // 0-indexed
    let line_content = lines[line as usize];
    let col = find_col(line_content, "turnVehicleOn", 1);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 781: Hover should resolve 'turnVehicleOn' in 'from driver.turnVehicleOn'\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}

#[test]
fn test_line_781_message_chain_member_trigger1() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    let line = 780;
    let line_content = lines[line as usize];
    let col = find_col(line_content, "trigger1", 1);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 781: Hover should resolve 'trigger1' in 'to vehicle.trigger1'\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}

/// Line 817: Deep flow chain - speedSensor.speedSensorPort.sensedSpeedSent
#[test]
fn test_line_817_deep_chain_sensedSpeedSent() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    let line = 816; // 0-indexed
    let line_content = lines[line as usize];
    let col = find_col(line_content, "sensedSpeedSent", 1);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 817: Hover should resolve 'sensedSpeedSent' in deep chain\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}

// =============================================================================
// CHAIN_FIRST failures (7 total)
// =============================================================================

/// Line 840: event occurrence setSpeedReceived=setSpeedPort.setSpeedReceived
/// The RHS chain starting with setSpeedPort fails
#[test]
fn test_line_840_event_short_chain() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    let line = 839; // 0-indexed
    let line_content = lines[line as usize];
    // Find the SECOND occurrence of setSpeedReceived (the one after the dot)
    let col = find_col(line_content, "setSpeedReceived", 2);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 840: Hover should resolve 'setSpeedReceived' in 'setSpeedPort.setSpeedReceived'\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}

/// Line 968: connect [1] lugNutCompositePort ::> wheel1.lugNutCompositePort
/// The first lugNutCompositePort (before ::>) fails
#[test]
fn test_line_968_connect_endpoint_name() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    let line = 967; // 0-indexed
    let line_content = lines[line as usize];
    // First occurrence: the endpoint name before ::>
    let col = find_col(line_content, "lugNutCompositePort", 1);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 968: Hover should resolve 'lugNutCompositePort' (endpoint name)\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}

// =============================================================================
// OTHER failures (1 total)
// =============================================================================

/// Line 893: status = StatusKind::closed
/// The enum member 'closed' fails to resolve
#[test]
fn test_line_893_enum_member_access() {
    let (mut host, file_id, content) = create_host_with_vehicle_model();
    let lines: Vec<&str> = content.lines().collect();

    let line = 892; // 0-indexed
    let line_content = lines[line as usize];
    let col = find_col(line_content, "closed", 1);

    let analysis = host.analysis();
    let hover = analysis.hover(file_id, line, col);

    assert!(
        hover.is_some(),
        "Line 893: Hover should resolve 'closed' in 'StatusKind::closed'\n\
         Line content: {}\n\
         Position: line={}, col={}",
        line_content,
        line + 1,
        col
    );
}
