//! Recursive descent parser for SysML v2 and KerML
//!
//! Builds a rowan GreenNode tree from tokens.
//! Supports error recovery and produces a lossless CST.
//! Can parse both SysML (.sysml) and KerML (.kerml) files.
//!
//! All parsing logic lives in the grammar modules:
//! - `grammar::kerml` - KerML grammar
//! - `grammar::sysml` - SysML grammar  
//! - `grammar::kerml_expressions` - Expression parsing

use super::lexer::{Lexer, Token};
use super::syntax_kind::SyntaxKind;
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

// Grammar modules for parsing
use super::grammar;
use super::grammar::kerml_expressions::{self, ExpressionParser};

/// Language mode for parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageMode {
    SysML,
    KerML,
}

/// Parse result containing the green tree and any errors
#[derive(Debug)]
pub struct Parse {
    pub green: GreenNode,
    pub errors: Vec<SyntaxError>,
}

impl Parse {
    pub fn syntax(&self) -> super::SyntaxNode {
        super::SyntaxNode::new_root(self.green.clone())
    }
    
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// A syntax error with location and message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    pub range: TextRange,
}

impl SyntaxError {
    pub fn new(message: impl Into<String>, range: TextRange) -> Self {
        Self {
            message: message.into(),
            range,
        }
    }
}

/// Parse SysML source code into a CST
pub fn parse(input: &str) -> Parse {
    parse_sysml(input)
}

/// Parse SysML source code into a CST
pub fn parse_sysml(input: &str) -> Parse {
    let tokens: Vec<_> = Lexer::new(input).collect();
    let mut parser = Parser::new(&tokens, LanguageMode::SysML);
    grammar::sysml::parse_sysml_file(&mut parser);
    parser.finish()
}

/// Parse KerML source code into a CST
pub fn parse_kerml(input: &str) -> Parse {
    let tokens: Vec<_> = Lexer::new(input).collect();
    let mut parser = Parser::new(&tokens, LanguageMode::KerML);
    grammar::kerml::parse_kerml_file(&mut parser);
    parser.finish()
}

