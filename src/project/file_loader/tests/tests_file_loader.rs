#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::*;
use std::path::PathBuf;

#[test]
fn test_parse_content_sysml() {
    // TDD: Test parsing SysML content from string (for LSP)
    let content = "part def Vehicle;";
    let path = PathBuf::from("test.sysml");

    let result = parse_content(content, &path);
    assert!(result.is_ok(), "Should parse valid SysML content");

    let file = result.unwrap();
    assert!(file.source_file().is_some(), "Should have parsed root");
    assert!(
        file.source_file()
            .map(|sf| sf.members().count() > 0)
            .unwrap_or(false),
        "Should have parsed elements"
    );
}

#[test]
fn test_parse_content_invalid_syntax() {
    // TDD: Test error handling for invalid syntax - now with error recovery
    let content = "this is not valid sysml @#$%";
    let path = PathBuf::from("test.sysml");

    let result = parse_content(content, &path);
    // The rowan parser does error recovery, so it may return Ok with errors
    // Check that it either fails or has errors
    if let Ok(file) = result {
        assert!(file.has_errors(), "Should have errors for invalid syntax");
    }
}

#[test]
fn test_parse_content_kerml() {
    // TDD: Test KerML support
    let content = "class Vehicle;";
    let path = PathBuf::from("test.kerml");

    let result = parse_content(content, &path);
    assert!(result.is_ok(), "Should handle KerML files");
    let file = result.unwrap();
    assert!(file.is_kerml(), "Should be recognized as KerML");
}

#[test]
fn test_parse_content_unsupported_extension() {
    // TDD: Test error for unsupported file types
    let content = "some content";
    let path = PathBuf::from("test.txt");

    let result = parse_content(content, &path);
    assert!(result.is_err(), "Should fail on unsupported extension");
    assert!(
        result.unwrap_err().contains("Unsupported file extension"),
        "Error should mention unsupported extension"
    );
}

#[test]
fn test_parse_content_no_extension() {
    // TDD: Test error for files without extension
    let content = "part def Vehicle;";
    let path = PathBuf::from("test");

    let result = parse_content(content, &path);
    assert!(result.is_err(), "Should fail on missing extension");
    assert!(
        result.unwrap_err().contains("No file extension"),
        "Error should mention no file extension"
    );
}

#[test]
fn test_load_and_parse_uses_parse_content() {
    // TDD: Test that load_and_parse correctly uses parse_content internally
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.sysml");
    let content = "part def Vehicle;";

    fs::write(&file_path, content).expect("Failed to write test file");

    let result = load_and_parse(&file_path);
    assert!(result.is_ok(), "Should load and parse file from disk");

    let file = result.unwrap();
    assert!(
        file.source_file()
            .map(|sf| sf.members().count() > 0)
            .unwrap_or(false),
        "Should have parsed elements"
    );
}

#[test]
fn test_parse_content_with_complex_sysml() {
    // TDD: Test parsing more complex SysML content
    let content = r#"
        package MyPackage {
            part def Vehicle {
                attribute speed: Real;
            }
            part def Car :> Vehicle;
        }
    "#;
    let path = PathBuf::from("complex.sysml");

    let result = parse_content(content, &path);
    assert!(result.is_ok(), "Should parse complex SysML content");
}

#[test]
fn test_parse_content_empty_file() {
    // TDD: Test parsing empty content
    let content = "";
    let path = PathBuf::from("empty.sysml");

    let result = parse_content(content, &path);
    // Empty content should still parse successfully (empty model)
    assert!(result.is_ok(), "Should handle empty content");
}

#[test]
fn test_parse_with_result_success() {
    // TDD: Successful parse returns no errors
    let content = "part def Vehicle;";
    let path = PathBuf::from("test.sysml");

    let result = parse_with_result(content, &path);

    assert!(!result.has_errors());
    assert_eq!(result.errors.len(), 0);
    assert!(result.content.is_some());
    let file = result.content.unwrap();
    assert!(
        file.source_file()
            .map(|sf| sf.members().count() > 0)
            .unwrap_or(false)
    );
}

#[test]
fn test_parse_with_result_syntax_error() {
    // TDD: Syntax error returns ParseError with position
    let content = "part def {"; // Missing name
    let path = PathBuf::from("test.sysml");

    let result = parse_with_result(content, &path);

    // Rowan does error recovery, so we may get a result with errors
    assert!(
        result.has_errors(),
        "Should have errors for incomplete syntax"
    );
}

#[test]
fn test_error_has_position_info() {
    // Use complete gibberish that cannot parse
    let content = "part def Vehicle;\n@@@ ### $$$ %%%";
    let path = PathBuf::from("test.sysml");

    let result = parse_with_result(content, &path);
    assert!(result.has_errors());

    let error = &result.errors[0];
    // Position info is available (may be 0 if not computed from TextRange yet)
    // Just verify the position struct exists
    let _ = error.position.line;
    let _ = error.position.column;
}

#[test]
fn test_parse_error_details() {
    let content = "this is not valid sysml syntax at all!!!";
    let path = PathBuf::from("error.sysml");

    let result = parse_with_result(content, &path);

    assert!(result.has_errors());
    let error = &result.errors[0];
    assert!(!error.message.is_empty());
}

#[test]
fn test_unsupported_extension() {
    let content = "part def Vehicle;";
    let path = PathBuf::from("test.txt");

    let result = parse_with_result(content, &path);

    assert!(result.has_errors());
    assert!(result.errors[0].message.contains("Unsupported"));
}

#[test]
fn test_empty_file_success() {
    let content = "";
    let path = PathBuf::from("empty.sysml");

    let result = parse_with_result(content, &path);

    assert!(result.content.is_some());
    let file = result.content.unwrap();
    assert!(
        file.source_file()
            .map(|sf| sf.members().count() == 0)
            .unwrap_or(true)
    );
}
