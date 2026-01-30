//! Recursive descent parser for SysML v2
//!
//! Builds a rowan GreenNode tree from tokens.
//! Supports error recovery and produces a lossless CST.

use super::lexer::{Lexer, Token};
use super::syntax_kind::SyntaxKind;
use super::grammar::kerml_expressions::{self, ExpressionParser};
use super::grammar::kerml::KerMLParser;
use super::grammar::sysml::SysMLParser;
use rowan::{GreenNode, GreenNodeBuilder, TextRange, TextSize};

/// Parse result containing the green tree and any errors
#[derive(Debug, Clone)]
pub struct Parse {
    pub green: GreenNode,
    pub errors: Vec<SyntaxError>,
}

impl Parse {
    /// Get the root syntax node
    pub fn syntax(&self) -> super::SyntaxNode {
        super::SyntaxNode::new_root(self.green.clone())
    }
    
    /// Check if parsing succeeded without errors
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
pub fn parse_sysml(input: &str) -> Parse {
    let tokens: Vec<_> = Lexer::new(input).collect();
    let mut parser = Parser::new(&tokens, input);
    super::grammar::sysml::parse_sysml_file(&mut parser);
    parser.finish()
}

/// Parse KerML source code into a CST
pub fn parse_kerml(input: &str) -> Parse {
    let tokens: Vec<_> = Lexer::new(input).collect();
    let mut parser = Parser::new(&tokens, input);
    super::grammar::kerml::parse_kerml_file(&mut parser);
    parser.finish()
}

/// Convert a SyntaxKind to a human-readable name for error messages
pub fn kind_to_name(kind: SyntaxKind) -> &'static str {
    match kind {
        // Trivia
        SyntaxKind::WHITESPACE => "whitespace",
        SyntaxKind::LINE_COMMENT => "comment",
        SyntaxKind::BLOCK_COMMENT => "comment",
        
        // Literals
        SyntaxKind::IDENT => "identifier",
        SyntaxKind::INTEGER => "integer",
        SyntaxKind::DECIMAL => "number",
        SyntaxKind::STRING => "string",
        SyntaxKind::ERROR => "error",
        
        // Punctuation
        SyntaxKind::SEMICOLON => "';'",
        SyntaxKind::COLON => "':'",
        SyntaxKind::COLON_COLON => "'::'",
        SyntaxKind::COLON_GT => "':>'",
        SyntaxKind::COLON_GT_GT => "':>>'",
        SyntaxKind::COLON_COLON_GT => "'::>'",
        SyntaxKind::COMMA => "','",
        SyntaxKind::DOT => "'.'",
        SyntaxKind::DOT_DOT => "'..'",
        SyntaxKind::L_PAREN => "'('",
        SyntaxKind::R_PAREN => "')'",
        SyntaxKind::L_BRACE => "'{'",
        SyntaxKind::R_BRACE => "'}'",
        SyntaxKind::L_BRACKET => "'['",
        SyntaxKind::R_BRACKET => "']'",
        SyntaxKind::LT => "'<'",
        SyntaxKind::GT => "'>'",
        SyntaxKind::LT_EQ => "'<='",
        SyntaxKind::GT_EQ => "'>='",
        SyntaxKind::EQ => "'='",
        SyntaxKind::EQ_EQ => "'=='",
        SyntaxKind::EQ_EQ_EQ => "'==='",
        SyntaxKind::BANG_EQ => "'!='",
        SyntaxKind::BANG_EQ_EQ => "'!=='",
        SyntaxKind::COLON_EQ => "':='",
        SyntaxKind::PLUS => "'+'",
        SyntaxKind::MINUS => "'-'",
        SyntaxKind::STAR => "'*'",
        SyntaxKind::STAR_STAR => "'**'",
        SyntaxKind::SLASH => "'/'",
        SyntaxKind::PERCENT => "'%'",
        SyntaxKind::CARET => "'^'",
        SyntaxKind::TILDE => "'~'",
        SyntaxKind::AMP => "'&'",
        SyntaxKind::AMP_AMP => "'&&'",
        SyntaxKind::PIPE => "'|'",
        SyntaxKind::PIPE_PIPE => "'||'",
        SyntaxKind::AT => "'@'",
        SyntaxKind::AT_AT => "'@@'",
        SyntaxKind::HASH => "'#'",
        SyntaxKind::QUESTION => "'?'",
        SyntaxKind::QUESTION_QUESTION => "'??'",
        SyntaxKind::BANG => "'!'",
        SyntaxKind::ARROW => "'->'",
        SyntaxKind::FAT_ARROW => "'=>'",
        SyntaxKind::DOLLAR => "'$'",
        
        // Common keywords - extract text from the enum variant name
        _ => {
            // For keywords, try to extract a user-friendly name
            let debug_str = format!("{:?}", kind);
            if debug_str.ends_with("_KW") {
                // It's a keyword - return as-is from debug, user will see the raw keyword
                return "keyword";
            }
            "token"
        }
    }
}

