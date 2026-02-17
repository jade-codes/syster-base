//! KerML grammar parsing
//!
//! This module contains functions for parsing KerML-specific constructs:
//! - Definitions (class, struct, datatype, behavior, function, etc.)
//! - Usages (feature, step, expr)
//! - Standalone relationships (specialization, subclassification, etc.)
//! - Annotations (comment, doc, locale)
//! - Core parsing utilities (qualified names, identification, typing, etc.)
//!
//! Based on kerml.pest grammar.

// Submodules
mod annotations;
mod connectors;
mod definitions;
mod entry;
mod relationships;
mod usages;

// Shared imports — pub(super) so submodules get them via `use super::*;`
pub(super) use super::BaseParser;
pub(super) use super::kerml_expressions;
pub(super) use super::{RELATIONSHIP_OPERATORS, STANDALONE_RELATIONSHIP_KEYWORDS};
pub(super) use crate::parser::syntax_kind::SyntaxKind;

// Internal re-exports — submodules access siblings via `use super::*;`
pub(super) use self::annotations::*;
pub(super) use self::entry::*;
pub(super) use self::relationships::*;

// Public API — visible outside kerml module (used by grammar/mod.rs, parser.rs, sysml/)
pub use self::connectors::{parse_connector_usage, parse_flow_usage};
pub use self::definitions::{
    parse_alias, parse_calc_body, parse_definition_impl, parse_import, parse_library_package,
    parse_multiplicity, parse_multiplicity_definition, parse_package, parse_specializations,
    parse_typing,
};
pub use self::entry::{parse_kerml_file, parse_namespace_element};
pub use self::usages::{
    parse_end_feature_or_parameter, parse_feature_prefix_modifiers, parse_invariant,
    parse_parameter_impl, parse_usage_impl,
};

// =============================================================================
// Constants
// =============================================================================

/// KerML definition keywords
pub const KERML_DEFINITION_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::CLASS_KW,
    SyntaxKind::STRUCT_KW,
    SyntaxKind::DATATYPE_KW,
    SyntaxKind::BEHAVIOR_KW,
    SyntaxKind::FUNCTION_KW,
    SyntaxKind::CLASSIFIER_KW,
    SyntaxKind::INTERACTION_KW,
    SyntaxKind::PREDICATE_KW,
    SyntaxKind::METACLASS_KW,
    SyntaxKind::ASSOC_KW,
];

/// KerML usage keywords (feature, step, expr)
pub const KERML_USAGE_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::FEATURE_KW,
    SyntaxKind::STEP_KW,
    SyntaxKind::EXPR_KW,
];

/// Feature prefix modifiers per Pest grammar:
/// feature_prefix_modifiers = { (abstract | composite | portion | member | const | derived | end | var)* }
pub const FEATURE_PREFIX_MODIFIERS: &[SyntaxKind] = &[
    SyntaxKind::VAR_KW,
    SyntaxKind::COMPOSITE_KW,
    SyntaxKind::PORTION_KW,
    SyntaxKind::MEMBER_KW,
    SyntaxKind::ABSTRACT_KW,
    SyntaxKind::DERIVED_KW,
    SyntaxKind::CONST_KW,
    SyntaxKind::END_KW,
];

/// Definition prefixes (abstract, variation)
pub const DEFINITION_PREFIXES: &[SyntaxKind] = &[SyntaxKind::ABSTRACT_KW, SyntaxKind::VARIATION_KW];

// =============================================================================
// Keyword predicate functions
// =============================================================================

/// Check if a kind is a KerML definition keyword
pub fn is_kerml_definition_keyword(kind: SyntaxKind) -> bool {
    KERML_DEFINITION_KEYWORDS.contains(&kind)
}

/// Check if a kind is a KerML usage keyword
pub fn is_kerml_usage_keyword(kind: SyntaxKind) -> bool {
    KERML_USAGE_KEYWORDS.contains(&kind)
}

/// Check if a kind is a feature prefix modifier
pub fn is_feature_prefix_modifier(kind: SyntaxKind) -> bool {
    FEATURE_PREFIX_MODIFIERS.contains(&kind)
}

