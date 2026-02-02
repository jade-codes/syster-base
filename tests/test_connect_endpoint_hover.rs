//! Test for connect endpoint hover
//!
//! Tests hover resolution for connect statement endpoints.

use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn setup() -> (AnalysisHost, syster::base::FileId, String) {
    let mut host = AnalysisHost::new();
    let stdlib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library");
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }

    let file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml");
    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();
    let _ = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    (host, file_id, content)
}

#[test]
fn test_line_970_shankPort_first_occurrence() {
    let (mut host, file_id, content) = setup();
    let analysis = host.analysis();

    let lines: Vec<&str> = content.lines().collect();
    let line_idx = 969; // 0-indexed - this is 1-indexed line 970
    let line_content = lines[line_idx];

    // Find first 'shankPort' (the one before ::>)
    let col = line_content.find("shankPort").expect("shankPort not found");

    let hover = analysis.hover(file_id, line_idx as u32, col as u32);

    assert!(
        hover.is_some(),
        "Line {}: Hover should resolve 'shankPort' at col {}\nLine: {}",
        line_idx + 1,
        col,
        line_content
    );
}

#[test]
fn test_line_970_shankPort_second_occurrence() {
    let (mut host, file_id, content) = setup();
    let analysis = host.analysis();

    let lines: Vec<&str> = content.lines().collect();
    let line_idx = 969; // 0-indexed - this is 1-indexed line 970
    let line_content = lines[line_idx];

    // Find second 'shankPort' (after the dot in shankCompositePort.shankPort)
    let first_pos = line_content.find("shankPort").unwrap();
    let second_pos = line_content[first_pos + 1..]
        .find("shankPort")
        .map(|p| p + first_pos + 1);
    let col = second_pos.expect("second shankPort not found");

    let hover = analysis.hover(file_id, line_idx as u32, col as u32);

    assert!(
        hover.is_some(),
        "Line {}: Hover should resolve 'shankPort' (second occurrence) at col {}\nLine: {}",
        line_idx + 1,
        col,
        line_content
    );
}
