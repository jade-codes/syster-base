//! SysML grammar parsing
//!
//! This module contains functions for parsing SysML-specific constructs:
//! - Action body elements (accept, send, perform, if, while, for, control nodes)
//! - State body elements (entry, exit, do, transitions)
//! - Requirement body elements (subject, actor, stakeholder, objective, constraints)
//!
//! # Grammar Sources
//! - **Primary**: `src/parser/sysml.pest` - SysML v2 specific grammar rules
//! - **Shared**: `src/parser/kerml_expressions.pest` - Expression grammar shared between KerML and SysML
//! - **Interop**: KerML constructs (class, struct, behavior) are parsed for standard library compatibility
//!
//! # Expression Parsing
//! Expression parsing uses `parse_expression()` from kerml_expressions module.
//! This is correct: expressions are defined in kerml_expressions.pest and shared by both grammars.
//! SysML extends KerML expressions but uses the same precedence and base operators.

use super::kerml::is_name_kind;
use super::kerml_expressions::{ExpressionParser, parse_expression};
use crate::parser::syntax_kind::SyntaxKind;

/// Standalone relationship keywords (SysML)
pub const STANDALONE_RELATIONSHIP_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::SPECIALIZATION_KW,
    SyntaxKind::SUBCLASSIFIER_KW,
    SyntaxKind::REDEFINITION_KW,
    SyntaxKind::SUBSET_KW,
    SyntaxKind::SUBTYPE_KW,
    SyntaxKind::TYPING_KW,
    SyntaxKind::CONJUGATION_KW,
    SyntaxKind::DISJOINING_KW,
    SyntaxKind::FEATURING_KW,
    SyntaxKind::INVERTING_KW,
];

/// Relationship operator keywords (SysML)
pub const RELATIONSHIP_OPERATORS: &[SyntaxKind] = &[
    SyntaxKind::SPECIALIZES_KW,
    SyntaxKind::COLON_GT,
    SyntaxKind::SUBSETS_KW,
    SyntaxKind::REDEFINES_KW,
    SyntaxKind::COLON_GT_GT,
    SyntaxKind::TYPED_KW,
    SyntaxKind::COLON,
    SyntaxKind::CONJUGATES_KW,
    SyntaxKind::TILDE,
    SyntaxKind::INVERSE_KW,
    SyntaxKind::OF_KW,
];

/// SysML definition keywords (used with 'def')
pub const SYSML_DEFINITION_KEYWORDS: &[SyntaxKind] = &[
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
    SyntaxKind::RENDERING_KW,
    SyntaxKind::VIEW_KW,
    SyntaxKind::VIEWPOINT_KW,
    SyntaxKind::ANALYSIS_KW,
    SyntaxKind::VERIFICATION_KW,
    SyntaxKind::OCCURRENCE_KW,
    SyntaxKind::CONCERN_KW,
    SyntaxKind::METADATA_KW,
    SyntaxKind::ENUM_KW,
];

/// SysML usage keywords (used without 'def')
pub const SYSML_USAGE_KEYWORDS: &[SyntaxKind] = &[
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
    SyntaxKind::RENDERING_KW,
    SyntaxKind::VIEW_KW,
    SyntaxKind::VIEWPOINT_KW,
    SyntaxKind::ANALYSIS_KW,
    SyntaxKind::VERIFICATION_KW,
    SyntaxKind::OCCURRENCE_KW,
    SyntaxKind::INDIVIDUAL_KW,
    SyntaxKind::REF_KW,
    SyntaxKind::EXHIBIT_KW,
    SyntaxKind::INCLUDE_KW,
    SyntaxKind::PERFORM_KW,
    SyntaxKind::ACCEPT_KW,
    SyntaxKind::SEND_KW,
    SyntaxKind::SATISFY_KW,
    SyntaxKind::CONCERN_KW,
    SyntaxKind::METADATA_KW,
    SyntaxKind::ENUM_KW,
    SyntaxKind::MESSAGE_KW,
    SyntaxKind::SNAPSHOT_KW,
    SyntaxKind::TIMESLICE_KW,
    SyntaxKind::FRAME_KW,
    SyntaxKind::RENDER_KW,
    SyntaxKind::THEN_KW,
    SyntaxKind::ELSE_KW,
    SyntaxKind::WHILE_KW,
    SyntaxKind::LOOP_KW,
    SyntaxKind::UNTIL_KW,
    SyntaxKind::IF_KW,
    SyntaxKind::ASSERT_KW,
    SyntaxKind::ASSUME_KW,
    SyntaxKind::REQUIRE_KW,
    SyntaxKind::SUBJECT_KW,
    SyntaxKind::ACTOR_KW,
    SyntaxKind::OBJECTIVE_KW,
    SyntaxKind::STAKEHOLDER_KW,
];

/// Usage prefix keywords
pub const USAGE_PREFIX_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::REF_KW,
    SyntaxKind::READONLY_KW,
    SyntaxKind::DERIVED_KW,
    SyntaxKind::CONSTANT_KW,
    SyntaxKind::END_KW,
    SyntaxKind::ABSTRACT_KW,
    SyntaxKind::VARIATION_KW,
    SyntaxKind::VAR_KW,
    SyntaxKind::COMPOSITE_KW,
    SyntaxKind::PORTION_KW,
    SyntaxKind::INDIVIDUAL_KW,
    SyntaxKind::IN_KW,
    SyntaxKind::OUT_KW,
    SyntaxKind::INOUT_KW,
    SyntaxKind::RETURN_KW,
    SyntaxKind::EVENT_KW,
    SyntaxKind::THEN_KW,
    // Portion kinds (snapshot/timeslice prefix)
    SyntaxKind::SNAPSHOT_KW,
    SyntaxKind::TIMESLICE_KW,
];

/// Check if a kind is a SysML definition keyword
pub fn is_sysml_definition_keyword(kind: SyntaxKind) -> bool {
    SYSML_DEFINITION_KEYWORDS.contains(&kind)
}

/// Check if a kind is a SysML usage keyword
pub fn is_sysml_usage_keyword(kind: SyntaxKind) -> bool {
    SYSML_USAGE_KEYWORDS.contains(&kind)
}

/// Check if a kind is a usage prefix keyword
pub fn is_usage_prefix(kind: SyntaxKind) -> bool {
    USAGE_PREFIX_KEYWORDS.contains(&kind)
}

/// Check if a SyntaxKind is a usage keyword (for lookahead)
fn is_usage_keyword(kind: SyntaxKind) -> bool {
    SYSML_USAGE_KEYWORDS.contains(&kind)
}

/// Trait for SysML parsing operations
///
/// This trait defines the interface for SysML-specific parsing.
/// SysML is a superset of KerML but this trait is independent.
/// For KerML constructs (package, import, class, struct), use KerMLParser methods.
/// The main parser implements this trait.
pub trait SysMLParser: ExpressionParser {
    // -----------------------------------------------------------------
    // Core parsing methods
    // -----------------------------------------------------------------

    /// Get the current token (for text inspection)
    fn current_token_text(&self) -> Option<&str>;

    /// Parse an identification (name or short name)
    fn parse_identification(&mut self);

    /// Parse a body (semicolon or braced block)
    fn parse_body(&mut self);

    /// Skip trivia except block comments
    fn skip_trivia_except_block_comments(&mut self);

    /// Parse a comma-separated list of qualified names
    fn parse_qualified_name_list(&mut self);

    /// Report a parse error
    fn error(&mut self, message: impl Into<String>);

    /// Error recovery - skip to recovery tokens
    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]);

    // -----------------------------------------------------------------
    // SysML namespace member parsing
    // -----------------------------------------------------------------

    /// Parse a namespace member (SysML level)
    ///
    /// This handles all SysML namespace body elements:
    /// - Definitions: part def, action def, etc.
    /// - Usages: part, attribute, action, state, etc.
    /// - Relationships, annotations, import/alias
    fn parse_namespace_member(&mut self)
    where
        Self: Sized,
    {
        parse_package_body_element(self);
    }

    // -----------------------------------------------------------------
    // SysML-specific methods
    // -----------------------------------------------------------------

    /// Check if we can start an expression
    fn can_start_expression(&self) -> bool;

    /// Parse typing (: Type or :> Type)
    fn parse_typing(&mut self);

    /// Parse multiplicity [n..m]
    fn parse_multiplicity(&mut self);

    /// Parse constraint body (expression-based body)
    fn parse_constraint_body(&mut self);

    // -----------------------------------------------------------------
    // SysML-specific element parsers (called by parse_package_body_element)
    // -----------------------------------------------------------------

    /// Parse a SysML definition (part def, action def, etc.)
    fn parse_definition_or_usage(&mut self);

    /// Parse a dependency
    fn parse_dependency(&mut self);

    /// Parse a filter
    fn parse_filter(&mut self);

    /// Parse metadata usage (@Metadata)
    fn parse_metadata_usage(&mut self);

    /// Parse connect usage
    fn parse_connect_usage(&mut self);

    /// Parse binding or succession
    fn parse_binding_or_succession(&mut self);

    /// Parse variant usage
    fn parse_variant_usage(&mut self);

    /// Parse redefines feature member
    fn parse_redefines_feature_member(&mut self);

    /// Parse shorthand feature member
    fn parse_shorthand_feature_member(&mut self);
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Emit an error for missing body terminator with context
fn error_missing_body_terminator<P: SysMLParser>(p: &mut P, context: &str) {
    let found = if let Some(text) = p.current_token_text() {
        format!("'{}' ({})", text, p.current_kind().display_name())
    } else {
        "end of file".to_string()
    };
    p.error(format!(
        "expected ';' to end {} or '{{' to start body, found {}",
        context, found
    ));
}

/// Helper to consume a keyword and skip trivia in one call
fn bump_keyword<P: SysMLParser>(p: &mut P) {
    p.bump();
    p.skip_trivia();
}

/// Helper to expect a token and skip trivia
fn expect_and_skip<P: SysMLParser>(p: &mut P, kind: SyntaxKind) {
    p.expect(kind);
    p.skip_trivia();
}

/// Helper to check, bump, and skip trivia for a specific token
fn consume_if<P: SysMLParser>(p: &mut P, kind: SyntaxKind) -> bool {
    if p.at(kind) {
        bump_keyword(p);
        true
    } else {
        false
    }
}

/// Helper to skip trivia in lookahead
fn skip_trivia_lookahead<P: SysMLParser>(p: &P, mut lookahead: usize) -> usize {
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }
    lookahead
}

/// Helper to peek past a name (possibly qualified with ::) and check if the following token is `target`
/// This is used to distinguish between:
/// - `binding myName bind x = y` (myName is identification)
/// - `binding payload = target` (payload is bind source, not identification)
fn peek_past_name_for<P: SysMLParser>(p: &P, target: SyntaxKind) -> bool {
    let mut lookahead = 0;
    lookahead = skip_trivia_lookahead(p, lookahead);

    // We know we're at a name token, skip it
    if p.peek_kind(lookahead) == SyntaxKind::IDENT {
        lookahead += 1;
    } else {
        return false;
    }

    // Handle qualified names (A::B::C) and dotted chains (a.b.c)
    loop {
        lookahead = skip_trivia_lookahead(p, lookahead);
        let next = p.peek_kind(lookahead);

        if next == SyntaxKind::COLON_COLON || next == SyntaxKind::DOT {
            lookahead += 1;
            lookahead = skip_trivia_lookahead(p, lookahead);
            if p.peek_kind(lookahead) == SyntaxKind::IDENT {
                lookahead += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    lookahead = skip_trivia_lookahead(p, lookahead);
    p.peek_kind(lookahead) == target
}

/// Helper to peek past optional identifier and get next significant token
fn peek_past_optional_name<P: SysMLParser>(p: &P, mut lookahead: usize) -> (usize, SyntaxKind) {
    lookahead = skip_trivia_lookahead(p, lookahead);
    let mut next = p.peek_kind(lookahead);
    if next == SyntaxKind::IDENT {
        lookahead += 1;
        lookahead = skip_trivia_lookahead(p, lookahead);
        next = p.peek_kind(lookahead);
    }
    (lookahead, next)
}

/// Helper to parse optional identification
fn parse_optional_identification<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }
}

/// Helper to parse optional qualified name
fn parse_optional_qualified_name<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Helper to parse qualified name and skip trivia
fn parse_qualified_name_and_skip<P: SysMLParser>(p: &mut P) {
    p.parse_qualified_name();
    p.skip_trivia();
}

/// SysML-specific identification parsing.
/// Identification = '<' ShortName '>' Name? | Name
///
/// This is separate from KerML's parse_identification to allow SysML-specific
/// behavior if needed, though currently the grammar is the same.
pub fn parse_identification<P: SysMLParser>(p: &mut P) {
    // Skip trivia BEFORE starting the NAME node so the node's range
    // doesn't include leading whitespace
    p.skip_trivia();
    p.start_node(SyntaxKind::NAME);

    // Short name: <shortname>
    if p.at(SyntaxKind::LT) {
        p.start_node(SyntaxKind::SHORT_NAME);
        p.bump(); // <
        p.skip_trivia();
        // Accept any identifier-like token including keywords for short names
        if p.at_name_token() || p.current_kind().is_keyword() {
            p.bump();
        }
        p.skip_trivia();
        p.expect(SyntaxKind::GT);
        p.finish_node();
        p.skip_trivia();
    }

    // Regular name
    if p.at_name_token() {
        p.bump();
    }

    p.finish_node();
}

/// Helper to parse body or semicolon (common pattern)
fn parse_body_or_semicolon<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }
}

/// Helper to parse optional default value
/// Pattern: default [=] expr or = expr or := expr
fn parse_optional_default_value<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::DEFAULT_KW) {
        bump_keyword(p);
        // Optional '=' after default
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            bump_keyword(p);
        }
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional multiplicity
fn parse_optional_multiplicity<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }
}

/// Helper to parse optional typing
fn parse_optional_typing<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }
}

/// Helper to parse specializations followed by skip_trivia
fn parse_specializations_with_skip<P: SysMLParser>(p: &mut P) {
    parse_specializations(p);
    p.skip_trivia();
}

/// Helper to parse optional via clause: via <expr>
fn parse_optional_via<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::VIA_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional to clause: to <expr>
fn parse_optional_to<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::TO_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional from/to clause: from <name> to <name>
/// Creates FROM_TO_CLAUSE, FROM_TO_SOURCE, and FROM_TO_TARGET nodes for AST extraction
fn parse_optional_from_to<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::FROM_KW) {
        p.start_node(SyntaxKind::FROM_TO_CLAUSE);
        p.bump(); // from
        p.skip_trivia();

        // Parse source wrapped in FROM_TO_SOURCE
        if p.at_name_token() {
            p.start_node(SyntaxKind::FROM_TO_SOURCE);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::TO_KW) {
            p.bump(); // to
            p.skip_trivia();

            // Parse target wrapped in FROM_TO_TARGET
            if p.at_name_token() {
                p.start_node(SyntaxKind::FROM_TO_TARGET);
                p.parse_qualified_name();
                p.finish_node();
                p.skip_trivia();
            }
        }
        p.finish_node(); // FROM_TO_CLAUSE
    }
}

/// Helper to parse comma-separated list with a parser function
#[allow(dead_code)]
fn parse_comma_separated_list<P: SysMLParser, F>(p: &mut P, mut parse_item: F)
where
    F: FnMut(&mut P),
{
    parse_item(p);

    while p.at(SyntaxKind::COMMA) {
        bump_keyword(p);
        parse_item(p);
    }
}

/// Helper to parse inline send action (without final semicolon/body)
/// Used in contexts where send appears inside transitions, successions, etc.
fn parse_inline_send_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SEND_ACTION_USAGE);
    p.expect(SyntaxKind::SEND_KW);
    p.skip_trivia();

    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    parse_optional_via(p);
    parse_optional_to(p);

    p.finish_node();
}

/// Helper to parse accept trigger (used in transitions)
/// Parses: [payload [:Type]] [at/after/when <expr>] [via <port>]
fn parse_accept_trigger<P: SysMLParser>(p: &mut P) {
    // Payload name (but not if it's a trigger keyword)
    if (p.at_name_token() || p.at(SyntaxKind::LT))
        && !p.at(SyntaxKind::AT_KW)
        && !p.at(SyntaxKind::AFTER_KW)
        && !p.at(SyntaxKind::WHEN_KW)
        && !p.at(SyntaxKind::VIA_KW)
    {
        p.parse_identification();
        p.skip_trivia();
    }

    // Optional typing
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Optional trigger expression (at/after/when)
    if p.at(SyntaxKind::AT_KW) || p.at(SyntaxKind::AFTER_KW) || p.at(SyntaxKind::WHEN_KW) {
        bump_keyword(p);
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional via
    if p.at(SyntaxKind::VIA_KW) {
        bump_keyword(p);
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Helper to parse inline action declaration: action <name> {...}
fn parse_inline_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);
    p.expect(SyntaxKind::ACTION_KW);
    p.skip_trivia();
    if p.at_name_token() {
        p.parse_identification();
        p.skip_trivia();
    }
    parse_action_body(p);
    p.finish_node();
}

// =============================================================================
// SysML Body Parsing
// =============================================================================

/// Parse a SysML body (semicolon or braced block with SysML members)
/// Per pest: package_body = { semi_colon | ( "{" ~ package_body_items ~ "}" ) }
pub fn parse_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        while !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
            let start_pos = p.get_pos();

            parse_package_body_element(p);

            p.skip_trivia();

            // Recovery if no progress made
            if p.get_pos() == start_pos && !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}' ({})", text, p.current_kind().display_name())
                } else {
                    p.current_kind().display_name().to_string()
                };
                p.error(format!(
                    "unexpected {} in definition body—expected a member declaration",
                    got
                ));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        // Provide more context about what we found
        let found = if let Some(text) = p.current_token_text() {
            format!("'{}' ({})", text, p.current_kind().display_name())
        } else {
            "end of file".to_string()
        };
        p.error(format!(
            "expected ';' to end declaration or '{{' to start body, found {}",
            found
        ))
    }

    p.finish_node();
}

// =============================================================================
// SysML Package/Import/Alias (Per sysml.pest)
// =============================================================================

