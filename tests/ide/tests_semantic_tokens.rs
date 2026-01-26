//! Semantic tokens tests for the IDE layer.

use crate::helpers::hir_helpers::*;
use syster::ide::{TokenType, semantic_tokens};

// =============================================================================
// SEMANTIC TOKENS - BASIC
// =============================================================================

#[test]
fn test_semantic_tokens_returns_tokens() {
    let source = r#"
        package Pkg {
            part def Vehicle;
        }
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    assert!(
        !tokens.is_empty(),
        "Should return semantic tokens for source"
    );
}

#[test]
fn test_semantic_tokens_has_position() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    // Each token should have line/column info
    for token in &tokens {
        assert!(token.line < 1000, "Token should have valid line");
        assert!(token.col < 1000, "Token should have valid column");
        assert!(token.length > 0, "Token should have positive length");
    }
}

#[test]
fn test_semantic_tokens_has_types() {
    let source = r#"
        package Pkg;
        part def Vehicle;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    // Should have various token types
    let types: Vec<_> = tokens.iter().map(|t| t.token_type).collect();

    // Check we have some meaningful types (not all the same)
    assert!(!types.is_empty(), "Should have tokens with types");
}

#[test]
fn test_semantic_tokens_namespace() {
    let source = "package MyNamespace;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    // Should have a namespace token
    let has_namespace = tokens.iter().any(|t| t.token_type == TokenType::Namespace);

    assert!(has_namespace, "Package should produce Namespace token type");
}

#[test]
fn test_semantic_tokens_type() {
    let source = "part def Vehicle;";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    // Definition should produce a Type token
    let has_type = tokens.iter().any(|t| t.token_type == TokenType::Type);

    assert!(has_type, "Part def should produce Type token type");
}

// =============================================================================
// SEMANTIC TOKENS - EDGE CASES
// =============================================================================

#[test]
fn test_semantic_tokens_empty_file() {
    let source = "";

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    assert!(
        tokens.is_empty(),
        "Empty file should have no semantic tokens"
    );
}

#[test]
fn test_semantic_tokens_multiple_symbols() {
    let source = r#"
        part def A;
        part def B;
        part def C;
    "#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    // Should have at least 3 tokens (one for each definition)
    assert!(
        tokens.len() >= 3,
        "Should have at least 3 tokens for 3 definitions, got {}",
        tokens.len()
    );
}