/// Check if parser debug logging is enabled
fn debug_enabled() -> bool {
    std::env::var("SYSTER_PARSER_DEBUG").is_ok()
}

/// The parser state
struct Parser<'a> {
    tokens: &'a [Token<'a>],
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
    source: &'a str,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token<'a>], source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            source,
            depth: 0,
        }
    }
    
    /// Log a debug message with indentation based on parse depth
    fn log(&self, msg: &str) {
        if debug_enabled() {
            let indent = "  ".repeat(self.depth);
            let token_info = if let Some(t) = self.current() {
                format!("{:?} '{}'", t.kind, t.text.chars().take(20).collect::<String>())
            } else {
                "EOF".to_string()
            };
            eprintln!("{}[PARSER] {} | token: {}", indent, msg, token_info);
        }
    }

    fn finish(self) -> Parse {
        Parse {
            green: self.builder.finish(),
            errors: self.errors,
        }
    }

    // =========================================================================
    // Token inspection
    // =========================================================================

    fn current(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos)
    }

    fn current_kind(&self) -> SyntaxKind {
        self.current().map(|t| t.kind).unwrap_or(SyntaxKind::ERROR)
    }

    fn current_text(&self) -> &str {
        self.current().map(|t| t.text).unwrap_or("")
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current_kind() == kind
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.contains(&self.current_kind())
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn nth(&self, n: usize) -> SyntaxKind {
        // Look ahead, skipping trivia
        let mut idx = self.pos;
        let mut count = 0;
        while idx < self.tokens.len() {
            if !self.tokens[idx].kind.is_trivia() {
                if count == n {
                    return self.tokens[idx].kind;
                }
                count += 1;
            }
            idx += 1;
        }
        SyntaxKind::ERROR
    }

    // =========================================================================
    // Token consumption
    // =========================================================================

    fn bump(&mut self) {
        if let Some(token) = self.current() {
            self.builder.token(token.kind.into(), token.text);
            self.pos += 1;
        }
    }

    fn bump_any(&mut self) {
        self.bump();
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
            let expected = kind_to_name(kind);
            let found = self.current()
                .map(|t| kind_to_name(t.kind))
                .unwrap_or("end of file");
            self.error(format!("expected {}, found {}", expected, found));
            false
        }
    }

    fn skip_trivia(&mut self) {
        while self.current().map(|t| t.kind.is_trivia()).unwrap_or(false) {
            self.bump();
        }
    }

    /// Skip only whitespace (preserves comments)
    fn skip_whitespace_only(&mut self) {
        while self.at(SyntaxKind::WHITESPACE) {
            self.bump();
        }
    }

    // =========================================================================
    // Error handling
    // =========================================================================

    fn error(&mut self, message: impl Into<String>) {
        let range = self.current()
            .map(|t| TextRange::at(t.offset, TextSize::of(t.text)))
            .unwrap_or_else(|| TextRange::empty(TextSize::new(0)));
        self.errors.push(SyntaxError::new(message, range));
    }

    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]) {
        self.error(message);
        self.builder.start_node(SyntaxKind::ERROR.into());
        // Always consume at least one token to make progress
        let mut consumed = false;
        while !self.at_eof() && !self.at_any(recovery) {
            self.bump_any();
            consumed = true;
        }
        // If we didn't consume anything and we're not at EOF, consume one token
        // to prevent infinite loops
        if !consumed && !self.at_eof() {
            self.bump_any();
        }
        self.builder.finish_node();
    }

    // =========================================================================
    // Node building helpers
    // =========================================================================

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }
}

// =============================================================================
// Trait Implementations for Grammar Modules
// =============================================================================

