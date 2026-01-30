//! Recursive descent parser for SysML v2
//!
//! Builds a rowan GreenNode tree from tokens.
//! Supports error recovery and produces a lossless CST.

use super::lexer::{Lexer, Token};
use super::syntax_kind::SyntaxKind;
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
pub fn parse(input: &str) -> Parse {
    let tokens: Vec<_> = Lexer::new(input).collect();
    let mut parser = Parser::new(&tokens);
    parser.parse_source_file();
    parser.finish()
}

/// The parser state
struct Parser<'a> {
    tokens: &'a [Token<'a>],
    pos: usize,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<SyntaxError>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token<'a>]) -> Self {
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
            self.error(format!("expected {:?}", kind));
            false
        }
    }

    fn skip_trivia(&mut self) {
        while self.current().map(|t| t.kind.is_trivia()).unwrap_or(false) {
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

    // =========================================================================
    // Grammar rules
    // =========================================================================

    /// SourceFile = PackageMember*
    fn parse_source_file(&mut self) {
        self.start_node(SyntaxKind::SOURCE_FILE);
        
        while !self.at_eof() {
            let pos_before = self.pos;
            self.skip_trivia();
            if self.at_eof() {
                break;
            }
            self.parse_namespace_member();
            // Safety: if we didn't make progress, force-skip a token
            if self.pos == pos_before && !self.at_eof() {
                self.error(format!("stuck on token: {:?}", self.current_kind()));
                self.bump_any();
            }
        }
        
        self.finish_node();
    }

    /// NamespaceMember = visibility? (Package | Import | Alias | Definition | Usage | ...)
    fn parse_namespace_member(&mut self) {
        self.skip_trivia();
        
        // Handle visibility prefix
        let has_visibility = self.at_any(&[
            SyntaxKind::PUBLIC_KW,
            SyntaxKind::PRIVATE_KW, 
            SyntaxKind::PROTECTED_KW,
        ]);
        
        if has_visibility {
            self.bump(); // consume visibility
            self.skip_trivia();
        }

        match self.current_kind() {
            SyntaxKind::PACKAGE_KW => self.parse_package(),
            SyntaxKind::LIBRARY_KW | SyntaxKind::STANDARD_KW => self.parse_library_package(),
            SyntaxKind::IMPORT_KW => self.parse_import(),
            SyntaxKind::ALIAS_KW => self.parse_alias(),
            SyntaxKind::DEPENDENCY_KW => self.parse_dependency(),
            SyntaxKind::COMMENT_KW | SyntaxKind::DOC_KW => self.parse_annotation(),
            SyntaxKind::FILTER_KW => self.parse_filter(),
            SyntaxKind::AT => self.parse_metadata_usage(),
            
            // Definitions
            SyntaxKind::ABSTRACT_KW | SyntaxKind::VARIATION_KW => {
                self.parse_definition_or_usage();
            }
            
            SyntaxKind::PART_KW | SyntaxKind::ATTRIBUTE_KW | SyntaxKind::PORT_KW |
            SyntaxKind::ITEM_KW | SyntaxKind::ACTION_KW | SyntaxKind::STATE_KW |
            SyntaxKind::CONSTRAINT_KW | SyntaxKind::REQUIREMENT_KW | SyntaxKind::CASE_KW |
            SyntaxKind::CALC_KW | SyntaxKind::CONNECTION_KW | SyntaxKind::INTERFACE_KW |
            SyntaxKind::ALLOCATION_KW | SyntaxKind::FLOW_KW | SyntaxKind::VIEW_KW |
            SyntaxKind::VIEWPOINT_KW | SyntaxKind::RENDERING_KW | SyntaxKind::METADATA_KW |
            SyntaxKind::OCCURRENCE_KW | SyntaxKind::INDIVIDUAL_KW | SyntaxKind::ENUM_KW |
            SyntaxKind::ANALYSIS_KW | SyntaxKind::VERIFICATION_KW | SyntaxKind::USE_KW |
            SyntaxKind::CONCERN_KW | SyntaxKind::REF_KW |
            // Message/event-related usages
            SyntaxKind::MESSAGE_KW | SyntaxKind::SNAPSHOT_KW | SyntaxKind::TIMESLICE_KW |
            SyntaxKind::FRAME_KW | SyntaxKind::EVENT_KW |
            // Action body keywords
            SyntaxKind::PERFORM_KW | SyntaxKind::ACCEPT_KW | SyntaxKind::SEND_KW |
            SyntaxKind::EXHIBIT_KW | SyntaxKind::INCLUDE_KW | SyntaxKind::SATISFY_KW |
            SyntaxKind::RENDER_KW |
            // Constraint keywords
            SyntaxKind::ASSERT_KW | SyntaxKind::ASSUME_KW | SyntaxKind::REQUIRE_KW |
            // Usage prefix keywords that can start a usage
            SyntaxKind::END_KW | SyntaxKind::READONLY_KW | SyntaxKind::DERIVED_KW |
            SyntaxKind::IN_KW | SyntaxKind::OUT_KW | SyntaxKind::INOUT_KW |
            // KerML keywords
            SyntaxKind::CLASS_KW | SyntaxKind::STRUCT_KW | SyntaxKind::ASSOC_KW |
            SyntaxKind::BEHAVIOR_KW | SyntaxKind::FUNCTION_KW | SyntaxKind::PREDICATE_KW |
            SyntaxKind::INTERACTION_KW | SyntaxKind::DATATYPE_KW | SyntaxKind::FEATURE_KW |
            SyntaxKind::STEP_KW | SyntaxKind::EXPR_KW | SyntaxKind::CONNECTOR_KW |
            SyntaxKind::CLASSIFIER_KW | SyntaxKind::TYPE_KW |
            // Transition/state keywords
            SyntaxKind::TRANSITION_KW | SyntaxKind::FIRST_KW | SyntaxKind::THEN_KW |
            SyntaxKind::ENTRY_KW | SyntaxKind::DO_KW | SyntaxKind::EXIT_KW |
            SyntaxKind::IF_KW | SyntaxKind::ELSE_KW |
            // Binding keyword
            SyntaxKind::BINDING_KW => {
                self.parse_definition_or_usage();
            }
            
            _ => {
                // Unknown token - skip with error
                self.error_recover(
                    format!("unexpected token: {:?}", self.current_kind()),
                    &[SyntaxKind::PACKAGE_KW, SyntaxKind::PART_KW, SyntaxKind::R_BRACE],
                );
            }
        }
    }

    /// Package = 'package' Identification? PackageBody
    fn parse_package(&mut self) {
        self.start_node(SyntaxKind::PACKAGE);
        
        self.expect(SyntaxKind::PACKAGE_KW);
        self.skip_trivia();
        
        // Optional identification (name)
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
        }
        
        self.skip_trivia();
        self.parse_package_body();
        
        self.finish_node();
    }

    /// LibraryPackage = 'standard'? 'library' Package  
    fn parse_library_package(&mut self) {
        self.start_node(SyntaxKind::LIBRARY_PACKAGE);
        
        self.eat(SyntaxKind::STANDARD_KW);
        self.skip_trivia();
        self.expect(SyntaxKind::LIBRARY_KW);
        self.skip_trivia();
        self.expect(SyntaxKind::PACKAGE_KW);
        self.skip_trivia();
        
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
        }
        
        self.skip_trivia();
        self.parse_package_body();
        
        self.finish_node();
    }

    /// PackageBody = ';' | '{' NamespaceMember* '}'
    fn parse_package_body(&mut self) {
        self.start_node(SyntaxKind::NAMESPACE_BODY);
        
        if self.eat(SyntaxKind::SEMICOLON) {
            // Empty body
        } else if self.eat(SyntaxKind::L_BRACE) {
            self.skip_trivia();
            
            while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
                let pos_before = self.pos;
                self.parse_namespace_member();
                self.skip_trivia();
                // Safety: if we didn't make progress, force-skip a token
                if self.pos == pos_before && !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
                    self.error(format!("stuck on token: {:?}", self.current_kind()));
                    self.bump_any();
                }
            }
            
            self.expect(SyntaxKind::R_BRACE);
        } else {
            self.error("expected ';' or '{'");
        }
        
        self.finish_node();
    }

    /// Import = 'import' 'all'? ImportedMembership FilterPackage? ';'
    fn parse_import(&mut self) {
        self.start_node(SyntaxKind::IMPORT);
        
        self.expect(SyntaxKind::IMPORT_KW);
        self.skip_trivia();
        
        // Optional 'all'
        self.eat(SyntaxKind::ALL_KW);
        self.skip_trivia();
        
        // Import target (qualified name with optional ::* or ::**)
        self.parse_qualified_name();
        
        // Check for wildcard
        self.skip_trivia();
        if self.at(SyntaxKind::COLON_COLON) {
            self.bump();
            self.skip_trivia();
            if self.at(SyntaxKind::STAR_STAR) {
                // Recursive import ::**
                self.bump();
            } else if self.at(SyntaxKind::STAR) {
                // Single wildcard ::*
                self.bump();
                // Check for second star (for ** as two tokens)
                if self.at(SyntaxKind::STAR) {
                    self.bump();
                }
            }
        }
        
        // Optional filter [@Filter]
        self.skip_trivia();
        if self.at(SyntaxKind::L_BRACKET) {
            self.parse_filter_package();
        }
        
        self.skip_trivia();
        self.expect(SyntaxKind::SEMICOLON);
        
        self.finish_node();
    }

    /// FilterPackage = '[' '@' QualifiedName ']'
    fn parse_filter_package(&mut self) {
        self.start_node(SyntaxKind::FILTER_PACKAGE);
        
        self.expect(SyntaxKind::L_BRACKET);
        self.skip_trivia();
        self.expect(SyntaxKind::AT);
        self.skip_trivia();
        self.parse_qualified_name();
        self.skip_trivia();
        self.expect(SyntaxKind::R_BRACKET);
        
        self.finish_node();
    }

    /// Alias = 'alias' Identification 'for' QualifiedName ';'
    fn parse_alias(&mut self) {
        self.start_node(SyntaxKind::ALIAS_MEMBER);
        
        self.expect(SyntaxKind::ALIAS_KW);
        self.skip_trivia();
        self.parse_identification();
        self.skip_trivia();
        self.expect(SyntaxKind::FOR_KW);
        self.skip_trivia();
        self.parse_qualified_name();
        self.skip_trivia();
        self.expect(SyntaxKind::SEMICOLON);
        
        self.finish_node();
    }

    /// Dependency = 'dependency' Identification? 'from' ... 'to' ... ';'
    fn parse_dependency(&mut self) {
        self.start_node(SyntaxKind::DEPENDENCY);
        
        self.expect(SyntaxKind::DEPENDENCY_KW);
        self.skip_trivia();
        
        // Optional identification
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
            self.skip_trivia();
        }
        
        // 'from' client list
        self.expect(SyntaxKind::FROM_KW);
        self.skip_trivia();
        self.parse_qualified_name_list();
        self.skip_trivia();
        
        // 'to' supplier list
        self.expect(SyntaxKind::TO_KW);
        self.skip_trivia();
        self.parse_qualified_name_list();
        self.skip_trivia();
        
        self.expect(SyntaxKind::SEMICOLON);
        
        self.finish_node();
    }

    /// Filter = 'filter' Expression ';'
    fn parse_filter(&mut self) {
        self.start_node(SyntaxKind::ELEMENT_FILTER_MEMBER);
        
        self.expect(SyntaxKind::FILTER_KW);
        self.skip_trivia();
        
        // Parse the filter expression (typically @MetadataType)
        if self.at(SyntaxKind::AT) {
            self.bump();
            self.skip_trivia();
            self.parse_qualified_name();
        } else {
            self.parse_expression();
        }
        
        self.skip_trivia();
        self.expect(SyntaxKind::SEMICOLON);
        
        self.finish_node();
    }

    /// Annotation = 'comment' | 'doc' ...
    fn parse_annotation(&mut self) {
        self.start_node(SyntaxKind::COMMENT_ELEMENT);
        
        if self.at(SyntaxKind::COMMENT_KW) {
            self.bump();
        } else if self.at(SyntaxKind::DOC_KW) {
            self.bump();
        }
        
        self.skip_trivia();
        
        // Optional identification
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
            self.skip_trivia();
        }
        
        // Optional 'about' targets
        if self.at(SyntaxKind::ABOUT_KW) {
            self.bump();
            self.skip_trivia();
            self.parse_qualified_name_list();
            self.skip_trivia();
        }
        
        // Body or string content
        if self.at(SyntaxKind::L_BRACE) {
            // Block comment
            self.parse_annotation_body();
        } else {
            self.expect(SyntaxKind::SEMICOLON);
        }
        
        self.finish_node();
    }

    fn parse_annotation_body(&mut self) {
        self.expect(SyntaxKind::L_BRACE);
        self.skip_trivia();
        
        // Consume content until closing brace
        while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
            self.bump_any();
        }
        
        self.expect(SyntaxKind::R_BRACE);
    }

    /// MetadataUsage = '@' QualifiedName ...
    fn parse_metadata_usage(&mut self) {
        self.start_node(SyntaxKind::METADATA_USAGE);
        
        self.expect(SyntaxKind::AT);
        self.skip_trivia();
        self.parse_qualified_name();
        self.skip_trivia();
        
        // Optional body
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }
        
        self.finish_node();
    }

    /// Definition or Usage - determined by presence of 'def' keyword
    fn parse_definition_or_usage(&mut self) {
        // Peek ahead to determine if this is a definition or usage
        // Definition: ... 'def' Name ...
        // Usage: ... Name ...
        
        let is_def = self.look_for_def();
        
        if is_def {
            self.parse_definition();
        } else {
            self.parse_usage();
        }
    }

    fn look_for_def(&self) -> bool {
        // Scan ahead (skipping trivia) looking for 'def' before ';' or '{'
        let mut idx = self.pos;
        while idx < self.tokens.len() {
            let kind = self.tokens[idx].kind;
            if kind == SyntaxKind::DEF_KW {
                return true;
            }
            if kind == SyntaxKind::SEMICOLON || kind == SyntaxKind::L_BRACE 
               || kind == SyntaxKind::COLON || kind == SyntaxKind::COLON_GT
               || kind == SyntaxKind::COLON_GT_GT || kind == SyntaxKind::EQ {
                return false;
            }
            idx += 1;
        }
        false
    }

    /// Definition = Prefix* Keyword 'def' Identification Specializations? Body
    fn parse_definition(&mut self) {
        self.start_node(SyntaxKind::DEFINITION);
        
        // Parse prefixes (abstract, variation, etc.)
        self.parse_definition_prefix();
        self.skip_trivia();
        
        // Parse definition keyword (part, attribute, etc.)
        self.parse_definition_keyword();
        self.skip_trivia();
        
        // Expect 'def'
        self.expect(SyntaxKind::DEF_KW);
        self.skip_trivia();
        
        // Parse identification
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
        }
        
        self.skip_trivia();
        
        // Parse specializations (:>, subsets, redefines, etc.)
        self.parse_specializations();
        
        self.skip_trivia();
        
        // Parse body
        self.parse_body();
        
        self.finish_node();
    }

    /// Usage = Prefix* Keyword? Identification? Multiplicity? Typing? Specializations? Body
    fn parse_usage(&mut self) {
        self.start_node(SyntaxKind::USAGE);
        
        // Parse prefixes (ref, derived, etc.)
        self.parse_usage_prefix();
        self.skip_trivia();
        
        // Check if this is a message usage (need special handling for 'of' keyword)
        let is_message = self.at(SyntaxKind::MESSAGE_KW);
        
        // Parse usage keyword (part, attribute, etc.) - optional for shorthand
        self.parse_usage_keyword();
        self.skip_trivia();
        
        // For message usages, skip the 'of' keyword if present
        if is_message && self.at(SyntaxKind::IDENT) && self.current_text() == "of" {
            self.bump(); // skip 'of'
            self.skip_trivia();
        }
        
        // Parse identification
        if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::LT) {
            self.parse_identification();
        }
        
        self.skip_trivia();
        
        // Parse multiplicity [n..m]
        if self.at(SyntaxKind::L_BRACKET) {
            self.parse_multiplicity();
        }
        
        self.skip_trivia();
        
        // Parse typing (:)
        if self.at(SyntaxKind::COLON) {
            self.parse_typing();
        }
        
        self.skip_trivia();
        
        // Parse specializations
        self.parse_specializations();
        
        self.skip_trivia();
        
        // For message usages, parse 'from source to target' clause
        if is_message && self.at(SyntaxKind::FROM_KW) {
            self.parse_message_endpoints();
        }
        
        self.skip_trivia();
        
        // Parse value (=)
        if self.at(SyntaxKind::EQ) {
            self.bump();
            self.skip_trivia();
            self.parse_expression();
        }
        
        self.skip_trivia();
        
        // Parse body
        self.parse_body();
        
        self.finish_node();
    }

    fn parse_definition_prefix(&mut self) {
        while self.at_any(&[
            SyntaxKind::ABSTRACT_KW,
            SyntaxKind::VARIATION_KW,
        ]) {
            self.bump();
            self.skip_trivia();
        }
    }

    fn parse_usage_prefix(&mut self) {
        while self.at_any(&[
            SyntaxKind::REF_KW,
            SyntaxKind::READONLY_KW,
            SyntaxKind::DERIVED_KW,
            SyntaxKind::END_KW,
            SyntaxKind::ABSTRACT_KW,
            SyntaxKind::VARIATION_KW,
            SyntaxKind::INDIVIDUAL_KW,
            SyntaxKind::IN_KW,
            SyntaxKind::OUT_KW,
            SyntaxKind::INOUT_KW,
        ]) {
            self.bump();
            self.skip_trivia();
        }
    }

    fn parse_definition_keyword(&mut self) {
        if self.at_any(&[
            SyntaxKind::PART_KW,
            SyntaxKind::ATTRIBUTE_KW,
            SyntaxKind::PORT_KW,
            SyntaxKind::ITEM_KW,
            SyntaxKind::ACTION_KW,
            SyntaxKind::STATE_KW,
            SyntaxKind::CONSTRAINT_KW,
            SyntaxKind::REQUIREMENT_KW,
            SyntaxKind::CASE_KW,
            SyntaxKind::CALC_KW,
            SyntaxKind::CONNECTION_KW,
            SyntaxKind::INTERFACE_KW,
            SyntaxKind::ALLOCATION_KW,
            SyntaxKind::FLOW_KW,
            SyntaxKind::VIEW_KW,
            SyntaxKind::VIEWPOINT_KW,
            SyntaxKind::RENDERING_KW,
            SyntaxKind::METADATA_KW,
            SyntaxKind::OCCURRENCE_KW,
            SyntaxKind::ENUM_KW,
            SyntaxKind::ANALYSIS_KW,
            SyntaxKind::VERIFICATION_KW,
            SyntaxKind::USE_KW,
            SyntaxKind::CONCERN_KW,
            // KerML definition keywords
            SyntaxKind::CLASS_KW,
            SyntaxKind::STRUCT_KW,
            SyntaxKind::ASSOC_KW,
            SyntaxKind::BEHAVIOR_KW,
            SyntaxKind::FUNCTION_KW,
            SyntaxKind::PREDICATE_KW,
            SyntaxKind::INTERACTION_KW,
            SyntaxKind::DATATYPE_KW,
            SyntaxKind::CLASSIFIER_KW,
            SyntaxKind::TYPE_KW,
        ]) {
            self.bump();
        }
    }

    fn parse_usage_keyword(&mut self) {
        if self.at_any(&[
            SyntaxKind::PART_KW,
            SyntaxKind::ATTRIBUTE_KW,
            SyntaxKind::PORT_KW,
            SyntaxKind::ITEM_KW,
            SyntaxKind::ACTION_KW,
            SyntaxKind::STATE_KW,
            SyntaxKind::CONSTRAINT_KW,
            SyntaxKind::REQUIREMENT_KW,
            SyntaxKind::CASE_KW,
            SyntaxKind::CALC_KW,
            SyntaxKind::CONNECTION_KW,
            SyntaxKind::INTERFACE_KW,
            SyntaxKind::ALLOCATION_KW,
            SyntaxKind::FLOW_KW,
            SyntaxKind::VIEW_KW,
            SyntaxKind::VIEWPOINT_KW,
            SyntaxKind::RENDERING_KW,
            SyntaxKind::OCCURRENCE_KW,
            SyntaxKind::INDIVIDUAL_KW,
            SyntaxKind::ANALYSIS_KW,
            SyntaxKind::VERIFICATION_KW,
            SyntaxKind::USE_KW,
            SyntaxKind::CONCERN_KW,
            SyntaxKind::REF_KW,
            SyntaxKind::BINDING_KW,
            SyntaxKind::SUCCESSION_KW,
            // Message/event keywords
            SyntaxKind::MESSAGE_KW,
            SyntaxKind::SNAPSHOT_KW,
            SyntaxKind::TIMESLICE_KW,
            SyntaxKind::FRAME_KW,
            SyntaxKind::EVENT_KW,
            // Action body keywords
            SyntaxKind::PERFORM_KW,
            SyntaxKind::ACCEPT_KW,
            SyntaxKind::SEND_KW,
            SyntaxKind::EXHIBIT_KW,
            SyntaxKind::INCLUDE_KW,
            SyntaxKind::SATISFY_KW,
            SyntaxKind::RENDER_KW,
            // Constraint keywords
            SyntaxKind::ASSERT_KW,
            SyntaxKind::ASSUME_KW,
            SyntaxKind::REQUIRE_KW,
            // KerML usage keywords
            SyntaxKind::FEATURE_KW,
            SyntaxKind::STEP_KW,
            SyntaxKind::EXPR_KW,
            SyntaxKind::CONNECTOR_KW,
            // Transition/state keywords
            SyntaxKind::TRANSITION_KW,
            SyntaxKind::FIRST_KW,
            SyntaxKind::THEN_KW,
            SyntaxKind::ENTRY_KW,
            SyntaxKind::DO_KW,
            SyntaxKind::EXIT_KW,
            SyntaxKind::IF_KW,
        ]) {
            self.bump();
        }
    }

    /// Identification = '<' ShortName '>' Name? | Name
    fn parse_identification(&mut self) {
        self.start_node(SyntaxKind::NAME);
        
        // Short name: <shortname>
        if self.at(SyntaxKind::LT) {
            self.start_node(SyntaxKind::SHORT_NAME);
            self.bump(); // <
            self.skip_trivia();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            }
            self.skip_trivia();
            self.expect(SyntaxKind::GT);
            self.finish_node();
            self.skip_trivia();
        }
        
        // Regular name
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }
        
        self.finish_node();
    }

    /// QualifiedName = Name ('::' Name)*
    fn parse_qualified_name(&mut self) {
        self.start_node(SyntaxKind::QUALIFIED_NAME);
        
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }
        
        while self.at(SyntaxKind::COLON_COLON) {
            // Check if next token after :: is a wildcard (* or **)
            // If so, don't consume the :: - let the caller handle it
            let mut peek_idx = self.pos + 1;
            while peek_idx < self.tokens.len() && self.tokens[peek_idx].kind.is_trivia() {
                peek_idx += 1;
            }
            if peek_idx < self.tokens.len() {
                let peek_kind = self.tokens[peek_idx].kind;
                if peek_kind == SyntaxKind::STAR || peek_kind == SyntaxKind::STAR_STAR {
                    break;
                }
            }
            
            self.bump(); // ::
            self.skip_trivia();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            } else {
                break;
            }
        }
        
        self.finish_node();
    }

    fn parse_qualified_name_list(&mut self) {
        self.parse_qualified_name();
        
        while self.at(SyntaxKind::COMMA) {
            self.bump();
            self.skip_trivia();
            self.parse_qualified_name();
        }
    }

    /// Typing = ':' QualifiedName
    fn parse_typing(&mut self) {
        self.start_node(SyntaxKind::TYPING);
        
        self.expect(SyntaxKind::COLON);
        self.skip_trivia();
        self.parse_qualified_name();
        
        self.finish_node();
    }

    /// MessageEndpoints = 'from' FeatureChain 'to' FeatureChain
    /// Used for message usages: message of name from source.endpoint to target.endpoint
    fn parse_message_endpoints(&mut self) {
        // Parse 'from' clause
        self.start_node(SyntaxKind::SPECIALIZATION);
        self.expect(SyntaxKind::FROM_KW);
        self.skip_trivia();
        self.parse_feature_chain();
        self.finish_node();
        
        self.skip_trivia();
        
        // Parse 'to' clause
        if self.at(SyntaxKind::TO_KW) {
            self.start_node(SyntaxKind::SPECIALIZATION);
            self.bump(); // 'to'
            self.skip_trivia();
            self.parse_feature_chain();
            self.finish_node();
        }
    }

    /// FeatureChain = Name ('.' Name)*
    /// For dotted references like driver.turnVehicleOn
    fn parse_feature_chain(&mut self) {
        self.start_node(SyntaxKind::QUALIFIED_NAME);
        
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }
        
        while self.at(SyntaxKind::DOT) {
            self.bump(); // .
            self.skip_trivia();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            }
        }
        
        self.finish_node();
    }

    /// Specializations = (':>' | 'specializes' | 'subsets' | 'redefines' | ...) QualifiedName
    fn parse_specializations(&mut self) {
        while self.at_any(&[
            SyntaxKind::COLON_GT,
            SyntaxKind::COLON_GT_GT,
            SyntaxKind::COLON_COLON_GT,
            SyntaxKind::SPECIALIZES_KW,
            SyntaxKind::SUBSETS_KW,
            SyntaxKind::REDEFINES_KW,
            SyntaxKind::REFERENCES_KW,
        ]) {
            self.start_node(SyntaxKind::SPECIALIZATION);
            
            self.bump(); // operator/keyword
            self.skip_trivia();
            self.parse_qualified_name();
            
            self.finish_node();
            self.skip_trivia();
            
            // Handle comma-separated list
            while self.at(SyntaxKind::COMMA) {
                self.bump();
                self.skip_trivia();
                
                self.start_node(SyntaxKind::SPECIALIZATION);
                self.parse_qualified_name();
                self.finish_node();
                
                self.skip_trivia();
            }
        }
    }

    /// Body = ';' | '{' BodyMember* '}'
    fn parse_body(&mut self) {
        self.start_node(SyntaxKind::NAMESPACE_BODY);
        
        if self.eat(SyntaxKind::SEMICOLON) {
            // Empty body
        } else if self.eat(SyntaxKind::L_BRACE) {
            self.skip_trivia();
            
            while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
                let pos_before = self.pos;
                self.parse_namespace_member();
                self.skip_trivia();
                // Safety: if we didn't make progress, force-skip a token
                if self.pos == pos_before && !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
                    self.error(format!("stuck on token: {:?}", self.current_kind()));
                    self.bump_any();
                }
            }
            
            self.expect(SyntaxKind::R_BRACE);
        } else {
            self.error("expected ';' or '{'");
        }
        
        self.finish_node();
    }

    /// Multiplicity = '[' (MultiplicityBounds | MultiplicityRange)? ']'
    fn parse_multiplicity(&mut self) {
        if !self.at(SyntaxKind::L_BRACKET) {
            return;
        }
        
        self.start_node(SyntaxKind::MULTIPLICITY);
        
        self.bump(); // [
        self.skip_trivia();
        
        // Parse multiplicity bounds/range
        if !self.at(SyntaxKind::R_BRACKET) {
            // Lower bound
            if self.at(SyntaxKind::INTEGER) || self.at(SyntaxKind::STAR) {
                self.bump();
            }
            
            self.skip_trivia();
            
            // Range (..)
            if self.at(SyntaxKind::DOT_DOT) {
                self.bump();
                self.skip_trivia();
                
                // Upper bound
                if self.at(SyntaxKind::INTEGER) || self.at(SyntaxKind::STAR) {
                    self.bump();
                }
            }
        }
        
        self.skip_trivia();
        self.expect(SyntaxKind::R_BRACKET);
        
        self.finish_node();
    }

    /// Expression parsing with proper precedence
    fn parse_expression(&mut self) {
        self.parse_conditional_expression();
    }
    
    /// ConditionalExpression = NullCoalescingExpression ('?' Expression ':' Expression)?
    fn parse_conditional_expression(&mut self) {
        self.start_node(SyntaxKind::EXPRESSION);
        
        self.parse_null_coalescing_expression();
        
        self.skip_trivia();
        if self.at(SyntaxKind::QUESTION) && !self.at(SyntaxKind::QUESTION_QUESTION) {
            self.bump(); // ?
            self.skip_trivia();
            self.parse_expression();
            self.skip_trivia();
            self.expect(SyntaxKind::COLON);
            self.skip_trivia();
            self.parse_expression();
        }
        
        self.finish_node();
    }
    
    /// NullCoalescingExpression = ImpliesExpression ('??' ImpliesExpression)*
    fn parse_null_coalescing_expression(&mut self) {
        self.parse_implies_expression();
        
        while self.at(SyntaxKind::QUESTION_QUESTION) {
            self.bump();
            self.skip_trivia();
            self.parse_implies_expression();
        }
    }
    
    /// ImpliesExpression = OrExpression ('implies' OrExpression)*
    fn parse_implies_expression(&mut self) {
        self.parse_or_expression();
        
        while self.at(SyntaxKind::IMPLIES_KW) {
            self.bump();
            self.skip_trivia();
            self.parse_or_expression();
        }
    }
    
    /// OrExpression = XorExpression (('|' | 'or') XorExpression)*
    fn parse_or_expression(&mut self) {
        self.parse_xor_expression();
        
        while self.at(SyntaxKind::PIPE) || self.at(SyntaxKind::OR_KW) {
            self.bump();
            self.skip_trivia();
            self.parse_xor_expression();
        }
    }
    
    /// XorExpression = AndExpression ('xor' AndExpression)*
    fn parse_xor_expression(&mut self) {
        self.parse_and_expression();
        
        while self.at(SyntaxKind::XOR_KW) {
            self.bump();
            self.skip_trivia();
            self.parse_and_expression();
        }
    }
    
    /// AndExpression = EqualityExpression (('&' | 'and') EqualityExpression)*
    fn parse_and_expression(&mut self) {
        self.parse_equality_expression();
        
        while self.at(SyntaxKind::AMP) || self.at(SyntaxKind::AND_KW) {
            self.bump();
            self.skip_trivia();
            self.parse_equality_expression();
        }
    }
    
    /// EqualityExpression = ClassificationExpression (('==' | '!=' | '===' | '!==') ClassificationExpression)*
    fn parse_equality_expression(&mut self) {
        self.parse_classification_expression();
        
        while self.at_any(&[SyntaxKind::EQ_EQ, SyntaxKind::BANG_EQ, SyntaxKind::EQ_EQ_EQ, SyntaxKind::BANG_EQ_EQ]) {
            self.bump();
            self.skip_trivia();
            self.parse_classification_expression();
        }
    }
    
    /// ClassificationExpression = RelationalExpression (('hastype' | 'istype' | 'as' | '@' | '@@') TypeReference)?
    fn parse_classification_expression(&mut self) {
        self.parse_relational_expression();
        
        self.skip_trivia();
        if self.at_any(&[SyntaxKind::HASTYPE_KW, SyntaxKind::ISTYPE_KW, SyntaxKind::AS_KW, SyntaxKind::AT, SyntaxKind::AT_AT]) {
            self.bump();
            self.skip_trivia();
            self.parse_qualified_name();
        }
    }
    
    /// RelationalExpression = RangeExpression (('<' | '>' | '<=' | '>=') RangeExpression)*
    fn parse_relational_expression(&mut self) {
        self.parse_range_expression();
        
        while self.at_any(&[SyntaxKind::LT, SyntaxKind::GT, SyntaxKind::LT_EQ, SyntaxKind::GT_EQ]) {
            self.bump();
            self.skip_trivia();
            self.parse_range_expression();
        }
    }
    
    /// RangeExpression = AdditiveExpression ('..' AdditiveExpression)?
    fn parse_range_expression(&mut self) {
        self.parse_additive_expression();
        
        self.skip_trivia();
        if self.at(SyntaxKind::DOT_DOT) {
            self.bump();
            self.skip_trivia();
            self.parse_additive_expression();
        }
    }
    
    /// AdditiveExpression = MultiplicativeExpression (('+' | '-') MultiplicativeExpression)*
    fn parse_additive_expression(&mut self) {
        self.parse_multiplicative_expression();
        
        while self.at(SyntaxKind::PLUS) || self.at(SyntaxKind::MINUS) {
            self.bump();
            self.skip_trivia();
            self.parse_multiplicative_expression();
        }
    }
    
    /// MultiplicativeExpression = ExponentiationExpression (('*' | '/' | '%') ExponentiationExpression)*
    fn parse_multiplicative_expression(&mut self) {
        self.parse_exponentiation_expression();
        
        while self.at_any(&[SyntaxKind::STAR, SyntaxKind::SLASH, SyntaxKind::PERCENT]) {
            self.bump();
            self.skip_trivia();
            self.parse_exponentiation_expression();
        }
    }
    
    /// ExponentiationExpression = UnaryExpression (('**' | '^') ExponentiationExpression)?
    fn parse_exponentiation_expression(&mut self) {
        self.parse_unary_expression();
        
        self.skip_trivia();
        if self.at(SyntaxKind::STAR_STAR) || self.at(SyntaxKind::CARET) {
            self.bump();
            self.skip_trivia();
            self.parse_exponentiation_expression();
        }
    }
    
    /// UnaryExpression = ('+' | '-' | '~' | 'not')? ExtentExpression
    fn parse_unary_expression(&mut self) {
        if self.at_any(&[SyntaxKind::PLUS, SyntaxKind::MINUS, SyntaxKind::TILDE, SyntaxKind::NOT_KW]) {
            self.bump();
            self.skip_trivia();
        }
        self.parse_extent_expression();
    }
    
    /// ExtentExpression = ('all')? PrimaryExpression
    fn parse_extent_expression(&mut self) {
        if self.at(SyntaxKind::ALL_KW) {
            self.bump();
            self.skip_trivia();
        }
        self.parse_primary_expression();
    }
    
    /// PrimaryExpression = BaseExpression FeatureChain*
    fn parse_primary_expression(&mut self) {
        self.parse_base_expression();
        
        // Feature chains (.name or .name())
        while self.at(SyntaxKind::DOT) {
            self.bump(); // .
            self.skip_trivia();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            }
            self.skip_trivia();
            // Check for invocation
            if self.at(SyntaxKind::L_PAREN) {
                self.parse_argument_list();
            }
        }
    }
    
    /// BaseExpression = LiteralExpression | FeatureReferenceExpression | InvocationExpression | '(' SequenceExpression ')'
    fn parse_base_expression(&mut self) {
        self.skip_trivia();
        
        match self.current_kind() {
            // Literals
            SyntaxKind::INTEGER | SyntaxKind::DECIMAL | SyntaxKind::STRING => {
                self.bump();
            }
            SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NULL_KW => {
                self.bump();
            }
            
            // Parenthesized expression or sequence
            SyntaxKind::L_PAREN => {
                self.bump(); // (
                self.skip_trivia();
                
                if !self.at(SyntaxKind::R_PAREN) {
                    self.parse_expression();
                    
                    // Check for sequence (comma-separated)
                    while self.at(SyntaxKind::COMMA) {
                        self.bump();
                        self.skip_trivia();
                        self.parse_expression();
                        self.skip_trivia();
                    }
                }
                
                self.skip_trivia();
                self.expect(SyntaxKind::R_PAREN);
            }
            
            // Feature reference or invocation
            SyntaxKind::IDENT => {
                self.parse_qualified_name();
                self.skip_trivia();
                
                // Check for invocation
                if self.at(SyntaxKind::L_PAREN) {
                    self.parse_argument_list();
                }
            }
            
            // Metadata access expression
            SyntaxKind::AT => {
                self.bump(); // @
                self.skip_trivia();
                self.parse_qualified_name();
            }
            
            _ => {
                // Unknown - don't consume
            }
        }
    }
    
    /// ArgumentList = '(' (Argument (',' Argument)*)? ')'
    fn parse_argument_list(&mut self) {
        self.start_node(SyntaxKind::ARGUMENT_LIST);
        
        self.expect(SyntaxKind::L_PAREN);
        self.skip_trivia();
        
        if !self.at(SyntaxKind::R_PAREN) {
            self.parse_expression();
            self.skip_trivia();
            
            while self.at(SyntaxKind::COMMA) {
                self.bump();
                self.skip_trivia();
                self.parse_expression();
                self.skip_trivia();
            }
        }
        
        self.expect(SyntaxKind::R_PAREN);
        
        self.finish_node();
    }
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
        
        let root = parse.syntax();
        assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
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
    fn test_parse_import_with_filter() {
        let parse = parse("import Library::*[@MyFilter];");
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
    fn test_parse_nested() {
        let source = r#"
            package Vehicle {
                part def Engine {
                    attribute power : Real;
                }
                part engine : Engine;
            }
        "#;
        let parse = parse(source);
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
    
    #[test]
    fn test_parse_attribute_with_default() {
        let parse = parse("attribute x : Integer = 42;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
    
    #[test]
    fn test_parse_attribute_with_expression() {
        let parse = parse("attribute y : Real = 3.14 + 2.0;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
    
    #[test]
    fn test_parse_multiplicity() {
        let parse = parse("part engines[2..*] : Engine;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
    
    #[test]
    fn test_parse_function_invocation() {
        let parse = parse("calc result = compute(x, y);");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
    
    #[test]
    fn test_parse_conditional_expression() {
        let parse = parse("attribute flag : Boolean = x > 0 ? true : false;");
        assert!(parse.ok(), "errors: {:?}", parse.errors);
    }
}
