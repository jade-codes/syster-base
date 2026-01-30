//! Common parsing utilities
//!
//! This module contains shared infrastructure used across all grammar modules:
//! - Recovery token sets
//! - Common keyword sets
//! - Utility functions
//! - Shared parsing functions (qualified names, identification, typing, etc.)

use crate::syntax::rowan_parser::syntax_kind::SyntaxKind;
use super::kerml_expressions::ExpressionParser;

/// Tokens that typically signal the end of a statement or member
/// Used for error recovery to skip to the next valid position
pub const STATEMENT_RECOVERY: &[SyntaxKind] = &[
    SyntaxKind::SEMICOLON,
    SyntaxKind::R_BRACE,
    SyntaxKind::PACKAGE_KW,
    SyntaxKind::IMPORT_KW,
    SyntaxKind::PART_KW,
    SyntaxKind::ATTRIBUTE_KW,
    SyntaxKind::PORT_KW,
    SyntaxKind::ITEM_KW,
    SyntaxKind::ACTION_KW,
    SyntaxKind::STATE_KW,
    SyntaxKind::CONSTRAINT_KW,
    SyntaxKind::REQUIREMENT_KW,
    SyntaxKind::CLASS_KW,
    SyntaxKind::STRUCT_KW,
    SyntaxKind::FEATURE_KW,
];

/// Tokens that can start a namespace member
pub const MEMBER_START: &[SyntaxKind] = &[
    SyntaxKind::PACKAGE_KW,
    SyntaxKind::LIBRARY_KW,
    SyntaxKind::STANDARD_KW,
    SyntaxKind::IMPORT_KW,
    SyntaxKind::ALIAS_KW,
    SyntaxKind::DEPENDENCY_KW,
    SyntaxKind::COMMENT_KW,
    SyntaxKind::DOC_KW,
    SyntaxKind::FILTER_KW,
    SyntaxKind::AT,
    SyntaxKind::HASH,
    SyntaxKind::ABSTRACT_KW,
    SyntaxKind::VARIATION_KW,
    SyntaxKind::PUBLIC_KW,
    SyntaxKind::PRIVATE_KW,
    SyntaxKind::PROTECTED_KW,
    // SysML keywords
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
    SyntaxKind::METADATA_KW,
    // KerML keywords
    SyntaxKind::CLASS_KW,
    SyntaxKind::STRUCT_KW,
    SyntaxKind::DATATYPE_KW,
    SyntaxKind::ASSOC_KW,
    SyntaxKind::BEHAVIOR_KW,
    SyntaxKind::FUNCTION_KW,
    SyntaxKind::PREDICATE_KW,
    SyntaxKind::INTERACTION_KW,
    SyntaxKind::FEATURE_KW,
    SyntaxKind::STEP_KW,
    SyntaxKind::EXPR_KW,
    SyntaxKind::CLASSIFIER_KW,
    SyntaxKind::TYPE_KW,
    SyntaxKind::METACLASS_KW,
    // Prefixes
    SyntaxKind::REF_KW,
    SyntaxKind::READONLY_KW,
    SyntaxKind::DERIVED_KW,
    SyntaxKind::END_KW,
    SyntaxKind::VAR_KW,
    SyntaxKind::COMPOSITE_KW,
    SyntaxKind::PORTION_KW,
    SyntaxKind::IN_KW,
    SyntaxKind::OUT_KW,
    SyntaxKind::INOUT_KW,
    SyntaxKind::CONST_KW,
    // Control flow
    SyntaxKind::IF_KW,
    SyntaxKind::THEN_KW,
    SyntaxKind::ELSE_KW,
    SyntaxKind::WHILE_KW,
    SyntaxKind::LOOP_KW,
    SyntaxKind::UNTIL_KW,
    SyntaxKind::FOR_KW,
    SyntaxKind::FORK_KW,
    SyntaxKind::JOIN_KW,
    SyntaxKind::MERGE_KW,
    SyntaxKind::DECIDE_KW,
    SyntaxKind::FIRST_KW,
    // Relationship keywords that can start members
    SyntaxKind::REDEFINES_KW,
    SyntaxKind::SUBSETS_KW,
    SyntaxKind::SPECIALIZES_KW,
    // Actions
    SyntaxKind::ACCEPT_KW,
    SyntaxKind::SEND_KW,
    SyntaxKind::PERFORM_KW,
    SyntaxKind::EXHIBIT_KW,
    SyntaxKind::INCLUDE_KW,
    SyntaxKind::SATISFY_KW,
    SyntaxKind::ASSERT_KW,
    SyntaxKind::ASSUME_KW,
    SyntaxKind::REQUIRE_KW,
    // Other member starters
    SyntaxKind::SUCCESSION_KW,
    SyntaxKind::BINDING_KW,
    SyntaxKind::CONNECT_KW,
    SyntaxKind::ENTRY_KW,
    SyntaxKind::EXIT_KW,
    SyntaxKind::DO_KW,
    SyntaxKind::TRANSITION_KW,
    SyntaxKind::SUBJECT_KW,
    SyntaxKind::ACTOR_KW,
    SyntaxKind::OBJECTIVE_KW,
    SyntaxKind::STAKEHOLDER_KW,
    SyntaxKind::VERIFY_KW,
    SyntaxKind::EXPOSE_KW,
    SyntaxKind::RENDER_KW,
];

