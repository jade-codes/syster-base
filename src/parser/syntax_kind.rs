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
    IDENT,   // identifier
    INTEGER, // 42
    DECIMAL, // 3.14
    STRING,  // "hello" or 'hello'

    // =========================================================================
    // PUNCTUATION
    // =========================================================================
    L_BRACE,           // {
    R_BRACE,           // }
    L_BRACKET,         // [
    R_BRACKET,         // ]
    L_PAREN,           // (
    R_PAREN,           // )
    SEMICOLON,         // ;
    COLON,             // :
    COLON_COLON,       // ::
    COLON_GT,          // :>  (specializes/subsets)
    COLON_GT_GT,       // :>> (redefines)
    COLON_COLON_GT,    // ::> (references)
    DOT,               // .
    DOT_DOT,           // ..
    COMMA,             // ,
    EQ,                // =
    EQ_EQ,             // ==
    EQ_EQ_EQ,          // ===
    BANG_EQ,           // !=
    BANG_EQ_EQ,        // !==
    LT,                // <
    GT,                // >
    LT_EQ,             // <=
    GT_EQ,             // >=
    ARROW,             // ->
    FAT_ARROW,         // =>
    AT,                // @
    AT_AT,             // @@
    HASH,              // #
    STAR,              // *
    STAR_STAR,         // **
    PLUS,              // +
    MINUS,             // -
    SLASH,             // /
    PERCENT,           // %
    CARET,             // ^
    TILDE,             // ~
    QUESTION,          // ?
    QUESTION_QUESTION, // ??
    BANG,              // !
    PIPE,              // |
    AMP,               // &
    AMP_AMP,           // &&
    PIPE_PIPE,         // ||
    COLON_EQ,          // :=
    DOLLAR,            // $

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
    FIRST_KW_2, // duplicate handling

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
    TOMBSTONE, // For incremental reparsing

    #[doc(hidden)]
    __LAST,
}

