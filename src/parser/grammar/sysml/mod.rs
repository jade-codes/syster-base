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

// Submodules
mod actions;
mod body;
mod definitions;
mod entry;
mod helpers;
mod namespace;
mod relationships;
mod requirements;
mod states;

// Shared imports — pub(super) so submodules get them via `use super::*;`
pub(super) use super::kerml::is_name_kind;
pub(super) use super::kerml_expressions::{ExpressionParser, parse_expression};
pub(super) use super::{RELATIONSHIP_OPERATORS, STANDALONE_RELATIONSHIP_KEYWORDS};
pub(super) use crate::parser::syntax_kind::SyntaxKind;

// Internal re-exports — submodules access siblings via `use super::*;`
pub(super) use self::actions::*;
pub(super) use self::definitions::*;
pub(super) use self::entry::*;
use self::helpers::*; // pub(super) relative to helpers → private import here, still visible to descendants
pub(super) use self::namespace::*;
pub(super) use self::relationships::*;
pub(super) use self::requirements::*;
pub(super) use self::states::*;

// Public API — visible outside sysml module (used by parser.rs, rule_parser.rs)
pub use self::body::parse_body;
pub use self::definitions::{
    parse_binding_or_succession, parse_connect_usage, parse_constraint_body,
    parse_definition_or_usage, parse_dependency, parse_filter, parse_metadata_usage,
    parse_redefines_feature_member, parse_shorthand_feature_member, parse_variant_usage,
};
pub use self::entry::parse_sysml_file;

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
