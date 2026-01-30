/// Test for supporting multiple packages per file
///
/// This test verifies that SysML files can contain multiple package declarations
/// and that all packages are properly tracked as elements.
use std::path::Path;
use syster::syntax::sysml::parser::parse_content;

#[test]
fn test_multiple_packages_in_single_file() {
    let input = r#"
        package Vehicle;
        package Engine;
        package Transmission;
    "#;

    let result = parse_content(input, Path::new("test.sysml"));
    assert!(result.is_ok(), "Parse should succeed");

    let sysml_file = result.unwrap();

    // All packages should be in elements
    assert_eq!(
        sysml_file.elements.len(),
        3,
        "Should find all 3 package elements"
    );
}

#[test]
fn test_single_package() {
    let input = "package SinglePackage;";

    let result = parse_content(input, Path::new("test.sysml"));
    assert!(result.is_ok());

    let sysml_file = result.unwrap();

    // Single package should be in elements
    assert_eq!(sysml_file.elements.len(), 1);
}

#[test]
fn test_no_packages() {
    let input = "part myPart;";

    let result = parse_content(input, Path::new("test.sysml"));
    assert!(result.is_ok());

    let sysml_file = result.unwrap();

    // Part should be in elements
    assert_eq!(sysml_file.elements.len(), 1);
}