impl SyntaxKind {
    /// Check if this is a trivia token (whitespace or comment)
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::WHITESPACE | Self::LINE_COMMENT | Self::BLOCK_COMMENT
        )
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
        matches!(
            self,
            Self::IDENT | Self::INTEGER | Self::DECIMAL | Self::STRING
        )
    }

    /// Human-readable name for error messages (moved from `kind_to_name`)
    pub fn display_name(self) -> &'static str {
        match self {
            // Trivia
            Self::WHITESPACE => "whitespace",
            Self::LINE_COMMENT => "comment",
            Self::BLOCK_COMMENT => "comment",

            // Literals
            Self::IDENT => "identifier",
            Self::INTEGER => "integer",
            Self::DECIMAL => "number",
            Self::STRING => "string",
            Self::ERROR => "error",

            // Punctuation
            Self::SEMICOLON => "';'",
            Self::COLON => "':'",
            Self::COLON_COLON => "'::'",
            Self::COLON_GT => "':>'",
            Self::COLON_GT_GT => "':>>'",
            Self::COLON_COLON_GT => "'::>'",
            Self::COMMA => "','",
            Self::DOT => "'.'",
            Self::DOT_DOT => "'..'",
            Self::L_PAREN => "'('",
            Self::R_PAREN => "')'",
            Self::L_BRACE => "'{'",
            Self::R_BRACE => "'}'",
            Self::L_BRACKET => "'['",
            Self::R_BRACKET => "']'",
            Self::LT => "'<'",
            Self::GT => "'>'",
            Self::LT_EQ => "'<='",
            Self::GT_EQ => "'>='",
            Self::EQ => "'='",
            Self::EQ_EQ => "'=='",
            Self::EQ_EQ_EQ => "'==='",
            Self::BANG_EQ => "'!='",
            Self::BANG_EQ_EQ => "'!=='",
            Self::COLON_EQ => "':='",
            Self::PLUS => "'+'",
            Self::MINUS => "'-'",
            Self::STAR => "'*'",
            Self::STAR_STAR => "'**'",
            Self::SLASH => "'/'",
            Self::PERCENT => "'%'",
            Self::CARET => "'^'",
            Self::TILDE => "'~'",
            Self::AMP => "'&'",
            Self::AMP_AMP => "'&&'",
            Self::PIPE => "'|'",
            Self::PIPE_PIPE => "'||'",
            Self::AT => "'@'",
            Self::AT_AT => "'@@'",
            Self::HASH => "'#'",
            Self::QUESTION => "'?'",
            Self::QUESTION_QUESTION => "'??'",
            Self::BANG => "'!'",
            Self::ARROW => "'->'",
            Self::FAT_ARROW => "'=>'",
            Self::DOLLAR => "'$'",

            // Keywords - SysML v2
            Self::PACKAGE_KW => "'package'",
            Self::LIBRARY_KW => "'library'",
            Self::STANDARD_KW => "'standard'",
            Self::NAMESPACE_KW => "'namespace'",
            Self::IMPORT_KW => "'import'",
            Self::ALIAS_KW => "'alias'",
            Self::ALL_KW => "'all'",
            Self::FILTER_KW => "'filter'",
            Self::PRIVATE_KW => "'private'",
            Self::PROTECTED_KW => "'protected'",
            Self::PUBLIC_KW => "'public'",
            Self::DEF_KW => "'def'",
            Self::ABSTRACT_KW => "'abstract'",
            Self::COMPOSITE_KW => "'composite'",
            Self::PORTION_KW => "'portion'",
            Self::VARIATION_KW => "'variation'",
            Self::VARIANT_KW => "'variant'",
            Self::PART_KW => "'part'",
            Self::ATTRIBUTE_KW => "'attribute'",
            Self::ENUMERATION_KW => "'enumeration'",
            Self::ENUM_KW => "'enum'",
            Self::ITEM_KW => "'item'",
            Self::OCCURRENCE_KW => "'occurrence'",
            Self::INDIVIDUAL_KW => "'individual'",
            Self::PORT_KW => "'port'",
            Self::CONNECTION_KW => "'connection'",
            Self::INTERFACE_KW => "'interface'",
            Self::BINDING_KW => "'binding'",
            Self::FLOW_KW => "'flow'",
            Self::ALLOCATION_KW => "'allocation'",
            Self::ALLOCATE_KW => "'allocate'",
            Self::ACTION_KW => "'action'",
            Self::STATE_KW => "'state'",
            Self::TRANSITION_KW => "'transition'",
            Self::ENTRY_KW => "'entry'",
            Self::EXIT_KW => "'exit'",
            Self::DO_KW => "'do'",
            Self::ACCEPT_KW => "'accept'",
            Self::SEND_KW => "'send'",
            Self::PERFORM_KW => "'perform'",
            Self::EXHIBIT_KW => "'exhibit'",
            Self::MESSAGE_KW => "'message'",
            Self::SNAPSHOT_KW => "'snapshot'",
            Self::TIMESLICE_KW => "'timeslice'",
            Self::FRAME_KW => "'frame'",
            Self::EVENT_KW => "'event'",
            Self::IF_KW => "'if'",
            Self::ELSE_KW => "'else'",
            Self::THEN_KW => "'then'",
            Self::LOOP_KW => "'loop'",
            Self::WHILE_KW => "'while'",
            Self::UNTIL_KW => "'until'",
            Self::FOR_KW => "'for'",
            Self::FORK_KW => "'fork'",
            Self::JOIN_KW => "'join'",
            Self::MERGE_KW => "'merge'",
            Self::DECIDE_KW => "'decide'",
            Self::FIRST_KW => "'first'",
            Self::DONE_KW => "'done'",
            Self::START_KW => "'start'",
            Self::TERMINATE_KW => "'terminate'",
            Self::PARALLEL_KW => "'parallel'",
            Self::ASSIGN_KW => "'assign'",
            Self::CONNECT_KW => "'connect'",
            Self::BIND_KW => "'bind'",
            Self::NEW_KW => "'new'",
            Self::AFTER_KW => "'after'",
            Self::AT_KW => "'at'",
            Self::WHEN_KW => "'when'",
            Self::VIA_KW => "'via'",
            Self::THIS_KW => "'this'",
            Self::CALC_KW => "'calc'",
            Self::CONSTRAINT_KW => "'constraint'",
            Self::ASSERT_KW => "'assert'",
            Self::ASSUME_KW => "'assume'",
            Self::REQUIRE_KW => "'require'",
            Self::REQUIREMENT_KW => "'requirement'",
            Self::SUBJECT_KW => "'subject'",
            Self::OBJECTIVE_KW => "'objective'",
            Self::STAKEHOLDER_KW => "'stakeholder'",
            Self::ACTOR_KW => "'actor'",
            Self::CONCERN_KW => "'concern'",
            Self::SATISFY_KW => "'satisfy'",
            Self::VERIFY_KW => "'verify'",
            Self::CASE_KW => "'case'",
            Self::ANALYSIS_KW => "'analysis'",
            Self::VERIFICATION_KW => "'verification'",
            Self::USE_KW => "'use'",
            Self::INCLUDE_KW => "'include'",
            Self::VIEW_KW => "'view'",
            Self::VIEWPOINT_KW => "'viewpoint'",
            Self::RENDERING_KW => "'rendering'",
            Self::RENDER_KW => "'render'",
            Self::EXPOSE_KW => "'expose'",
            Self::METACLASS_KW => "'metaclass'",
            Self::METADATA_KW => "'metadata'",
            Self::ABOUT_KW => "'about'",
            Self::DOC_KW => "'doc'",
            Self::COMMENT_KW => "'comment'",
            Self::LANGUAGE_KW => "'language'",
            Self::LOCALE_KW => "'locale'",
            Self::REP_KW => "'rep'",
            Self::SPECIALIZES_KW => "'specializes'",
            Self::SUBSETS_KW => "'subsets'",
            Self::REDEFINES_KW => "'redefines'",
            Self::REFERENCES_KW => "'references'",
            Self::TYPED_KW => "'typed'",
            Self::DEFINED_KW => "'defined'",
            Self::BY_KW => "'by'",
            Self::INTERSECTS_KW => "'intersects'",
            Self::UNIONS_KW => "'unions'",
            Self::DISJOINT_KW => "'disjoint'",
            Self::DISJOINING_KW => "'disjoining'",
            Self::CONJUGATES_KW => "'conjugates'",
            Self::CONJUGATE_KW => "'conjugate'",
            Self::DIFFERS_KW => "'differs'",
            Self::CROSSES_KW => "'crosses'",
            Self::INVERSE_KW => "'inverse'",
            Self::CHAINS_KW => "'chains'",
            Self::DIFFERENCES_KW => "'differences'",
            Self::FEATURED_KW => "'featured'",
            Self::FEATURING_KW => "'featuring'",
            Self::INVERTING_KW => "'inverting'",
            Self::OF_KW => "'of'",
            Self::SPECIALIZATION_KW => "'specialization'",
            Self::SUBCLASSIFIER_KW => "'subclassifier'",
            Self::REDEFINITION_KW => "'redefinition'",
            Self::SUBSET_KW => "'subset'",
            Self::SUBTYPE_KW => "'subtype'",
            Self::TYPING_KW => "'typing'",
            Self::CONJUGATION_KW => "'conjugation'",
            Self::MULTIPLICITY_KW => "'multiplicity'",
            Self::REF_KW => "'ref'",
            Self::READONLY_KW => "'readonly'",
            Self::DERIVED_KW => "'derived'",
            Self::END_KW => "'end'",
            Self::ORDERED_KW => "'ordered'",
            Self::NONUNIQUE_KW => "'nonunique'",
            Self::DEFAULT_KW => "'default'",
            Self::VAR_KW => "'var'",
            Self::CONST_KW => "'const'",
            Self::CONSTANT_KW => "'constant'",
            Self::MEMBER_KW => "'member'",
            Self::RETURN_KW => "'return'",
            Self::IN_KW => "'in'",
            Self::OUT_KW => "'out'",
            Self::INOUT_KW => "'inout'",
            Self::DEPENDENCY_KW => "'dependency'",
            Self::FROM_KW => "'from'",
            Self::TO_KW => "'to'",
            Self::SUCCESSION_KW => "'succession'",
            Self::FIRST_KW_2 => "'first'",
            Self::TRUE_KW => "'true'",
            Self::FALSE_KW => "'false'",
            Self::NULL_KW => "'null'",
            Self::AND_KW => "'and'",
            Self::OR_KW => "'or'",
            Self::NOT_KW => "'not'",
            Self::XOR_KW => "'xor'",
            Self::IMPLIES_KW => "'implies'",
            Self::HASTYPE_KW => "'hastype'",
            Self::ISTYPE_KW => "'istype'",
            Self::AS_KW => "'as'",
            Self::META_KW => "'meta'",

            // Keywords - KerML
            Self::TYPE_KW => "'type'",
            Self::CLASSIFIER_KW => "'classifier'",
            Self::CLASS_KW => "'class'",
            Self::STRUCT_KW => "'struct'",
            Self::DATATYPE_KW => "'datatype'",
            Self::ASSOC_KW => "'assoc'",
            Self::BEHAVIOR_KW => "'behavior'",
            Self::FUNCTION_KW => "'function'",
            Self::PREDICATE_KW => "'predicate'",
            Self::INTERACTION_KW => "'interaction'",
            Self::FEATURE_KW => "'feature'",
            Self::STEP_KW => "'step'",
            Self::EXPR_KW => "'expr'",
            Self::CONNECTOR_KW => "'connector'",
            Self::INV_KW => "'inv'",

            // Composite nodes
            Self::SOURCE_FILE => "source file",
            Self::PACKAGE => "package",
            Self::LIBRARY_PACKAGE => "library package",
            Self::NAMESPACE_BODY => "namespace body",
            Self::IMPORT => "import",
            Self::ALIAS_MEMBER => "alias",
            Self::DEFINITION => "definition",
            Self::USAGE => "usage",
            Self::EXPRESSION => "expression",
            Self::QUALIFIED_NAME => "qualified name",
            Self::NAME => "name",
            Self::MULTIPLICITY => "multiplicity",
            Self::MULTIPLICITY_RANGE => "multiplicity range",

            // Fallback
            _ => "token",
        }
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
#[allow(dead_code)]
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<SysMLLanguage>;
