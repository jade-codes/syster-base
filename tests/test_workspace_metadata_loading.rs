//! Tests for automatic metadata loading in workspaces.
//!
//! Verifies that when metadata files (*.metadata, meta.json) are present
//! alongside SysML files, they are automatically detected and applied
//! to restore element IDs.

#[cfg(feature = "interchange")]
#[test]
fn test_workspace_loader_detects_and_applies_metadata() {
    use syster::ide::AnalysisHost;
    use syster::project::WorkspaceLoader;
    use syster::interchange::metadata::{ImportMetadata, ElementMeta};
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory with SysML file and metadata
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path();

    // Write a simple SysML file
    let sysml_content = r#"
package TestWorkspace {
    part def Vehicle;
    part def Car :> Vehicle;
}
"#;
    fs::write(workspace_path.join("model.sysml"), sysml_content).unwrap();

    // Write companion metadata file with element IDs
    let mut metadata = ImportMetadata::new();
    metadata.add_element("TestWorkspace", ElementMeta::with_id("xmi-workspace-001"));
    metadata.add_element("TestWorkspace::Vehicle", ElementMeta::with_id("xmi-vehicle-001"));
    metadata.add_element("TestWorkspace::Car", ElementMeta::with_id("xmi-car-001"));
    
    metadata.write_to_file(workspace_path.join("model.sysml.metadata")).unwrap();

    // Load workspace with WorkspaceLoader
    let mut host = AnalysisHost::new();
    let loader = WorkspaceLoader::new();
    
    // Load SysML files
    loader.load_directory_into_host(workspace_path, &mut host)
        .expect("Should load SysML files");
    
    // Load metadata files
    loader.load_metadata_from_directory(workspace_path, &mut host)
        .expect("Should load metadata files");

    // Verify element IDs were applied
    let analysis = host.analysis();
    
    let workspace = analysis.symbol_index().lookup_qualified("TestWorkspace")
        .expect("Should find TestWorkspace");
    assert_eq!(workspace.element_id.as_ref(), "xmi-workspace-001",
        "Package should have XMI element_id from metadata");

    let vehicle = analysis.symbol_index().lookup_qualified("TestWorkspace::Vehicle")
        .expect("Should find Vehicle");
    assert_eq!(vehicle.element_id.as_ref(), "xmi-vehicle-001",
        "Vehicle should have XMI element_id from metadata");

    let car = analysis.symbol_index().lookup_qualified("TestWorkspace::Car")
        .expect("Should find Car");
    assert_eq!(car.element_id.as_ref(), "xmi-car-001",
        "Car should have XMI element_id from metadata");
}

#[cfg(feature = "interchange")]
#[test]
fn test_workspace_loader_handles_missing_metadata_gracefully() {
    use syster::ide::AnalysisHost;
    use syster::project::WorkspaceLoader;
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory with only SysML file (no metadata)
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path();

    let sysml_content = r#"
package NoMetadata {
    part def Part1;
}
"#;
    fs::write(workspace_path.join("model.sysml"), sysml_content).unwrap();

    // Load workspace without metadata
    let mut host = AnalysisHost::new();
    let loader = WorkspaceLoader::new();
    
    loader.load_directory_into_host(workspace_path, &mut host)
        .expect("Should load SysML files");
    
    // This should not fail even without metadata files
    let result = loader.load_metadata_from_directory(workspace_path, &mut host);
    
    // Should succeed (no metadata to load is OK)
    assert!(result.is_ok(), "Should handle missing metadata gracefully");

    // Symbols should still exist with auto-generated UUIDs
    let analysis = host.analysis();
    let pkg = analysis.symbol_index().lookup_qualified("NoMetadata");
    assert!(pkg.is_some(), "Package should exist even without metadata");
}

#[cfg(feature = "interchange")]
#[test]
fn test_workspace_loader_finds_meta_json() {
    use syster::ide::AnalysisHost;
    use syster::project::WorkspaceLoader;
    use syster::interchange::metadata::{ImportMetadata, ElementMeta};
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path();

    // Write SysML file
    let sysml_content = r#"
package MyPackage {
    part def Component;
}
"#;
    fs::write(workspace_path.join("model.sysml"), sysml_content).unwrap();

    // Write meta.json (KPAR-style metadata)
    let mut metadata = ImportMetadata::new();
    metadata.add_element("MyPackage", ElementMeta::with_id("kpar-pkg-1"));
    metadata.add_element("MyPackage::Component", ElementMeta::with_id("kpar-comp-1"));
    
    metadata.write_to_file(workspace_path.join("meta.json")).unwrap();

    // Load workspace
    let mut host = AnalysisHost::new();
    let loader = WorkspaceLoader::new();
    
    loader.load_directory_into_host(workspace_path, &mut host).unwrap();
    loader.load_metadata_from_directory(workspace_path, &mut host).unwrap();

    // Verify meta.json was loaded
    let analysis = host.analysis();
    
    let component = analysis.symbol_index().lookup_qualified("MyPackage::Component")
        .expect("Should find Component");
    assert_eq!(component.element_id.as_ref(), "kpar-comp-1",
        "Should apply element_id from meta.json");
}
