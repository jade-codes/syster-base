//! Parser Tests - Views and Metadata
//!
//! Phase 1: Parser/AST Layer
//! Tests for views, viewpoints, renderings, and metadata.
//!
//! Test data from tests_parser_sysml_pest.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, SourceFile, parse_sysml};

fn parses_sysml(input: &str) -> bool {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).is_some()
}

// ============================================================================
// View Definitions
// ============================================================================

#[rstest]
#[case("view def MyView;")]
#[case("view def MyView {}")]
#[case("view def MyView { expose MyElement; }")]
fn test_view_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// View Usages
// ============================================================================

#[rstest]
#[case("package P { view myView; }")]
#[case("package P { view myView { } }")]
fn test_view_usage(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Viewpoint Definitions
// ============================================================================

#[rstest]
#[case("viewpoint def MyViewpoint;")]
#[case("viewpoint def MyViewpoint {}")]
fn test_viewpoint_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Rendering Definitions
// ============================================================================

#[rstest]
#[case("rendering def MyRendering;")]
#[case("rendering def MyRendering {}")]
fn test_rendering_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Metadata Definitions
// ============================================================================

#[rstest]
#[case("metadata def MyMetadata;")]
#[case("metadata def MyMetadata {}")]
fn test_metadata_def(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Metadata Usage (annotations)
// ============================================================================

#[rstest]
#[case("#MyMeta part def P;")]
#[case("@MyAnnotation part def P;")]
fn test_metadata_annotations(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Expose
// ============================================================================

#[rstest]
#[case("view def V { expose MyElement; }")]
#[case("view def V { expose MyElement::member; }")]
#[case("view def V { expose MyNamespace::*; }")]
fn test_expose(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Filter
// ============================================================================

#[rstest]
#[case("view def V { filter @MyMeta; }")]
fn test_filter(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}

// ============================================================================
// Render
// ============================================================================

#[rstest]
#[case("view def V { render myRendering; }")]
fn test_render(#[case] input: &str) {
    assert!(parses_sysml(input), "Failed to parse: {}", input);
}
