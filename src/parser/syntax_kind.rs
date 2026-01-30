//! Syntax kinds for the Rowan-based CST
//!
//! This enum defines all possible node and token kinds in the syntax tree.
//! It follows the SysML v2 specification grammar structure.

/// All syntax kinds (tokens and nodes) in SysML v2
///
/// Tokens are leaf nodes (identifiers, keywords, punctuation).
/// Nodes are composite (packages, definitions, usages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum SyntaxKind {
    // =========================================================================
    // TRIVIA (whitespace and comments - preserved but not semantically meaningful)
    // =========================================================================
    WHITESPACE = 0,
    LINE_COMMENT,
    BLOCK_COMMENT,

    // =========================================================================
    // LITERALS
    // =========================================================================
    IDENT,              // identifier
    INTEGER,            // 42
    DECIMAL,            // 3.14
    STRING,             // "hello" or 'hello'
    
    // =========================================================================
    // PUNCTUATION
    // =========================================================================
    L_BRACE,            // {
    R_BRACE,            // }
    L_BRACKET,          // [
    R_BRACKET,          // ]
    L_PAREN,            // (
    R_PAREN,            // )
    SEMICOLON,          // ;
    COLON,              // :
    COLON_COLON,        // ::
    COLON_GT,           // :>  (specializes/subsets)
    COLON_GT_GT,        // :>> (redefines)
    COLON_COLON_GT,     // ::> (references)
    DOT,                // .
    DOT_DOT,            // ..
    COMMA,              // ,
    EQ,                 // =
    EQ_EQ,              // ==
    EQ_EQ_EQ,           // ===
    BANG_EQ,            // !=
    BANG_EQ_EQ,         // !==
    LT,                 // <
    GT,                 // >
    LT_EQ,              // <=
    GT_EQ,              // >=
    ARROW,              // ->
    FAT_ARROW,          // =>
    AT,                 // @
    AT_AT,              // @@
    HASH,               // #
    STAR,               // *
    STAR_STAR,          // **
    PLUS,               // +
    MINUS,              // -
    SLASH,              // /
    PERCENT,            // %
    CARET,              // ^
    TILDE,              // ~
    QUESTION,           // ?
    QUESTION_QUESTION,  // ??
    BANG,               // !
    PIPE,               // |
    AMP,                // &
    AMP_AMP,            // &&
    PIPE_PIPE,          // ||
    COLON_EQ,           // :=
    DOLLAR,             // $

    // =========================================================================
    // KEYWORDS - SysML v2
    // =========================================================================
    // Namespace keywords
    PACKAGE_KW,
    LIBRARY_KW,
    STANDARD_KW,
    
    // Import/visibility
    IMPORT_KW,
    ALIAS_KW,
    ALL_KW,
    FILTER_KW,
    PRIVATE_KW,
    PROTECTED_KW,
    PUBLIC_KW,
    
    // Definition keywords
    DEF_KW,
    ABSTRACT_KW,
    COMPOSITE_KW,
    PORTION_KW,
    VARIATION_KW,
    VARIANT_KW,
    
    // Structure definitions
    PART_KW,
    ATTRIBUTE_KW,
    ENUMERATION_KW,
    ENUM_KW,
    ITEM_KW,
    OCCURRENCE_KW,
    INDIVIDUAL_KW,
    
    // Port/connection keywords
    PORT_KW,
    CONNECTION_KW,
    INTERFACE_KW,
    BINDING_KW,
    FLOW_KW,
    ALLOCATION_KW,
    ALLOCATE_KW,
    
    // Behavior keywords
    ACTION_KW,
    STATE_KW,
    TRANSITION_KW,
    ENTRY_KW,
    EXIT_KW,
    DO_KW,
    ACCEPT_KW,
    SEND_KW,
    PERFORM_KW,
    EXHIBIT_KW,
    
    // Message/event keywords
    MESSAGE_KW,
    SNAPSHOT_KW,
    TIMESLICE_KW,
    FRAME_KW,
    EVENT_KW,
    
    // Control flow
    IF_KW,
    ELSE_KW,
    THEN_KW,
    LOOP_KW,
    WHILE_KW,
    UNTIL_KW,
    FOR_KW,
    FORK_KW,
    JOIN_KW,
    MERGE_KW,
    DECIDE_KW,
    FIRST_KW,
    DONE_KW,
    START_KW,
    TERMINATE_KW,
    PARALLEL_KW,
    ASSIGN_KW,
    CONNECT_KW,
    
    // Action-specific
    BIND_KW,
    NEW_KW,
    AFTER_KW,
    AT_KW,
    WHEN_KW,
    VIA_KW,
    THIS_KW,
    
    // Calculation/constraint
    CALC_KW,
    CONSTRAINT_KW,
    ASSERT_KW,
    ASSUME_KW,
    REQUIRE_KW,
    
    // Requirement keywords
    REQUIREMENT_KW,
    SUBJECT_KW,
    OBJECTIVE_KW,
    STAKEHOLDER_KW,
    ACTOR_KW,
    CONCERN_KW,
    SATISFY_KW,
    VERIFY_KW,
    
    // Case keywords
    CASE_KW,
    ANALYSIS_KW,
    VERIFICATION_KW,
    USE_KW,
    INCLUDE_KW,
    
    // View keywords
    VIEW_KW,
    VIEWPOINT_KW,
    RENDERING_KW,
    RENDER_KW,
    EXPOSE_KW,
    
    // Metadata
    METACLASS_KW,
    METADATA_KW,
    ABOUT_KW,
    
    // Documentation
    DOC_KW,
    COMMENT_KW,
    LANGUAGE_KW,
    LOCALE_KW,
    REP_KW,
    
    // Relationship keywords
    SPECIALIZES_KW,
    SUBSETS_KW,
    REDEFINES_KW,
    REFERENCES_KW,
    TYPED_KW,
    DEFINED_KW,
    BY_KW,
    INTERSECTS_KW,
    UNIONS_KW,
    DISJOINT_KW,
    DISJOINING_KW,
    CONJUGATES_KW,
    CONJUGATE_KW,
    DIFFERS_KW,
    CROSSES_KW,
    INVERSE_KW,
    CHAINS_KW,
    DIFFERENCES_KW,
    FEATURED_KW,
    FEATURING_KW,
    INVERTING_KW,
    OF_KW,
    
    // Standalone relationship keywords
    SPECIALIZATION_KW,
    SUBCLASSIFIER_KW,
    REDEFINITION_KW,
    SUBSET_KW,
    SUBTYPE_KW,
    TYPING_KW,
    CONJUGATION_KW,
    MULTIPLICITY_KW,
    NAMESPACE_KW,
    
    // Feature modifiers
    REF_KW,
    READONLY_KW,
    DERIVED_KW,
    END_KW,
    ORDERED_KW,
    NONUNIQUE_KW,
    DEFAULT_KW,
    VAR_KW,
    CONST_KW,
    MEMBER_KW,
    RETURN_KW,
    
    // Direction
    IN_KW,
    OUT_KW,
    INOUT_KW,
    
    // Dependency
    DEPENDENCY_KW,
    FROM_KW,
    TO_KW,
    
    // Succession
    SUCCESSION_KW,
    FIRST_KW_2,  // duplicate handling
    
    // Boolean/null
    TRUE_KW,
    FALSE_KW,
    NULL_KW,
    
    // Logical operators
    AND_KW,
    OR_KW,
    NOT_KW,
    XOR_KW,
    IMPLIES_KW,
    
    // Classification
    HASTYPE_KW,
    ISTYPE_KW,
    AS_KW,
    META_KW,
    
    // =========================================================================
    // KEYWORDS - KerML (underlying language)
    // =========================================================================
    TYPE_KW,
    CLASSIFIER_KW,
    CLASS_KW,
    STRUCT_KW,
    DATATYPE_KW,
    ASSOC_KW,
    BEHAVIOR_KW,
    FUNCTION_KW,
    PREDICATE_KW,
    INTERACTION_KW,
    FEATURE_KW,
    STEP_KW,
    EXPR_KW,
    CONNECTOR_KW,
    INV_KW,

    // =========================================================================
    // COMPOSITE NODES (non-terminals in the grammar)
    // =========================================================================
    // Root
    SOURCE_FILE,
    
    // Namespace elements
    PACKAGE,
    LIBRARY_PACKAGE,
    NAMESPACE_BODY,
    
    // Member elements  
    PACKAGE_MEMBER,
    ELEMENT_FILTER_MEMBER,
    RELATIONSHIP_MEMBER,
    
    // Annotations
    COMMENT_ELEMENT,
    DOCUMENTATION,
    TEXTUAL_REP,
    METADATA_USAGE,
    PREFIX_METADATA,
    
    // Import
    IMPORT,
    MEMBERSHIP_IMPORT,
    NAMESPACE_IMPORT,
    FILTER_PACKAGE,
    
    // Alias
    ALIAS_MEMBER,
    
    // Dependencies
    DEPENDENCY,
    
    // Names and references
    NAME,
    SHORT_NAME,
    QUALIFIED_NAME,
    FEATURE_CHAIN,
    
    // Definitions
    DEFINITION,
    DEFINITION_BODY,
    DEFINITION_PREFIX,
    
    // Definition kinds
    PART_DEFINITION,
    ATTRIBUTE_DEFINITION,
    ENUMERATION_DEFINITION,
    ITEM_DEFINITION,
    OCCURRENCE_DEFINITION,
    PORT_DEFINITION,
    CONNECTION_DEFINITION,
    INTERFACE_DEFINITION,
    ALLOCATION_DEFINITION,
    FLOW_DEFINITION,
    ACTION_DEFINITION,
    STATE_DEFINITION,
    CALC_DEFINITION,
    CONSTRAINT_DEFINITION,
    REQUIREMENT_DEFINITION,
    CASE_DEFINITION,
    ANALYSIS_CASE_DEFINITION,
    VERIFICATION_CASE_DEFINITION,
    USE_CASE_DEFINITION,
    VIEW_DEFINITION,
    VIEWPOINT_DEFINITION,
    RENDERING_DEFINITION,
    METADATA_DEFINITION,
    
    // Usages  
    USAGE,
    USAGE_BODY,
    USAGE_PREFIX,
    
    // Usage kinds
    PART_USAGE,
    ATTRIBUTE_USAGE,
    ENUM_USAGE,
    ITEM_USAGE,
    OCCURRENCE_USAGE,
    PORT_USAGE,
    CONNECTION_USAGE,
    INTERFACE_USAGE,
    ALLOCATION_USAGE,
    FLOW_USAGE,
    ACTION_USAGE,
    STATE_USAGE,
    CALC_USAGE,
    CONSTRAINT_USAGE,
    REQUIREMENT_USAGE,
    CASE_USAGE,
    ANALYSIS_CASE_USAGE,
    VERIFICATION_CASE_USAGE,
    USE_CASE_USAGE,
    VIEW_USAGE,
    VIEWPOINT_USAGE,
    RENDERING_USAGE,
    
    // Relationships
    SPECIALIZATION,
    SUBSETTING,
    REDEFINITION,
    TYPING,
    FEATURING,
    CONJUGATION,
    
    // Multiplicity
    MULTIPLICITY,
    MULTIPLICITY_RANGE,
    
    // Expressions
    EXPRESSION,
    LITERAL_EXPR,
    FEATURE_REF_EXPR,
    INVOCATION_EXPR,
    SEQUENCE_EXPR,
    CONDITIONAL_EXPR,
    BINARY_EXPR,
    UNARY_EXPR,
    BRACKET_EXPR,
    ARGUMENT_LIST,
    
    // Body items
    BODY_ITEM,
    MEMBER,
    
    // Additional node types for grammar modules
    ACCEPT_ACTION_USAGE,
    ACTOR_USAGE,
    BINDING_CONNECTOR,
    CONNECTION_END,
    CONNECTOR,
    CONNECTOR_END,
    CONNECTOR_END_REFERENCE,
    CONNECTOR_PART,
    CONNECT_USAGE,
    CONSTRAINT_BODY,
    CONTROL_NODE,
    FOR_LOOP_ACTION_USAGE,
    IF_ACTION_USAGE,
    OBJECTIVE_USAGE,
    PERFORM_ACTION_USAGE,
    RELATIONSHIP,
    REQUIREMENT_CONSTRAINT,
    REQUIREMENT_VERIFICATION,
    SEND_ACTION_USAGE,
    STAKEHOLDER_USAGE,
    STATE_SUBACTION,
    SUBJECT_USAGE,
    SUCCESSION,
    SUCCESSION_ITEM,
    TEXTUAL_REPRESENTATION,
    TRANSITION_USAGE,
    WHILE_LOOP_ACTION_USAGE,
    CONSTANT_KW,
    
    // Message/flow from-to clause
    FROM_TO_CLAUSE,
    FROM_TO_SOURCE,
    FROM_TO_TARGET,
    
    // Special
    ERROR,
    TOMBSTONE,  // For incremental reparsing
    
    #[doc(hidden)]
    __LAST,
}