/// Check if a kind is a standalone relationship keyword
pub fn is_standalone_relationship_keyword(kind: SyntaxKind) -> bool {
    STANDALONE_RELATIONSHIP_KEYWORDS.contains(&kind)
}

// =============================================================================
// KerMLParser trait
// =============================================================================

/// Trait for KerML parsing operations
///
/// Extends ExpressionParser with KerML-specific methods.
/// The main parser implements this trait.
pub trait KerMLParser: BaseParser {
    /// Parse a body (semicolon or braced block with KerML members)
    fn parse_body(&mut self);

    // -----------------------------------------------------------------
    // KerML namespace member parsing
    // -----------------------------------------------------------------

    /// Parse a namespace member (KerML level)
    ///
    /// This handles all KerML namespace body elements:
    /// - Definitions: class, struct, datatype, behavior, function, etc.
    /// - Usages: feature, step, expr
    /// - Relationships: specialization, subclassification, etc.
    /// - Annotations: comment, doc
    /// - Import/alias
    fn parse_namespace_member(&mut self)
    where
        Self: Sized,
    {
        parse_namespace_element(self);
    }

    // -----------------------------------------------------------------
    // KerML-specific element parsers (called by parse_namespace_element)
    // -----------------------------------------------------------------

    /// Parse a package: 'package' | 'namespace' Identification? Body
    fn parse_package(&mut self);

    /// Parse a library package: 'standard'? 'library' 'package' ...
    fn parse_library_package(&mut self);

    /// Parse an import statement
    fn parse_import(&mut self);

    /// Parse an alias
    fn parse_alias(&mut self);

    /// Parse a definition (class, struct, datatype, etc.)
    fn parse_definition(&mut self);

    /// Parse a usage (feature, step, expr)
    fn parse_usage(&mut self);

    /// Parse an invariant (inv [not]? name? { expr })
    fn parse_invariant(&mut self);

    /// Parse a parameter (in, out, inout, return)
    fn parse_parameter(&mut self);

    /// Parse end feature or parameter
    fn parse_end_feature_or_parameter(&mut self);

    /// Parse a connector usage
    fn parse_connector_usage(&mut self);

    /// Parse a flow usage (KerML item_flow)
    fn parse_flow_usage(&mut self);
}

// =============================================================================
// Helper Functions — Common Patterns (used across all submodules)
// =============================================================================

/// Emit an error for missing body terminator with context
pub(super) fn error_missing_body_terminator<P: KerMLParser>(p: &mut P, context: &str) {
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

/// Bump current token and skip trivia (used 100+ times)
#[inline]
pub(super) fn bump_and_skip<P: KerMLParser>(p: &mut P) {
    p.bump();
    p.skip_trivia();
}

/// Expect a token and skip trivia (used 20+ times)
#[inline]
pub(super) fn expect_and_skip<P: KerMLParser>(p: &mut P, kind: SyntaxKind) {
    p.expect(kind);
    p.skip_trivia();
}

/// Conditionally bump if at a specific token, then skip trivia
#[inline]
pub(super) fn consume_if<P: KerMLParser>(p: &mut P, kind: SyntaxKind) -> bool {
    if p.at(kind) {
        p.bump();
        p.skip_trivia();
        true
    } else {
        false
    }
}

/// Parse qualified name and skip trivia
#[inline]
pub(super) fn parse_qualified_name_and_skip<P: KerMLParser>(p: &mut P) {
    p.parse_qualified_name();
    p.skip_trivia();
}

/// Parse identification and skip trivia
#[inline]
pub(super) fn parse_identification_and_skip<P: KerMLParser>(p: &mut P) {
    p.parse_identification();
    p.skip_trivia();
}

/// Parse optional identification (if at name token or <)
pub(super) fn parse_optional_identification<P: KerMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }
}

/// Parse optional qualified name
pub(super) fn parse_optional_qualified_name<P: KerMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::THIS_KW) {
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Parse optional visibility (public, private, protected)
/// Per pest: visibility_kind = { public | private | protected }
#[inline]
pub(super) fn parse_optional_visibility<P: KerMLParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        bump_and_skip(p);
    }
}

/// Parse optional multiplicity [expression]
pub(super) fn parse_optional_multiplicity<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }
}

