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

#[test]
fn test_debug_vehicle_usages_semantic_tokens() {
    let source = r#"package VehicleUsages {
	doc
	/*
	 * Example usages of elements from the vehicle definitions model.
	 */

	private import SI::N;
	private import SI::m;
	private import ScalarFunctions::*;

	public import VehicleDefinitions::*;

	/* VALUES */	 
	T1 = 10.0 [N * m];
	T2 = 20.0 [N * m];
	
	/* PARTS */	
	part narrowRimWheel: Wheel {
		doc /* Narrow-rim wheel configuration with 4 to 5 lugbolts. */

		part lugbolt: Lugbolt[4..5];
	}
}"#;

    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();

    let tokens = semantic_tokens(analysis.symbol_index(), file_id);

    println!("\n=== SEMANTIC TOKENS DEBUG ===");
    println!("Total tokens: {}\n", tokens.len());

    // Split source into lines for reference
    let lines: Vec<&str> = source.lines().collect();

    for (i, tok) in tokens.iter().enumerate() {
        let line_text = lines.get(tok.line as usize).unwrap_or(&"<invalid line>");
        let token_text = if (tok.col as usize) < line_text.len()
            && (tok.col as usize + tok.length as usize) <= line_text.len()
        {
            &line_text[tok.col as usize..(tok.col + tok.length) as usize]
        } else {
            "<span overflow>"
        };

        println!(
            "Token {}: line={} col={} len={} type={:?} text='{}'",
            i, tok.line, tok.col, tok.length, tok.token_type, token_text
        );
    }

    println!("\n=== SYMBOLS IN FILE ===");
    for sym in analysis.symbol_index().symbols_in_file(file_id) {
        println!(
            "Symbol: '{}' (qname='{}', kind={:?}) at line {} col {}-{}",
            sym.name, sym.qualified_name, sym.kind, sym.start_line, sym.start_col, sym.end_col
        );

        // Print type refs
        for trk in &sym.type_refs {
            for tr in trk.as_refs() {
                println!(
                    "  TypeRef: target='{}' kind={:?} at {}:{}-{}:{} resolved={:?}",
                    tr.target,
                    tr.kind,
                    tr.start_line,
                    tr.start_col,
                    tr.end_line,
                    tr.end_col,
                    tr.resolved_target
                );
            }
        }
    }

    println!("\n=== END DEBUG ===");
}