/// Parse a package
/// Per pest: package = { prefix_metadata? ~ package_declaration ~ package_body }
/// Per pest: package_declaration = { package_token ~ identification? }
/// Per pest: package_body = { semi_colon | ( "{" ~ package_body_items ~ "}" ) }
pub fn parse_package<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PACKAGE);

    p.expect(SyntaxKind::PACKAGE_KW);
    p.skip_trivia();

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Package body: ; or { ... }
    parse_namespace_body(p);

    p.finish_node();
}

/// Parse a library package
/// Per pest: library_package = { standard_token? ~ library_token ~ prefix_metadata? ~ package_declaration ~ package_body }
pub fn parse_library_package<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::LIBRARY_PACKAGE);

    // Optional 'standard'
    if p.at(SyntaxKind::STANDARD_KW) {
        p.bump();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::LIBRARY_KW);
    p.skip_trivia();

    p.expect(SyntaxKind::PACKAGE_KW);
    p.skip_trivia();

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Package body
    parse_namespace_body(p);

    p.finish_node();
}

/// Parse an import
/// Per pest: import = { (namespace_import | membership_import) ~ filter_package? ~ relationship_body }
/// Per pest: import_prefix = { visibility? ~ import_token ~ all_token? }
/// Per pest: imported_membership = { qualified_name }
/// Per pest: imported_namespace = { qualified_name ~ "::" ~ "*" ~ ("::*"*")? }
pub fn parse_import<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    p.expect(SyntaxKind::IMPORT_KW);
    p.skip_trivia();

    // Optional 'all'
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Qualified name
    p.parse_qualified_name();
    p.skip_trivia();

    // Optional wildcards: ::* or ::**, or ::**::*
    while p.at(SyntaxKind::COLON_COLON) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::STAR_STAR) {
            // Recursive wildcard: **
            p.bump();
            p.skip_trivia();
        } else if p.at(SyntaxKind::STAR) {
            // Simple wildcard: *
            p.bump();
            p.skip_trivia();
        } else {
            break;
        }
    }

    // Optional filter package: [@filter]
    if p.at(SyntaxKind::L_BRACKET) {
        parse_filter_package(p);
        p.skip_trivia();
    }

    // Relationship body: ; or { ... }
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "import statement");
    }

    p.finish_node();
}

/// Parse an alias
/// Per pest: alias_member_element = { visibility? ~ alias_token ~ identification? ~ for_token ~ element_reference ~ relationship_body }
pub fn parse_alias<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ALIAS_MEMBER);

    p.expect(SyntaxKind::ALIAS_KW);
    p.skip_trivia();

    // Optional identification (alias name)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // 'for' keyword
    p.expect(SyntaxKind::FOR_KW);
    p.skip_trivia();

    // Element reference (qualified name)
    p.parse_qualified_name();
    p.skip_trivia();

    // Relationship body: ; or { ... }
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "alias declaration");
    }

    p.finish_node();
}

/// Parse a namespace body: ; or { members* }
fn parse_namespace_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        error_missing_body_terminator(p, "declaration");
    }
}

/// Parse filter package: [@expression]
fn parse_filter_package<P: SysMLParser>(p: &mut P) {
    if !p.at(SyntaxKind::L_BRACKET) {
        return;
    }

    p.start_node(SyntaxKind::FILTER_PACKAGE);

    while p.at(SyntaxKind::L_BRACKET) {
        p.bump(); // [
        p.skip_trivia();

        // Optional @ prefix
        if p.at(SyntaxKind::AT) {
            p.bump();
            p.skip_trivia();
        }

        // Filter expression (qualified name or expression)
        if p.at_name_token() {
            p.parse_qualified_name();
        }
        p.skip_trivia();

        p.expect(SyntaxKind::R_BRACKET);
        p.skip_trivia();
    }

    p.finish_node(); // FILTER_PACKAGE
}

// =============================================================================
// SysML File Entry Point
// =============================================================================

/// Parse a SysML source file
/// Per Pest: file = { SOI ~ root_namespace ~ EOI }
/// root_namespace = { package_body_element* }
pub fn parse_sysml_file<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SOURCE_FILE);

    while !p.at(SyntaxKind::ERROR) {
        // ERROR indicates EOF
        p.skip_trivia();
        if p.at(SyntaxKind::ERROR) {
            break;
        }
        let start_pos = p.get_pos();
        parse_package_body_element(p);

        // Safety: if we didn't make progress, skip the token to avoid infinite loop
        if p.get_pos() == start_pos {
            let got = if p.at(SyntaxKind::ERROR) {
                "end of file".to_string()
            } else if let Some(text) = p.current_token_text() {
                format!("'{}'", text)
            } else {
                p.current_kind().display_name().to_string()
            };
            p.error(format!("unexpected {} in top level", got));
            p.bump();
        }
    }

    p.finish_node();
}

/// Parse a SysML package body element
/// Per Pest grammar:
/// package_body_element = {
///     package | library_package | import | alias_member_element
///     | element_filter_member | visible_annotating_member
///     | usage_member | definition_member_element
///     | relationship_member_element | dependency
/// }\n/// Per pest: package_body_item = { (metadata_usage | visibility_prefix? ~ (package_member | import_alias)) ~ \";\"? }\n/// Per pest: package_member = { definition | usage | alias_member | annotation_element | ... }\n/// Pattern: Dispatch to appropriate parser based on current token (package, import, def, usage keywords, annotations, etc.)
///
/// Pattern: Dispatch to appropriate parser based on current token (package, import, def, usage keywords, annotations, etc.)
pub fn parse_package_body_element<P: SysMLParser>(p: &mut P) {
    p.skip_trivia();

    // Handle visibility prefix
    if p.at_any(&[
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        bump_keyword(p);
    }

    // Handle prefix metadata (#name)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    match p.current_kind() {
        // Package
        SyntaxKind::PACKAGE_KW => parse_package(p),
        SyntaxKind::LIBRARY_KW | SyntaxKind::STANDARD_KW => parse_library_package(p),

        // Import/Alias
        SyntaxKind::IMPORT_KW => parse_import(p),
        SyntaxKind::ALIAS_KW => parse_alias(p),

        // Dependency (SysML-specific)
        SyntaxKind::DEPENDENCY_KW => p.parse_dependency(),

        // Annotating member
        SyntaxKind::COMMENT_KW | SyntaxKind::DOC_KW | SyntaxKind::LOCALE_KW => {
            parse_annotation(p);
        }

        // Filter (SysML-specific)
        SyntaxKind::FILTER_KW => p.parse_filter(),

        // Metadata usage (SysML-specific)
        SyntaxKind::AT => p.parse_metadata_usage(),

        // Prefix keywords that can precede definitions or usages
        SyntaxKind::ABSTRACT_KW
        | SyntaxKind::VARIATION_KW
        | SyntaxKind::DERIVED_KW
        | SyntaxKind::READONLY_KW
        | SyntaxKind::CONSTANT_KW
        | SyntaxKind::VAR_KW
        | SyntaxKind::COMPOSITE_KW
        | SyntaxKind::PORTION_KW
        | SyntaxKind::IN_KW
        | SyntaxKind::OUT_KW
        | SyntaxKind::INOUT_KW
        | SyntaxKind::END_KW
        | SyntaxKind::INDIVIDUAL_KW => {
            p.parse_definition_or_usage();
        }

        // ACTION_KW needs lookahead to distinguish:
        // - action def/usage: action name { ... } or action def name { ... }
        // - action node with send/accept: action name send { ... }
        SyntaxKind::ACTION_KW => {
            // Look ahead to check for send/accept/perform after name
            let (_, after_name) = peek_past_optional_name(p, 1);
            // Check if it's a send/accept/perform action node
            if after_name == SyntaxKind::SEND_KW {
                bump_keyword(p); // action
                p.parse_identification(); // name
                p.skip_trivia();
                parse_send_action(p);
                return;
            } else if after_name == SyntaxKind::ACCEPT_KW {
                bump_keyword(p); // action
                p.parse_identification(); // name
                p.skip_trivia();
                parse_accept_action(p);
                return;
            } else if after_name == SyntaxKind::PERFORM_KW {
                bump_keyword(p); // action
                p.parse_identification(); // name
                p.skip_trivia();
                parse_perform_action(p);
                return;
            }
            // Otherwise, it's a regular action definition or usage
            p.parse_definition_or_usage();
        }

        // DEF_KW alone (e.g., after metadata: #service def Name)
        SyntaxKind::DEF_KW => {
            p.parse_definition_or_usage();
        }

        // SysML definition/usage keywords (can be def or usage)
        // Note: INDIVIDUAL_KW handled as prefix keyword above
        // Note: OCCURRENCE_KW can be standalone usage (not prefix)
        SyntaxKind::PART_KW
        | SyntaxKind::ATTRIBUTE_KW
        | SyntaxKind::PORT_KW
        | SyntaxKind::ITEM_KW
        | SyntaxKind::STATE_KW
        | SyntaxKind::OCCURRENCE_KW
        | SyntaxKind::CONSTRAINT_KW
        | SyntaxKind::REQUIREMENT_KW
        | SyntaxKind::CASE_KW
        | SyntaxKind::CALC_KW
        | SyntaxKind::CONNECTION_KW
        | SyntaxKind::INTERFACE_KW
        | SyntaxKind::ALLOCATION_KW
        | SyntaxKind::VIEW_KW
        | SyntaxKind::VIEWPOINT_KW
        | SyntaxKind::RENDERING_KW
        | SyntaxKind::METADATA_KW
        | SyntaxKind::ENUM_KW
        | SyntaxKind::ANALYSIS_KW
        | SyntaxKind::VERIFICATION_KW
        | SyntaxKind::USE_KW
        | SyntaxKind::CONCERN_KW
        | SyntaxKind::PARALLEL_KW
        | SyntaxKind::EVENT_KW
        | SyntaxKind::MESSAGE_KW
        | SyntaxKind::SNAPSHOT_KW
        | SyntaxKind::TIMESLICE_KW
        | SyntaxKind::ABOUT_KW
        // KerML definition keywords (for standard library and KerML interop)
        | SyntaxKind::CLASS_KW
        | SyntaxKind::STRUCT_KW
        | SyntaxKind::DATATYPE_KW
        | SyntaxKind::BEHAVIOR_KW
        | SyntaxKind::FUNCTION_KW
        | SyntaxKind::ASSOC_KW
        | SyntaxKind::INTERACTION_KW
        | SyntaxKind::PREDICATE_KW
        | SyntaxKind::METACLASS_KW
        | SyntaxKind::CLASSIFIER_KW
        | SyntaxKind::TYPE_KW => {
            p.parse_definition_or_usage();
        }

        // Frame and Render (may be followed by keyword like 'frame concern c1' or 'render rendering r1')
        SyntaxKind::FRAME_KW => parse_frame_usage(p),
        SyntaxKind::RENDER_KW => parse_render_usage(p),

        // REF_KW: check if it's followed by :>> (shorthand redefines) or not
        SyntaxKind::REF_KW => {
            // Look ahead to check for :>> or :>
            let lookahead = skip_trivia_lookahead(p, 1);
            if matches!(
                p.peek_kind(lookahead),
                SyntaxKind::COLON_GT_GT | SyntaxKind::COLON_GT
            ) {
                // It's a shorthand redefines: ref :>> name
                p.parse_redefines_feature_member();
            } else {
                // It's a regular definition/usage with ref prefix
                p.parse_definition_or_usage();
            }
        }

        // Allocate usage
        SyntaxKind::ALLOCATE_KW => {
            parse_allocate_usage(p);
        }

        // Terminate action
        SyntaxKind::TERMINATE_KW => {
            parse_terminate_action(p);
        }

        // Flow: needs lookahead to distinguish flow def vs flow usage
        SyntaxKind::FLOW_KW => {
            // Look ahead to check for 'def' keyword
            let lookahead = skip_trivia_lookahead(p, 1);
            if p.peek_kind(lookahead) == SyntaxKind::DEF_KW {
                // flow def - it's a definition
                p.parse_definition_or_usage();
            } else {
                // flow usage - call SysML-specific parser
                parse_flow_usage(p);
            }
        }

        // Parameter keywords (IN_KW, OUT_KW, INOUT_KW already handled as prefix keywords above)
        // RETURN_KW can be either:
        // 1. Return parameter: return x : Type; or return : Type; (with optional name)
        // 2. Return expression: return a == b; (expression)
        SyntaxKind::RETURN_KW => {
            // Look ahead to distinguish: return <name>? : ... vs return <expr>
            let lookahead = skip_trivia_lookahead(p, 1);
            let after_return = p.peek_kind(lookahead);

            // If return is followed directly by colon, it's a parameter: return : Type
            if after_return == SyntaxKind::COLON || after_return == SyntaxKind::TYPED_KW {
                parse_sysml_parameter(p);
            } else if after_return == SyntaxKind::IDENT {
                let after_that = p.peek_kind(skip_trivia_lookahead(p, lookahead + 1));
                // If followed by name + colon/typing/default, it's a parameter declaration
                // EQ handles: return p = expr; (named result with default value)
                if after_that == SyntaxKind::COLON
                    || after_that == SyntaxKind::TYPED_KW
                    || after_that == SyntaxKind::L_BRACKET
                    || after_that == SyntaxKind::COLON_GT
                    || after_that == SyntaxKind::COLON_GT_GT
                    || after_that == SyntaxKind::SEMICOLON
                    || after_that == SyntaxKind::EQ
                {
                    parse_sysml_parameter(p);
                } else {
                    // return expression statement
                    parse_return_expression(p);
                }
            } else if is_usage_keyword(after_return) {
                // return part x; or return attribute y;
                parse_sysml_parameter(p);
            } else {
                // return expression statement
                parse_return_expression(p);
            }
        }

        // CONST_KW for end feature/parameter (END_KW already handled as prefix keyword above)
        SyntaxKind::CONST_KW => {
            parse_sysml_parameter(p);
        }

        // Connector
        SyntaxKind::CONNECTOR_KW => parse_connector_usage(p),

        // Action body elements (valid inside action definitions)
        SyntaxKind::PERFORM_KW => parse_perform_action(p),
        SyntaxKind::ACCEPT_KW => parse_accept_action(p),
        SyntaxKind::SEND_KW => parse_send_action(p),
        SyntaxKind::IF_KW => parse_if_action(p),
        SyntaxKind::WHILE_KW | SyntaxKind::LOOP_KW => parse_loop_action(p),
        SyntaxKind::FOR_KW => parse_for_loop(p),
        SyntaxKind::FIRST_KW => parse_first_action(p),
        SyntaxKind::THEN_KW => parse_then_succession(p),
        SyntaxKind::ELSE_KW => parse_else_succession(p),
        SyntaxKind::FORK_KW
        | SyntaxKind::JOIN_KW
        | SyntaxKind::MERGE_KW
        | SyntaxKind::DECIDE_KW => {
            parse_control_node(p);
        }

        // State body elements
        SyntaxKind::ENTRY_KW | SyntaxKind::EXIT_KW | SyntaxKind::DO_KW => {
            parse_state_subaction(p);
        }
        SyntaxKind::TRANSITION_KW => parse_transition(p),

        // Requirement body elements
        SyntaxKind::SUBJECT_KW => {
            // Check if this is a subject member declaration or shorthand redefine
            let lookahead = skip_trivia_lookahead(p, 1);
            if p.peek_kind(lookahead) == SyntaxKind::EQ
                || p.peek_kind(lookahead) == SyntaxKind::COLON_GT_GT
            {
                // It's a shorthand: subject = value; or subject :>> ref;
                p.parse_shorthand_feature_member();
            } else {
                // It's a subject member: subject v : V;
                parse_subject_usage(p);
            }
        }
        SyntaxKind::ACTOR_KW => {
            let (_, next) = peek_past_optional_name(p, 1);
            if next == SyntaxKind::EQ
                || next == SyntaxKind::COLON_GT_GT
                || next == SyntaxKind::COLON_GT
            {
                p.parse_shorthand_feature_member();
            } else {
                parse_actor_usage(p);
            }
        }
        SyntaxKind::STAKEHOLDER_KW => {
            let lookahead = skip_trivia_lookahead(p, 1);
            if p.peek_kind(lookahead) == SyntaxKind::EQ {
                p.parse_shorthand_feature_member();
            } else {
                parse_stakeholder_usage(p);
            }
        }
        SyntaxKind::OBJECTIVE_KW => {
            let lookahead = skip_trivia_lookahead(p, 1);
            if p.peek_kind(lookahead) == SyntaxKind::EQ {
                p.parse_shorthand_feature_member();
            } else {
                parse_objective_usage(p);
            }
        }
        SyntaxKind::ASSERT_KW => {
            // Check if followed by 'not' or 'satisfy' -> requirement verification
            // Otherwise -> requirement constraint
            let next = p.peek_kind(1);
            if next == SyntaxKind::NOT_KW || next == SyntaxKind::SATISFY_KW {
                parse_requirement_verification(p);
            } else {
                parse_requirement_constraint(p);
            }
        }
        SyntaxKind::ASSUME_KW | SyntaxKind::REQUIRE_KW => {
            parse_requirement_constraint(p);
        }
        SyntaxKind::NOT_KW | SyntaxKind::SATISFY_KW | SyntaxKind::VERIFY_KW => {
            parse_requirement_verification(p)
        }

        // Exhibit/Include
        SyntaxKind::EXHIBIT_KW => parse_exhibit_usage(p),
        SyntaxKind::INCLUDE_KW => parse_include_usage(p),

        // Connect/Binding/Succession/Bind/Assign
        SyntaxKind::CONNECT_KW => p.parse_connect_usage(),
        SyntaxKind::BINDING_KW | SyntaxKind::SUCCESSION_KW => p.parse_binding_or_succession(),
        SyntaxKind::BIND_KW => parse_bind_usage(p),
        SyntaxKind::ASSIGN_KW => parse_assign_action(p),

        // Standalone relationships
        // Per pest: Various standalone relationship keywords that create relationship elements
        SyntaxKind::SPECIALIZATION_KW
        | SyntaxKind::SUBCLASSIFIER_KW
        | SyntaxKind::REDEFINITION_KW
        | SyntaxKind::SUBSET_KW
        | SyntaxKind::TYPING_KW
        | SyntaxKind::CONJUGATION_KW
        | SyntaxKind::DISJOINING_KW
        | SyntaxKind::FEATURING_KW
        | SyntaxKind::INVERTING_KW
        | SyntaxKind::SUBTYPE_KW => {
            parse_standalone_relationship(p);
        }

        // Variant
        // Per pest: variant_membership = { variant_token ~ variant_usage_element }
        SyntaxKind::VARIANT_KW => p.parse_variant_usage(),

        // Expose (import/expose statement in views)
        // Per pest: expose = { expose_prefix ~ (namespace_expose | membership_expose) ~ filter_package? }
        SyntaxKind::EXPOSE_KW => parse_expose_statement(p),

        // Textual representation
        // Per pest: Textual representation with rep <name> language <string> pattern
        SyntaxKind::REP_KW | SyntaxKind::LANGUAGE_KW => parse_textual_representation(p),

        // Shorthand feature operators
        SyntaxKind::REDEFINES_KW
        | SyntaxKind::COLON_GT_GT
        | SyntaxKind::SUBSETS_KW
        | SyntaxKind::COLON_GT => p.parse_redefines_feature_member(),

        // Anonymous usage: `: Type;` - no name, just a colon and type
        // This is an anonymous feature typed by the given type
        SyntaxKind::COLON | SyntaxKind::TYPED_KW => {
            parse_anonymous_usage(p);
        }

        // Enum variant without name: = value;
        SyntaxKind::EQ => {
            // Parse as shorthand feature with just value assignment
            p.parse_shorthand_feature_member();
        }

        // Identifier - shorthand feature member
        SyntaxKind::IDENT => p.parse_shorthand_feature_member(),

        // Contextual keywords used as names (e.g., enum variants like 'done', 'closed')
        _ if p.at_name_token() => p.parse_shorthand_feature_member(),

        _ => {
            let got = if let Some(text) = p.current_token_text() {
                format!("'{}'", text)
            } else {
                p.current_kind().display_name().to_string()
            };
            p.error_recover(
                format!("unexpected {} in namespace body", got),
                &[
                    SyntaxKind::PACKAGE_KW,
                    SyntaxKind::PART_KW,
                    SyntaxKind::R_BRACE,
                ],
            );
        }
    }
}

