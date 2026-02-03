//! Semantic tokens — syntax highlighting based on semantic analysis.
//!
//! This module provides semantic token extraction directly from the HIR layer,
//! without depending on the legacy semantic layer.

use crate::base::FileId;
use crate::hir::{SymbolIndex, SymbolKind};

/// Token type for semantic highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Namespace,
    Type,
    Variable,
    Property,
    Keyword,
    Comment,
}

impl TokenType {
    /// Convert to LSP token type index.
    pub fn to_lsp_index(self) -> u32 {
        match self {
            TokenType::Namespace => 0,
            TokenType::Type => 1,
            TokenType::Variable => 2,
            TokenType::Property => 3,
            TokenType::Keyword => 4,
            TokenType::Comment => 5,
        }
    }
}

impl From<SymbolKind> for TokenType {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::Package => TokenType::Namespace,
            // All definition types
            SymbolKind::PartDef
            | SymbolKind::ItemDef
            | SymbolKind::ActionDef
            | SymbolKind::PortDef
            | SymbolKind::AttributeDef
            | SymbolKind::ConnectionDef
            | SymbolKind::InterfaceDef
            | SymbolKind::AllocationDef
            | SymbolKind::RequirementDef
            | SymbolKind::ConstraintDef
            | SymbolKind::StateDef
            | SymbolKind::CalculationDef
            | SymbolKind::UseCaseDef
            | SymbolKind::AnalysisCaseDef
            | SymbolKind::ConcernDef
            | SymbolKind::ViewDef
            | SymbolKind::ViewpointDef
            | SymbolKind::RenderingDef
            | SymbolKind::EnumerationDef
            | SymbolKind::MetaclassDef
            | SymbolKind::InteractionDef => TokenType::Type,
            // All usage types
            SymbolKind::PartUsage
            | SymbolKind::ItemUsage
            | SymbolKind::ActionUsage
            | SymbolKind::PortUsage
            | SymbolKind::AttributeUsage
            | SymbolKind::ConnectionUsage
            | SymbolKind::InterfaceUsage
            | SymbolKind::AllocationUsage
            | SymbolKind::RequirementUsage
            | SymbolKind::ConstraintUsage
            | SymbolKind::StateUsage
            | SymbolKind::CalculationUsage
            | SymbolKind::ReferenceUsage
            | SymbolKind::OccurrenceUsage
            | SymbolKind::FlowUsage
            | SymbolKind::ViewUsage
            | SymbolKind::ViewpointUsage
            | SymbolKind::RenderingUsage => TokenType::Property,
            // Other types
            SymbolKind::Alias => TokenType::Variable,
            SymbolKind::Import => TokenType::Namespace,
            SymbolKind::Comment => TokenType::Comment,
            SymbolKind::Dependency => TokenType::Variable,
            SymbolKind::ExposeRelationship => TokenType::Variable,
            SymbolKind::Other => TokenType::Variable,
        }
    }
}

/// A semantic token for syntax highlighting.
#[derive(Debug, Clone)]
pub struct SemanticToken {
    /// Line number (0-indexed)
    pub line: u32,
    /// Column number (0-indexed)
    pub col: u32,
    /// Length of the token in characters
    pub length: u32,
    /// The token type
    pub token_type: TokenType,
}