/// Implement ExpressionParser trait to allow grammar modules to work with Parser
impl<'a> ExpressionParser for Parser<'a> {
    fn current_kind(&self) -> SyntaxKind {
        Parser::current_kind(self)
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        Parser::at(self, kind)
    }

    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        Parser::at_any(self, kinds)
    }

    fn at_name_token(&self) -> bool {
        // In SysML/KerML, certain keywords can be used as identifiers in context
        // (contextual keywords). This includes names like "start", "end", "done" etc.
        // which are common member names in action definitions.
        matches!(self.current_kind(), 
            SyntaxKind::IDENT |
            SyntaxKind::START_KW |
            SyntaxKind::END_KW |
            SyntaxKind::DONE_KW
        )
    }

    fn get_pos(&self) -> usize {
        self.pos
    }

    fn peek_kind(&self, n: usize) -> SyntaxKind {
        self.nth(n)
    }

    fn bump(&mut self) {
        Parser::bump(self)
    }

    fn bump_any(&mut self) {
        Parser::bump_any(self)
    }

    fn expect(&mut self, kind: SyntaxKind) {
        Parser::expect(self, kind);
    }

    fn skip_trivia(&mut self) {
        Parser::skip_trivia(self)
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        Parser::start_node(self, kind)
    }

    fn finish_node(&mut self) {
        Parser::finish_node(self)
    }

    fn parse_qualified_name(&mut self) {
        super::grammar::kerml::parse_qualified_name(self, &[])
    }

    fn parse_argument(&mut self) {
        kerml_expressions::parse_argument(self)
    }
}

/// Implement KerMLParser trait for kerml grammar module
impl<'a> KerMLParser for Parser<'a> {
    fn current_token_text(&self) -> Option<&str> {
        self.current().map(|t| t.text)
    }

    fn parse_identification(&mut self) {
        super::grammar::kerml::parse_identification(self)
    }

    fn parse_body(&mut self) {
        super::grammar::kerml::parse_body(self)
    }

    fn skip_trivia_except_block_comments(&mut self) {
        while self.current().map(|t| t.kind == SyntaxKind::WHITESPACE || t.kind == SyntaxKind::LINE_COMMENT).unwrap_or(false) {
            self.bump();
        }
    }

    fn parse_qualified_name_list(&mut self) {
        super::grammar::kerml::parse_qualified_name(self, &[]);
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.skip_trivia();
            super::grammar::kerml::parse_qualified_name(self, &[]);
        }
    }

    fn parse_package(&mut self) {
        super::grammar::kerml::parse_package(self)
    }

    fn parse_library_package(&mut self) {
        super::grammar::kerml::parse_library_package(self)
    }

    fn parse_import(&mut self) {
        super::grammar::kerml::parse_import(self)
    }

    fn parse_alias(&mut self) {
        super::grammar::kerml::parse_alias(self)
    }

    fn parse_definition(&mut self) {
        super::grammar::kerml::parse_definition_impl(self)
    }

    fn parse_usage(&mut self) {
        super::grammar::kerml::parse_usage_impl(self)
    }

    fn parse_invariant(&mut self) {
        super::grammar::kerml::parse_invariant(self)
    }

    fn parse_parameter(&mut self) {
        super::grammar::kerml::parse_parameter_impl(self)
    }

    fn parse_end_feature_or_parameter(&mut self) {
        super::grammar::kerml::parse_end_feature_or_parameter(self)
    }

    fn parse_connector_usage(&mut self) {
        super::grammar::kerml::parse_connector_usage(self)
    }

    fn parse_flow_usage(&mut self) {
        super::grammar::kerml::parse_flow_usage(self)
    }

    fn error(&mut self, message: impl Into<String>) {
        Parser::error(self, message)
    }

    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]) {
        Parser::error_recover(self, message, recovery)
    }
}

/// Implement SysMLParser trait for sysml grammar module
impl<'a> SysMLParser for Parser<'a> {
    // -----------------------------------------------------------------
    // Core parsing methods
    // -----------------------------------------------------------------
    
    fn current_token_text(&self) -> Option<&str> {
        self.current().map(|t| t.text)
    }

    fn parse_identification(&mut self) {
        super::grammar::kerml::parse_identification(self)
    }

    fn parse_body(&mut self) {
        super::grammar::sysml::parse_body(self)
    }

    fn skip_trivia_except_block_comments(&mut self) {
        while self.current().map(|t| t.kind == SyntaxKind::WHITESPACE || t.kind == SyntaxKind::LINE_COMMENT).unwrap_or(false) {
            self.bump();
        }
    }