/// Parse prefix metadata (#name)
/// Per pest: Prefix metadata appears as #identifier before various declarations
fn parse_prefix_metadata<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PREFIX_METADATA);
    expect_and_skip(p, SyntaxKind::HASH);
    if p.at_name_token() {
        p.bump();
    }
    p.finish_node();
}

// =============================================================================
// Action Body Elements
// =============================================================================

/// Parse perform action usage
/// Per pest: perform_action_usage = { perform_token ~ perform_action_usage_declaration ~ action_body }
/// Per pest: perform_action_usage_declaration = { (action_declaration_header | qualified_name) ~ feature_specialization_part? }
/// Per pest: action_declaration_header = { action_token ~ usage_declaration? }
/// Per pest: usage_declaration = { identification ~ multiplicity_part? ~ feature_specialization_part }
pub fn parse_perform_action<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE so it's recognized as a NamespaceMember
    p.start_node(SyntaxKind::USAGE);
    p.start_node(SyntaxKind::PERFORM_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::PERFORM_KW);

    // Check if followed by 'action' keyword (action_declaration_header)
    if p.at(SyntaxKind::ACTION_KW) {
        bump_keyword(p); // consume 'action'

        // Parse optional usage_declaration (identification, multiplicity, specialization_part)
        parse_optional_identification(p);

        // Optional multiplicity [*], [1..*], etc.
        parse_optional_multiplicity(p);

        // Optional specializations
        parse_specializations_with_skip(p);
    } else {
        // Otherwise just a qualified name - parse as specialization for relationship extraction
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        // Optional specializations (redefines, subsets, etc.)
        parse_specializations(p);
    }

    p.skip_trivia();
    p.parse_body();

    p.finish_node(); // PERFORM_ACTION_USAGE
    p.finish_node(); // USAGE
}

/// Parse frame usage
/// Per pest: Frame usage for requirement framing
/// Pattern: 'frame' [<keyword>] <name> ';'
pub fn parse_frame_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::FRAME_KW);

    // Check if followed by usage keyword (e.g., frame concern c1)
    if p.at_any(SYSML_USAGE_KEYWORDS) {
        bump_keyword(p);
    }

    // Parse identification
    parse_optional_identification(p);

    // Specializations
    parse_specializations_with_skip(p);

    p.parse_body();

    p.finish_node();
}

/// Parse render usage
/// Per pest: view_rendering_usage = { render_token ~ (rendering_usage_keyword ~ usage_declaration)? ~ semi_colon }
/// Pattern: 'render' [<keyword>] <name> [: Type] [multiplicity] ';'
pub fn parse_render_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::RENDER_KW);
    p.skip_trivia();

    // Check if followed by usage keyword (e.g., render rendering r1)
    if p.at_any(SYSML_USAGE_KEYWORDS) {
        p.bump();
        p.skip_trivia();
    }

    // Parse identification
    parse_optional_identification(p);

    // Typing
    parse_optional_typing(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    // Specializations
    parse_specializations_with_skip(p);

    p.parse_body();

    p.finish_node();
}

/// Parse accept action usage
/// Per pest: accept_node = { occurrence_usage_prefix ~ accept_node_declaration ~ action_body }
/// Per pest: accept_node_declaration = { action_node_usage_declaration? ~ accept_token ~ accept_parameter_part }
/// Per pest: accept_parameter_part = { payload_parameter_member ~ (via_token ~ node_parameter_member)? }
/// Per pest: payload_parameter = { (identification? ~ payload_feature_specialization_part? ~ trigger_value_part) | payload }
/// Per pest: trigger_expression = { time_trigger_kind ~ argument_member | change_trigger_kind ~ argument_expression_member }
pub fn parse_accept_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACCEPT_ACTION_USAGE);

    // Optional 'action' keyword before 'accept'
    if p.at(SyntaxKind::ACTION_KW) {
        bump_keyword(p);
        // Optional name after 'action'
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
            p.skip_trivia();
        }
    }

    expect_and_skip(p, SyntaxKind::ACCEPT_KW);

    // Check for 'via' first (accept via port pattern - no payload)
    // Otherwise parse optional payload and trigger
    if !p.at(SyntaxKind::VIA_KW) {
        parse_accept_trigger(p);
    } else {
        // Just parse via port
        bump_keyword(p);
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Body (optional if followed by 'then' transition in state context)
    // Also skip if followed by 'do' (effect) or 'if' (guard) in target transition context
    if p.at(SyntaxKind::THEN_KW) || p.at(SyntaxKind::DO_KW) || p.at(SyntaxKind::IF_KW) {
        // In state bodies, accept can be followed by target transition without a body
        // The transition will be parsed by parse_state_body_element
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse send action usage
/// Per pest: send_node = { occurrence_usage_prefix ~ send_node_declaration ~ action_body }
/// Per pest: send_node_declaration = { action_node_usage_declaration? ~ send_token ~ (action_body | (node_parameter_member ~ sender_receiver_part? | empty_parameter_member ~ sender_receiver_part) ~ action_body) }
/// Per pest: node_parameter_member = { owned_expression }
/// Per pest: sender_receiver_part = { via_token ~ ... | empty_parameter_member ~ to_token ~ ... }
pub fn parse_send_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SEND_ACTION_USAGE);

    p.expect(SyntaxKind::SEND_KW);
    p.skip_trivia();

    // Check if we have a body directly (pattern: send { ... })
    if p.at(SyntaxKind::L_BRACE) || p.at(SyntaxKind::SEMICOLON) {
        parse_action_body(p);
        p.finish_node();
        return;
    }

    // Check for sender_receiver_part directly (empty parameter member pattern)
    // Per pest: sender_receiver_part = { via_token ~ ... | empty_parameter_member ~ to_token ~ ... }
    // When via/to appears directly, skip the expression parsing
    if !p.at(SyntaxKind::VIA_KW) && !p.at(SyntaxKind::TO_KW) && p.can_start_expression() {
        // What to send (node_parameter_member = owned_expression)
        parse_expression(p);
        p.skip_trivia();
    }

    parse_optional_via(p);
    parse_optional_to(p);

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_action_body(p);
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse if action usage
/// Per pest: if_node = { occurrence_usage_prefix ~ if_node_parameter_member ~ action_body ~ (else_token ~ action_body_parameter)? }
/// Per pest: if_node_parameter_member = { if_token ~ argument_expression_member ~ (then_token? ~ target_succession_member)? }
pub fn parse_if_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IF_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::IF_KW);

    // Condition (parenthesized or not)
    if p.at(SyntaxKind::L_PAREN) {
        bump_keyword(p);
        parse_expression(p);
        p.skip_trivia();
        p.expect(SyntaxKind::R_PAREN);
    } else if p.can_start_expression() {
        parse_expression(p);
    }

    p.skip_trivia();

    // Check for 'then' keyword - this is a guarded succession, not a full if-action
    if p.at(SyntaxKind::THEN_KW) {
        // Pattern: if <expr> then <target>;
        bump_keyword(p); // then

        // Target (qualified name or inline action)
        if p.at(SyntaxKind::MERGE_KW)
            || p.at(SyntaxKind::DECIDE_KW)
            || p.at(SyntaxKind::JOIN_KW)
            || p.at(SyntaxKind::FORK_KW)
        {
            parse_control_node(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
            parse_accept_action(p);
        } else if p.at(SyntaxKind::SEND_KW) {
            parse_send_action(p);
        } else {
            p.parse_qualified_name();
            p.skip_trivia();
            expect_and_skip(p, SyntaxKind::SEMICOLON);
        }
    } else {
        // Standard if-action with body
        p.parse_body();

        p.skip_trivia();

        // Optional 'else'
        if consume_if(p, SyntaxKind::ELSE_KW) {
            p.parse_body();
        }
    }

    p.finish_node();
}

/// Parse loop/while action usage
/// Per pest: while_loop_node = { occurrence_usage_prefix ~ (while_token ~ argument_expression_member? | loop_token) ~ action_body ~ (until_token ~ argument_expression_member ~ semi_colon)? }
pub fn parse_loop_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::WHILE_LOOP_ACTION_USAGE);

    // 'while' or 'loop'
    bump_keyword(p);

    // Optional condition for 'while'
    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    p.parse_body();

    p.skip_trivia();

    // Optional 'until'
    if p.at(SyntaxKind::UNTIL_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse for loop action usage
/// Per pest: for_loop_node = { occurrence_usage_prefix ~ for_token ~ for_variable_declaration_member ~ in_token ~ node_parameter_member ~ action_body }
/// Per pest: for_variable_declaration_member = { for_variable_declaration }
/// Per pest: for_variable_declaration = { identification? }
pub fn parse_for_loop<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::FOR_LOOP_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::FOR_KW);

    // Loop variable
    parse_optional_identification(p);

    // 'in' keyword
    if p.at(SyntaxKind::IN_KW) {
        bump_keyword(p);
    }

    // Collection expression
    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    p.parse_body();

    p.finish_node();
}

/// Parse first action usage (initial succession)
/// Per pest: empty_succession = { first_token ~ empty_succession_member ~ (then_token ~ empty_succession_member)? ~ semi_colon }
/// Pattern: 'first' [mult]? TargetRef ('then' [mult]? TargetRef)? (';' | '{' '}')
pub fn parse_first_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::FIRST_KW);

    // First endpoint wrapped in SUCCESSION_ITEM
    p.start_node(SyntaxKind::SUCCESSION_ITEM);

    // Optional multiplicity before first endpoint
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    p.parse_qualified_name();
    p.finish_node(); // SUCCESSION_ITEM
    p.skip_trivia();

    // Optional 'then' clause
    if p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();

        // Second endpoint wrapped in SUCCESSION_ITEM
        p.start_node(SyntaxKind::SUCCESSION_ITEM);

        // Optional multiplicity before second endpoint
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        p.parse_qualified_name();
        p.finish_node(); // SUCCESSION_ITEM
        p.skip_trivia();
    }

    // Body (semicolon or braces)
    p.parse_body();

    p.finish_node();
}

/// Parse then succession
/// Per pest: action_target_succession = { target_succession | guarded_target_succession | default_target_succession }
/// Per pest: target_succession = { empty_succession_member ~ then_token ~ target_succession_member ~ usage_body }
/// Pattern: 'then' TargetRef ';'
pub fn parse_then_succession<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::THEN_KW);

    // After 'then', we can have:
    // 1. A control node: merge m;, decide;, join j;, fork f;
    // 2. An action node: accept ..., send ..., etc.
    // 3. An inline action: action <name> {...}
    // 4. An inline state: state <name> {...}
    // 5. A qualified name reference: then someAction;

    // Check for inline action or state: then action/state <name> {...}
    // Also handle prefixed actions and assign actions
    if p.at(SyntaxKind::ACTION_KW)
        || p.at(SyntaxKind::STATE_KW)
        || p.at(SyntaxKind::PRIVATE_KW)
        || p.at(SyntaxKind::PROTECTED_KW)
        || p.at(SyntaxKind::ABSTRACT_KW)
        || p.at(SyntaxKind::READONLY_KW)
        || p.at(SyntaxKind::DERIVED_KW)
        || p.at(SyntaxKind::ASSIGN_KW)
    {
        parse_package_body_element(p);
        p.skip_trivia();

        // After inline action/state, check for additional target successions (then X, then Y)
        // Per pest grammar: behavior_usage_member ~ target_succession_member*
        while p.at(SyntaxKind::THEN_KW) {
            bump_keyword(p); // then

            // Parse the target (send/accept/action/etc.)
            // NOTE: In succession chaining, semicolon comes after entire chain, not after each target
            if p.at(SyntaxKind::ACTION_KW) || p.at(SyntaxKind::STATE_KW) {
                // Inline action or state in chain: then action name;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::PRIVATE_KW)
                || p.at(SyntaxKind::PROTECTED_KW)
                || p.at(SyntaxKind::ABSTRACT_KW)
                || p.at(SyntaxKind::READONLY_KW)
                || p.at(SyntaxKind::DERIVED_KW)
            {
                // Prefix keyword followed by action: then private action name;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::ASSIGN_KW) {
                // Inline assign action: then assign x := y;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::SEND_KW) {
                // Parse send inline without semicolon expectation
                parse_inline_send_action(p);
            } else if p.at(SyntaxKind::ACCEPT_KW) {
                parse_accept_action(p);
            } else if p.at(SyntaxKind::PERFORM_KW) {
                parse_perform_action(p);
            } else if p.at_name_token() {
                p.start_node(SyntaxKind::SUCCESSION_ITEM);
                p.parse_qualified_name();
                p.finish_node();
                p.skip_trivia();
                p.expect(SyntaxKind::SEMICOLON);
            }
            p.skip_trivia();
        }
        // After the succession chain, expect a semicolon
        // (The semicolon comes after the entire chain, not after each element)
        if !p.at(SyntaxKind::SEMICOLON) {
            // Already consumed by the last element
        } else {
            p.expect(SyntaxKind::SEMICOLON);
        }
    }
    // Check for event occurrence: then event occurrence <name>
    else if p.at(SyntaxKind::EVENT_KW) {
        parse_package_body_element(p);
        p.skip_trivia();
    }
    // Check for control node keywords
    else if p.at(SyntaxKind::MERGE_KW)
        || p.at(SyntaxKind::DECIDE_KW)
        || p.at(SyntaxKind::JOIN_KW)
        || p.at(SyntaxKind::FORK_KW)
    {
        parse_control_node(p);
    }
    // Check for action nodes
    else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else if p.at(SyntaxKind::PERFORM_KW) {
        parse_perform_action(p);
    } else if p.at(SyntaxKind::IF_KW) {
        parse_if_action(p);
    } else if p.at(SyntaxKind::WHILE_KW) || p.at(SyntaxKind::LOOP_KW) {
        parse_loop_action(p);
    } else if p.at(SyntaxKind::FOR_KW) {
        parse_for_loop(p);
    } else if p.at(SyntaxKind::TERMINATE_KW) {
        bump_keyword(p); // terminate
        // Optional target name
        if p.at_name_token() {
            p.start_node(SyntaxKind::SUCCESSION_ITEM);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
        p.expect(SyntaxKind::SEMICOLON);
    }
    // Otherwise it's a reference - wrap in SUCCESSION_ITEM
    else {
        p.start_node(SyntaxKind::SUCCESSION_ITEM);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        // Handle optional 'after' clause: then step2 after trigger1;
        // This creates a guarded succession where step2 happens after trigger1 completes
        if p.at(SyntaxKind::AFTER_KW) {
            p.bump(); // after
            p.skip_trivia();
            // Parse the event/trigger reference (can be a chain like step1.done)
            p.start_node(SyntaxKind::SUCCESSION_ITEM);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }

        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse terminate action
/// Per pest: terminate_node = { terminate_token ~ target_succession_member ~ semi_colon }
/// Pattern: terminate [<target>] ;
pub fn parse_terminate_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONTROL_NODE); // or create TERMINATE_ACTION_USAGE if needed

    expect_and_skip(p, SyntaxKind::TERMINATE_KW);

    // Optional target name
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// Parse else succession (default target succession)
/// Per pest: default_target_succession = { else_token ~ target_succession_member ~ usage_body }
/// Pattern: else <target>;
pub fn parse_else_succession<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::ELSE_KW);

    // Target (qualified name or inline action/control node)
    if p.at(SyntaxKind::MERGE_KW)
        || p.at(SyntaxKind::DECIDE_KW)
        || p.at(SyntaxKind::JOIN_KW)
        || p.at(SyntaxKind::FORK_KW)
    {
        parse_control_node(p);
    } else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else {
        p.parse_qualified_name();
        p.skip_trivia();
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse control node (fork, join, merge, decide)
/// Per pest: control_node = { control_node_prefix? ~ (merge_node | decision_node | join_node | fork_node) }
/// Per pest: merge_node = { merge_token ~ identification? ~ action_body }
/// Per pest: decision_node = { decide_token ~ identification? ~ action_body }
/// Per pest: join_node = { join_token ~ identification? ~ action_body }
/// Per pest: fork_node = { fork_token ~ identification? ~ action_body }
/// Pattern: ('fork' | 'join' | 'merge' | 'decide') Identification? Body
pub fn parse_control_node<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONTROL_NODE);

    // Control keyword
    bump_keyword(p);

    // Optional name
    parse_optional_identification(p);

    p.parse_body();

    p.finish_node();
}