/// The parser state
pub(crate) struct Parser<'a> {
    pub(crate) tokens: &'a [Token<'a>],
    pub(crate) pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token<'a>], _mode: LanguageMode) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }
    
    fn finish(self) -> Parse {
        Parse {
            green: self.builder.finish(),
            errors: self.errors,
        }
    }

    // === Token inspection ===
    
    fn current(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos)
    }

    fn current_kind(&self) -> SyntaxKind {
        self.current().map(|t| t.kind).unwrap_or(SyntaxKind::ERROR)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current_kind() == kind
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.contains(&self.current_kind())
    }

    fn at_name_token(&self) -> bool {
        if self.pos >= self.tokens.len() {
            return false;
        }
        let kind = self.current_kind();
        if kind == SyntaxKind::IDENT {
            return true;
        }
        !matches!(kind,
            SyntaxKind::ERROR | SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT |
            SyntaxKind::BLOCK_COMMENT | SyntaxKind::L_BRACE | SyntaxKind::R_BRACE |
            SyntaxKind::L_BRACKET | SyntaxKind::R_BRACKET | SyntaxKind::L_PAREN |
            SyntaxKind::R_PAREN | SyntaxKind::SEMICOLON | SyntaxKind::COLON |
            SyntaxKind::COLON_COLON | SyntaxKind::COLON_GT | SyntaxKind::COLON_GT_GT |
            SyntaxKind::COLON_COLON_GT | SyntaxKind::DOT | SyntaxKind::DOT_DOT |
            SyntaxKind::COMMA | SyntaxKind::EQ | SyntaxKind::EQ_EQ | SyntaxKind::EQ_EQ_EQ |
            SyntaxKind::BANG_EQ | SyntaxKind::BANG_EQ_EQ | SyntaxKind::LT | SyntaxKind::GT |
            SyntaxKind::LT_EQ | SyntaxKind::GT_EQ | SyntaxKind::AT | SyntaxKind::AT_AT |
            SyntaxKind::HASH | SyntaxKind::STAR | SyntaxKind::STAR_STAR | SyntaxKind::PLUS |
            SyntaxKind::MINUS | SyntaxKind::SLASH | SyntaxKind::PERCENT | SyntaxKind::CARET |
            SyntaxKind::AMP | SyntaxKind::AMP_AMP | SyntaxKind::PIPE | SyntaxKind::PIPE_PIPE |
            SyntaxKind::BANG | SyntaxKind::TILDE | SyntaxKind::QUESTION |
            SyntaxKind::QUESTION_QUESTION | SyntaxKind::ARROW | SyntaxKind::FAT_ARROW |
            SyntaxKind::INTEGER | SyntaxKind::DECIMAL | SyntaxKind::STRING
        )
    }

    // === Token consumption ===

    fn bump(&mut self) {
        if let Some(token) = self.current() {
            self.builder.token(token.kind.into(), token.text);
            self.pos += 1;
        }
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            true
        } else {
            self.error(format!("expected {:?}", kind));
            false
        }
    }

    fn skip_trivia(&mut self) {
        while self.current().map(|t| t.kind.is_trivia()).unwrap_or(false) {
            self.bump();
        }
    }

    fn skip_trivia_except_block_comments(&mut self) {
        while self.current().map(|t| matches!(t.kind, SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT)).unwrap_or(false) {
            self.bump();
        }
    }

    fn can_start_expression(&self) -> bool {
        matches!(
            self.current_kind(),
            SyntaxKind::IDENT | SyntaxKind::INTEGER | SyntaxKind::DECIMAL |
            SyntaxKind::STRING | SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW |
            SyntaxKind::NULL_KW | SyntaxKind::L_PAREN | SyntaxKind::AT |
            SyntaxKind::PLUS | SyntaxKind::MINUS | SyntaxKind::TILDE |
            SyntaxKind::NOT_KW | SyntaxKind::ALL_KW
        )
    }

    // === Error handling ===

    fn error(&mut self, message: impl Into<String>) {
        let range = self.current()
            .map(|t| TextRange::at(t.offset, TextSize::of(t.text)))
            .unwrap_or_else(|| TextRange::empty(TextSize::new(0)));
        self.errors.push(SyntaxError::new(message, range));
    }

    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]) {
        self.error(message);
        self.builder.start_node(SyntaxKind::ERROR.into());
        while self.pos < self.tokens.len() && !self.at_any(recovery) {
            self.bump();
        }
        self.builder.finish_node();
    }

    // === Node building ===

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }
}

// =============================================================================
// Trait implementations - delegate to grammar modules
// =============================================================================

impl<'a> ExpressionParser for Parser<'a> {
    fn current_kind(&self) -> SyntaxKind { self.current_kind() }
    fn at(&self, kind: SyntaxKind) -> bool { self.at(kind) }
    fn at_any(&self, kinds: &[SyntaxKind]) -> bool { self.at_any(kinds) }
    fn at_name_token(&self) -> bool { self.at_name_token() }
    fn get_pos(&self) -> usize { self.pos }
    fn bump(&mut self) { self.bump() }
    fn bump_any(&mut self) { self.bump() }
    fn expect(&mut self, kind: SyntaxKind) { self.expect(kind); }
    fn skip_trivia(&mut self) { self.skip_trivia() }
    fn start_node(&mut self, kind: SyntaxKind) { self.start_node(kind) }
    fn finish_node(&mut self) { self.finish_node() }
    
    fn parse_qualified_name(&mut self) {
        grammar::kerml::parse_qualified_name(self, &[]);
    }
    