/// Parse optional typing (: Type or typed by Type)
pub(super) fn parse_optional_typing<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) || p.at(SyntaxKind::OF_KW) {
        parse_typing(p);
        p.skip_trivia();
    }
}

/// Parse comma-separated qualified names
pub(super) fn parse_comma_separated_names<P: KerMLParser>(p: &mut P) {
    parse_qualified_name_and_skip(p);

    while p.at(SyntaxKind::COMMA) {
        bump_and_skip(p);
        if p.at_name_token() {
            parse_qualified_name_and_skip(p);
        }
    }
}

/// Check if current position looks like a qualified name (with dots/::) followed by specific keywords.
/// This helps distinguish between identification and direct endpoint syntax.
/// Returns true if we see: name.name... (or name::name...) followed by one of target_kinds
pub(super) fn looks_like_qualified_name_before<P: KerMLParser>(p: &P, target_kinds: &[SyntaxKind]) -> bool {
    if !p.at_name_token() {
        return false;
    }

    let mut peek_idx = 1;
    let mut has_qualifier = false;

    loop {
        // Skip trivia to get next meaningful token
        let mut kind = p.peek_kind(peek_idx);
        while kind.is_trivia() {
            peek_idx += 1;
            kind = p.peek_kind(peek_idx);
        }

        if kind == SyntaxKind::DOT || kind == SyntaxKind::COLON_COLON {
            has_qualifier = true;
            peek_idx += 1;

            // Skip trivia after qualifier
            kind = p.peek_kind(peek_idx);
            while kind.is_trivia() {
                peek_idx += 1;
                kind = p.peek_kind(peek_idx);
            }

            // Expect a name after qualifier
            if is_name_kind(kind) {
                peek_idx += 1;
                continue;
            } else {
                return false;
            }
        } else if target_kinds.contains(&kind) {
            // Found target keyword - only treat as qualified if we saw dots/::
            return has_qualifier;
        } else {
            return false;
        }
    }
}

/// Returns true if we see: name (possibly qualified with :: or .) followed by target_kind
/// Unlike looks_like_qualified_name_before, this returns true for both simple and qualified names.
/// Used for binding patterns where `binding payload = target` has `payload` as a direct endpoint.
pub(super) fn looks_like_name_then<P: KerMLParser>(p: &P, target_kind: SyntaxKind) -> bool {
    if !p.at_name_token() {
        return false;
    }

    let mut peek_idx = 1;

    loop {
        // Skip trivia to get next meaningful token
        let mut kind = p.peek_kind(peek_idx);
        while kind.is_trivia() {
            peek_idx += 1;
            kind = p.peek_kind(peek_idx);
        }

        if kind == SyntaxKind::DOT || kind == SyntaxKind::COLON_COLON {
            peek_idx += 1;

            // Skip trivia after qualifier
            kind = p.peek_kind(peek_idx);
            while kind.is_trivia() {
                peek_idx += 1;
                kind = p.peek_kind(peek_idx);
            }

            // Expect a name after qualifier
            if is_name_kind(kind) {
                peek_idx += 1;
                continue;
            } else {
                return false;
            }
        } else if kind == target_kind {
            // Found target keyword
            return true;
        } else {
            return false;
        }
    }
}

// =============================================================================
// Core Parsing Functions — Foundational, used across all submodules
// =============================================================================