/// Parse action body (for action definitions and action usages)
/// Per pest: action_body = { semi_colon | (forward_curl_brace ~ action_body_item* ~ backward_curl_brace) }
/// Per pest: action_body_item can include: directed_parameter_member, structure_usage_member, behavior_usage_member,
///           action_node_member, initial_node_member, etc.
pub fn parse_action_body<P: SysMLParser>(p: &mut P) {
    p.skip_trivia();

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        return;
    }

    // Start NAMESPACE_BODY node so members can be extracted
    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_package_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node(); // NAMESPACE_BODY
}

/// Parse state body (for state usages)
/// Per pest: state_usage_body = { semi_colon | (parallel_marker? ~ forward_curl_brace ~ state_body_part ~ backward_curl_brace) }
/// Per pest: state_body_part = { state_body_item* }
/// Pattern: ";" | parallel? "{" state_body_part "}"
pub fn parse_state_body<P: SysMLParser>(p: &mut P) {
    p.skip_trivia();

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    // Optional 'parallel' marker before body
    if p.at(SyntaxKind::PARALLEL_KW) {
        p.bump();
        p.skip_trivia();
    }

    if !p.at(SyntaxKind::L_BRACE) {
        return;
    }

    // Start NAMESPACE_BODY node so members can be extracted
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_state_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);

    p.finish_node(); // NAMESPACE_BODY
}

/// Parse a state body element
/// Per pest: state_body_item includes: entry_action_member, do_action_member, exit_action_member,
///           entry_transition_member, transition_usage_member, target_transition_usage_member,
///           behavior_usage_member, and more
/// Per pest: behavior_usage_member ~ target_transition_usage_member*
/// This means after accept/action/etc., we can have "then target;" transitions
/// BUT: entry/do/exit subactions are standalone and don't have transitions after
fn parse_state_body_element<P: SysMLParser>(p: &mut P) {
    // Check if this is a standalone state subaction (entry/do/exit)
    // These are complete statements and should NOT be followed by target transitions
    let is_state_subaction =
        p.at(SyntaxKind::ENTRY_KW) || p.at(SyntaxKind::DO_KW) || p.at(SyntaxKind::EXIT_KW);

    // Parse the main element (could be transition, state, accept, do, etc.)
    parse_package_body_element(p);
    p.skip_trivia();

    // Only check for target transitions if this was NOT a state subaction
    // State subactions (entry/do/exit) are standalone per the pest grammar
    if !is_state_subaction {
        // After behavior usages, check for target transitions
        // Target transitions can start with: accept, if, do, or then
        while p.at(SyntaxKind::THEN_KW)
            || p.at(SyntaxKind::ACCEPT_KW)
            || p.at(SyntaxKind::IF_KW)
            || p.at(SyntaxKind::DO_KW)
        {
            parse_target_transition(p);
            p.skip_trivia();
        }
    }
}

/// Parse target transition usage
/// Per pest grammar:
/// target_transition_usage = empty_parameter_member
///   ~ (transition_usage_keyword ~ ... | trigger_action_member ~ ... | guard_expression_member ~ ...)?\n///   ~ then_token ~ transition_succession_member ~ action_body\n/// This handles: [accept ...] [if expr] [do action] then target;\n/// Per pest: target_transition_usage = { target_transition_usage_declaration ~ transition_succession_block }\n/// Per pest: target_transition_usage_declaration = { (trigger_action_member? ~ guard_expression_member? ~ effect_behavior_member?)? ~ then_token ~ transition_succession_member }\n/// Pattern: [accept <trigger>] [if <guard>] [do <effect>] then <target> [body]
fn parse_target_transition<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TRANSITION_USAGE);

    let has_prefix_keywords =
        p.at(SyntaxKind::ACCEPT_KW) || p.at(SyntaxKind::IF_KW) || p.at(SyntaxKind::DO_KW);

    // Optional trigger: accept <payload> [at/after/when <expr>] [via <port>]
    if p.at(SyntaxKind::ACCEPT_KW) {
        p.bump(); // accept
        p.skip_trivia();

        // Payload name (but not if it's a trigger keyword)
        if (p.at_name_token() || p.at(SyntaxKind::LT))
            && !p.at(SyntaxKind::AT_KW)
            && !p.at(SyntaxKind::AFTER_KW)
            && !p.at(SyntaxKind::WHEN_KW)
            && !p.at(SyntaxKind::VIA_KW)
        {
            p.parse_identification();
            p.skip_trivia();
        }

        // Optional typing
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Optional trigger expression (at/after/when)
        if p.at(SyntaxKind::AT_KW) || p.at(SyntaxKind::AFTER_KW) || p.at(SyntaxKind::WHEN_KW) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }

        // Optional via
        if p.at(SyntaxKind::VIA_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
            p.skip_trivia();
        }
    }

    // Optional guard: if <expression>
    if p.at(SyntaxKind::IF_KW) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional effect: do <action>
    if consume_if(p, SyntaxKind::DO_KW) {
        // Effect can be a performed action reference, send, accept, or assignment
        // NOTE: In target transition context, these don't have semicolons
        if p.at(SyntaxKind::SEND_KW) {
            parse_inline_send_action(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
            parse_accept_action(p);
        } else if p.at(SyntaxKind::ASSIGN_KW) {
            bump_keyword(p);
            p.parse_qualified_name();
            p.skip_trivia();
            if p.at(SyntaxKind::COLON_EQ) {
                bump_keyword(p);
                parse_expression(p);
            }
        } else if p.at(SyntaxKind::ACTION_KW) {
            parse_inline_action(p);
        } else if p.at_name_token() {
            // Typed reference (action name)
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }

    // 'then' target is required per grammar
    // If we don't have it but we had prefix keywords, it's a malformed target transition
    // If we don't have prefix keywords and no THEN, this shouldn't have been called
    if !p.at(SyntaxKind::THEN_KW) {
        if has_prefix_keywords {
            p.error("expected 'then' after transition trigger/guard/effect");
        }
        // Finish early if malformed
        p.finish_node();
        return;
    }

    p.expect(SyntaxKind::THEN_KW);
    p.skip_trivia();

    // Optional 'state' keyword before target
    if p.at(SyntaxKind::STATE_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Target (succession member - can be a state name or qualified name)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Semicolon or body
    p.parse_body();

    p.finish_node();
}

// =============================================================================
// State Body Elements
// =============================================================================

/// StateSubaction = ('entry' | 'do' | 'exit') Identification? Body\n/// Per pest: entry_transition_member = { \"entry\" ~ (entry_transition_member_declaration|semi_colon) }\n/// Per pest: do_behavior_member = { \"do\" ~ (behavior_usage_member_declaration|semi_colon) }\n/// Per pest: exit_transition_member = { \"exit\" ~ (exit_transition_member_declaration|semi_colon) }\n/// Pattern: entry|do|exit [assign|send|accept|action|<name>] [body|semicolon]
pub fn parse_state_subaction<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STATE_SUBACTION);

    // entry/do/exit keyword
    bump_keyword(p);

    // State action usage can be:
    // - assignment: assign target := expr ;
    // - send: send expr [via expr] [to expr] ;
    // - accept: accept ...
    // - action: action [name] { ... }
    // - identifier [body or ;]
    // - qualified_name ;
    // - ;

    if p.at(SyntaxKind::ASSIGN_KW) {
        parse_assign_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::ACTION_KW) {
        // action [name] [: Type] [:>> ref, ...] body
        // Per pest: action_keyword ~ (identifier ~ semi_colon | usage_declaration? ~ action_body)
        // where usage_declaration includes typing and specializations
        p.bump(); // action
        p.skip_trivia();
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
            p.skip_trivia();
        }

        // Typing
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Specializations
        parse_specializations(p);
        p.skip_trivia();

        // Multiplicity (rarely, but per spec)
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        parse_action_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at_name_token() {
        // Could be: identifier ; or identifier [: Type] [:> ref, ...] { ... } or semicolon
        // Pattern: do myAction : ActionType { ... }
        p.parse_identification();
        p.skip_trivia();

        // Optional typing (: Type)
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Optional specializations (:>, :>>, etc.)
        parse_specializations(p);
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::L_BRACE) {
            parse_action_body(p);
        } else if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }
        // else: no body, no semicolon - might be valid shorthand
    }

    p.finish_node();
}

/// TransitionUsage per Pest grammar:
/// transition_usage = transition_usage_keyword
///   ~ (usage_declaration ~ (first_token ~ transition_source_member | transition_source_member)
///     | first_token ~ transition_source_member
///     | transition_source_member)
///   ~ empty_parameter_member
///   ~ (empty_parameter_member ~ trigger_action_member)?  // accept trigger
///   ~ guard_expression_member?                          // if guard
///   ~ effect_behavior_member?                           // do effect
///   ~ then_token ~ transition_succession_member
///   ~ action_body
/// Per pest: transition_usage = { (transition_usage_declaration | first_node) ~ transition_succession_block }
/// Per pest: transition_succession = { succession_as_usage | transition_feature_membership }
/// Pattern: transition [name] [first] <source>? accept [trigger] [if guard] [do effect] then <target> body
pub fn parse_transition<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);
    p.start_node(SyntaxKind::TRANSITION_USAGE);

    p.expect(SyntaxKind::TRANSITION_KW);
    p.skip_trivia();

    // Optional usage declaration (transition name)
    // Per pest: usage_declaration ~ (first_token ~ transition_source_member | transition_source_member)
    // Heuristic: if we see a name that's NOT 'first', and peek shows 'first' or newline after it, it's a name
    if p.at_name_token() && !p.at(SyntaxKind::FIRST_KW) {
        // Check if next token (after skipping this name) is 'first'
        // If so, this is a transition name, not the source
        let is_transition_name = p.peek_kind(1) == SyntaxKind::FIRST_KW
            || p.peek_kind(1) == SyntaxKind::WHITESPACE && p.peek_kind(2) == SyntaxKind::FIRST_KW;

        if is_transition_name {
            p.parse_identification();
            p.skip_trivia();
        }
    }

    // Optional 'first' keyword
    if p.at(SyntaxKind::FIRST_KW) {
        bump_keyword(p);
    }

    // Source state (transition_source_member) - wrap in SPECIALIZATION for type_ref extraction
    if p.at_name_token()
        && !p.at(SyntaxKind::ACCEPT_KW)
        && !p.at(SyntaxKind::IF_KW)
        && !p.at(SyntaxKind::DO_KW)
        && !p.at(SyntaxKind::THEN_KW)
    {
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();
    }

    // Optional trigger: accept <payload> [at/after/when <expr>] [via <port>]
    if p.at(SyntaxKind::ACCEPT_KW) {
        p.bump(); // accept
        p.skip_trivia();

        // Payload name (but not if it's a trigger keyword)
        if (p.at_name_token() || p.at(SyntaxKind::LT))
            && !p.at(SyntaxKind::AT_KW)
            && !p.at(SyntaxKind::AFTER_KW)
            && !p.at(SyntaxKind::WHEN_KW)
            && !p.at(SyntaxKind::VIA_KW)
        {
            p.parse_identification();
            p.skip_trivia();
        }

        // Optional typing
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Optional trigger expression (at/after/when)
        if p.at(SyntaxKind::AT_KW) || p.at(SyntaxKind::AFTER_KW) || p.at(SyntaxKind::WHEN_KW) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }

        // Optional via
        if p.at(SyntaxKind::VIA_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
            p.skip_trivia();
        }
    }

    // Optional guard: if <expression>
    if consume_if(p, SyntaxKind::IF_KW) {
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional effect: do <action>
    if consume_if(p, SyntaxKind::DO_KW) {
        // Effect can be a performed action, send, accept, or assignment
        // NOTE: In transition context, these don't have semicolons - the semicolon comes after 'then'
        if p.at(SyntaxKind::SEND_KW) {
            parse_inline_send_action(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
            // parse_accept_action handles no-semicolon case already
            parse_accept_action(p);
        } else if p.at(SyntaxKind::ASSIGN_KW) {
            bump_keyword(p);
            p.parse_qualified_name();
            p.skip_trivia();
            if p.at(SyntaxKind::COLON_EQ) {
                bump_keyword(p);
                parse_expression(p);
            }
        } else if p.at(SyntaxKind::ACTION_KW) {
            parse_inline_action(p);
        } else if p.at_name_token() {
            // Typed reference (action name)
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }

    // 'then' target state - wrap in SPECIALIZATION for type_ref extraction
    if p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
    }

    p.finish_node(); // TRANSITION_USAGE

    // Body (action_body)
    p.parse_body();

    p.finish_node(); // USAGE
}

// =============================================================================
// Requirement Body Elements
// =============================================================================

/// SubjectUsage = 'subject' Identification? Typing? ';'
/// Per pest: requirement_subject_usage = { requirement_subject_usage_declaration ~ (";"|requirement_body) }
/// Per pest: requirement_subject_usage_declaration = { subject_prefix? ~ usage_declaration }
/// Pattern: subject <name>? <typing>? <specializations>? <multiplicity>? <default>? <body|semicolon>
pub fn parse_subject_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUBJECT_USAGE);

    p.expect(SyntaxKind::SUBJECT_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    // Typing (optional, can come before or after specializations)
    parse_optional_typing(p);

    // Specializations (redefines, subsets, etc.)
    parse_specializations_with_skip(p);

    // Multiplicity (can come after specializations)
    parse_optional_multiplicity(p);

    // Default value (with 'default' keyword or '=' operator)
    parse_optional_default_value(p);

    p.parse_body();

    p.finish_node();
}

/// ActorUsage = 'actor' Identification? Typing? ';'
/// Per pest: requirement_actor_member = { requirement_actor_member_declaration ~ value_part? ~ multiplicity_part? ~ (";"|requirement_body) }
/// Per pest: requirement_actor_member_declaration = { "actor" ~ usage_declaration? }
/// Pattern: actor <name>? <typing>? <specializations>? <multiplicity>? <default>? semicolon
pub fn parse_actor_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACTOR_USAGE);

    p.expect(SyntaxKind::ACTOR_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    // Specializations (redefines, subsets, etc.)
    parse_specializations_with_skip(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    // Default value
    parse_optional_default_value(p);

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// StakeholderUsage = 'stakeholder' Identification? Typing? ';'
/// Per pest: requirement_stakeholder_member = { "stakeholder" ~ usage_declaration? ~ (";"|requirement_body) }
/// Pattern: stakeholder <name>? <typing>? semicolon
pub fn parse_stakeholder_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STAKEHOLDER_USAGE);

    p.expect(SyntaxKind::STAKEHOLDER_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// ObjectiveUsage = 'objective' Identification? [: Type] [:>> ref, ...] Body
/// Per pest: requirement_objective_member = { "objective" ~ usage_declaration? ~ requirement_body }
/// Pattern: objective <name>? <typing>? <specializations>? <multiplicity>? body
pub fn parse_objective_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::OBJECTIVE_USAGE);

    p.expect(SyntaxKind::OBJECTIVE_KW);
    p.skip_trivia();

    parse_optional_identification(p);

    parse_optional_typing(p);

    // Specializations (per pest: constraint_usage_declaration = usage_declaration? ~ value_part?)
    parse_specializations_with_skip(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    p.parse_body();

    p.finish_node();
}

/// RequirementConstraint = ('assert' | 'assume' | 'require') 'constraint'? Identification? ConstraintBody
/// Per pest: requirement_constraint_member = { constraint_prefix? ~ metadata_prefix* ~ "constraint" ~ usage_declaration? ~ value_part? ~ constraint_body }
/// Per pest: constraint_prefix = { ("assert"|"assume"|"require") }
/// Pattern: assert|assume|require [#metadata] [constraint] <name>? <typing|specializations>? <body|semicolon>
pub fn parse_requirement_constraint<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE node so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);

    // Also wrap in REQUIREMENT_CONSTRAINT for semantic info
    p.start_node(SyntaxKind::REQUIREMENT_CONSTRAINT);

    // assert/assume/require
    p.bump();
    p.skip_trivia();

    // Prefix metadata (e.g., assume #goal constraint)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    // Optional 'constraint' keyword - bump it to set usage kind
    let has_constraint_kw = p.at(SyntaxKind::CONSTRAINT_KW);
    if has_constraint_kw {
        p.bump();
        p.skip_trivia();
    }

    p.finish_node(); // REQUIREMENT_CONSTRAINT

    // Optional name or reference
    // When 'constraint' keyword present: parse as identification (defining new constraint)
    // When no 'constraint' keyword: parse as qualified name (referencing existing requirement)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        if has_constraint_kw {
            p.parse_identification();
        } else {
            // Reference to existing requirement (allow feature chains like X.Y)
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }

    // Optional typing/specializations (e.g., "assume constraint c1 : C;" or "require constraint c1 :>> c;")
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) || p.at(SyntaxKind::COLON_GT_GT) {
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        } else {
            // Handle redefines/subsets as SPECIALIZATION
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.bump(); // :> or :>>
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
            p.finish_node();
            p.skip_trivia();
        }
    }

    // Body: can be constraint body {...} or just semicolon
    if p.at(SyntaxKind::L_BRACE) || p.at(SyntaxKind::L_PAREN) {
        p.parse_constraint_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node(); // USAGE
}

/// RequirementVerification = ('assert'? 'not'? 'satisfy' | 'verify') 'requirement'? Identification? ('by' QualifiedName)? ';'
/// Per pest: requirement_verification_member = { satisfy_requirement_usage | verify_requirement_usage }
/// Per pest: satisfy_requirement_usage = { "assert"? ~ "not"? ~ "satisfy" ~ "requirement"? ~ usage_declaration? ~ value_part? ~ (";"|requirement_body) }
/// Per pest: verify_requirement_usage = { "verify" ~ "requirement"? ~ usage_declaration? ~ ("by" ~ qualified_name)? ~ (";"|requirement_body) }
/// Pattern: [assert] [not] satisfy|verify [requirement] <name|typing>? [by <verifier>]? <body|semicolon>
pub fn parse_requirement_verification<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE node so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);

    p.start_node(SyntaxKind::REQUIREMENT_VERIFICATION);

    // Optional 'assert' modifier
    consume_if(p, SyntaxKind::ASSERT_KW);

    // Optional 'not' modifier
    consume_if(p, SyntaxKind::NOT_KW);

    // satisfy/verify
    if p.at(SyntaxKind::SATISFY_KW) || p.at(SyntaxKind::VERIFY_KW) {
        bump_keyword(p);
    }

    // Optional 'requirement' keyword
    consume_if(p, SyntaxKind::REQUIREMENT_KW);

    // Target: can be usage declaration (name : Type), anonymous typing (: Type), or qualified reference
    if p.at(SyntaxKind::COLON) {
        // Anonymous requirement with typing: verify requirement : R;
        p.parse_typing();
        p.skip_trivia();
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Parse as qualified name to support Requirements::engineSpecification
        parse_qualified_name_and_skip(p);

        // Optional typing (only COLON, not specialization operators)
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }
    }

    // Optional 'by' clause
    if consume_if(p, SyntaxKind::BY_KW) {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    p.finish_node(); // REQUIREMENT_VERIFICATION

    // Body or semicolon (body allows binding parameters)
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_constraint_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node(); // USAGE
}