/// Get semantic tokens for a file.
///
/// Uses the symbol index to generate tokens for symbol definitions and type references.
///
/// # Arguments
///
/// * `index` - The symbol index containing all symbols
/// * `file` - The file to get tokens for
///
/// # Returns
///
/// Vector of semantic tokens sorted by position.
pub fn semantic_tokens(index: &SymbolIndex, file: FileId) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();

    // Add tokens for all symbols in this file
    for symbol in index.symbols_in_file(file) {
        // Skip anonymous/synthetic symbols (names like `<:>>cyl#8@L7>`)
        // These are generated names for anonymous usages and shouldn't be highlighted
        if symbol.name.starts_with('<') {
            // Still process type_refs for anonymous symbols
        } else {
            // Calculate token length:
            // - For single-line symbols, use the actual span (end_col - start_col)
            //   This correctly handles quoted names like 'My Name' where the source
            //   includes quotes but symbol.name doesn't
            // - For multi-line symbols (shouldn't happen for names), fall back to name.len()
            let length =
                if symbol.start_line == symbol.end_line && symbol.end_col > symbol.start_col {
                    symbol.end_col - symbol.start_col
                } else {
                    symbol.name.len() as u32
                };

            // Skip symbols with invalid spans or at position (0,0) unless they're truly at the start
            let is_valid_span = symbol.start_col > 0
                || (symbol.start_col == 0 && symbol.start_line == 0 && length > 0);

            if is_valid_span {
                tokens.push(SemanticToken {
                    line: symbol.start_line,
                    col: symbol.start_col,
                    length,
                    token_type: TokenType::from(symbol.kind),
                });
            }
        }

        // Tokens for type references (the types in `:>` or `:` relationships)
        for type_ref_kind in &symbol.type_refs {
            for type_ref in type_ref_kind.as_refs() {
                // Skip type_refs with invalid spans (same logic as symbols)
                let ref_length = (type_ref.end_col - type_ref.start_col).max(1);
                let is_valid_ref = type_ref.start_col > 0
                    || (type_ref.start_col == 0 && type_ref.end_col == ref_length);

                if is_valid_ref {
                    tokens.push(SemanticToken {
                        line: type_ref.start_line,
                        col: type_ref.start_col,
                        length: ref_length,
                        token_type: TokenType::Type,
                    });
                }
            }
        }
    }

    // Sort tokens by position (line, then column)
    tokens.sort_by_key(|t| (t.line, t.col));

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::FileId;
    use crate::hir::{SymbolIndex, extract_symbols_unified};
    use crate::syntax::parser::parse_content;

    fn build_index_from_source(source: &str) -> SymbolIndex {
        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();
        let symbols = extract_symbols_unified(FileId(1), &syntax);

        let mut index = SymbolIndex::new();
        index.add_file(FileId(1), symbols);
        index
    }

    #[test]
    fn test_semantic_tokens_package_positions() {
        let source = r#"package VehicleIndividuals {
	package IndividualDefinitions {
	}
}"#;
        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("All tokens (sorted by position):");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Check for any tokens at position (0,0) which might indicate a bug
        let zero_pos_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.line == 0 && t.col == 0)
            .collect();
        if !zero_pos_tokens.is_empty() {
            println!("WARNING: Found tokens at (0,0):");
            for tok in &zero_pos_tokens {
                println!("  len={} type={:?}", tok.length, tok.token_type);
            }
        }

        // Should have 2 tokens: one for each package
        assert_eq!(tokens.len(), 2, "Should have 2 package tokens");

        // First token: "VehicleIndividuals" at line 0, col 8, len 18
        let tok1 = &tokens[0];
        assert_eq!(tok1.line, 0);
        assert_eq!(tok1.col, 8, "VehicleIndividuals should start at col 8");
        assert_eq!(tok1.length, 18, "VehicleIndividuals has 18 chars");
        assert_eq!(tok1.token_type, TokenType::Namespace);

        // Second token: "IndividualDefinitions" at line 1, col 9, len 21
        // (after tab and "package ")
        let tok2 = &tokens[1];
        assert_eq!(tok2.line, 1);
        assert_eq!(tok2.col, 9, "IndividualDefinitions should start at col 9");
        assert_eq!(tok2.length, 21, "IndividualDefinitions has 21 chars");
        assert_eq!(tok2.token_type, TokenType::Namespace);
    }

    #[test]
    fn test_semantic_tokens_stdlib_requirements() {
        // Test with actual stdlib-like content
        let source = r#"standard library package Requirements {
	private import Base::Anything;
	
	private abstract constraint def RequirementConstraintCheck {
	}
}"#;
        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Requirements.sysml tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Check for any suspicious tokens at (0,0)
        let zero_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.line == 0 && t.col == 0)
            .collect();
        assert!(
            zero_tokens.is_empty(),
            "Should not have tokens at (0,0), found: {:?}",
            zero_tokens
        );

        // The package name "Requirements" should be at the correct position
        // "standard library package Requirements" - "Requirements" starts at col 25
        let pkg_token = tokens.iter().find(|t| t.token_type == TokenType::Namespace);
        assert!(
            pkg_token.is_some(),
            "Should have a Namespace token for Requirements"
        );
        let pkg_token = pkg_token.unwrap();
        println!(
            "Package token: line={} col={} len={}",
            pkg_token.line, pkg_token.col, pkg_token.length
        );

        // "standard library package " = 25 chars, then "Requirements" = 12 chars
        assert_eq!(pkg_token.col, 25, "Requirements should start at col 25");
        assert_eq!(pkg_token.length, 12, "Requirements has 12 chars");
    }

    #[test]
    fn test_quoted_name_span_starts_at_quote() {
        // Test that quoted names like 'Chassis Assembly' have correct span
        // The span should start at the opening quote, not before it
        let source = "part def 'Chassis Assembly';";
        //            0123456789...
        //            "part def " = 9 chars, then 'Chassis Assembly' starts at col 9

        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Quoted name tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Should have 1 token for the part def
        assert_eq!(tokens.len(), 1, "Should have 1 token for part def");

        let tok = &tokens[0];
        assert_eq!(tok.line, 0);
        // "part def " = 9 chars, quoted name starts at col 9
        assert_eq!(
            tok.col, 9,
            "Quoted name 'Chassis Assembly' should start at col 9 (at the quote)"
        );
        // The name without quotes is "Chassis Assembly" = 16 chars
        // But source includes quotes, so highlighting should cover the quoted portion
        assert_eq!(tok.token_type, TokenType::Type);
    }

    #[test]
    fn test_import_span_is_path_only() {
        // Test that import symbols have span covering only the path, not the whole statement
        let source = "public import KeyWord_MetadataDefinitions::*;";
        //            0123456789...
        //            "public import " = 14 chars, then path starts at col 14

        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Import tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Should have 1 token for the import (plus possibly the type ref)
        assert!(!tokens.is_empty(), "Should have at least 1 token");

        // Find the Namespace token (imports are typed as Namespace)
        let import_tok = tokens.iter().find(|t| t.token_type == TokenType::Namespace);
        assert!(import_tok.is_some(), "Should have import token");

        let tok = import_tok.unwrap();
        assert_eq!(tok.line, 0);
        // "public import " = 14 chars, path starts at col 14
        assert_eq!(
            tok.col, 14,
            "Import path should start at col 14, not at 'public'"
        );
    }

    #[test]
    fn test_redefines_multiplicity_not_highlighted() {
        // Test that multiplicity like [6..8] is NOT highlighted when following a redefines target
        // The issue: `part redefines cyl[6..8]` - only `cyl` should be highlighted, not `6..8`
        // Root cause: anonymous usages get synthetic names like `<:>>cyl#8@L7>` which have
        // length 13, and the span starts at `cyl`, so the token covered `cyl[6..8]`.
        let source = r#"part def Vehicle {
    part eng {
        part cyl[6..8];
    }
}
part def SportsCar :> Vehicle {
    part redefines eng {
        part redefines cyl[6..8];
    }
}"#;

        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Redefines with multiplicity tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Find tokens on line 7 (the `part redefines cyl[6..8]` line, 0-indexed)
        let line7_tokens: Vec<_> = tokens.iter().filter(|t| t.line == 7).collect();

        println!("Line 7 tokens: {:?}", line7_tokens);

        // There should NOT be a Property token for the anonymous usage
        // (anonymous symbols with names like `<:>>cyl#8@L7>` should be skipped)
        let property_tokens: Vec<_> = line7_tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Property)
            .collect();
        assert!(
            property_tokens.is_empty(),
            "Should NOT have Property token for anonymous usage, found: {:?}",
            property_tokens
        );

        // There should be a Type token for `cyl` (the redefines target) with length 3
        let type_tokens: Vec<_> = line7_tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Type)
            .collect();
        assert_eq!(type_tokens.len(), 1, "Should have 1 Type token for `cyl`");
        assert_eq!(
            type_tokens[0].length, 3,
            "Type token should be 3 chars for `cyl`"
        );
    }

    #[test]
    fn test_alias_span_is_name_only() {
        // Test that alias symbols have span covering only the name, not the whole statement
        let source = "alias QuantityValue for TensorQuantityValue;";
        //            0123456789...
        //            "alias " = 6 chars, then "QuantityValue" starts at col 6

        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Alias tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Find the Variable token (aliases are typed as Variable)
        let alias_tok = tokens.iter().find(|t| t.token_type == TokenType::Variable);
        assert!(alias_tok.is_some(), "Should have alias token");

        let tok = alias_tok.unwrap();
        assert_eq!(tok.line, 0);
        // "alias " = 6 chars, name starts at col 6
        assert_eq!(tok.col, 6, "Alias name should start at col 6");
        // "QuantityValue" = 13 chars
        assert_eq!(tok.length, 13, "Alias name should be 13 chars");
    }

    #[test]
    fn test_quoted_name_includes_quotes_in_span() {
        // Test that quoted names like 'vehicle model 1' include quotes in the span
        let source = "part 'vehicle model 1' :> vehicle;";
        //            01234567890123456789012345
        //            "part " = 5 chars, then 'vehicle model 1' starts at col 5

        let index = build_index_from_source(source);
        let tokens = semantic_tokens(&index, FileId(1));

        println!("Quoted name tokens:");
        for tok in &tokens {
            println!(
                "  line={} col={} len={} type={:?}",
                tok.line, tok.col, tok.length, tok.token_type
            );
        }

        // Find the Property token for the part usage
        let part_tok = tokens.iter().find(|t| t.token_type == TokenType::Property);
        assert!(part_tok.is_some(), "Should have part usage token");

        let tok = part_tok.unwrap();
        assert_eq!(tok.line, 0);
        assert_eq!(tok.col, 5, "Quoted name should start at col 5");
        // 'vehicle model 1' with quotes = 17 chars
        assert_eq!(
            tok.length, 17,
            "Quoted name should be 17 chars (including quotes)"
        );
    }
}