/// Check if a syntax kind can be used as a name token.
pub fn is_name_kind(kind: SyntaxKind) -> bool {
    if kind == SyntaxKind::IDENT {
        return true;
    }
    !matches!(
        kind,
        SyntaxKind::ERROR
            | SyntaxKind::WHITESPACE
            | SyntaxKind::LINE_COMMENT
            | SyntaxKind::BLOCK_COMMENT
            | SyntaxKind::L_BRACE
            | SyntaxKind::R_BRACE
            | SyntaxKind::L_BRACKET
            | SyntaxKind::R_BRACKET
            | SyntaxKind::L_PAREN
            | SyntaxKind::R_PAREN
            | SyntaxKind::SEMICOLON
            | SyntaxKind::COLON
            | SyntaxKind::COLON_COLON
            | SyntaxKind::COLON_GT
            | SyntaxKind::COLON_GT_GT
            | SyntaxKind::COLON_COLON_GT
            | SyntaxKind::DOT
            | SyntaxKind::DOT_DOT
            | SyntaxKind::COMMA
            | SyntaxKind::EQ
            | SyntaxKind::EQ_EQ
            | SyntaxKind::EQ_EQ_EQ
            | SyntaxKind::BANG_EQ
            | SyntaxKind::BANG_EQ_EQ
            | SyntaxKind::LT
            | SyntaxKind::GT
            | SyntaxKind::LT_EQ
            | SyntaxKind::GT_EQ
            | SyntaxKind::AT
            | SyntaxKind::AT_AT
            | SyntaxKind::HASH
            | SyntaxKind::STAR
            | SyntaxKind::STAR_STAR
            | SyntaxKind::PLUS
            | SyntaxKind::MINUS
            | SyntaxKind::SLASH
            | SyntaxKind::PERCENT
            | SyntaxKind::CARET
            | SyntaxKind::AMP
            | SyntaxKind::AMP_AMP
            | SyntaxKind::PIPE
            | SyntaxKind::PIPE_PIPE
            | SyntaxKind::BANG
            | SyntaxKind::TILDE
            | SyntaxKind::QUESTION
            | SyntaxKind::QUESTION_QUESTION
            | SyntaxKind::ARROW
            | SyntaxKind::FAT_ARROW
            | SyntaxKind::INTEGER
            | SyntaxKind::DECIMAL
            | SyntaxKind::STRING
    )
}