/// ExhibitUsage = 'exhibit' 'state'? QualifiedName ';'
/// Per pest: case_exhibit_member = { "exhibit" ~ (exhibit_state_usage|owned_reference) }
/// Per pest: exhibit_state_usage = { "state" ~ usage_declaration? ~ state_body }
/// Pattern: exhibit [state <declaration> <body>] | exhibit <reference> semicolon
pub fn parse_exhibit_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::EXHIBIT_KW);

    // Check if this is 'exhibit state' (exhibit state usage with full declaration)
    if p.at(SyntaxKind::STATE_KW) {
        bump_keyword(p); // state

        // Parse action_usage_declaration (identification, etc.)
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
        }
        p.skip_trivia();

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
        }
        p.skip_trivia();

        // Typing
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
        }
        p.skip_trivia();

        // Specializations
        parse_specializations(p);
        p.skip_trivia();

        // State body (with optional parallel marker)
        parse_state_body(p);
    } else {
        // Simple exhibit reference: exhibit <name> ;
        if p.at_name_token() {
            p.parse_qualified_name();
        }

        p.skip_trivia();
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse allocate usage: allocate <source> to <target> ;
pub fn parse_allocate_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::ALLOCATE_KW);

    // Check for n-ary pattern: allocate (a, b, c)
    if p.at(SyntaxKind::L_PAREN) {
        bump_keyword(p); // (

        // Parse first member
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse additional members
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p); // ,
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }

        p.expect(SyntaxKind::R_PAREN);
        p.skip_trivia();
    } else {
        // Binary pattern: allocate source to target
        // Source
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // 'to' keyword
        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();

            // Target
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }
    }

    // Body or semicolon (body can contain nested allocate statements)
    parse_body_or_semicolon(p);

    p.finish_node();
}

/// IncludeUsage = 'include' 'use'? 'case'? (Name | QualifiedName) Typing? Specializations? ';'
/// When followed by typing/specialization, the first identifier is a NAME (defines new element)
/// Otherwise, it's a QualifiedName (references existing element)
pub fn parse_include_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::INCLUDE_KW);

    // Optional 'use case' keywords
    consume_if(p, SyntaxKind::USE_KW);
    consume_if(p, SyntaxKind::CASE_KW);

    // Peek ahead to determine if this is a name (followed by typing/specialization) or a reference
    // If next identifier is followed by : or :> etc, treat it as a NAME
    if p.at_name_token() {
        let peek1 = p.peek_kind(1);
        let has_typing_or_spec = matches!(
            peek1,
            SyntaxKind::COLON
                | SyntaxKind::TYPED_KW
                | SyntaxKind::OF_KW
                | SyntaxKind::COLON_GT
                | SyntaxKind::COLON_GT_GT
                | SyntaxKind::COLON_COLON_GT
                | SyntaxKind::SPECIALIZES_KW
                | SyntaxKind::SUBSETS_KW
                | SyntaxKind::REDEFINES_KW
                | SyntaxKind::REFERENCES_KW
                | SyntaxKind::L_BRACKET  // multiplicity after name
                | SyntaxKind::L_BRACE // body after name
        );

        if has_typing_or_spec {
            // This is a name (defines new element)
            p.parse_identification();
        } else {
            // This is a reference to existing element
            p.parse_qualified_name();
        }
    }

    p.skip_trivia();

    // Optional specializations (e.g., :>, redefines, etc.)
    if p.at_any(&[
        SyntaxKind::COLON,
        SyntaxKind::TYPED_KW,
        SyntaxKind::OF_KW,
        SyntaxKind::COLON_GT,
        SyntaxKind::COLON_GT_GT,
        SyntaxKind::COLON_COLON_GT,
        SyntaxKind::SPECIALIZES_KW,
        SyntaxKind::SUBSETS_KW,
        SyntaxKind::REDEFINES_KW,
        SyntaxKind::REFERENCES_KW,
    ]) {
        parse_specializations(p);
        p.skip_trivia();
    }

    // Optional multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Body or semicolon
    parse_body_or_semicolon(p);

    p.finish_node();
}

/// Expose statement: expose QualifiedName ('::' ('*' | '**'))? ';'
pub fn parse_expose_statement<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    p.expect(SyntaxKind::EXPOSE_KW);
    p.skip_trivia();

    // Qualified name with optional wildcard
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Optional wildcard suffix: :: * or :: **
    if p.at(SyntaxKind::COLON_COLON) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::STAR) || p.at(SyntaxKind::STAR_STAR) {
            p.bump();
            p.skip_trivia();
        }
    }

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

// =============================================================================
// SysML-specific parsing functions (called from trait implementations)
// =============================================================================

/// ConstraintBody = ';' | '{' Expression '}'
/// Per pest: constraint_body = { ";" | ("{" ~ constraint_body_part ~ "}") }
/// Per pest: constraint_body_part = { definition_body_item* ~ (visible_annotating_member* ~ owned_expression)? }
/// Pattern: semicolon | { [members]* [expression] }
pub fn parse_constraint_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONSTRAINT_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        // Per pest grammar: constraint_body_part = definition_body_item* ~ (visible_annotating_member* ~ owned_expression)?
        // This means we can have doc comments, imports, parameters, etc. before the expression
        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            // Check for annotations (doc, comment, etc.)
            if p.at(SyntaxKind::COMMENT_KW)
                || p.at(SyntaxKind::DOC_KW)
                || p.at(SyntaxKind::LOCALE_KW)
            {
                parse_annotation(p);
                p.skip_trivia();
            }
            // Check for textual representation
            else if p.at(SyntaxKind::REP_KW) {
                parse_textual_representation(p);
                p.skip_trivia();
            }
            // Check for parameters (in, out, inout, return)
            else if p.at(SyntaxKind::IN_KW)
                || p.at(SyntaxKind::OUT_KW)
                || p.at(SyntaxKind::INOUT_KW)
                || p.at(SyntaxKind::RETURN_KW)
            {
                // Parse as usage which handles parameters
                parse_usage(p);
                p.skip_trivia();
            }
            // Check for if expression (not if action)
            // IF_KW can start either expression or action, but in constraint bodies it's an expression
            else if p.at(SyntaxKind::IF_KW) {
                parse_expression(p);
                p.skip_trivia();
                break;
            }
            // Check for usage members (attribute, part, etc.) that can appear in constraint bodies
            else if p.at_any(SYSML_USAGE_KEYWORDS) {
                // Constraint bodies can contain attribute/part/etc. member declarations
                parse_usage(p);
                p.skip_trivia();
            }
            // Check for shorthand redefines/subsets operators
            else if p.at(SyntaxKind::COLON_GT_GT)
                || p.at(SyntaxKind::COLON_GT)
                || p.at(SyntaxKind::REDEFINES_KW)
                || p.at(SyntaxKind::SUBSETS_KW)
            {
                // Shorthand member like :>> name = value;
                parse_redefines_feature_member(p);
                p.skip_trivia();
            }
            // Check for shorthand feature declaration: name : Type;
            // This is common in constraint bodies for local features
            else if p.at_name_token() {
                // Lookahead to check if this is a feature declaration or expression start
                let lookahead = skip_trivia_lookahead(p, 1);
                if p.peek_kind(lookahead) == SyntaxKind::COLON {
                    // It's a shorthand feature: name : Type;
                    bump_keyword(p); // name
                    bump_keyword(p); // :
                    parse_qualified_name_and_skip(p); // Type
                    consume_if(p, SyntaxKind::SEMICOLON);
                    // Continue to check for more members
                } else {
                    // Not a feature declaration, must be the constraint expression
                    parse_expression(p);
                    p.skip_trivia();
                    break;
                }
            }
            // Otherwise, parse the expression (the actual constraint)
            else if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
                break; // Expression is the last item
            }
            // If we can't parse anything, break to avoid infinite loop
            else {
                break;
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        error_missing_body_terminator(p, "constraint");
    }

    p.finish_node();
}

/// Textual representation: rep <name> language <string> or just language <string>
fn parse_textual_representation<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TEXTUAL_REPRESENTATION);

    // Optional 'rep' keyword with name
    if p.at(SyntaxKind::REP_KW) {
        bump_keyword(p); // rep

        // Name (e.g., inOCL)
        if p.at_name_token() {
            bump_keyword(p);
        }
    }

    // 'language' keyword
    if p.at(SyntaxKind::LANGUAGE_KW) {
        bump_keyword(p);

        // Language string (e.g., "ocl", "alf")
        if p.at(SyntaxKind::STRING) {
            bump_keyword(p);
        }
    }

    // The actual code is in a comment block, which is trivia
    // So we don't need to explicitly parse it

    p.finish_node();
}