    fn parse_qualified_name_list(&mut self) {
        super::grammar::kerml::parse_qualified_name(self, &[]);
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.skip_trivia();
            super::grammar::kerml::parse_qualified_name(self, &[]);
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        Parser::error(self, message)
    }

    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]) {
        Parser::error_recover(self, message, recovery)
    }

    // -----------------------------------------------------------------
    // SysML-specific methods
    // -----------------------------------------------------------------
    
    fn can_start_expression(&self) -> bool {
        matches!(self.current_kind(),
            // Literals
            SyntaxKind::INTEGER | SyntaxKind::DECIMAL | SyntaxKind::STRING |
            SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NULL_KW |
            // Expression starters
            SyntaxKind::NEW_KW | SyntaxKind::L_BRACE | SyntaxKind::L_PAREN |
            SyntaxKind::IF_KW | SyntaxKind::IDENT | SyntaxKind::THIS_KW |
            // Unary prefix operators
            SyntaxKind::NOT_KW | SyntaxKind::MINUS | SyntaxKind::PLUS |
            SyntaxKind::TILDE | SyntaxKind::BANG |
            // Type classification operators (prefix form)
            SyntaxKind::HASTYPE_KW | SyntaxKind::ISTYPE_KW | SyntaxKind::ALL_KW |
            // Metadata access
            SyntaxKind::AT
        )
    }

    fn parse_typing(&mut self) {
        super::grammar::kerml::parse_typing(self)
    }

    fn parse_multiplicity(&mut self) {
        super::grammar::kerml::parse_multiplicity(self)
    }

    fn parse_constraint_body(&mut self) {
        super::grammar::sysml::parse_constraint_body(self)
    }

    fn parse_definition_or_usage(&mut self) {
        super::grammar::sysml::parse_definition_or_usage(self)
    }

    fn parse_dependency(&mut self) {
        super::grammar::sysml::parse_dependency(self)
    }

    fn parse_filter(&mut self) {
        super::grammar::sysml::parse_filter(self)
    }

    fn parse_metadata_usage(&mut self) {
        super::grammar::sysml::parse_metadata_usage(self)
    }

    fn parse_connect_usage(&mut self) {
        super::grammar::sysml::parse_connect_usage(self)
    }

    fn parse_binding_or_succession(&mut self) {
        super::grammar::sysml::parse_binding_or_succession(self)
    }

    fn parse_variant_usage(&mut self) {
        super::grammar::sysml::parse_variant_usage(self)
    }

    fn parse_redefines_feature_member(&mut self) {
        super::grammar::sysml::parse_redefines_feature_member(self)
    }

    fn parse_shorthand_feature_member(&mut self) {
        super::grammar::sysml::parse_shorthand_feature_member(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_sysml("");
        assert!(result.ok());
    }

    #[test]
    fn test_parse_simple_package() {
        let result = parse_sysml("package Test;");
        assert!(result.ok(), "errors: {:?}", result.errors);
        
        let root = result.syntax();
        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
    }

    #[test]
    fn test_parse_package_with_body() {
        let result = parse_sysml("package Vehicle { part def Engine; }");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_import() {
        let result = parse_sysml("import ISQ::*;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_import_with_filter() {
        let result = parse_sysml("import Library::*[@MyFilter];");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_part_definition() {
        let result = parse_sysml("part def Vehicle :> Base;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_part_usage() {
        let result = parse_sysml("part engine : Engine;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_nested() {
        let source = r#"
            package Vehicle {
                part def Engine {
                    attribute power : Real;
                }
                part engine : Engine;
            }
        "#;
        let result = parse_sysml(source);
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_attribute_with_default() {
        let result = parse_sysml("attribute x : Integer = 42;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_attribute_with_expression() {
        let result = parse_sysml("attribute y : Real = 3.14 + 2.0;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_multiplicity() {
        let result = parse_sysml("part engines[2..*] : Engine;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_function_invocation() {
        let result = parse_sysml("calc result = compute(x, y);");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_conditional_expression() {
        let result = parse_sysml("attribute flag : Boolean = x > 0 ? true : false;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_comment_about() {
        let source = r#"
            package Test {
                comment about Foo, Bar
                /*
                 * This is a comment about Foo and Bar
                 */
                part def Foo { }
            }
        "#;
        let result = parse_sysml(source);
        assert!(result.ok(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_if_expression() {
        let result = parse_sysml("attribute x = if a ? 1 else 0;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_parse_nested_if_expression() {
        let result = parse_sysml("attribute x = if a ? 1 else if b ? 2 else 0;");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
}