/// Relationship keywords
pub const RELATIONSHIP_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::SPECIALIZES_KW,
    SyntaxKind::SUBSETS_KW,
    SyntaxKind::REDEFINES_KW,
    SyntaxKind::REFERENCES_KW,
    SyntaxKind::TYPED_KW,
    SyntaxKind::CONJUGATES_KW,
    SyntaxKind::DISJOINT_KW,
    SyntaxKind::INVERSE_KW,
    SyntaxKind::INTERSECTS_KW,
    SyntaxKind::DIFFERENCES_KW,
    SyntaxKind::UNIONS_KW,
    SyntaxKind::CHAINS_KW,
    SyntaxKind::FEATURING_KW,
    // Symbol forms
    SyntaxKind::COLON_GT,
    SyntaxKind::COLON_GT_GT,
    SyntaxKind::COLON_COLON_GT,
];

/// Direction keywords
pub const DIRECTION_KEYWORDS: &[SyntaxKind] = &[
    SyntaxKind::IN_KW,
    SyntaxKind::OUT_KW,
    SyntaxKind::INOUT_KW,
];

/// Check if a kind can start a member
pub fn can_start_member(kind: SyntaxKind) -> bool {
    MEMBER_START.contains(&kind)
}

/// Check if a kind is a relationship keyword
pub fn is_relationship_keyword(kind: SyntaxKind) -> bool {
    RELATIONSHIP_KEYWORDS.contains(&kind)
}

/// Check if a kind is a direction keyword
pub fn is_direction_keyword(kind: SyntaxKind) -> bool {
    DIRECTION_KEYWORDS.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_can_start_member() {
        assert!(can_start_member(SyntaxKind::PART_KW));
        assert!(can_start_member(SyntaxKind::CLASS_KW));
        assert!(can_start_member(SyntaxKind::IMPORT_KW));
        assert!(!can_start_member(SyntaxKind::SEMICOLON));
    }
    
    #[test]
    fn test_relationship_keywords() {
        assert!(is_relationship_keyword(SyntaxKind::SPECIALIZES_KW));
        assert!(is_relationship_keyword(SyntaxKind::COLON_GT));
        assert!(!is_relationship_keyword(SyntaxKind::PART_KW));
    }
    
    #[test]
    fn test_direction_keywords() {
        assert!(is_direction_keyword(SyntaxKind::IN_KW));
        assert!(is_direction_keyword(SyntaxKind::OUT_KW));
        assert!(!is_direction_keyword(SyntaxKind::PART_KW));
    }
}