/// Definition or Usage - determined by presence of 'def' keyword
/// Per pest: package_body_item = { (metadata_usage | visibility_prefix? ~ (package_member | import_alias)) ~ ";"? }
/// Per pest: package_member = { (definition | usage | alias_member | ...)
/// Pattern: Determines whether to parse as definition (has 'def') or usage (no 'def')
pub fn parse_definition_or_usage<P: SysMLParser>(p: &mut P) {
    let classification = classify_definition_or_usage(p);

    match classification {
        DefinitionClassification::SysmlDefinition => parse_definition(p),
        DefinitionClassification::KermlDefinition => parse_kerml_definition(p),
        DefinitionClassification::Usage => parse_usage(p),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionClassification {
    SysmlDefinition,
    KermlDefinition,
    Usage,
}

/// Check if kind is a KerML-only definition keyword (without 'def')
fn is_kerml_definition_keyword(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::CLASS_KW
            | SyntaxKind::STRUCT_KW
            | SyntaxKind::DATATYPE_KW
            | SyntaxKind::BEHAVIOR_KW
            | SyntaxKind::FUNCTION_KW
            | SyntaxKind::ASSOC_KW
            | SyntaxKind::INTERACTION_KW
            | SyntaxKind::PREDICATE_KW
            | SyntaxKind::METACLASS_KW
            | SyntaxKind::CLASSIFIER_KW
            | SyntaxKind::TYPE_KW
    )
}

fn classify_definition_or_usage<P: SysMLParser>(p: &P) -> DefinitionClassification {
    // Scan ahead (skipping trivia) to determine what we have
    // KerML definition: struct, class, datatype, etc. (no 'def' keyword)
    // SysML definition: has 'def' keyword
    // Usage: everything else (no 'def' keyword)

    // Check first token for KerML definition keywords
    // Skip over ABSTRACT_KW if present
    let first_non_prefix = if p.peek_kind(0) == SyntaxKind::ABSTRACT_KW {
        p.peek_kind(1)
    } else {
        p.peek_kind(0)
    };

    if is_kerml_definition_keyword(first_non_prefix) {
        return DefinitionClassification::KermlDefinition;
    }

    for i in 0..20 {
        // Look ahead up to 20 tokens
        let kind = p.peek_kind(i);

        // SysML definition: has 'def' keyword
        if kind == SyntaxKind::DEF_KW {
            return DefinitionClassification::SysmlDefinition;
        }

        // Stop scanning at statement-ending tokens
        if kind == SyntaxKind::SEMICOLON
            || kind == SyntaxKind::L_BRACE
            || kind == SyntaxKind::COLON
            || kind == SyntaxKind::COLON_GT
            || kind == SyntaxKind::COLON_GT_GT
            || kind == SyntaxKind::EQ
            || kind == SyntaxKind::ERROR
        {
            return DefinitionClassification::Usage;
        }
    }
    DefinitionClassification::Usage
}

fn parse_definition<P: SysMLParser>(p: &mut P) {
    // Per pest: definition = { prefix* ~ definition_declaration ~ definition_body }
    // Per pest: definition_declaration = { keyword ~ "def"? ~ (identifier ~ ";") | (usage_prefix ~ definition_declaration) }
    // Pattern: [abstract|variation|individual] <keyword> def <name> <specializations> <body>
    p.start_node(SyntaxKind::DEFINITION);

    // Prefixes (variation point and individual markers)
    while p.at(SyntaxKind::ABSTRACT_KW)
        || p.at(SyntaxKind::VARIATION_KW)
        || p.at(SyntaxKind::INDIVIDUAL_KW)
    {
        bump_keyword(p);
    }

    let is_constraint = p.at(SyntaxKind::CONSTRAINT_KW);
    let is_calc = p.at(SyntaxKind::CALC_KW);
    let is_action = p.at(SyntaxKind::ACTION_KW);
    let is_state = p.at(SyntaxKind::STATE_KW);
    let is_analysis = p.at(SyntaxKind::ANALYSIS_KW);
    let is_verification = p.at(SyntaxKind::VERIFICATION_KW);
    let is_metadata = p.at(SyntaxKind::METADATA_KW);
    let is_usecase = p.at(SyntaxKind::USE_KW); // use case def

    // Definition keyword
    parse_definition_keyword(p);
    p.skip_trivia();

    // 'def' keyword (or 'case def' for analysis/verification)
    consume_if(p, SyntaxKind::CASE_KW);
    expect_and_skip(p, SyntaxKind::DEF_KW);

    // Identification
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

    // Body
    if is_constraint {
        parse_constraint_body(p);
    } else if is_calc {
        parse_sysml_calc_body(p);
    } else if is_action {
        parse_action_body(p);
    } else if is_state {
        parse_state_body(p);
    } else if is_analysis || is_verification || is_usecase {
        parse_case_body(p);
    } else if is_metadata {
        parse_metadata_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse a KerML definition (class, struct, datatype, etc.)
/// These definitions don't use 'def' keyword like SysML definitions
/// Per pest: structure = { abstract? ~ struct_token ~ identification? ~ specializations ~ namespace_body }
fn parse_kerml_definition<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEFINITION);

    // Optional abstract prefix
    consume_if(p, SyntaxKind::ABSTRACT_KW);

    // KerML definition keyword (class, struct, datatype, etc.)
    if is_kerml_definition_keyword(p.current_kind()) {
        bump_keyword(p);
    }
    p.skip_trivia();

    // Identification (name)
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

    // Body
    p.parse_body();

    p.finish_node();
}

fn parse_usage<P: SysMLParser>(p: &mut P) {
    // Per pest: usage = { (usage_prefix* ~ metadata_prefix* ~ event_prefix? ~ usage_element) | owned_crossing_feature }
    // Per pest: usage_element = { keyword ~ usage_declaration ~ value_part? ~ (body | ";") }
    // Per pest: owned_crossing_feature = { "end" ~ (identifier ~ multiplicity?)? ~ keyword ~ usage_declaration }
    // Pattern: [prefixes] [#metadata] [event] <keyword> [<name>] [<mult>] [<typing>] [<specializations>] [<default>] <body>
    p.start_node(SyntaxKind::USAGE);

    // Prefixes - returns true if END_KW was seen
    let saw_end = parse_usage_prefix(p);
    p.skip_trivia();

    // Prefix metadata (after prefix keywords, before usage keyword)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    // Event modifier (event occurrence pattern)
    if p.at(SyntaxKind::EVENT_KW) {
        bump_keyword(p);
    }

    // Check for owned crossing feature after END_KW: end name [mult] usage_kw name
    // If we see a name after END prefix (not a usage keyword), it's an owned_crossing_feature
    if saw_end && p.at_name_token() {
        // Parse: name [mult] usage_keyword name :> ... { }
        p.parse_identification();
        p.skip_trivia();

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Now we expect a usage keyword
        if p.at_any(SYSML_USAGE_KEYWORDS) {
            parse_usage_keyword(p);
            p.skip_trivia();

            // Parse the actual feature name
            if p.at_name_token() || p.at(SyntaxKind::LT) {
                p.parse_identification();
                p.skip_trivia();
            }

            // Continue with multiplicity, typing, specializations as normal
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }

            if p.at(SyntaxKind::COLON) {
                p.parse_typing();
                p.skip_trivia();
            }

            parse_specializations(p);
            p.skip_trivia();

            // Ordering modifiers
            while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
                p.bump();
                p.skip_trivia();
            }

            // Body
            p.parse_body();
            p.finish_node();
            return;
        }
    }

    // Check for owned crossing feature: end [mult] keyword ...
    // If we see multiplicity before the usage keyword, parse it
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    let is_constraint = p.at(SyntaxKind::CONSTRAINT_KW);
    let is_action = p.at(SyntaxKind::ACTION_KW);
    let is_calc = p.at(SyntaxKind::CALC_KW);
    let is_state = p.at(SyntaxKind::STATE_KW);
    let is_analysis = p.at(SyntaxKind::ANALYSIS_KW);
    let is_verification = p.at(SyntaxKind::VERIFICATION_KW);
    let is_metadata = p.at(SyntaxKind::METADATA_KW);
    let is_message = p.at(SyntaxKind::MESSAGE_KW);
    let is_usecase = p.at(SyntaxKind::USE_KW); // use case usage
    let is_connection_kw = p.at(SyntaxKind::CONNECTION_KW);
    let is_interface_kw = p.at(SyntaxKind::INTERFACE_KW);

    // Usage keyword
    parse_usage_keyword(p);
    p.skip_trivia();

    // Per pest: constraint_usage_declaration is optional (usage_declaration? ~ value_part?)
    // So we can have just "requirement;" or "constraint;" with no name/typing/body content
    // Check if we're at body start immediately after keyword
    if p.at(SyntaxKind::SEMICOLON) || p.at(SyntaxKind::L_BRACE) {
        // Minimal usage: just keyword + body
        if is_constraint {
            parse_constraint_body(p);
        } else if is_calc {
            parse_sysml_calc_body(p);
        } else if is_action {
            parse_action_body(p);
        } else if is_state {
            parse_state_body(p);
        } else if is_analysis || is_verification || is_usecase {
            parse_case_body(p);
        } else if is_metadata {
            parse_metadata_body(p);
        } else {
            p.parse_body();
        }
        p.finish_node();
        return;
    }

    // For message usages, handle 'of' keyword before identification
    // Pattern: message of payload:Type from source to target;
    if is_message {
        consume_if(p, SyntaxKind::OF_KW);
    }

    // Handle shorthand redefines: 'attribute :>> name' (no identifier before :>>)
    if p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::COLON_GT)
        || p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::SUBSETS_KW)
    {
        // This is a shorthand feature member after a usage keyword
        // Wrap in SPECIALIZATION node so AST can extract the relationship
        p.start_node(SyntaxKind::SPECIALIZATION);
        bump_keyword(p); // the operator

        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
        p.finish_node(); // finish first SPECIALIZATION

        // Handle multiplicity after first name: :>> name[mult]
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Handle comma-separated references: :>> A, B, C
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p); // comma
            p.start_node(SyntaxKind::SPECIALIZATION);
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
            p.finish_node(); // finish additional SPECIALIZATION
            // Multiplicity after each name
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }
        }

        // Additional specializations (including ::> references)
        parse_specializations(p);
        p.skip_trivia();

        // Typing after shorthand redefinition
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Ordering modifiers
        while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
            bump_keyword(p);
        }

        // Default value
        parse_optional_default_value(p);

        // Body (check type-specific bodies for shorthand redefines too)
        if is_constraint {
            parse_constraint_body(p);
        } else if is_calc {
            parse_sysml_calc_body(p);
        } else if is_action {
            parse_action_body(p);
        } else if is_state {
            parse_state_body(p);
        } else if is_analysis || is_verification || is_usecase {
            parse_case_body(p);
        } else {
            p.parse_body();
        }
        p.finish_node();
        return;
    }

    // Identification - but NOT if we're at CONNECT_KW (which is part of connector clause)
    // Check if this is a feature chain reference (name.member) vs a simple name
    // For patterns like `event sendSpeed.sourceEvent;`, the chain is a reference, not a name
    let has_chain = if (p.at_name_token() || p.at(SyntaxKind::LT)) && !p.at(SyntaxKind::CONNECT_KW)
    {
        // Look ahead to see if there's a dot after the name
        let mut lookahead = 0;
        if p.at(SyntaxKind::LT) {
            // Skip past short name <name>
            lookahead += 1; // <
            if is_name_kind(p.peek_kind(lookahead)) {
                lookahead += 1;
            }
            if p.peek_kind(lookahead) == SyntaxKind::GT {
                lookahead += 1;
            }
        }
        if is_name_kind(p.peek_kind(lookahead)) {
            lookahead += 1;
        }
        // Skip any whitespace/trivia in lookahead (simplified - just check next few)
        p.peek_kind(lookahead) == SyntaxKind::DOT
    } else {
        false
    };

    // For interface/connection usages, check if this looks like a connector pattern
    // Pattern: interface X.y to Z.w - the feature chain is a connector endpoint, not a specialization
    let looks_like_connector_endpoint = if (is_connection_kw || is_interface_kw) && has_chain {
        // Look ahead to see if there's a 'to' keyword after the chain
        let mut depth = 0;
        let mut found_to = false;
        for i in 0..30 {
            match p.peek_kind(i) {
                SyntaxKind::TO_KW if depth == 0 => {
                    found_to = true;
                    break;
                }
                SyntaxKind::DOT | SyntaxKind::IDENT => {}
                SyntaxKind::L_BRACKET => depth += 1,
                SyntaxKind::R_BRACKET => depth -= 1,
                SyntaxKind::WHITESPACE => {} // Skip whitespace in lookahead
                SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE | SyntaxKind::COLON => break,
                _ => break,
            }
        }
        found_to
    } else {
        false
    };

    if has_chain && !looks_like_connector_endpoint {
        // This is a feature chain reference like `sendSpeed.sourceEvent`
        // Parse as a SPECIALIZATION with a QUALIFIED_NAME containing the chain
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name(); // Parses the full chain including dots
        p.skip_trivia();
        p.finish_node();
    } else if (p.at_name_token() || p.at(SyntaxKind::LT))
        && !p.at(SyntaxKind::CONNECT_KW)
        && !looks_like_connector_endpoint
    {
        p.parse_identification();
    }
    p.skip_trivia();

    // For message usages: handle 'of' payload type after name
    // Pattern: message sendSensedSpeed of SensedSpeed from ... to ...
    if is_message && p.at(SyntaxKind::OF_KW) {
        bump_keyword(p);
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
    }

    // Handle feature chain continuation (e.g., producerBehavior.publish[1])
    // This handles cases where chain wasn't detected by lookahead
    while p.at(SyntaxKind::DOT) {
        bump_keyword(p); // .
        if p.at_name_token() {
            bump_keyword(p);
        }
        // Optional indexing after feature access
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }
    }

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
    }
    p.skip_trivia();

    // Typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity after specializations (e.g., port myPort :>> basePort [5])
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Ordering modifiers
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_keyword(p);
    }

    // More specializations
    parse_specializations(p);
    p.skip_trivia();

    // For connection/interface usage: n-ary endpoint syntax after typing
    // Pattern: connection : Type ( end1 ::> a, end2 ::> b );
    // Pattern: interface : Type ( end1 ::> a, end2 ::> b );
    if (is_connection_kw || is_interface_kw) && p.at(SyntaxKind::L_PAREN) {
        p.start_node(SyntaxKind::CONNECTOR_PART);
        bump_keyword(p); // (

        parse_connector_end(p);
        p.skip_trivia();

        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.expect(SyntaxKind::R_PAREN);
        p.finish_node(); // CONNECTOR_PART
        p.skip_trivia();
    }

    // For connection/interface usage: binary endpoint syntax with 'to'
    // Pattern: interface source.port to target.port;
    if (is_connection_kw || is_interface_kw) && p.at_name_token() && !p.at(SyntaxKind::CONNECT_KW) {
        // Check if there's a 'to' keyword ahead
        let has_to = {
            let mut depth = 0;
            let mut found_to = false;
            for i in 0..20 {
                match p.peek_kind(i) {
                    SyntaxKind::TO_KW if depth == 0 => {
                        found_to = true;
                        break;
                    }
                    SyntaxKind::DOT | SyntaxKind::IDENT => {}
                    SyntaxKind::L_BRACKET => depth += 1,
                    SyntaxKind::R_BRACKET => depth -= 1,
                    SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE => break,
                    _ => break,
                }
            }
            found_to
        };

        if has_to {
            p.start_node(SyntaxKind::CONNECTOR_PART);

            // Parse source endpoint (chain like source.port)
            parse_connector_end(p);
            p.skip_trivia();

            // 'to' keyword
            if p.at(SyntaxKind::TO_KW) {
                bump_keyword(p);

                // Parse target endpoint
                parse_connector_end(p);
                p.skip_trivia();
            }

            p.finish_node(); // CONNECTOR_PART
        }
    }

    // For allocation usage: optional allocate clause
    let is_allocation = p.at(SyntaxKind::ALLOCATE_KW);
    if is_allocation {
        // Parse allocate keyword part: allocate <source> to <target>
        bump_keyword(p); // allocate

        // Check for n-ary or binary pattern
        if p.at(SyntaxKind::L_PAREN) {
            // N-ary: allocate (a, b ::> c, ...)
            bump_keyword(p); // (

            parse_allocate_end_member(p);

            while p.at(SyntaxKind::COMMA) {
                bump_keyword(p);
                parse_allocate_end_member(p);
            }

            p.expect(SyntaxKind::R_PAREN);
            p.skip_trivia();
        } else {
            // Binary: allocate source to target
            parse_allocate_end_member(p);

            if consume_if(p, SyntaxKind::TO_KW) {
                parse_allocate_end_member(p);
            }
        }
    }

    // For connection usage: optional connect clause
    let is_connection = p.at(SyntaxKind::CONNECT_KW);
    if is_connection {
        // Parse connect keyword part: connect <end> to <end> or connect (<ends>)
        p.start_node(SyntaxKind::CONNECTOR_PART);
        bump_keyword(p); // connect

        // Check for n-ary or binary pattern
        if p.at(SyntaxKind::L_PAREN) {
            // N-ary: connect (a ::> b, c ::> d, ...)
            bump_keyword(p); // (

            parse_connector_end(p);
            p.skip_trivia();

            while p.at(SyntaxKind::COMMA) {
                bump_keyword(p);
                parse_connector_end(p);
                p.skip_trivia();
            }

            p.expect(SyntaxKind::R_PAREN);
            p.skip_trivia();
        } else {
            // Binary: connect source to target
            parse_connector_end(p);
            p.skip_trivia();

            if consume_if(p, SyntaxKind::TO_KW) {
                parse_connector_end(p);
                p.skip_trivia();
            }
        }
        p.finish_node(); // CONNECTOR_PART
    }

    // For message: optional from/to clause
    parse_optional_from_to(p);

    // Default value: 'default' [expr] or '=' expr or ':=' expr
    parse_optional_default_value(p);

    // About clause (for metadata usages)
    // Pattern: about annotation ("," annotation)*
    if p.at(SyntaxKind::ABOUT_KW) {
        bump_keyword(p); // about

        // Parse first annotation (qualified name or identifier)
        parse_optional_qualified_name(p);

        // Parse additional annotations
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            parse_optional_qualified_name(p);
        }
    }

    // Body
    if is_constraint {
        parse_constraint_body(p);
    } else if is_calc {
        parse_sysml_calc_body(p);
    } else if is_action {
        parse_action_body(p);
    } else if is_state {
        parse_state_body(p);
    } else if is_analysis || is_verification || is_usecase {
        parse_case_body(p);
    } else if is_metadata {
        parse_metadata_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse allocate end member: [name ::>] qualified_name
fn parse_allocate_end_member<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() {
        // Check if this is "name ::> ref" pattern
        let lookahead = 1;
        if p.peek_kind(lookahead) == SyntaxKind::COLON_COLON_GT {
            // Pattern: name ::> qualified_name
            p.bump(); // name
            p.skip_trivia();
            p.bump(); // ::>
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
        } else {
            // Just a qualified name
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }
}

fn parse_definition_keyword<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::USE_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::CASE_KW) {
            p.bump();
        }
        return;
    }

    if p.at_any(SYSML_DEFINITION_KEYWORDS) {
        p.bump();
    }
}

fn parse_usage_keyword<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::USE_KW) {
        bump_keyword(p);
        if p.at(SyntaxKind::CASE_KW) {
            p.bump();
        }
        return;
    }

    if p.at_any(SYSML_USAGE_KEYWORDS) {
        // Don't consume a keyword if it's actually being used as a name.
        // Check if the next non-trivia token indicates this is a name (followed by : or :> or [ etc.)
        // This handles cases like `in frame : Integer` where `frame` is a name, not a usage keyword.
        if p.at_name_token() {
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);
            if matches!(
                next,
                SyntaxKind::COLON
                    | SyntaxKind::COLON_GT
                    | SyntaxKind::COLON_GT_GT
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::SEMICOLON
                    | SyntaxKind::L_BRACE
                    | SyntaxKind::REDEFINES_KW
                    | SyntaxKind::SUBSETS_KW
                    | SyntaxKind::REFERENCES_KW
            ) {
                // This looks like a name followed by typing/specialization, not a usage keyword
                return;
            }
        }
        p.bump();
    }
}

fn parse_usage_prefix<P: SysMLParser>(p: &mut P) -> bool {
    let mut saw_end = false;
    while p.at_any(USAGE_PREFIX_KEYWORDS) {
        if p.at(SyntaxKind::END_KW) {
            saw_end = true;
        }
        bump_keyword(p);
    }
    saw_end
}

/// Dependency = 'dependency' (identification 'from' | 'from')? source (',' source)* 'to' target (',' target)*
pub fn parse_dependency<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEPENDENCY);

    expect_and_skip(p, SyntaxKind::DEPENDENCY_KW);

    // Check for identification followed by 'from', or just 'from', or direct source
    if p.at(SyntaxKind::FROM_KW) {
        // No identification, just 'from source'
        bump_keyword(p);
    } else if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        // Could be identification (if followed by 'from') or direct source
        // Peek ahead to see if 'from' follows
        let next = p.peek_kind(1);
        if next == SyntaxKind::FROM_KW {
            // It's an identification: dependency myDep from source to target
            p.parse_identification();
            p.skip_trivia();
            expect_and_skip(p, SyntaxKind::FROM_KW);
        }
        // Otherwise it's a direct source: dependency source to target
    }

    // Parse source(s)
    if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        parse_qualified_name_and_skip(p);

        // Multiple sources separated by comma
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
                parse_qualified_name_and_skip(p);
            }
        }
    }

    // 'to' target(s)
    if p.at(SyntaxKind::TO_KW) {
        bump_keyword(p);
        parse_qualified_name_and_skip(p);

        // Multiple targets separated by comma
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            if p.at_name_token() {
                parse_qualified_name_and_skip(p);
            }
        }
    }

    p.parse_body();
    p.finish_node();
}

/// Filter = 'filter' Expression ';'
pub fn parse_filter<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ELEMENT_FILTER_MEMBER);

    p.expect(SyntaxKind::FILTER_KW);
    p.skip_trivia();

    // Parse the filter expression (can be metadata reference or general expression)
    // Examples:
    // - filter @Safety;
    // - filter @Safety or @Security;
    // - filter @Safety and Safety::isMandatory;
    parse_expression(p);

    p.skip_trivia();
    p.expect(SyntaxKind::SEMICOLON);
    p.finish_node();
}

/// MetadataUsage = '@' QualifiedName ...
/// Per pest: metadata_usage = { "@" ~ qualified_name ~ ("about" ~ qualified_name_list)? ~ (";"|metadata_body) }
/// Pattern: @ <qualified_name> [about <references>] <body|semicolon>
/// Also handles prefix annotations: @Metadata part x; where the metadata annotates the part
pub fn parse_metadata_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::METADATA_USAGE);

    p.expect(SyntaxKind::AT);
    p.skip_trivia();
    p.parse_qualified_name();
    p.skip_trivia();

    // Optional 'about' clause
    if p.at(SyntaxKind::ABOUT_KW) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name_list();
        p.skip_trivia();
    }

    // Check if this is a prefix annotation (followed by a definition/usage keyword)
    // In that case, the metadata annotates the following element
    if is_definition_or_usage_start(p) {
        // This is a prefix annotation - finish the metadata node and parse the annotated element
        p.finish_node();
        p.parse_definition_or_usage();
        return;
    }

    parse_body_or_semicolon(p);

    p.finish_node();
}

/// Check if the current token could start a definition or usage
fn is_definition_or_usage_start<P: SysMLParser>(p: &P) -> bool {
    p.at_any(&[
        // SysML definition/usage keywords
        SyntaxKind::PART_KW,
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PORT_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::OCCURRENCE_KW,
        SyntaxKind::CONSTRAINT_KW,
        SyntaxKind::REQUIREMENT_KW,
        SyntaxKind::CASE_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::CONNECTION_KW,
        SyntaxKind::INTERFACE_KW,
        SyntaxKind::ALLOCATION_KW,
        SyntaxKind::VIEW_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::VIEWPOINT_KW,
        SyntaxKind::RENDERING_KW,
        SyntaxKind::METADATA_KW,
        SyntaxKind::ENUM_KW,
        SyntaxKind::ANALYSIS_KW,
        SyntaxKind::VERIFICATION_KW,
        SyntaxKind::USE_KW,
        SyntaxKind::CONCERN_KW,
        SyntaxKind::FLOW_KW,
        SyntaxKind::PARALLEL_KW,
        SyntaxKind::EVENT_KW,
        SyntaxKind::MESSAGE_KW,
        SyntaxKind::SNAPSHOT_KW,
        SyntaxKind::TIMESLICE_KW,
        // Prefix keywords
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::VARIATION_KW,
        SyntaxKind::INDIVIDUAL_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::VAR_KW,
        SyntaxKind::REF_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::END_KW,
    ])
}

/// BindUsage = 'bind' connector_end '=' connector_end body
/// e.g., bind start = done { ... }
/// Per pest: binding_connector = { "bind" ~ connector_end ~ "=" ~ connector_end ~ (";"|connector_body) }
/// Per pest: connector_end = { multiplicity? ~ owned_feature_chain }
/// Pattern: bind [mult] <source> = [mult] <target> <body|semicolon>
pub fn parse_bind_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::BINDING_CONNECTOR);

    p.expect(SyntaxKind::BIND_KW);
    p.skip_trivia();

    // Optional multiplicity after bind keyword (connector_end can have multiplicity)
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // First end (left side)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // '=' separator
    if p.at(SyntaxKind::EQ) {
        p.bump();
        p.skip_trivia();
    }

    // Optional multiplicity before second end
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Second end (right side)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Body or semicolon
    p.parse_body();

    p.finish_node();
}

/// AssignAction = 'assign' target ':=' expr ';'
/// e.g., assign x := value;
/// Per pest: assignment_node = { "assign" ~ feature_reference ~ ":=" ~ owned_expression ~ (";"|action_body) }
/// Pattern: assign <feature> := <expression> <body|semicolon>
pub fn parse_assign_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::ASSIGN_KW);
    p.skip_trivia();

    // Assignment target (can be a feature chain like counter.count)
    if p.at_name_token() {
        p.parse_qualified_name(); // handles feature chains via dots
        p.skip_trivia();
    }

    // ':=' assignment operator
    if p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body or semicolon
    p.parse_body();

    p.finish_node();
}