/// Identification = '<' ShortName '>' Name? | Name
/// Per pest: identification = { (short_name ~ regular_name?) | regular_name }
/// Per pest: short_name = { "<" ~ name ~ ">" }
/// Per pest: regular_name = { name }
/// Per pest: name_identifier allows keywords as identifiers (for short names like `<var>`)
pub fn parse_identification<P: KerMLParser>(p: &mut P) {
    // Skip trivia BEFORE starting the NAME node so the node's range
    // doesn't include leading whitespace
    p.skip_trivia();
    p.start_node(SyntaxKind::NAME);

    // Short name: <shortname>
    // Per pest grammar, short names can contain keywords as identifiers
    if p.at(SyntaxKind::LT) {
        p.start_node(SyntaxKind::SHORT_NAME);
        bump_and_skip(p); // <
        // Accept any identifier-like token including keywords for short names
        // This handles cases like `<var>` in SI.sysml
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

/// QualifiedName = Name ('::' Name | '.' Name)*
/// Also supports global qualification: $:: prefix
/// Check if we should stop before wildcard patterns (::* or ::**)
fn should_stop_before_wildcard<P: KerMLParser>(p: &P) -> bool {
    if p.at(SyntaxKind::COLON_COLON) {
        let peek = p.peek_kind(1);
        peek == SyntaxKind::STAR || peek == SyntaxKind::STAR_STAR
    } else {
        false
    }
}

/// Check if DOT should be consumed (next must be name)
fn should_consume_dot<P: KerMLParser>(p: &P) -> bool {
    if p.at(SyntaxKind::DOT) {
        is_name_kind(p.peek_kind(1))
    } else {
        true
    }
}

/// Per pest: qualified_name = { ("$" ~ "::")? ~ name ~ (("::" | ".") ~ name)* }
/// Supports global qualification ($::), namespace paths (::), and feature chains (.)
/// Wildcards (::*, ::**) are excluded and handled separately by import rules
pub fn parse_qualified_name<P: KerMLParser>(p: &mut P, _tokens: &[(SyntaxKind, usize)]) {
    p.start_node(SyntaxKind::QUALIFIED_NAME);

    // Handle global qualification $::
    if p.at(SyntaxKind::DOLLAR) {
        bump_and_skip(p);
        consume_if(p, SyntaxKind::COLON_COLON);
    }

    // Handle 'this' keyword as a special name (KerML self-reference)
    if p.at(SyntaxKind::THIS_KW) {
        p.bump();
        p.finish_node();
        return;
    }

    if p.at_name_token() {
        p.bump();
    }

    while p.at_any(&[SyntaxKind::COLON_COLON, SyntaxKind::DOT]) {
        if should_stop_before_wildcard(p) || !should_consume_dot(p) {
            break;
        }

        bump_and_skip(p); // :: or .
        if p.at_name_token() {
            p.bump();
        } else {
            break;
        }
    }

    p.finish_node();
}

/// Body = ';' | '{' BodyMember* '}'
/// Heuristic: check if current token looks like start of expression
/// This is used to determine if a namespace body element is an expression (like x->y)
/// vs a member declaration (like feature x: Type)
fn looks_like_expression<P: KerMLParser>(p: &P) -> bool {
    if p.at(SyntaxKind::IDENT) {
        let peek1 = p.peek_kind(1);
        matches!(
            peek1,
            SyntaxKind::GT | SyntaxKind::LT | SyntaxKind::PLUS | SyntaxKind::MINUS |
            SyntaxKind::STAR | SyntaxKind::SLASH | SyntaxKind::PERCENT |
            SyntaxKind::EQ_EQ | SyntaxKind::BANG_EQ | SyntaxKind::LT_EQ | SyntaxKind::GT_EQ |
            SyntaxKind::EQ_EQ_EQ | SyntaxKind::BANG_EQ_EQ |
            SyntaxKind::AMP_AMP | SyntaxKind::PIPE_PIPE | SyntaxKind::CARET |
            SyntaxKind::DOT | // Could be feature chain in expression
            SyntaxKind::ARROW | // x->forAll{...} etc collection operations
            SyntaxKind::R_BRACE // Bare identifier at end of body is result expression
        )
    } else {
        false
    }
}

/// Try to recover from parsing failure in body
fn recover_body_element<P: KerMLParser>(p: &mut P) {
    let expr_start = p.get_pos();
    kerml_expressions::parse_expression(p);
    if p.get_pos() > expr_start {
        // Expression was parsed, continue
        p.skip_trivia();
    } else {
        let got = if let Some(text) = p.current_token_text() {
            format!("'{}'", text)
        } else {
            p.current_kind().display_name().to_string()
        };
        p.error(format!("unexpected {} in body", got));
        p.bump();
    }
}

/// Per pest: namespace_body = { ";" | ("{" ~ namespace_body_elements ~ "}") }
/// Per pest: type_body = { namespace_body | ";" }
pub fn parse_body<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        bump_and_skip(p);

        while !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
            let start_pos = p.get_pos();

            if looks_like_expression(p) {
                kerml_expressions::parse_expression(p);
            } else {
                parse_namespace_element(p);
            }

            p.skip_trivia();

            // Recovery if no progress made
            if p.get_pos() == start_pos && !p.at(SyntaxKind::ERROR) && !p.at(SyntaxKind::R_BRACE) {
                recover_body_element(p);
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
// Feature Relationships — used by both usages and connectors
// =============================================================================

/// Parse feature relationship parts that appear after specializations
/// Handles: featured by, inverse of, chains, crosses
/// Parse 'featured by X' relationship
fn parse_featured_by<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);
    bump_and_skip(p);
    if p.at(SyntaxKind::BY_KW) {
        bump_and_skip(p);
        parse_optional_qualified_name(p);
    }
    p.finish_node();
}

/// Parse 'inverse of X' relationship
fn parse_inverse_of<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);
    bump_and_skip(p);
    if p.at(SyntaxKind::OF_KW) {
        bump_and_skip(p);
        parse_optional_qualified_name(p);
    }
    p.finish_node();
}

/// Parse 'chains X', 'crosses X', or '=> X' relationship
fn parse_simple_relationship<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);
    bump_and_skip(p);
    parse_optional_qualified_name(p);
    p.finish_node();
}

pub(super) fn parse_feature_relationships<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::FEATURED_KW,
        SyntaxKind::INVERSE_KW,
        SyntaxKind::CHAINS_KW,
        SyntaxKind::CROSSES_KW,
        SyntaxKind::FAT_ARROW,
    ]) {
        p.skip_trivia();
        match p.current_kind() {
            SyntaxKind::FEATURED_KW => parse_featured_by(p),
            SyntaxKind::INVERSE_KW => parse_inverse_of(p),
            SyntaxKind::CHAINS_KW | SyntaxKind::CROSSES_KW | SyntaxKind::FAT_ARROW => {
                parse_simple_relationship(p);
            }
            _ => break,
        }
    }
}