impl SyntaxKind {
    /// Check if this is a trivia token (whitespace or comment)
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::WHITESPACE | Self::LINE_COMMENT | Self::BLOCK_COMMENT)
    }
    
    /// Check if this is a keyword
    pub fn is_keyword(self) -> bool {
        (self as u16) >= (Self::PACKAGE_KW as u16) && (self as u16) <= (Self::INV_KW as u16)
    }
    
    /// Check if this is a punctuation token
    pub fn is_punct(self) -> bool {
        (self as u16) >= (Self::L_BRACE as u16) && (self as u16) <= (Self::PIPE_PIPE as u16)
    }
    
    /// Check if this is a literal
    pub fn is_literal(self) -> bool {
        matches!(self, Self::IDENT | Self::INTEGER | Self::DECIMAL | Self::STRING)
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl From<rowan::SyntaxKind> for SyntaxKind {
    fn from(raw: rowan::SyntaxKind) -> Self {
        assert!(raw.0 < SyntaxKind::__LAST as u16);
        // Safety: we control all syntax kinds and check bounds above
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }
}

/// Language definition for Rowan
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SysMLLanguage {}

impl rowan::Language for SysMLLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        raw.into()
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// Type aliases for convenience
pub type SyntaxNode = rowan::SyntaxNode<SysMLLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SysMLLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<SysMLLanguage>;
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<SysMLLanguage>;