/// ConnectUsage = 'connect' ...\n/// Per pest: binary_connection_usage = { \"connect\" ~ connector_part ~ (\";\"|connector_body) }\n/// Per pest: connector_part = { nary_connector_part | binary_connector_part }\n/// Per pest: binary_connector_part = { connector_end ~ \"to\" ~ connector_end }\n/// Per pest: nary_connector_part = { \"(\" ~ connector_end ~ (\",\" ~ connector_end)+ ~ \")\" }\n/// Pattern: connect (<end>, <end>) | connect <end> to <end> <body|semicolon>
pub fn parse_connect_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECT_USAGE);

    p.expect(SyntaxKind::CONNECT_KW);
    p.skip_trivia();

    // Per pest grammar: connect has connector_part which is either:
    // - binary: end to end
    // - nary: ( end, end, ... )

    if p.at(SyntaxKind::L_PAREN) {
        // N-ary connector part: ( end, end, ... )
        p.start_node(SyntaxKind::CONNECTOR_PART);
        p.bump(); // (
        p.skip_trivia();

        // Parse first connector end
        parse_connector_end(p);
        p.skip_trivia();

        // Parse remaining ends with commas
        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.expect(SyntaxKind::R_PAREN);
        p.finish_node(); // CONNECTOR_PART
        p.skip_trivia();
    } else {
        // Binary connector part: end to end
        p.start_node(SyntaxKind::CONNECTOR_PART);

        // First end
        parse_connector_end(p);
        p.skip_trivia();

        // 'to' keyword
        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();

            // Second end
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.finish_node(); // CONNECTOR_PART
    }

    p.parse_body();
    p.finish_node();
}

/// Parse a connector end
/// Per pest: connector_end = multiplicity? connector_end_reference
/// connector_end_reference = feature_chain | (identifier|quoted_name) ::> (feature_chain|reference) | reference
fn parse_connector_end<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR_END);

    // Optional multiplicity (e.g., [1..3])
    parse_optional_multiplicity(p);

    // connector_end_reference
    parse_connector_end_reference(p);

    p.finish_node();
}

/// Parse connector end reference
/// identifier ::> reference | identifier references reference | qualified_name
fn parse_connector_end_reference<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR_END_REFERENCE);

    if p.at_name_token() {
        // Parse the first identifier or qualified name
        parse_qualified_name_and_skip(p);

        // Check for ::> or 'references' (references operator)
        if p.at(SyntaxKind::COLON_COLON_GT) || p.at(SyntaxKind::REFERENCES_KW) {
            bump_keyword(p);

            // Parse target (qualified name or feature chain)
            parse_qualified_name_and_skip(p);
        }
    }

    p.finish_node();
}

/// Parse connector usage (standalone connector keyword)
/// connector [name] [:> Type] [from source to target] body
pub fn parse_connector_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR);

    expect_and_skip(p, SyntaxKind::CONNECTOR_KW);

    // Optional identification
    parse_optional_identification(p);

    // Optional typing
    parse_optional_typing(p);

    // Optional specializations
    parse_specializations_with_skip(p);

    // Optional from...to clause
    if p.at(SyntaxKind::FROM_KW) {
        bump_keyword(p);
        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::TO_KW) {
            bump_keyword(p);
            parse_optional_qualified_name(p);
        }
    }

    p.parse_body();
    p.finish_node();
}

/// Parse multiplicity: [expression] or [lower..upper] or [lower..expr()]
/// Supports expressions including function calls as bounds
fn parse_multiplicity<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::MULTIPLICITY);
    p.expect(SyntaxKind::L_BRACKET);
    p.skip_trivia();

    // Parse multiplicity bounds - can be:
    // - Single value: [5], [*]
    // - Range: [0..*], [1..10], [0..size(items)]
    // - Expression: [size(items)]

    // Parse first bound (could be expression)
    if !p.at(SyntaxKind::R_BRACKET) {
        parse_multiplicity_bound(p);
        p.skip_trivia();

        // Check for range operator (..)
        if p.at(SyntaxKind::DOT_DOT) {
            p.bump();
            p.skip_trivia();

            // Parse upper bound (could be expression or *)
            if !p.at(SyntaxKind::R_BRACKET) {
                parse_multiplicity_bound(p);
                p.skip_trivia();
            }
        }
    }

    p.expect(SyntaxKind::R_BRACKET);
    p.finish_node();
}

/// Parse a single multiplicity bound (number, *, or expression including function calls)
/// Per spec: multiplicity_bound = { inline_expression | number | "*" }
/// Bounds are typed as Expression in the metamodel
fn parse_multiplicity_bound<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::STAR) {
        p.bump();
    } else if p.at(SyntaxKind::INTEGER) {
        // Integers are literals - parse as expression for consistency
        super::kerml_expressions::parse_expression(p);
    } else if p.at_name_token() || p.at(SyntaxKind::L_PAREN) {
        // Parse as full expression (handles identifiers, function calls, etc.)
        super::kerml_expressions::parse_expression(p);
    }
}

/// Binding or Succession
/// succession [identification] [typing] [multiplicity] first [mult] source then [mult] target;
/// binding [identification] source = target;
pub fn parse_binding_or_succession<P: SysMLParser>(p: &mut P) {
    let is_succession = p.at(SyntaxKind::SUCCESSION_KW);

    // Check for succession flow pattern
    if is_succession && p.peek_kind(1) == SyntaxKind::FLOW_KW {
        // Delegate to SysML-specific flow parser
        parse_flow_usage(p);
        return;
    }

    if is_succession {
        p.start_node(SyntaxKind::SUCCESSION);
    } else {
        p.start_node(SyntaxKind::BINDING_CONNECTOR);
    }

    p.bump(); // binding or succession
    p.skip_trivia();

    // Optional multiplicity (for both binding and succession)
    // Examples: binding [1] bind ..., succession [0..*] first ...
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Check for 'bind' keyword (binding_connector_as_usage pattern)
    // Pattern: binding [mult]? name? bind [mult]? x = [mult]? y;
    if !is_succession && p.at(SyntaxKind::BIND_KW) {
        p.bump(); // bind
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // First end (left side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // '=' separator
        if p.at(SyntaxKind::EQ) {
            p.bump();
            p.skip_trivia();
        }

        // Optional multiplicity before second end
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Second end (right side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Optional redefines
    let mut parsed_name = false;
    if p.at(SyntaxKind::REDEFINES_KW) || p.at(SyntaxKind::COLON_GT_GT) {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
            parsed_name = true;
        }
    // Optional identification (name) - but NOT for binding `name = target` pattern
    // In `binding payload = target`, `payload` is the source endpoint, not the name
    } else if p.at_name_token() && !p.at(SyntaxKind::FIRST_KW) && !p.at(SyntaxKind::BIND_KW) {
        // For bindings, check if token after name is '=' - if so, it's the source endpoint
        // For successions, check if token after name is 'then' - if so, it's the source endpoint
        // Peek ahead: name might be qualified (A::B) so look for EQ/THEN_KW after names
        let is_binding_source = !is_succession && peek_past_name_for(p, SyntaxKind::EQ);
        let is_succession_source = is_succession && peek_past_name_for(p, SyntaxKind::THEN_KW);

        if !is_binding_source && !is_succession_source {
            // It's an identification, not a source endpoint
            p.parse_identification();
            p.skip_trivia();
            parsed_name = true;
        }
    }

    // Check for 'bind' keyword AFTER optional identification
    // Pattern: binding myBinding bind [mult]? x = [mult]? y;
    if !is_succession && p.at(SyntaxKind::BIND_KW) {
        p.bump(); // bind
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // First end (left side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // '=' separator
        if p.at(SyntaxKind::EQ) {
            p.bump();
            p.skip_trivia();
        }

        // Optional multiplicity before second end
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Second end (right side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // For binding: 'of' keyword
    if !is_succession && p.at(SyntaxKind::OF_KW) {
        p.bump();
        p.skip_trivia();
    }

    // For succession: optional typing
    if is_succession && p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Succession with first/then
    if is_succession && p.at(SyntaxKind::FIRST_KW) {
        p.bump(); // first
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Source feature
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // One transition target: then X | if guard then X | else X
        if p.at(SyntaxKind::THEN_KW) {
            // then target
            p.bump(); // then
            p.skip_trivia();

            // Optional multiplicity
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }

            // Target feature
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        } else if p.at(SyntaxKind::IF_KW) {
            // if guard then target
            p.bump(); // if
            p.skip_trivia();

            // Guard expression
            if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
            }

            // then
            if p.at(SyntaxKind::THEN_KW) {
                p.bump();
                p.skip_trivia();

                // Target
                if p.at_name_token() {
                    p.parse_qualified_name();
                    p.skip_trivia();
                }
            }
        } else if p.at(SyntaxKind::ELSE_KW) {
            // else target
            p.bump(); // else
            p.skip_trivia();

            // Target
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }
    } else {
        // Simple succession/binding: source = target or source then target
        // Only parse the source name if we didn't already parse it via identification
        if !parsed_name && p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::THEN_KW) {
            p.bump();
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
        }
    }

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// VariantUsage = 'variant' ...
pub fn parse_variant_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::VARIANT_KW);
    p.skip_trivia();

    // Optional usage keyword (e.g., variant part x, variant action a1, variant use case uc1)
    if p.at(SyntaxKind::USE_KW) {
        p.bump(); // use
        p.skip_trivia();
        if p.at(SyntaxKind::CASE_KW) {
            p.bump(); // case
            p.skip_trivia();
        }
    } else if p.at_any(SYSML_USAGE_KEYWORDS) {
        p.bump();
        p.skip_trivia();
    }

    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Multiplicity (e.g., variant part withSunroof[1])
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Redefines feature member\n/// Per pest: owned_feature_member = { visibility_prefix? ~ (owned_feature_declaration|owned_redefinition) ~ value_part? ~ (body|\";\") }\n/// Per pest: owned_redefinition = { usage_prefix* ~ (\":>>\" ~ qualified_name_list | \"subsets\" ~ qualified_name_list) }\n/// Pattern: [prefixes] :>>|subsets <name>[,<name>]* [typing] [mult] [specializations] [default] <body|semicolon>
pub fn parse_redefines_feature_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Handle optional prefix (e.g., ref :>> name)
    while p.at_any(USAGE_PREFIX_KEYWORDS) {
        p.bump();
        p.skip_trivia();
    }

    // Wrap in SPECIALIZATION node so AST can extract the relationship
    p.start_node(SyntaxKind::SPECIALIZATION);
    p.bump(); // redefines/subsets operator
    p.skip_trivia();

    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }
    p.finish_node(); // finish first SPECIALIZATION

    // Handle comma-separated qualified names for :>> A, B pattern
    while p.at(SyntaxKind::COMMA) {
        p.bump();
        p.skip_trivia();
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name();
        p.skip_trivia();
        p.finish_node();
    }

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    // Default value or assignment
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Shorthand feature member
/// Parse anonymous usage: `: Type;` or `typed by Type;`
/// This is an anonymous feature/usage that has no name, just a type
fn parse_anonymous_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parse typing (: Type or typed by Type)
    p.parse_typing();
    p.skip_trivia();

    // Optional specializations
    parse_specializations(p);
    p.skip_trivia();

    // Optional value assignment
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        if p.at(SyntaxKind::DEFAULT_KW) {
            p.bump();
            p.skip_trivia();
        }
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

pub fn parse_shorthand_feature_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Check if there's a keyword prefix (actor, subject, stakeholder, etc.)
    if matches!(
        p.current_kind(),
        SyntaxKind::ACTOR_KW
            | SyntaxKind::SUBJECT_KW
            | SyntaxKind::STAKEHOLDER_KW
            | SyntaxKind::OBJECTIVE_KW
            | SyntaxKind::FILTER_KW
    ) {
        p.bump(); // Consume the keyword
        p.skip_trivia();
    }

    p.parse_identification();
    p.skip_trivia();

    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Only COLON is typing; COLON_GT and COLON_GT_GT are specializations
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        if p.at(SyntaxKind::DEFAULT_KW) {
            p.bump();
            p.skip_trivia();
        }
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        // Parse expression after '=' or 'default' (default can omit '=')
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse case body (for analysis/verification definitions)
// Per pest: case_body = { ";" | ("{" ~ case_body_part ~ "}") }
// Per pest: case_body_part = { case_calculation_body_item* ~ case_objective* ~ case_subject* ~ case_actor* ~ case_stakeholder* ~ result_expression_member? }
// Pattern: semicolon | { [objective|subject|actor|stakeholder|calculation items]* [result expression]? }
fn parse_case_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "case definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Parse case body items: objective, subject, actor, case_calculation_body_item
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        if p.at(SyntaxKind::OBJECTIVE_KW) {
            parse_objective_member(p);
        } else if p.at(SyntaxKind::SUBJECT_KW) {
            parse_subject_member(p);
        } else if p.at(SyntaxKind::ACTOR_KW) {
            parse_actor_member(p);
        } else if p.at(SyntaxKind::STAKEHOLDER_KW) {
            parse_stakeholder_member(p);
        } else if p.at(SyntaxKind::RETURN_KW) {
            parse_sysml_parameter(p);
        } else if p.at(SyntaxKind::IDENT) {
            // Check if this looks like an expression (followed by operator) or a feature member
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);

            // If followed by expression operators, parse as expression
            if matches!(
                next,
                SyntaxKind::DOT
                    | SyntaxKind::COLON_COLON
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::L_PAREN
                    | SyntaxKind::PLUS
                    | SyntaxKind::MINUS
                    | SyntaxKind::STAR
                    | SyntaxKind::SLASH
                    | SyntaxKind::PERCENT
                    | SyntaxKind::GT
                    | SyntaxKind::LT
                    | SyntaxKind::EQ_EQ
                    | SyntaxKind::BANG_EQ
            ) {
                // Parse as expression (shared expression grammar from kerml_expressions.pest)
                super::kerml_expressions::parse_expression(p);
            } else {
                // Parse as package body element (feature member)
                parse_package_body_element(p);
            }
        } else {
            // Other case body items (calculation body, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body (for metadata definitions)
fn parse_metadata_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "metadata definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Metadata body can contain metadata_body_usage items
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        // Metadata body usage pattern: [ref] [:>>] identifier
        // Need to distinguish from other body elements
        let is_metadata_usage = if p.at(SyntaxKind::REF_KW) {
            // Check if ref is followed by :>> or identifier (not a usage keyword)
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);
            matches!(
                next,
                SyntaxKind::COLON_GT_GT
                    | SyntaxKind::COLON_GT
                    | SyntaxKind::REDEFINES_KW
                    | SyntaxKind::IDENT
            )
        } else if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::REDEFINES_KW) {
            // Starts with redefines operator
            true
        } else if p.at(SyntaxKind::IDENT) {
            // Just an identifier - could be metadata usage or other element
            // In metadata body, bare identifiers are metadata body usages
            true
        } else {
            false
        };

        if is_metadata_usage {
            parse_metadata_body_usage(p);
        } else {
            // Other body elements (imports, relationships, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body usage: ref? :>>? identifier typing? specializations? default? meta? value? body
// Handles patterns like:
//   - `ref :>> annotatedElement : SysML::Usage;`
//   - `:>> baseType default global_sd meta SysML::PortUsage;`
fn parse_metadata_body_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Optional 'ref'
    if p.at(SyntaxKind::REF_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Optional redefines operator - wrap in SPECIALIZATION node for AST extraction
    if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::COLON_GT) || p.at(SyntaxKind::REDEFINES_KW)
    {
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.bump(); // :>> or :> or redefines
        p.skip_trivia();

        // Required identifier (as qualified name)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
        p.finish_node(); // SPECIALIZATION
    } else {
        // No redefines - parse name directly as NAME node for hoverable symbol
        if p.at(SyntaxKind::IDENT) {
            p.start_node(SyntaxKind::NAME);
            p.bump();
            p.finish_node();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
    }

    // Optional typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Optional specializations
    parse_specializations(p);
    p.skip_trivia();

    // Optional 'default' clause with expression
    // Pattern: `default <expression>`
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump(); // default
        p.skip_trivia();
        // The default value is an expression (usually an identifier reference)
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }

    // Optional 'meta' clause with type reference
    // Pattern: `meta <qualified_name>`
    // We use TYPING node to wrap this since it functions similarly
    if p.at(SyntaxKind::META_KW) {
        p.start_node(SyntaxKind::TYPING);
        p.bump(); // meta
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
        p.finish_node();
    }

    // Optional value (= expression or 'as' cast)
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body (semicolon or nested metadata body)
    if p.at(SyntaxKind::L_BRACE) {
        parse_metadata_body(p);
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse objective member: 'objective' [name] ':' type [:>> ref, ...] body
fn parse_objective_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::OBJECTIVE_USAGE);
    p.bump(); // objective
    p.skip_trivia();

    // Optional identifier (wrapped in NAME)
    parse_optional_identification(p);

    // Optional typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Specializations (per pest: constraint_usage_declaration includes usage_declaration)
    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Body (requirement body)
    parse_requirement_body(p);

    p.finish_node();
}

// Parse subject member: 'subject' usage_declaration
fn parse_subject_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUBJECT_USAGE);
    p.bump(); // subject
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME, typing, etc.)
    parse_optional_identification(p);

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

    // Typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Default value or assignment
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse actor member: 'actor' usage_declaration
fn parse_actor_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACTOR_USAGE);
    p.bump(); // actor
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME)
    parse_optional_identification(p);

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Default value
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse stakeholder member: 'stakeholder' usage_declaration
fn parse_stakeholder_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STAKEHOLDER_USAGE);
    p.bump(); // stakeholder
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME)
    parse_optional_identification(p);

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse requirement body (for objective members and requirements)
fn parse_requirement_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "requirement");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Requirement body can contain definition_body_items, subject members, constraints, etc.
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_package_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

