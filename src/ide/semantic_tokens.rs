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
            | SymbolKind::FlowUsage => TokenType::Property,
            // Other types
            SymbolKind::Alias => TokenType::Variable,
            SymbolKind::Import => TokenType::Namespace,
            SymbolKind::Comment => TokenType::Comment,
            SymbolKind::Dependency => TokenType::Variable,
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
        // Token for the symbol name itself
        // Use the actual span width rather than name.len() since the name may differ
        // from the source (e.g., stripped quotes) and the span is authoritative
        let length = if symbol.end_col > symbol.start_col {
            symbol.end_col - symbol.start_col
        } else {
            symbol.name.len() as u32
        };

        // Skip symbols with invalid spans (start_col=0 but end_col > 0 suggests bad span data)
        // Valid tokens at col 0 should have consistent span data where end_col = start_col + name_len
        // Tokens at (0, 0) with varying end_cols are bogus - they're likely from symbols
        // whose spans weren't properly set during parsing
        let is_valid_span =
            symbol.start_col > 0 || (symbol.start_col == 0 && symbol.end_col == length);

        if is_valid_span {
            tokens.push(SemanticToken {
                line: symbol.start_line,
                col: symbol.start_col,
                length,
                token_type: TokenType::from(symbol.kind),
            });
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
}