    fn parse_argument(&mut self) {
        // Named argument check
        if self.at(SyntaxKind::IDENT) {
            let mut peek_idx = self.pos + 1;
            while peek_idx < self.tokens.len() && self.tokens[peek_idx].kind.is_trivia() {
                peek_idx += 1;
            }
            if peek_idx < self.tokens.len() && self.tokens[peek_idx].kind == SyntaxKind::EQ {
                self.bump(); // name
                self.skip_trivia();
                self.bump(); // =
                self.skip_trivia();
            }
        }
        kerml_expressions::parse_expression(self);
    }
}

impl<'a> grammar::KerMLParser for Parser<'a> {
    fn parse_identification(&mut self) { grammar::kerml::parse_identification(self) }
    fn parse_body(&mut self) { grammar::kerml::parse_body(self) }
    fn skip_trivia_except_block_comments(&mut self) { self.skip_trivia_except_block_comments() }
    fn parse_qualified_name_list(&mut self) {
        self.parse_qualified_name();
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.skip_trivia();
            self.parse_qualified_name();
        }
    }
    fn parse_package(&mut self) { grammar::kerml::parse_package(self) }
    fn parse_library_package(&mut self) { grammar::kerml::parse_library_package(self) }
    fn parse_import(&mut self) { grammar::kerml::parse_import(self) }
    fn parse_alias(&mut self) { grammar::kerml::parse_alias(self) }
    fn parse_kerml_definition(&mut self) { grammar::kerml::parse_kerml_definition(self) }
    fn parse_kerml_usage(&mut self) { grammar::kerml::parse_kerml_usage(self) }
    fn parse_kerml_parameter(&mut self) { grammar::kerml::parse_kerml_parameter(self) }
    fn parse_end_feature_or_parameter(&mut self) { grammar::kerml::parse_end_feature_or_parameter(self) }
    fn parse_connector_usage(&mut self) { grammar::kerml::parse_connector_usage(self) }
    fn parse_flow_usage(&mut self) { grammar::kerml::parse_flow_usage(self) }
    fn error(&mut self, message: impl Into<String>) { self.error(message) }
    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]) { self.error_recover(message, recovery) }
}

impl<'a> grammar::SysMLParser for Parser<'a> {
    fn can_start_expression(&self) -> bool { self.can_start_expression() }
    fn parse_typing(&mut self) { grammar::kerml::parse_typing(self) }
    fn parse_multiplicity(&mut self) { grammar::kerml::parse_multiplicity(self) }
    fn parse_constraint_body(&mut self) { grammar::sysml::parse_constraint_body(self) }
    fn parse_definition_or_usage(&mut self) { grammar::sysml::parse_definition_or_usage(self) }
    fn parse_dependency(&mut self) { grammar::sysml::parse_dependency(self) }
    fn parse_filter(&mut self) { grammar::sysml::parse_filter(self) }
    fn parse_metadata_usage(&mut self) { grammar::sysml::parse_metadata_usage(self) }
    fn parse_connect_usage(&mut self) { grammar::sysml::parse_connect_usage(self) }
    fn parse_binding_or_succession(&mut self) { grammar::sysml::parse_binding_or_succession(self) }
    fn parse_variant_usage(&mut self) { grammar::sysml::parse_variant_usage(self) }
    fn parse_redefines_feature_member(&mut self) { grammar::sysml::parse_redefines_feature_member(self) }
    fn parse_shorthand_feature_member(&mut self) { grammar::sysml::parse_shorthand_feature_member(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let parse = parse("");
        assert!(parse.ok());
    }

    #[test]
    fn test_parse_simple_package() {
        let parse = parse("package Test;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }

    #[test]
    fn test_parse_package_with_body() {
        let parse = parse("package Vehicle { part def Engine; }");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }

    #[test]
    fn test_parse_import() {
        let parse = parse("import ISQ::*;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }

    #[test]
    fn test_parse_part_definition() {
        let parse = parse("part def Vehicle :> Base;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }

    #[test]
    fn test_parse_part_usage() {
        let parse = parse("part engine : Engine;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }

    #[test]
    fn test_kerml_class_definition() {
        let parse = parse_kerml("class Vehicle { feature mass : Real; }");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
}