/// Parse calc body for SysML (extends KerML calc body with behavior usages)
/// Per pest: calculation_body_item includes behavior_usage_member (perform, send, etc.)
/// and result_expression_member (final expression without semicolon)
pub fn parse_sysml_calc_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            let start_pos = p.get_pos();

            // Parameters (in, out) - treat as SysML usages to handle ref prefix
            if p.at_any(&[SyntaxKind::IN_KW, SyntaxKind::OUT_KW, SyntaxKind::INOUT_KW]) {
                parse_usage(p);
            }
            // RETURN_KW can be either a return parameter or return expression
            else if p.at(SyntaxKind::RETURN_KW) {
                // Look ahead to distinguish: return <name>? : ... vs return <expr>
                let lookahead = skip_trivia_lookahead(p, 1);
                let after_return = p.peek_kind(lookahead);

                // If return is followed directly by colon, it's a parameter: return : Type
                if after_return == SyntaxKind::COLON || after_return == SyntaxKind::TYPED_KW {
                    parse_usage(p);
                } else if after_return == SyntaxKind::IDENT {
                    let after_that = p.peek_kind(skip_trivia_lookahead(p, lookahead + 1));
                    // If followed by name + colon/typing/default, it's a parameter declaration
                    // EQ handles: return p = expr; (named result with default value)
                    if after_that == SyntaxKind::COLON
                        || after_that == SyntaxKind::TYPED_KW
                        || after_that == SyntaxKind::L_BRACKET
                        || after_that == SyntaxKind::COLON_GT
                        || after_that == SyntaxKind::COLON_GT_GT
                        || after_that == SyntaxKind::SEMICOLON
                        || after_that == SyntaxKind::EQ
                    {
                        parse_usage(p);
                    } else {
                        // return expression statement
                        parse_return_expression(p);
                    }
                } else if is_usage_keyword(after_return) {
                    // return part x; or return attribute y;
                    parse_usage(p);
                } else {
                    // return expression statement
                    parse_return_expression(p);
                }
            }
            // Behavior usages (perform, send, accept, etc.)
            else if p.at(SyntaxKind::PERFORM_KW) {
                parse_perform_action(p);
            } else if p.at(SyntaxKind::SEND_KW) {
                parse_send_action(p);
            } else if p.at(SyntaxKind::ACCEPT_KW) {
                parse_accept_action(p);
            }
            // General namespace elements (definitions, usages, etc.)
            else if p.at_any(&[
                SyntaxKind::ATTRIBUTE_KW,
                SyntaxKind::PART_KW,
                SyntaxKind::ITEM_KW,
                SyntaxKind::CALC_KW,
                SyntaxKind::CONSTRAINT_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::DOC_KW,
                SyntaxKind::COMMENT_KW,
                SyntaxKind::PRIVATE_KW,
                SyntaxKind::PUBLIC_KW,
                SyntaxKind::PROTECTED_KW,
            ]) {
                parse_package_body_element(p);
            }
            // Result expression (identifier, new, literal, or any expression start)
            // Per sysml.pest: calculation_body_item includes result_expression_member
            else if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
                // Optional semicolon for expression statements
                if p.at(SyntaxKind::SEMICOLON) {
                    p.bump();
                }
            } else {
                parse_package_body_element(p);
            }

            p.skip_trivia();

            if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}'", text)
                } else {
                    p.current_kind().display_name().to_string()
                };
                p.error(format!("unexpected {} in calc body", got));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        error_missing_body_terminator(p, "calc definition");
    }

    p.finish_node();
}

/// Parse flow usage (SysML-specific)\n/// Pattern: [succession] flow [name] [of Type] [from X] to Y [body]\n/// Per pest: flow_connection_usage = { succession_flow_connection_usage | item_flow }\n/// Per pest: item_flow = { \"flow\" ~ (item_flow_end ~ \"to\" ~ item_flow_end | \"of\" ~ item_feature ~ item_flow_end?) }\n/// Per pest: succession_flow_connection_usage = { \"succession\" ~ \"flow\" ~ (succession_item_flow | flow_usage_declaration ~ succession_flow_connection_block) }\n/// Pattern: [succession] flow [all] [<name>|of <type>] [from <source>] to <target> <body|semicolon>
pub fn parse_flow_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    if p.at(SyntaxKind::ABSTRACT_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Handle optional succession keyword (succession flow)
    if p.at(SyntaxKind::SUCCESSION_KW) {
        p.bump();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::FLOW_KW);
    p.skip_trivia();

    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Check for direct flow pattern first (e.g., "flow X.Y to A.B")
    let is_direct_flow = peek_for_direct_flow(p);

    // Check for "flow of Type" pattern (no name, just typing)
    let has_of_clause = p.at(SyntaxKind::OF_KW);

    if is_direct_flow {
        p.parse_qualified_name();
        p.skip_trivia();

        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
        }
    } else if has_of_clause {
        // Pattern: flow of Type [mult] from X to Y
        p.bump(); // of
        p.skip_trivia();
        p.parse_qualified_name(); // Type
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Flow part: from X to Y or X to Y - wrap in FROM_TO_CLAUSE
        parse_optional_from_to(p);
    } else {
        // Pattern: flow [name] [: Type] [...] [from X to Y]
        // But skip identification if we're directly at FROM_KW (pattern: flow from X to Y)
        if (p.at_name_token() || p.at(SyntaxKind::LT)) && !p.at(SyntaxKind::FROM_KW) {
            p.parse_identification();
            p.skip_trivia();
        }

        // Parse multiplicity bounds (e.g., [1])
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        parse_specializations(p);
        p.skip_trivia();

        // Default value assignment (per sysml.pest: value_part in flow declarations)
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }

        // Optional 'of Type' for named flows
        if p.at(SyntaxKind::OF_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
            p.skip_trivia();

            // Multiplicity after of clause
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }
        }

        // Flow part: from X to Y - wrap in FROM_TO_CLAUSE
        parse_optional_from_to(p);
    }

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// Helper to detect direct flow pattern (flow X.Y to A.B) vs named flow (flow name from X to Y)
fn peek_for_direct_flow<P: SysMLParser>(p: &P) -> bool {
    // Check if we see "name [.name]* to ..." pattern (direct flow endpoints)
    // vs "name : Type ..." pattern (declaration)

    // If we're currently at FROM_KW, this is definitely a from/to pattern, not direct
    if p.current_kind() == SyntaxKind::FROM_KW {
        return false;
    }

    // If we see a colon, it's a typed declaration
    if p.peek_kind(1) == SyntaxKind::COLON {
        return false;
    }

    // If we see FROM_KW before TO_KW, it's a named flow with from/to pattern, not direct
    // Pattern: "flow name from X to Y" vs "flow X to Y"
    let mut saw_from = false;

    // Look ahead for 'to' keyword within first few tokens
    for i in 1..9 {
        let kind = p.peek_kind(i);

        if kind == SyntaxKind::FROM_KW {
            saw_from = true;
        }

        if kind == SyntaxKind::TO_KW {
            // If we saw FROM before TO, it's a from/to pattern with a name, not direct flow
            if saw_from {
                return false;
            }
            return true;
        }

        // Stop if we hit something that indicates declaration (colon, equals, specialization)
        if matches!(
            kind,
            SyntaxKind::COLON
                | SyntaxKind::EQ
                | SyntaxKind::COLON_EQ
                | SyntaxKind::COLON_GT
                | SyntaxKind::COLON_GT_GT
                | SyntaxKind::SPECIALIZES_KW
        ) {
            return false;
        }
        // Stop if we hit end of statement
        if matches!(
            kind,
            SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE | SyntaxKind::ERROR
        ) {
            return false;
        }
    }

    false
}

// =============================================================================
// SysML-specific Specialization, Annotation, and Relationship Parsing
// These are SysML-native implementations that don't depend on kerml.rs
// =============================================================================

/// Parse feature specializations (SysML-specific)
/// Per SysML Pest grammar:
/// feature_specialization_part = feature_specialization+ ~ multiplicity_part ~ feature_specialization*
///                              | feature_specialization+
///                              | multiplicity_part ~ feature_specialization*
///                              | multiplicity_part
/// feature_specialization = typings | subsettings | references | crosses | redefinitions
/// Per pest: feature_specialization = { typing | subsetting | redefinition | reference_subsetting | featuring | conjugation | ... }\n/// Per pest: typing = { \":\" ~ qualified_name ~ (\",\" ~ qualified_name)* | \"typed\" ~ \"by\" ~ qualified_name }\n/// Pattern: Handles all specialization operators: :, :>, :>>, ::>, typed, subsets, redefines, etc.
pub fn parse_specializations<P: SysMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::COLON,
        SyntaxKind::TYPED_KW,
        // Note: OF_KW removed - it's handled separately in message/flow parsing
        SyntaxKind::COLON_GT,
        SyntaxKind::COLON_GT_GT,
        SyntaxKind::COLON_COLON_GT,
        SyntaxKind::SPECIALIZES_KW,
        SyntaxKind::SUBSETS_KW,
        SyntaxKind::REDEFINES_KW,
        SyntaxKind::REFERENCES_KW,
        SyntaxKind::CONJUGATES_KW,
        SyntaxKind::TILDE,
        SyntaxKind::DISJOINT_KW,
        SyntaxKind::INTERSECTS_KW,
        SyntaxKind::DIFFERENCES_KW,
        SyntaxKind::UNIONS_KW,
        SyntaxKind::CHAINS_KW,
        SyntaxKind::INVERSE_KW,
        SyntaxKind::FEATURING_KW,
        SyntaxKind::CROSSES_KW,
        SyntaxKind::FAT_ARROW,
    ]) {
        // Handle typing specially as it has different structure
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
            p.parse_typing();
            p.skip_trivia();
            continue;
        }

        p.start_node(SyntaxKind::SPECIALIZATION);

        let keyword = p.current_kind();
        p.bump();
        p.skip_trivia();

        if (keyword == SyntaxKind::DISJOINT_KW && p.at(SyntaxKind::FROM_KW))
            || (keyword == SyntaxKind::INVERSE_KW && p.at(SyntaxKind::OF_KW))
        {
            p.bump();
            p.skip_trivia();
        }

        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
    }
}

/// Parse annotation (comment, doc, locale) - SysML-specific
/// Per SysML Pest grammar:
/// - locale_annotation = { locale_token ~ string_value ~ block_comment? }
/// - comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference)* ~ (block_comment | semi_colon)? }
/// - documentation = { doc_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (block_comment | semi_colon)? }
pub fn parse_annotation<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::COMMENT_ELEMENT);

    // Metadata feature: @ or @@ or metadata keyword followed by reference
    if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) || p.at(SyntaxKind::METADATA_KW) {
        // All annotation markers get bumped the same way
        p.bump(); // metadata, @, or @@
        p.skip_trivia();

        // Optional identification with typing
        if p.at_name_token()
            && p.peek_kind(1) != SyntaxKind::SEMICOLON
            && p.peek_kind(1) != SyntaxKind::L_BRACE
        {
            // Could be identification if followed by : or typed
            let next = p.peek_kind(1);
            if next == SyntaxKind::COLON || next == SyntaxKind::TYPED_KW {
                p.parse_identification();
                p.skip_trivia();
                if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
                    p.bump();
                    p.skip_trivia();
                    if p.at(SyntaxKind::BY_KW) {
                        p.bump();
                        p.skip_trivia();
                    }
                }
            }
        }

        // Qualified reference
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Optional 'about' clause
        if p.at(SyntaxKind::ABOUT_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name_list();
            p.skip_trivia();
        }

        // Body or semicolon
        if p.at(SyntaxKind::L_BRACE) {
            parse_annotation_body(p);
        } else if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }

        p.finish_node();
        return;
    }

    // Locale annotation: locale "en_US" /* text */
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();

        // String value after locale
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }

        // Optional block comment content
        if p.at(SyntaxKind::BLOCK_COMMENT) {
            p.bump();
        }

        p.finish_node();
        return;
    }

    // comment or doc keyword
    if p.at(SyntaxKind::COMMENT_KW) || p.at(SyntaxKind::DOC_KW) {
        p.bump();
    }

    p.skip_trivia_except_block_comments();

    // Check for block comment content first (doc /* text */)
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional identification (can be identifier or short name with <)
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Check this isn't 'about' or 'locale'
        if !p.at(SyntaxKind::ABOUT_KW) && !p.at(SyntaxKind::LOCALE_KW) {
            p.parse_identification();
            p.skip_trivia_except_block_comments();
        }
    }

    // Check for block comment after identification
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional locale (can appear after identification)
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }
    }

    // Check for block comment after locale
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Optional 'about' targets
    if p.at(SyntaxKind::ABOUT_KW) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name_list();
        p.skip_trivia_except_block_comments();

        // locale can also appear after 'about'
        if p.at(SyntaxKind::LOCALE_KW) {
            p.bump();
            p.skip_trivia_except_block_comments();
            if p.at(SyntaxKind::STRING) {
                p.bump();
                p.skip_trivia_except_block_comments();
            }
        }
    }

    // Check for block comment after 'about'
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        p.finish_node();
        return;
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

/// Parse annotation body
fn parse_annotation_body<P: SysMLParser>(p: &mut P) {
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    // Content inside braces - for now just skip to closing brace
    let mut depth = 1;
    while depth > 0 {
        if p.at(SyntaxKind::L_BRACE) {
            depth += 1;
            p.bump();
        } else if p.at(SyntaxKind::R_BRACE) {
            depth -= 1;
            if depth > 0 {
                p.bump();
            }
        } else if p.current_kind() == SyntaxKind::ERROR {
            break; // EOF
        } else {
            p.bump();
        }
    }

    p.expect(SyntaxKind::R_BRACE);
}

/// Parse standalone relationship declarations (SysML-specific)
/// E.g., `specialization Super subclassifier A specializes B;`
/// E.g., `subclassifier C specializes A;`
/// E.g., `redefinition MyRedef redefines x :>> y;`
/// Per SysML Pest grammar: specialization_prefix ~ relationship_keyword ~ from ~ operator ~ to ~ relationship_body
pub fn parse_standalone_relationship<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Handle optional 'specialization' prefix with optional identification
    let _has_specialization = if p.at(SyntaxKind::SPECIALIZATION_KW) {
        p.bump(); // specialization
        p.skip_trivia();

        // Check for optional identification after 'specialization'
        // The next token should be one of the relationship keywords if no identification
        if p.at_name_token()
            && !p.at_any(&[
                SyntaxKind::SUBCLASSIFIER_KW,
                SyntaxKind::SUBTYPE_KW,
                SyntaxKind::SUBSET_KW,
                SyntaxKind::REDEFINITION_KW,
                SyntaxKind::TYPING_KW,
            ])
        {
            p.parse_identification();
            p.skip_trivia();
        }
        true
    } else {
        false
    };

    // Handle special featuring syntax: featuring [id? of]? feature by type
    if p.at(SyntaxKind::FEATURING_KW) {
        p.bump(); // featuring
        p.skip_trivia();

        // Check for optional identification + 'of'
        // We can tell by looking ahead for 'of' after a name
        if p.at_name_token() {
            // Parse first name (could be id or feature)
            p.parse_identification();
            p.skip_trivia();

            // If 'of' follows, parse the actual feature reference
            if p.at(SyntaxKind::OF_KW) {
                p.bump(); // of
                p.skip_trivia();
                if p.at_name_token() {
                    p.parse_qualified_name();
                    p.skip_trivia();
                }
            }
            // Otherwise the identification was the feature reference itself
        }

        // Parse 'by' clause
        if p.at(SyntaxKind::BY_KW) {
            p.bump(); // by
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle special typing syntax: [specialization id?]? typing feature (':' | 'typed by') type
    if p.at(SyntaxKind::TYPING_KW) {
        p.bump(); // typing
        p.skip_trivia();

        // Parse the feature being typed
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse typing operator (: or 'typed by')
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
            p.parse_typing();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle special conjugation syntax: [conjugation id?]? conjugate type1 ('~' | 'conjugates') type2
    if p.at(SyntaxKind::CONJUGATION_KW) {
        p.bump(); // conjugation
        p.skip_trivia();

        // Optional identification
        if p.at_name_token() && !p.at(SyntaxKind::CONJUGATE_KW) {
            p.parse_identification();
            p.skip_trivia();
        }

        // Expect 'conjugate' keyword
        if p.at(SyntaxKind::CONJUGATE_KW) {
            p.bump(); // conjugate
            p.skip_trivia();
        }

        // Parse first type (the conjugate type)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // Parse 'conjugates' or '~' operator
        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            p.bump();
            p.skip_trivia();
        }

        // Parse second type (the original type)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Handle the relationship keyword (subclassifier, subtype, subset, redefinition, etc.)
    if p.at_any(STANDALONE_RELATIONSHIP_KEYWORDS) {
        p.bump(); // relationship keyword
        p.skip_trivia();
    }

    // Parse the source element (before the operator)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Parse the operator (specializes/:>, subsets/:>, redefines/:>>, etc.)
    if p.at_any(RELATIONSHIP_OPERATORS) {
        p.bump(); // operator
        p.skip_trivia();
    }

    // Parse the target element (after the operator)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}

/// Parse SysML parameter (return, in, out, inout)\n/// This extends KerML parameters with SysML-specific prefixes like REF_KW\n/// Per pest: feature_member = { direction? ~ (usage_prefix* ~ usage_element | owned_feature_declaration) }\n/// Per pest: direction = { \"in\" | \"out\" | \"inout\" }\n/// Per pest: usage_prefix = { ref_prefix | abstract_prefix | readonly_prefix | derived_prefix | end_prefix | ... }\n/// Pattern: in|out|inout|return [ref|readonly|...] [usage_keyword] [<name>|:>> <ref>] [mult] [typing] [specializations] [default] semicolon
pub fn parse_sysml_parameter<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parameter direction keyword
    if p.at_any(&[
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::END_KW,
        SyntaxKind::RETURN_KW,
    ]) {
        p.bump();
    }
    p.skip_trivia();

    // SysML-specific prefixes (ref, readonly, etc.)
    while p.at_any(&[
        SyntaxKind::REF_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::MEMBER_KW,
        SyntaxKind::ABSTRACT_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    // Optional usage keyword (attribute, part, etc.)
    if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::PORT_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    // Redefines/subsets or identification
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
    {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }

    p.skip_trivia();

    // Multiplicity before typing
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Typing
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
        p.parse_typing();
    }

    p.skip_trivia();

    // Multiplicity after typing
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Default value
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
    }

    p.skip_trivia();
    p.parse_body();

    p.finish_node();
}

/// Parse a return expression statement: return <expression>;
/// This is different from return parameter declaration (return x : Type;)
/// Pattern: return <expression> ;
fn parse_return_expression<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::RETURN_KW);
    p.skip_trivia();

    // Parse the expression
    parse_expression(p);
    p.skip_trivia();

    // Expect semicolon
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysml_definition_keywords() {
        assert!(is_sysml_definition_keyword(SyntaxKind::PART_KW));
        assert!(is_sysml_definition_keyword(SyntaxKind::ACTION_KW));
        assert!(is_sysml_definition_keyword(SyntaxKind::REQUIREMENT_KW));
        assert!(!is_sysml_definition_keyword(SyntaxKind::CLASS_KW)); // KerML
    }

    #[test]
    fn test_sysml_usage_keywords() {
        assert!(is_sysml_usage_keyword(SyntaxKind::PART_KW));
        assert!(is_sysml_usage_keyword(SyntaxKind::SEND_KW));
        assert!(is_sysml_usage_keyword(SyntaxKind::PERFORM_KW));
    }
}
