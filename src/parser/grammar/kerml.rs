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

use super::kerml_expressions::ExpressionParser;
use crate::parser::parser::kind_to_name;
use crate::parser::syntax_kind::SyntaxKind;

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

/// Standalone relationship keywords
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

/// Relationship operator keywords
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

/// Trait for KerML parsing operations
///
/// Extends ExpressionParser with KerML-specific methods.
/// The main parser implements this trait.
pub trait KerMLParser: ExpressionParser {
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

    /// Report a parse error
    fn error(&mut self, message: impl Into<String>);

    /// Error recovery - skip to recovery tokens
    fn error_recover(&mut self, message: impl Into<String>, recovery: &[SyntaxKind]);
}

// =============================================================================
// Helper Functions - Common Patterns
// =============================================================================

/// Bump current token and skip trivia (used 100+ times)
#[inline]
fn bump_and_skip<P: KerMLParser>(p: &mut P) {
    p.bump();
    p.skip_trivia();
}

/// Expect a token and skip trivia (used 20+ times)
#[inline]
fn expect_and_skip<P: KerMLParser>(p: &mut P, kind: SyntaxKind) {
    p.expect(kind);
    p.skip_trivia();
}

/// Conditionally bump if at a specific token, then skip trivia
#[inline]
fn consume_if<P: KerMLParser>(p: &mut P, kind: SyntaxKind) -> bool {
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
fn parse_qualified_name_and_skip<P: KerMLParser>(p: &mut P) {
    p.parse_qualified_name();
    p.skip_trivia();
}

/// Parse identification and skip trivia
#[inline]
fn parse_identification_and_skip<P: KerMLParser>(p: &mut P) {
    p.parse_identification();
    p.skip_trivia();
}

/// Parse optional identification (if at name token or <)
fn parse_optional_identification<P: KerMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }
}

/// Parse optional qualified name
fn parse_optional_qualified_name<P: KerMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::THIS_KW) {
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Parse optional visibility (public, private, protected)
/// Per pest: visibility_kind = { public | private | protected }
#[inline]
fn parse_optional_visibility<P: KerMLParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        bump_and_skip(p);
    }
}

/// Parse optional multiplicity [expression]
fn parse_optional_multiplicity<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }
}

/// Parse optional typing (: Type or typed by Type)
fn parse_optional_typing<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) || p.at(SyntaxKind::OF_KW) {
        parse_typing(p);
        p.skip_trivia();
    }
}

/// Parse comma-separated qualified names
fn parse_comma_separated_names<P: KerMLParser>(p: &mut P) {
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
fn looks_like_qualified_name_before<P: KerMLParser>(p: &P, target_kinds: &[SyntaxKind]) -> bool {
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

// =============================================================================
// KerML File Entry Point
// =============================================================================

/// Parse a KerML source file
/// Per Pest: file = { SOI ~ namespace_element* ~ EOI }
pub fn parse_kerml_file<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SOURCE_FILE);

    while !p.at(SyntaxKind::ERROR) {
        // ERROR indicates EOF in our lexer
        p.skip_trivia();
        if p.at(SyntaxKind::ERROR) {
            break;
        }
        let start_pos = p.get_pos();
        parse_namespace_element(p);

        // Safety: if we didn't make progress, skip the token to avoid infinite loop
        if p.get_pos() == start_pos {
            let got = if p.at(SyntaxKind::ERROR) {
                "end of file".to_string()
            } else if let Some(text) = p.current_token_text() {
                format!("'{}'", text)
            } else {
                kind_to_name(p.current_kind()).to_string()
            };
            p.error(format!("unexpected {} in top level", got));
            p.bump();
        }
    }

    p.finish_node();
}

/// Parse a KerML namespace element
/// Per Pest grammar:
/// namespace_element = {
///     package | library_package | import | alias_member
///     | annotating_member | namespace_feature_member
///     | non_feature_member | relationship_member
/// }
/// Per pest: namespace_body_element = { visibility_kind? ~ prefix_metadata? ~ (non_feature_member | namespace_feature_member | type_feature_member | relationship_member | annotating_member | alias_member | import) }
/// Per pest: non_feature_element = { namespace | package | library_package | multiplicity | type_def | classifier | class | structure | metaclass | data_type | association | association_structure | interaction | behavior | function | predicate }
/// Per pest: feature_element = { end_feature | feature | step | expression | boolean_expression | invariant | connector | binding_connector | succession | item_flow | succession_item_flow }
pub fn parse_namespace_element<P: KerMLParser>(p: &mut P) {
    p.skip_trivia();

    // Handle visibility prefix
    if p.at_any(&[
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        bump_and_skip(p);
    }

    // Handle prefix metadata (#name)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    match p.current_kind() {
        SyntaxKind::PACKAGE_KW | SyntaxKind::NAMESPACE_KW => p.parse_package(),
        SyntaxKind::LIBRARY_KW | SyntaxKind::STANDARD_KW => p.parse_library_package(),
        SyntaxKind::IMPORT_KW => p.parse_import(),
        SyntaxKind::ALIAS_KW => p.parse_alias(),

        SyntaxKind::COMMENT_KW
        | SyntaxKind::DOC_KW
        | SyntaxKind::LOCALE_KW
        | SyntaxKind::AT
        | SyntaxKind::AT_AT
        | SyntaxKind::METADATA_KW => parse_annotation(p),

        SyntaxKind::CLASS_KW
        | SyntaxKind::STRUCT_KW
        | SyntaxKind::DATATYPE_KW
        | SyntaxKind::BEHAVIOR_KW
        | SyntaxKind::FUNCTION_KW
        | SyntaxKind::ASSOC_KW
        | SyntaxKind::CLASSIFIER_KW
        | SyntaxKind::INTERACTION_KW
        | SyntaxKind::PREDICATE_KW
        | SyntaxKind::METACLASS_KW
        | SyntaxKind::TYPE_KW => p.parse_definition(),

        SyntaxKind::ABSTRACT_KW => handle_abstract_prefix(p),

        SyntaxKind::FEATURE_KW | SyntaxKind::STEP_KW | SyntaxKind::EXPR_KW => p.parse_usage(),

        SyntaxKind::INV_KW => p.parse_invariant(),

        SyntaxKind::REP_KW | SyntaxKind::LANGUAGE_KW => parse_textual_representation(p),

        SyntaxKind::IN_KW | SyntaxKind::OUT_KW | SyntaxKind::INOUT_KW | SyntaxKind::RETURN_KW => {
            p.parse_parameter()
        }

        SyntaxKind::END_KW => p.parse_end_feature_or_parameter(),

        // const can be followed by 'end' (const end ...) or 'feature' (const feature ...)
        SyntaxKind::CONST_KW => handle_const_prefix(p),

        SyntaxKind::CONNECTOR_KW | SyntaxKind::BINDING_KW => p.parse_connector_usage(),

        SyntaxKind::SUCCESSION_KW | SyntaxKind::FIRST_KW => handle_succession_prefix(p),

        SyntaxKind::FLOW_KW => p.parse_flow_usage(),

        // Multiplicity definition: multiplicity exactlyOne [1..1] { }
        SyntaxKind::MULTIPLICITY_KW => parse_multiplicity_definition(p),

        SyntaxKind::SPECIALIZATION_KW
        | SyntaxKind::SUBCLASSIFIER_KW
        | SyntaxKind::REDEFINITION_KW
        | SyntaxKind::SUBSET_KW
        | SyntaxKind::TYPING_KW
        | SyntaxKind::CONJUGATION_KW
        | SyntaxKind::CONJUGATE_KW
        | SyntaxKind::DISJOINING_KW
        | SyntaxKind::FEATURING_KW
        | SyntaxKind::SUBTYPE_KW => parse_standalone_relationship(p),

        SyntaxKind::INVERTING_KW | SyntaxKind::INVERSE_KW => parse_inverting_relationship(p),
        SyntaxKind::DEPENDENCY_KW => parse_dependency(p),
        SyntaxKind::DISJOINT_KW => parse_disjoint(p),
        SyntaxKind::FILTER_KW => parse_filter(p),

        SyntaxKind::REDEFINES_KW
        | SyntaxKind::COLON_GT_GT
        | SyntaxKind::SUBSETS_KW
        | SyntaxKind::COLON_GT => p.parse_usage(),

        SyntaxKind::VAR_KW
        | SyntaxKind::COMPOSITE_KW
        | SyntaxKind::PORTION_KW
        | SyntaxKind::MEMBER_KW
        | SyntaxKind::DERIVED_KW
        | SyntaxKind::READONLY_KW => {
            handle_feature_modifier_prefix(p);
        }

        SyntaxKind::IDENT => p.parse_usage(),

        // Expression-starting tokens (for result expressions in function/predicate bodies)
        SyntaxKind::NOT_KW
        | SyntaxKind::TRUE_KW
        | SyntaxKind::FALSE_KW
        | SyntaxKind::NULL_KW
        | SyntaxKind::INTEGER
        | SyntaxKind::DECIMAL
        | SyntaxKind::STRING
        | SyntaxKind::L_PAREN => {
            super::kerml_expressions::parse_expression(p);
            p.skip_trivia();
            if p.at(SyntaxKind::SEMICOLON) {
                p.bump();
            }
        }

        _ => {
            let got = if let Some(text) = p.current_token_text() {
                format!("'{}'", text)
            } else {
                kind_to_name(p.current_kind()).to_string()
            };
            p.error_recover(
                format!("unexpected {} in namespace body", got),
                &[
                    SyntaxKind::PACKAGE_KW,
                    SyntaxKind::CLASS_KW,
                    SyntaxKind::R_BRACE,
                ],
            );
        }
    }
}

/// Parse prefix metadata (#name)
/// Per pest: prefix_metadata = { user_defined_keyword+ }
/// Per pest: user_defined_keyword = { "#" ~ (identifier ~ ("::" ~ identifier)*) }
fn parse_prefix_metadata<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PREFIX_METADATA);
    expect_and_skip(p, SyntaxKind::HASH);
    if p.at_name_token() {
        p.bump();
    }
    p.finish_node();
}

/// Handle abstract keyword by looking ahead to determine element type
fn handle_abstract_prefix<P: KerMLParser>(p: &mut P) {
    let next = p.peek_kind(1);
    if matches!(
        next,
        SyntaxKind::CLASS_KW
            | SyntaxKind::STRUCT_KW
            | SyntaxKind::DATATYPE_KW
            | SyntaxKind::BEHAVIOR_KW
            | SyntaxKind::FUNCTION_KW
            | SyntaxKind::ASSOC_KW
            | SyntaxKind::CLASSIFIER_KW
            | SyntaxKind::PREDICATE_KW
            | SyntaxKind::METACLASS_KW
            | SyntaxKind::INTERACTION_KW
            | SyntaxKind::TYPE_KW
    ) {
        p.parse_definition();
    } else if next == SyntaxKind::FLOW_KW {
        p.parse_flow_usage();
    } else if matches!(
        next,
        SyntaxKind::CONNECTOR_KW | SyntaxKind::BINDING_KW | SyntaxKind::SUCCESSION_KW
    ) {
        p.parse_connector_usage();
    } else {
        p.parse_usage();
    }
}

/// Handle const keyword - either "const end ..." or "const feature ..."
fn handle_const_prefix<P: KerMLParser>(p: &mut P) {
    let next = p.peek_kind(1);
    if next == SyntaxKind::END_KW {
        // const end ... -> end feature with const modifier
        p.parse_end_feature_or_parameter();
    } else {
        // const feature ..., const derived feature ..., etc. -> regular usage with const modifier
        p.parse_usage();
    }
}

/// Handle feature modifier keywords by looking ahead
fn handle_feature_modifier_prefix<P: KerMLParser>(p: &mut P) {
    let next = p.peek_kind(1);
    if matches!(
        next,
        SyntaxKind::CONNECTOR_KW | SyntaxKind::BINDING_KW | SyntaxKind::SUCCESSION_KW
    ) {
        p.parse_connector_usage();
    } else {
        p.parse_usage();
    }
}

/// Handle succession keyword by looking ahead for flow
fn handle_succession_prefix<P: KerMLParser>(p: &mut P) {
    let next = p.peek_kind(1);
    if next == SyntaxKind::FLOW_KW {
        p.parse_flow_usage();
    } else {
        p.parse_connector_usage();
    }
}

// =============================================================================
// Standalone Relationship Parsing
// =============================================================================

/// Parse featuring relationship: featuring [id? of]? feature by type
/// Per pest: type_featuring = { featuring_token ~ (identification ~ of_token)? ~ qualified_reference_chain ~ by_token ~ qualified_reference_chain }
fn parse_featuring_relationship<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    // Check for optional identification + 'of'
    if p.at_name_token() {
        parse_identification_and_skip(p);

        if p.at(SyntaxKind::OF_KW) {
            bump_and_skip(p);
            parse_optional_qualified_name(p);
        }
    }

    // Parse 'by' clause
    if p.at(SyntaxKind::BY_KW) {
        bump_and_skip(p);
        parse_optional_qualified_name(p);
    }
}

/// Parse typing relationship: typing feature (':' | 'typed by') type
/// Per pest: standalone_feature_typing = { typing_token ~ qualified_reference_chain ~ feature_typing }
/// Per pest: feature_typing = { typed_by_operator ~ qualified_reference_chain ~ multiplicity_bounds? ~ ordering_modifiers }
fn parse_typing_relationship<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    parse_optional_qualified_name(p);

    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) {
        parse_typing(p);
    }
}

/// Parse conjugation relationship: conjugate type1 ('~' | 'conjugates') type2
/// Per pest: standalone_conjugation = { conjugation_token ~ identification? ~ conjugate_token? ~ qualified_reference_chain ~ conjugates_operator ~ qualified_reference_chain ~ relationship_body }
/// Per pest: conjugates_operator = { "~" | conjugates_token }
/// Also handles shorthand: conjugate A ~ B;
fn parse_conjugation_relationship<P: KerMLParser>(p: &mut P) {
    // Check if we start with 'conjugation' (full form) or 'conjugate' (shorthand)
    let is_shorthand = p.at(SyntaxKind::CONJUGATE_KW);
    bump_and_skip(p);

    if is_shorthand {
        // Shorthand form: conjugate A ~ B;
        // Parse source type directly
        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            bump_and_skip(p);
        }

        parse_optional_qualified_name(p);
    } else {
        // Full form: conjugation [id] conjugate A ~ B;
        // Optional identification
        if p.at_name_token() && !p.at(SyntaxKind::CONJUGATE_KW) {
            parse_identification_and_skip(p);
        }

        consume_if(p, SyntaxKind::CONJUGATE_KW);

        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::CONJUGATES_KW) || p.at(SyntaxKind::TILDE) {
            bump_and_skip(p);
        }

        parse_optional_qualified_name(p);
    }
}

/// Parse generic relationship: keyword source operator target
/// Handles relationships that don't fit other specific patterns
fn parse_generic_relationship<P: KerMLParser>(p: &mut P) {
    if p.at_any(STANDALONE_RELATIONSHIP_KEYWORDS) {
        bump_and_skip(p);
    }

    parse_optional_qualified_name(p);

    if p.at_any(RELATIONSHIP_OPERATORS) {
        bump_and_skip(p);
    }

    parse_optional_qualified_name(p);
}

/// Parse KerML standalone relationship declarations
/// E.g., `specialization Super subclassifier A specializes B;`
/// E.g., `subclassifier C specializes A;`
/// E.g., `redefinition MyRedef redefines x :>> y;`
/// Per Pest grammar: specialization_prefix ~ relationship_keyword ~ from ~ operator ~ to ~ relationship_body
/// Per pest: standalone_specialization | standalone_conjugation | standalone_feature_typing | subclassification | disjoining | feature_inverting | standalone_subsetting | standalone_redefinition | type_featuring
/// Per pest: specialization_prefix = { (specialization_token ~ identification?)? }
pub fn parse_standalone_relationship<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Handle optional 'specialization' prefix with optional identification
    if p.at(SyntaxKind::SPECIALIZATION_KW) {
        bump_and_skip(p);

        if p.at_name_token()
            && !p.at_any(&[
                SyntaxKind::SUBCLASSIFIER_KW,
                SyntaxKind::SUBTYPE_KW,
                SyntaxKind::SUBSET_KW,
                SyntaxKind::REDEFINITION_KW,
                SyntaxKind::TYPING_KW,
            ])
        {
            parse_identification_and_skip(p);
        }
    }

    // Dispatch to specific relationship handlers
    if p.at(SyntaxKind::FEATURING_KW) {
        parse_featuring_relationship(p);
    } else if p.at(SyntaxKind::TYPING_KW) {
        parse_typing_relationship(p);
    } else if p.at(SyntaxKind::CONJUGATION_KW) || p.at(SyntaxKind::CONJUGATE_KW) {
        parse_conjugation_relationship(p);
    } else {
        parse_generic_relationship(p);
    }

    p.parse_body();
    p.finish_node();
}

/// Parse dependency relationship
/// Syntax: dependency [identification from]? source (',' source)* to target (',' target)* body
/// Per pest: dependency = { dependency_token ~ (identification ~ from_token)? ~ qualified_reference_chain ~ ("," ~ qualified_reference_chain)* ~ to_token ~ qualified_reference_chain ~ ("," ~ qualified_reference_chain)* ~ relationship_body }
pub fn parse_dependency<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEPENDENCY);

    expect_and_skip(p, SyntaxKind::DEPENDENCY_KW);

    // Check for identification (can start with < for short name or an identifier)
    // Identification is followed by 'from', or just 'from' keyword
    if p.at(SyntaxKind::FROM_KW) {
        bump_and_skip(p);
    } else if p.at(SyntaxKind::LT) {
        // Short name like <short>
        parse_identification_and_skip(p);
        if p.at(SyntaxKind::FROM_KW) {
            bump_and_skip(p);
        }
    } else if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        // Check if this is an identification followed by 'from'
        // by looking for 'from' after the name(s)
        let peek1 = p.peek_kind(1);
        let peek2 = p.peek_kind(2);
        if peek1 == SyntaxKind::FROM_KW || peek2 == SyntaxKind::FROM_KW {
            parse_identification_and_skip(p);
            if p.at(SyntaxKind::FROM_KW) {
                bump_and_skip(p);
            }
        }
    }

    // Parse source(s)
    if p.at_name_token() && !p.at(SyntaxKind::TO_KW) {
        parse_comma_separated_names(p);
    }

    expect_and_skip(p, SyntaxKind::TO_KW);

    // Parse target(s)
    if p.at_name_token() {
        parse_comma_separated_names(p);
    }

    p.parse_body();
    p.finish_node();
}

/// Parse textual representation
/// Syntax: [rep id?]? language "string" [comment]? ;?
/// Per pest: textual_representation = { (rep_token ~ identification?)? ~ language_token ~ string_value ~ block_comment? ~ ";"? }
pub fn parse_textual_representation<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TEXTUAL_REP);

    // Optional 'rep' with identification
    if p.at(SyntaxKind::REP_KW) {
        bump_and_skip(p);
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            parse_identification_and_skip(p);
        }
    }

    // Required 'language' keyword
    expect_and_skip(p, SyntaxKind::LANGUAGE_KW);

    // Required string value
    expect_and_skip(p, SyntaxKind::STRING);

    // Optional block comment (already part of trivia, will be skipped)
    // Optional semicolon
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }

    p.finish_node();
}

/// Parse disjoint statement
/// Syntax: disjoint source [from target] ;
/// Per pest: disjoining = { disjoint_token ~ (element_reference ~ from_token ~ element_reference | from_token ~ relationship | visibility_kind? ~ element_reference) }
/// Source and target can be qualified names (::) or feature chains (.)
pub fn parse_disjoint<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    expect_and_skip(p, SyntaxKind::DISJOINT_KW);

    // Parse source - can be a qualified name or feature chain (with .)
    if p.at_name_token() {
        parse_feature_chain_or_qualified_name(p);
    }

    // Optional 'from' keyword followed by target
    if p.at(SyntaxKind::FROM_KW) {
        bump_and_skip(p);

        // Parse target
        if p.at_name_token() {
            parse_feature_chain_or_qualified_name(p);
        }
    }

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}

/// Parse a name that could be a qualified name (A::B::C) or feature chain (a.b.c)
fn parse_feature_chain_or_qualified_name<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::QUALIFIED_NAME);

    if p.at_name_token() {
        p.bump();
    }
    p.skip_trivia();

    // Handle both :: (qualified) and . (feature chain) separators
    while p.at(SyntaxKind::COLON_COLON) || p.at(SyntaxKind::DOT) {
        p.bump(); // :: or .
        p.skip_trivia();
        if p.at_name_token() {
            p.bump();
        }
        p.skip_trivia();
    }

    p.finish_node();
    p.skip_trivia();
}

/// Parse filter statement
/// Syntax: filter <expression> ;
/// Per pest: filter_package = { filter_token ~ inline_expression ~ ";" }
pub fn parse_filter<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ELEMENT_FILTER_MEMBER);

    expect_and_skip(p, SyntaxKind::FILTER_KW);

    // Parse the filter expression
    super::kerml_expressions::parse_expression(p);
    p.skip_trivia();

    // Expect semicolon
    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// Parse inverting/inverse relationship
/// Syntax: [inverting identification?] inverse source of target body
/// Per pest: feature_inverting = { (inverting_token ~ identification?)? ~ inverse_token ~ qualified_reference_chain ~ of_token ~ qualified_reference_chain ~ relationship_body }
pub fn parse_inverting_relationship<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::RELATIONSHIP);

    // Optional 'inverting' keyword with optional identification
    if p.at(SyntaxKind::INVERTING_KW) {
        bump_and_skip(p); // inverting

        // Optional identification after 'inverting'
        if p.at_name_token() && !p.at(SyntaxKind::INVERSE_KW) {
            parse_identification_and_skip(p);
        }
    }

    // Expect 'inverse' keyword
    expect_and_skip(p, SyntaxKind::INVERSE_KW);

    // Parse source (feature or chain)
    parse_optional_qualified_name(p);

    // Expect 'of' keyword
    expect_and_skip(p, SyntaxKind::OF_KW);

    // Parse target (feature or chain)
    parse_optional_qualified_name(p);

    // Parse body or semicolon
    p.parse_body();

    p.finish_node();
}

// =============================================================================
// Annotation Parsing
// =============================================================================

/// Parse metadata annotation (@ or @@ or metadata keyword)
/// Per pest: metadata_feature = { prefix_metadata? ~ (at_symbol | metadata_token) ~ (identification ~ (":" | typed_token ~ by_token))? ~ qualified_reference_chain ~ (about_token ~ annotation ~ ("," ~ annotation)*)? ~ metadata_body }
fn parse_metadata_annotation<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p); // METADATA_KW, @, or @@

    // Optional identification with typing (name : Type)
    if p.at_name_token()
        && p.peek_kind(1) != SyntaxKind::SEMICOLON
        && p.peek_kind(1) != SyntaxKind::L_BRACE
        && matches!(p.peek_kind(1), SyntaxKind::COLON | SyntaxKind::TYPED_KW)
    {
        parse_identification_and_skip(p);
        bump_and_skip(p); // COLON or TYPED_KW
        consume_if(p, SyntaxKind::BY_KW);
    }

    parse_optional_qualified_name(p);

    if p.at(SyntaxKind::ABOUT_KW) {
        bump_and_skip(p);
        p.parse_qualified_name_list();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }
}

/// Parse locale annotation: locale "string" /* comment */
/// Per pest: Note - locale is typically part of comment_annotation and documentation
fn parse_locale_annotation<P: KerMLParser>(p: &mut P) {
    p.bump(); // locale
    p.skip_trivia_except_block_comments();

    if p.at(SyntaxKind::STRING) {
        p.bump();
        p.skip_trivia_except_block_comments();
    }

    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
    }
}

/// Parse comment/doc annotation with optional identification, locale, about
/// Check for block comment and return true if found
fn check_block_comment<P: KerMLParser>(p: &mut P) -> bool {
    if p.at(SyntaxKind::BLOCK_COMMENT) {
        p.bump();
        true
    } else {
        false
    }
}

/// Parse optional locale clause
fn parse_locale_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::LOCALE_KW) {
        p.bump();
        p.skip_trivia_except_block_comments();
        if p.at(SyntaxKind::STRING) {
            p.bump();
            p.skip_trivia_except_block_comments();
        }
    }
}

/// Parse 'about' clause with optional locale
fn parse_about_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::ABOUT_KW) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name_list();
        p.skip_trivia_except_block_comments();
        parse_locale_clause(p);
    }
}

/// Per pest: comment_annotation = { (comment_token ~ identification? ~ (about_token ~ element_reference ~ ("," ~ element_reference)*)?)? ~ (locale_token ~ string_value)? ~ block_comment }
/// Per pest: documentation = { doc_token ~ identification? ~ (locale_token ~ string_value)? ~ (block_comment | ";")? }
fn parse_comment_doc_annotation<P: KerMLParser>(p: &mut P) {
    // comment or doc keyword already consumed
    p.skip_trivia_except_block_comments();

    if check_block_comment(p) {
        return;
    }

    // Optional identification
    if (p.at_name_token() || p.at(SyntaxKind::LT))
        && !p.at(SyntaxKind::ABOUT_KW)
        && !p.at(SyntaxKind::LOCALE_KW)
    {
        p.parse_identification();
        p.skip_trivia_except_block_comments();

        if check_block_comment(p) {
            return;
        }
    }

    parse_locale_clause(p);
    if check_block_comment(p) {
        return;
    }

    parse_about_clause(p);
    if check_block_comment(p) {
        return;
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_annotation_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    }
}

/// Parse annotation (comment, doc, locale)
/// Per Pest grammar:
/// - locale_annotation = { locale_token ~ string_value ~ block_comment? }
/// - comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference)* ~ (block_comment | semi_colon)? }
/// - documentation = { doc_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (block_comment | semi_colon)? }
pub fn parse_annotation<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::COMMENT_ELEMENT);

    if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) || p.at(SyntaxKind::METADATA_KW) {
        parse_metadata_annotation(p);
    } else if p.at(SyntaxKind::LOCALE_KW) {
        parse_locale_annotation(p);
    } else {
        // comment or doc keyword
        if p.at(SyntaxKind::COMMENT_KW) || p.at(SyntaxKind::DOC_KW) {
            p.bump();
        }
        parse_comment_doc_annotation(p);
    }

    p.finish_node();
}

/// Parse annotation body
/// Per pest: metadata_body = { ";" | "{" ~ metadata_body_element* ~ "}" }
/// Per pest: metadata_body_element = { non_feature_member | metadata_body_feature_member | alias_member | import }
fn parse_annotation_body<P: KerMLParser>(p: &mut P) {
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

// =============================================================================
// Core Parsing Functions
// These are the actual implementations, called via traits from parser.rs
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
pub fn parse_identification<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAME);

    // Short name: <shortname>
    if p.at(SyntaxKind::LT) {
        p.start_node(SyntaxKind::SHORT_NAME);
        bump_and_skip(p); // <
        if p.at_name_token() {
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

/// Per pest: qualified_name = { ("$" ~ "::")? ~ name ~ ("::" | ".") ~ name )* }
/// Supports global qualification ($::), namespace paths (::), and feature chains (.)
/// Per pest: qualified_name = { ("$" ~ "::")? ~ name ~ (("::" | ".") ~ name)* }
/// Supports global qualification ($::), namespace paths (::), and feature chains (.)
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
    super::kerml_expressions::parse_expression(p);
    if p.get_pos() > expr_start {
        // Expression was parsed, continue
        p.skip_trivia();
    } else {
        let got = if let Some(text) = p.current_token_text() {
            format!("'{}'", text)
        } else {
            kind_to_name(p.current_kind()).to_string()
        };
        p.error(format!("unexpected {} in body", got));
        p.bump();
    }
}

/// Per pest: namespace_body = { ";" | ("{" ~ namespace_body_elements ~ "}") }
/// Per pest: type_body = { namespace_body | ";" }
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
                super::kerml_expressions::parse_expression(p);
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
        p.error("expected ';' or '{'")
    }

    p.finish_node();
}

/// Typing = ':' QualifiedName (with multiplicity and modifiers)
/// Per pest: typed_by_operator = { ":" | (typed_token ~ " " ~ by_token) }
/// Per pest: feature_typing = { typed_by_operator ~ qualified_reference_chain ~ multiplicity_bounds? ~ ordering_modifiers }
pub fn parse_typing<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TYPING);

    // Accept ':' or 'typed by' or 'of'
    if p.at(SyntaxKind::TYPED_KW) {
        bump_and_skip(p);
        p.expect(SyntaxKind::BY_KW);
    } else if p.at(SyntaxKind::OF_KW) {
        p.bump();
    } else {
        p.expect(SyntaxKind::COLON);
    }
    p.skip_trivia();

    consume_if(p, SyntaxKind::TILDE);

    parse_type_with_modifiers(p);

    // Comma-separated types
    while p.at(SyntaxKind::COMMA) {
        bump_and_skip(p);
        parse_type_with_modifiers(p);
    }

    p.finish_node();
}

/// Parse single type with optional multiplicity and ordering modifiers
fn parse_type_with_modifiers<P: KerMLParser>(p: &mut P) {
    parse_qualified_name_and_skip(p);
    parse_optional_multiplicity(p);

    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

/// Parse a multiplicity bound (lower or upper)
/// Supports: integers, *, qualified names, and expressions like function calls
fn parse_multiplicity_bound<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::INTEGER) || p.at(SyntaxKind::STAR) {
        p.bump();
    } else if p.at_name_token() {
        // Could be a simple name reference or a function call like size(items)
        p.parse_qualified_name();
        p.skip_trivia();

        // Check for function call syntax: name(args)
        if p.at(SyntaxKind::L_PAREN) {
            parse_multiplicity_function_call(p);
        }
    }
}

/// Parse function call arguments in multiplicity context
/// Handles nested parens for expressions like size(items) or compute(a, b)
fn parse_multiplicity_function_call<P: KerMLParser>(p: &mut P) {
    if !p.at(SyntaxKind::L_PAREN) {
        return;
    }

    bump_and_skip(p); // (

    let mut paren_depth = 1;
    while paren_depth > 0 && !p.at(SyntaxKind::ERROR) {
        if p.at(SyntaxKind::L_PAREN) {
            paren_depth += 1;
            p.bump();
        } else if p.at(SyntaxKind::R_PAREN) {
            paren_depth -= 1;
            if paren_depth > 0 {
                p.bump();
            }
        } else {
            p.bump();
        }

        if paren_depth > 0 {
            p.skip_trivia();
        }
    }

    if p.at(SyntaxKind::R_PAREN) {
        p.bump(); // )
    }
}

/// Parse multiplicity modifiers (ordered, nonunique)
fn parse_multiplicity_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

/// Multiplicity = '[' bounds ']'
/// Per pest: multiplicity_bounds = { "[" ~ multiplicity_bounds_range ~ "]" }
/// Per pest: multiplicity_bounds_range = { multiplicity_bound ~ (".." ~ multiplicity_bound)? }
/// Per pest: multiplicity_bound = { inline_expression | number | "*" }
/// Per pest: ordering_modifiers = { (ordered_token | nonunique_token)* }
pub fn parse_multiplicity<P: KerMLParser>(p: &mut P) {
    if !p.at(SyntaxKind::L_BRACKET) {
        return;
    }

    p.start_node(SyntaxKind::MULTIPLICITY);
    bump_and_skip(p);

    if !p.at(SyntaxKind::R_BRACKET) {
        let is_modifier = p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW);

        if !is_modifier {
            parse_multiplicity_bound(p);
            p.skip_trivia();

            if p.at(SyntaxKind::DOT_DOT) {
                bump_and_skip(p);
                parse_multiplicity_bound(p);
            }
            p.skip_trivia();
        }

        parse_multiplicity_modifiers(p);
    }

    p.skip_trivia();
    p.expect(SyntaxKind::R_BRACKET);
    p.finish_node();
}

/// Multiplicity definition: multiplicity exactlyOne [1..1] { }
/// Per pest: multiplicity = { multiplicity_token ~ identification? ~ multiplicity_bounds? ~ namespace_body }
pub fn parse_multiplicity_definition<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::MULTIPLICITY_KW);

    // Optional identification
    parse_optional_identification(p);

    // Optional multiplicity bounds [1..1]
    parse_optional_multiplicity(p);

    // Body
    p.parse_body();

    p.finish_node();
}

/// Parse a single specialization relationship
/// Per pest: relationship = { visibility_kind? ~ element_reference ~ ...}
/// Per pest: inheritance = { relationship }
/// So many relationship clauses like :>, conjugates, chains, disjoint, etc. accept optional visibility
fn parse_single_specialization<P: KerMLParser>(p: &mut P, keyword: SyntaxKind) {
    p.start_node(SyntaxKind::SPECIALIZATION);
    bump_and_skip(p);

    if (keyword == SyntaxKind::DISJOINT_KW && p.at(SyntaxKind::FROM_KW))
        || (keyword == SyntaxKind::INVERSE_KW && p.at(SyntaxKind::OF_KW))
    {
        bump_and_skip(p);
    }

    // Parse optional visibility before the qualified name
    // Per pest: relationship = { visibility_kind? ~ element_reference }
    parse_optional_visibility(p);

    parse_qualified_name_and_skip(p);

    // Handle comma-separated references: :>> A, B, C
    while p.at(SyntaxKind::COMMA) {
        bump_and_skip(p);
        // Each item in the list can have its own visibility
        parse_optional_visibility(p);
        parse_qualified_name_and_skip(p);
    }

    p.finish_node();
    p.skip_trivia();
}

/// Specializations = (':>' | 'specializes' | etc.) QualifiedName
/// Per pest: heritage = { specialization | reference_subsetting | subsetting | redefinition | cross_subsetting | conjugation }
/// Per pest: specializes_operator = { ":>" | specializes_token }
/// Per pest: redefines_operator = { ":>>" | redefines_token }
/// Per pest: subsets_operator = { ":>" | subsets_token }
pub fn parse_specializations<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
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
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::TYPED_KW) || p.at(SyntaxKind::OF_KW) {
            parse_typing(p);
            p.skip_trivia();
            continue;
        }

        let keyword = p.current_kind();
        parse_single_specialization(p, keyword);
    }
}

/// Package = 'package' | 'namespace' Identification? Body
/// Per pest: package = { prefix_metadata? ~ (package_token | namespace_token) ~ identification? ~ namespace_body }
pub fn parse_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PACKAGE);

    if p.at(SyntaxKind::PACKAGE_KW) || p.at(SyntaxKind::NAMESPACE_KW) {
        p.bump();
    } else {
        p.expect(SyntaxKind::PACKAGE_KW);
    }
    p.skip_trivia();

    parse_optional_identification(p);

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// LibraryPackage = 'standard'? 'library' 'package' ...
/// Per pest: library_package = { prefix_metadata? ~ (library_token | standard_token) ~ library_token? ~ identification? ~ namespace_body }
pub fn parse_library_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::LIBRARY_PACKAGE);

    consume_if(p, SyntaxKind::STANDARD_KW);
    expect_and_skip(p, SyntaxKind::LIBRARY_KW);
    expect_and_skip(p, SyntaxKind::PACKAGE_KW);

    parse_optional_identification(p);

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// Import = 'import' 'all'? ImportedMembership ... relationship_body
/// Per pest: import = { import_prefix ~ imported_reference ~ filter_package? ~ relationship_body }
/// Per pest: relationship_body = { ";" | ("{" ~ relationship_owned_elements ~ "}") }
pub fn parse_import<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IMPORT);

    expect_and_skip(p, SyntaxKind::IMPORT_KW);
    consume_if(p, SyntaxKind::ALL_KW);
    parse_qualified_name_and_skip(p);

    parse_import_wildcards(p);

    p.skip_trivia();
    if p.at(SyntaxKind::L_BRACKET) {
        parse_filter_package(p);
    }

    // Per pest: relationship_body = ";" | ("{" ~ relationship_owned_elements ~ "}")
    p.skip_trivia();
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.error("expected ';' or '{'");
    }
    p.finish_node();
}

/// Parse import wildcards: ::* or ::** or ::*::**
fn parse_import_wildcards<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::COLON_COLON) {
        bump_and_skip(p);
        if p.at(SyntaxKind::STAR_STAR) {
            bump_and_skip(p);
        } else if p.at(SyntaxKind::STAR) {
            bump_and_skip(p);
            consume_if(p, SyntaxKind::STAR);
        } else {
            break;
        }
    }
}

fn parse_filter_package<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::FILTER_PACKAGE);

    // Parse one or more [expression] filter members
    while p.at(SyntaxKind::L_BRACKET) {
        bump_and_skip(p); // [

        // Check if it's metadata annotation syntax [@Type] or just filter expression
        if p.at(SyntaxKind::AT) || p.at(SyntaxKind::AT_AT) {
            bump_and_skip(p); // @ or @@
            parse_qualified_name_and_skip(p);
        } else {
            // Parse filter expression
            super::kerml_expressions::parse_expression(p);
        }

        p.skip_trivia();
        expect_and_skip(p, SyntaxKind::R_BRACKET);
    }

    p.finish_node(); // FILTER_PACKAGE
}

/// Alias = 'alias' Identification 'for' QualifiedName ';'
pub fn parse_alias<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ALIAS_MEMBER);

    expect_and_skip(p, SyntaxKind::ALIAS_KW);
    parse_identification_and_skip(p);
    expect_and_skip(p, SyntaxKind::FOR_KW);
    parse_qualified_name_and_skip(p);

    // Per pest: relationship_body = ";" | ("{" ~ relationship_owned_elements ~ "}")
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.error("expected ';' or '{'");
    }

    p.finish_node();
}

/// KerML definition (class, struct, etc.)
/// Per pest: class = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ class_token ~ all_token? ~ identification? ~ multiplicity_bounds? ~ classifier_relationships ~ namespace_body }
/// Per pest: structure = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ struct_token ~ identification? ~ all_token? ~ multiplicity_bounds? ~ classifier_relationships ~ namespace_body }
/// Per pest: datatype = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ datatype_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ namespace_body }
/// Per pest: behavior = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ behavior_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ namespace_body }
/// Per pest: function = { prefix_metadata? ~ visibility_kind? ~ abstract_marker? ~ function_token ~ identification? ~ all_token? ~ classifier_relationships ~ multiplicity? ~ result_expression_membership? ~ namespace_body }
pub fn parse_definition_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEFINITION);

    // Prefixes
    while p.at(SyntaxKind::ABSTRACT_KW) || p.at(SyntaxKind::VARIATION_KW) {
        bump_and_skip(p);
    }

    let is_predicate = p.at(SyntaxKind::PREDICATE_KW);
    let is_function = p.at(SyntaxKind::FUNCTION_KW);

    // KerML keyword
    if p.at_any(&[
        SyntaxKind::CLASS_KW,
        SyntaxKind::STRUCT_KW,
        SyntaxKind::DATATYPE_KW,
        SyntaxKind::BEHAVIOR_KW,
        SyntaxKind::FUNCTION_KW,
        SyntaxKind::CLASSIFIER_KW,
        SyntaxKind::INTERACTION_KW,
        SyntaxKind::PREDICATE_KW,
        SyntaxKind::METACLASS_KW,
        SyntaxKind::TYPE_KW,
    ]) {
        p.bump();
    } else if p.at(SyntaxKind::ASSOC_KW) {
        bump_and_skip(p);
        consume_if(p, SyntaxKind::STRUCT_KW);
    }
    p.skip_trivia();

    consume_if(p, SyntaxKind::ALL_KW);

    parse_optional_identification(p);

    parse_optional_multiplicity(p);

    parse_specializations(p);
    p.skip_trivia();

    // Parse ordering modifiers (ordered, nonunique)
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }

    parse_optional_multiplicity(p);

    parse_specializations(p);
    p.skip_trivia();

    // Parse ordering modifiers again (can appear after relationships)
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }

    if is_predicate || is_function {
        parse_calc_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse a single element in a calc body (parameter, namespace element, or expression)
fn parse_calc_body_element<P: KerMLParser>(p: &mut P) -> bool {
    if p.at_any(&[
        SyntaxKind::IN_KW,
        SyntaxKind::OUT_KW,
        SyntaxKind::INOUT_KW,
        SyntaxKind::RETURN_KW,
    ]) {
        parse_parameter_impl(p);
        true
    } else if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::CONSTRAINT_KW,
        SyntaxKind::DOC_KW,
        SyntaxKind::COMMENT_KW,
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        parse_namespace_element(p);
        true
    } else if p.at_name_token()
        || p.at(SyntaxKind::L_PAREN)
        || p.at(SyntaxKind::INTEGER)
        || p.at(SyntaxKind::STRING)
    {
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
        if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }
        true
    } else {
        parse_namespace_element(p);
        true
    }
}

/// Per pest: Used for function/predicate result expression body
/// Similar to namespace_body but specialized for calculation results
pub fn parse_calc_body<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        bump_and_skip(p);

        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            let start_pos = p.get_pos();

            parse_calc_body_element(p);
            p.skip_trivia();

            if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}'", text)
                } else {
                    kind_to_name(p.current_kind()).to_string()
                };
                p.error(format!("unexpected {} in calc body", got));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        p.error("expected ';' or '{'");
    }

    p.finish_node();
}

/// Parse feature prefix modifiers (var, composite, const, etc.)
/// Per pest: feature_prefix_modifiers = { (abstract_token | composite_token | portion_token | member_token | const_modifier | derived | end_marker | variable_marker)* }
fn parse_feature_prefix_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::MEMBER_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::CONST_KW,
        SyntaxKind::END_KW,
        SyntaxKind::VARIATION_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }

    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }
}

/// Parse optional feature keyword (feature, step, expr, inv)
/// Parse optional feature keyword (feature, step, expr, inv)
/// Per pest: invariant = { prefix_metadata? ~ inv_token ~ not_token? ~ identification? ~ ... }
fn parse_optional_feature_keyword<P: KerMLParser>(p: &mut P) -> bool {
    if p.at(SyntaxKind::INV_KW) {
        p.bump();
        p.skip_trivia();
        // Per pest: inv_token ~ not_token? - handle optional 'not' after 'inv'
        if p.at(SyntaxKind::NOT_KW) {
            p.bump();
            p.skip_trivia();
        }
        true
    } else if p.at_any(&[
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
    ]) {
        p.bump();
        true
    } else if p.at(SyntaxKind::IDENT) {
        if let Some(text) = p.current_token_text() {
            if text == "feature" || text == "step" || text == "expr" {
                p.bump();
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// Parse usage identification or specialization shortcuts
fn parse_usage_name_or_shorthand<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
    {
        // Wrap in SPECIALIZATION node so AST can extract the relationship
        p.start_node(SyntaxKind::SPECIALIZATION);
        bump_and_skip(p);
        parse_optional_qualified_name(p);
        p.finish_node();
        while p.at(SyntaxKind::COMMA) {
            bump_and_skip(p);
            p.start_node(SyntaxKind::SPECIALIZATION);
            parse_qualified_name_and_skip(p);
            p.finish_node();
        }
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        // Check for "type name" pattern: two identifiers in a row
        // e.g., "bool signalCondition { }" = feature signalCondition : bool
        if p.at(SyntaxKind::IDENT) {
            let peek1 = p.peek_kind(1);
            if peek1 == SyntaxKind::IDENT || peek1 == SyntaxKind::LT {
                // First identifier is the type, create typing node
                p.start_node(SyntaxKind::TYPING);
                p.bump(); // type name
                p.skip_trivia();
                p.finish_node();
                // Second identifier is the feature name
                p.parse_identification();
                return;
            }
        }
        p.parse_identification();
    }
}

/// Parse usage details (multiplicity, typing, specializations, relationships)
fn parse_usage_details<P: KerMLParser>(p: &mut P) {
    p.skip_trivia();
    parse_optional_multiplicity(p);
    parse_optional_typing(p);
    parse_optional_multiplicity(p);
    // Per pest: ordering_modifiers can appear before or after specializations
    parse_ordering_modifiers(p);
    parse_specializations(p);
    p.skip_trivia();
    // Parse ordering modifiers again (can appear after specializations too)
    parse_ordering_modifiers(p);
    parse_feature_relationships(p);
    p.skip_trivia();
}

/// Parse ordering modifiers (ordered, nonunique)
fn parse_ordering_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_and_skip(p);
    }
}

/// Parse optional default value (= or := or default)
/// Per pest: feature_value = { ("=" | ":=" | default_token) ~ owning_membership }
fn parse_optional_default_value<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }
}

/// KerML usage (feature, step, expr)
/// Per pest: feature = { prefix_metadata? ~ visibility_kind? ~ feature_direction_kind? ~ feature_prefix_modifiers ~ feature_token ~ all_token? ~ identification? ~ feature_specialization_part? ~ ordering_modifiers ~ feature_relationship_part* ~ feature_value? ~ namespace_body }
/// Per pest: step = { prefix_metadata? ~ feature_direction_kind? ~ connector_feature_modifiers ~ step_token ~ identification? ~ feature_specialization_part? ~ feature_value? ~ membership? ~ owning_membership? ~ namespace_body }
/// Per pest: expression = similar to feature with expr_token
pub fn parse_usage_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    parse_feature_prefix_modifiers(p);

    let _consumed_feature_keyword = parse_optional_feature_keyword(p);
    p.skip_trivia();

    parse_usage_name_or_shorthand(p);

    parse_usage_details(p);

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}

/// KerML invariant (inv [not]? name? { expression })
/// Per pest: invariant = { prefix_metadata? ~ inv_token ~ not_token? ~ identification? ~ invariant_body }
pub fn parse_invariant<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::INV_KW);
    p.skip_trivia();

    // Optional 'not'
    if p.at(SyntaxKind::NOT_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Optional identification
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }

    // Body: { expression }
    if p.at(SyntaxKind::L_BRACE) {
        p.start_node(SyntaxKind::NAMESPACE_BODY);
        p.bump(); // {
        p.skip_trivia();

        // Parse the invariant expression
        if !p.at(SyntaxKind::R_BRACE) {
            super::kerml_expressions::parse_expression(p);
        }

        p.skip_trivia();
        p.expect(SyntaxKind::R_BRACE);
        p.finish_node();
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// KerML parameter (in, out, inout, return)
/// Per pest: feature_direction_kind = { inout_token | in_token | out_token }
/// Per pest: parameter_membership = { direction ~ (type_name ~ name | name | ...) ~ ... }
/// Parameters are features with explicit direction keywords
pub fn parse_parameter_impl<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parse parameter direction keyword
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

    parse_feature_prefix_modifiers(p);

    // Optional usage keyword
    if p.at_any(&[
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ITEM_KW,
        SyntaxKind::CALC_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::PORT_KW,
        SyntaxKind::FEATURE_KW,
        SyntaxKind::STEP_KW,
        SyntaxKind::EXPR_KW,
    ]) {
        bump_and_skip(p);
    }

    // Per pest grammar, parameters can have: type_name name | name | ...
    // Check for two identifiers in a row (type + name pattern)
    if p.at(SyntaxKind::IDENT) {
        let peek1 = p.peek_kind(1);
        if peek1 == SyntaxKind::IDENT {
            // First identifier is the type, bump it
            p.bump();
            p.skip_trivia();
            // Second identifier is the name
            if p.at_name_token() {
                p.parse_identification();
            }
        } else {
            // Just a name (or starts with shorthand)
            parse_usage_name_or_shorthand(p);
        }
    } else {
        parse_usage_name_or_shorthand(p);
    }

    parse_usage_details(p);

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}

/// Parse multiplicity with ordering modifiers
fn parse_multiplicity_with_ordering<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();

        while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
            bump_and_skip(p);
        }
    }
}

/// Parse end feature prefix (metadata and modifiers)
/// Parse all prefix modifiers (ref, readonly, derived, etc.)
fn parse_prefix_modifiers<P: KerMLParser>(p: &mut P) {
    while p.at_any(&[
        SyntaxKind::REF_KW,
        SyntaxKind::READONLY_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::VARIATION_KW,
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::INDIVIDUAL_KW,
    ]) {
        bump_and_skip(p);
    }
}

/// Parse optional SysML usage keyword (item, part, action, etc.)
fn parse_optional_sysml_usage_keyword<P: KerMLParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::ITEM_KW,
        SyntaxKind::PART_KW,
        SyntaxKind::ACTION_KW,
        SyntaxKind::STATE_KW,
        SyntaxKind::ATTRIBUTE_KW,
        SyntaxKind::PORT_KW,
    ]) {
        bump_and_skip(p);
    }
}

fn parse_end_feature_prefix<P: KerMLParser>(p: &mut P) {
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    parse_prefix_modifiers(p);
    parse_multiplicity_with_ordering(p);
    parse_optional_sysml_usage_keyword(p);
}

/// Parse feature details (identification, typing, specializations)
fn parse_feature_details<P: KerMLParser>(p: &mut P, parse_id: bool) {
    if parse_id {
        parse_optional_identification(p);
    }

    parse_multiplicity_with_ordering(p);

    parse_optional_typing(p);

    parse_specializations(p);
    p.skip_trivia();

    parse_feature_relationships(p);
    p.skip_trivia();
}

/// Parse end feature when FEATURE_KW is present
fn parse_end_feature_with_keyword<P: KerMLParser>(p: &mut P) {
    bump_and_skip(p);

    // Check for specialization-first pattern or identification
    if p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::COLON_GT)
        || p.at(SyntaxKind::REFERENCES_KW)
        || p.at(SyntaxKind::COLON_COLON_GT)
    {
        parse_feature_details(p, false);
    } else {
        parse_feature_details(p, true);
    }
}

/// Parse end feature when starting with name/identification
/// Parse typing and relationships without FEATURE keyword
fn parse_typing_and_relationships<P: KerMLParser>(p: &mut P) {
    parse_optional_typing(p);
    parse_specializations(p);
    p.skip_trivia();
    parse_feature_relationships(p);
    p.skip_trivia();
}

fn parse_end_feature_with_name<P: KerMLParser>(p: &mut P) {
    parse_identification_and_skip(p);
    parse_multiplicity_with_ordering(p);
    parse_specializations(p);
    p.skip_trivia();
    parse_feature_relationships(p);
    p.skip_trivia();

    if p.at(SyntaxKind::FEATURE_KW) {
        bump_and_skip(p);
        parse_feature_details(p, true);
    } else {
        parse_typing_and_relationships(p);
    }
}

/// Parse minimal end feature (no name, no FEATURE_KW initially)
fn parse_end_feature_minimal<P: KerMLParser>(p: &mut P) {
    parse_multiplicity_with_ordering(p);
    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::FEATURE_KW) {
        bump_and_skip(p);
        parse_feature_details(p, true);
    }
}

/// End feature or parameter
/// Per pest: end_feature = { prefix_metadata? ~ const_token? ~ end_marker ~ (...various patterns...) ~ feature_value? ~ namespace_body }
/// Per pest: EndFeaturePrefix = ( isConstant ?= 'const')? isEnd ?= 'end'
pub fn parse_end_feature_or_parameter<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    consume_if(p, SyntaxKind::CONST_KW);
    expect_and_skip(p, SyntaxKind::END_KW);

    parse_end_feature_prefix(p);

    if p.at(SyntaxKind::FEATURE_KW) {
        parse_end_feature_with_keyword(p);
    } else if p.at_name_token() || p.at(SyntaxKind::LT) {
        parse_end_feature_with_name(p);
    } else {
        parse_end_feature_minimal(p);
    }

    parse_optional_default_value(p);

    p.parse_body();
    p.finish_node();
}

/// Parse connector identification or specialization prefix
fn parse_connector_name_or_specialization<P: KerMLParser>(
    p: &mut P,
    looks_like_direct_endpoint: bool,
) {
    if p.at(SyntaxKind::COLON_GT)
        || p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::SUBSETS_KW)
        || p.at(SyntaxKind::SPECIALIZES_KW)
        || p.at(SyntaxKind::REDEFINES_KW)
    {
        parse_specializations(p);
        p.skip_trivia();
    } else if p.at(SyntaxKind::EQ) {
        bump_and_skip(p);
        parse_optional_qualified_name(p);
    } else if !looks_like_direct_endpoint && (p.at_name_token() || p.at(SyntaxKind::LT)) {
        parse_identification_and_skip(p);

        if p.at(SyntaxKind::EQ) {
            bump_and_skip(p);
            parse_optional_qualified_name(p);
        } else {
            parse_specializations(p);
            p.skip_trivia();
        }
    }
}

/// Parse N-ary connector endpoints: (endpoint1, endpoint2, ...)
fn parse_nary_connector_endpoints<P: KerMLParser>(p: &mut P) -> bool {
    if !p.at(SyntaxKind::L_PAREN) {
        return false;
    }

    p.bump(); // (
    p.skip_trivia();

    if p.at_name_token() || p.at(SyntaxKind::L_BRACKET) {
        parse_connection_end(p);
        p.skip_trivia();
    }

    while p.at(SyntaxKind::COMMA) {
        p.bump(); // ,
        p.skip_trivia();
        if p.at_name_token() || p.at(SyntaxKind::L_BRACKET) {
            parse_connection_end(p);
            p.skip_trivia();
        }
    }

    if p.at(SyntaxKind::R_PAREN) {
        p.bump(); // )
        p.skip_trivia();
    }

    true
}

/// Parse binary connector endpoints: [from X] to Y
fn parse_binary_connector_endpoints<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::FROM_KW) {
        p.bump();
        p.skip_trivia();
        parse_connection_end(p);
        p.skip_trivia();
    } else if !p.at(SyntaxKind::TO_KW) && (p.at_name_token() || p.at(SyntaxKind::L_PAREN)) {
        parse_connection_end(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::TO_KW) {
        p.bump();
        p.skip_trivia();
        parse_connection_end(p);
        p.skip_trivia();
    }
}

/// Connector usage
pub fn parse_connector_usage<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR);

    while p.at_any(&[
        SyntaxKind::VAR_KW,
        SyntaxKind::COMPOSITE_KW,
        SyntaxKind::PORTION_KW,
        SyntaxKind::MEMBER_KW,
        SyntaxKind::ABSTRACT_KW,
        SyntaxKind::DERIVED_KW,
        SyntaxKind::CONST_KW,
        SyntaxKind::END_KW,
    ]) {
        bump_and_skip(p);
    }

    // Dispatch to binding/succession if applicable
    if p.at_any(&[
        SyntaxKind::BINDING_KW,
        SyntaxKind::SUCCESSION_KW,
        SyntaxKind::FIRST_KW,
    ]) {
        parse_binding_or_succession_impl(p);
        return;
    }

    expect_and_skip(p, SyntaxKind::CONNECTOR_KW);

    // Handle 'all' keyword for sufficient connectors (can appear before or after name)
    consume_if(p, SyntaxKind::ALL_KW);

    // Handle 'featured by' immediately after connector keyword (anonymous connector with featured by)
    if p.at(SyntaxKind::FEATURED_KW) {
        parse_feature_relationships(p);
        p.skip_trivia();
        parse_binary_connector_endpoints(p);
        p.parse_body();
        p.finish_node();
        return;
    }

    let looks_like_direct =
        looks_like_qualified_name_before(p, &[SyntaxKind::TO_KW, SyntaxKind::FROM_KW]);
    parse_connector_name_or_specialization(p, looks_like_direct);

    parse_optional_typing(p);
    parse_optional_multiplicity(p);
    parse_feature_relationships(p);
    p.skip_trivia();

    if parse_nary_connector_endpoints(p) {
        p.parse_body();
        p.finish_node();
        return;
    }

    parse_binary_connector_endpoints(p);

    p.parse_body();
    p.finish_node();
}

/// Parse connector endpoint
/// Per pest: connector_endpoint = { multiplicity_bounds? ~ (name ~ references_operator)? ~ feature_or_chain }
/// references_operator = @{ "::>" | "references" }
fn parse_connection_end<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTION_END);

    // Parse optional multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

    // Parse endpoint name, optionally followed by 'references' or '::>'
    if p.at_name_token() {
        let _checkpoint_pos = p.get_pos();
        p.parse_qualified_name();
        p.skip_trivia();

        // Check for references operator (::> or 'references')
        if p.at(SyntaxKind::REFERENCES_KW) || p.at(SyntaxKind::COLON_COLON_GT) {
            p.bump();
            p.skip_trivia();

            // Parse the target feature chain
            if p.at_name_token() {
                p.parse_qualified_name();
            }
        }
    }

    p.finish_node();
}

/// Parse binding/succession identification or specialization prefix
/// Helper to parse common prefix for binding/succession (identification, typing, etc.)
/// Returns true if a name was parsed
fn parse_binding_succession_prefix<P: KerMLParser>(
    p: &mut P,
    looks_like_direct_endpoint: bool,
) -> bool {
    let mut parsed_name = false;

    if p.at(SyntaxKind::REDEFINES_KW) || p.at(SyntaxKind::COLON_GT_GT) {
        // Wrap in SPECIALIZATION node so AST can extract the relationship
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
            parsed_name = true;
        }
        p.finish_node();
    } else if !looks_like_direct_endpoint && (p.at_name_token() || p.at(SyntaxKind::LT)) {
        p.parse_identification();
        p.skip_trivia();
        parsed_name = true;
    }

    parsed_name
}

/// Parse succession-specific modifiers (typing and multiplicity)
fn parse_succession_modifiers<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::COLON) {
        parse_typing(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }
}

/// Parse FIRST keyword pattern for successions
/// Per pest: Succession can use 'first' keyword for initial endpoint
/// succession = { ... (first_token ~ multiplicity_bounds? ~ feature_or_chain)? ~ (then_token ~ multiplicity_bounds? ~ feature_or_chain)? ... }
fn parse_succession_first_pattern<P: KerMLParser>(p: &mut P) {
    p.bump(); // FIRST
    p.skip_trivia();

    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

    p.parse_qualified_name();
    p.skip_trivia();

    if p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();

        if p.at(SyntaxKind::L_BRACKET) {
            parse_multiplicity(p);
            p.skip_trivia();
        }

        p.parse_qualified_name();
    }
}

/// Parse endpoint references (= or then keywords)
/// Per pest: binding patterns include multiplicity_bounds? before endpoints
/// Per pest: succession patterns include multiplicity_bounds? before endpoints
fn parse_endpoint_references<P: KerMLParser>(p: &mut P, parsed_name: bool) {
    // Parse optional multiplicity before first endpoint
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

    if !parsed_name && p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();

        // Parse optional multiplicity before second endpoint
        if p.at(SyntaxKind::L_BRACKET) {
            parse_multiplicity(p);
            p.skip_trivia();
        }

        if p.at_name_token() {
            p.parse_qualified_name();
        }
    }
}

/// Parse 'of' clause for binding connectors
/// Per pest: (of_token ~ multiplicity_bounds? ~ owned_feature_chain)
fn parse_binding_of_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::OF_KW) {
        p.bump();
        p.skip_trivia();

        // Optional multiplicity before the endpoint
        if p.at(SyntaxKind::L_BRACKET) {
            parse_multiplicity(p);
            p.skip_trivia();
        }

        // Feature chain (can use . separator)
        if p.at_name_token() {
            parse_feature_chain_or_qualified_name(p);
        }
    }
}

/// Check if should parse succession FIRST pattern
fn should_parse_first_pattern<P: KerMLParser>(p: &P, is_succession: bool) -> bool {
    is_succession && p.at(SyntaxKind::FIRST_KW)
}

/// Per pest: binding_connector = { prefix_metadata? ~ feature_direction_kind? ~ connector_feature_modifiers ~ binding_token ~ (...patterns...) }
/// Per pest: succession = { prefix_metadata? ~ feature_direction_kind? ~ connector_feature_modifiers ~ succession_token ~ (...patterns...) }
fn parse_binding_or_succession_impl<P: KerMLParser>(p: &mut P) {
    let is_succession = p.at(SyntaxKind::SUCCESSION_KW) || p.at(SyntaxKind::FIRST_KW);
    let is_shorthand_first = p.at(SyntaxKind::FIRST_KW);

    if !is_shorthand_first {
        bump_and_skip(p);
    }

    let parsed_name = if should_parse_first_pattern(p, is_succession) {
        false // FIRST indicates direct endpoint syntax
    } else {
        let looks_like_direct =
            looks_like_qualified_name_before(p, &[SyntaxKind::EQ, SyntaxKind::THEN_KW]);
        parse_binding_succession_prefix(p, looks_like_direct)
    };

    if !is_succession {
        parse_binding_of_clause(p);
    }

    if is_succession {
        parse_succession_modifiers(p);
    }

    parse_specializations(p);
    p.skip_trivia();

    if should_parse_first_pattern(p, is_succession) {
        parse_succession_first_pattern(p);
    } else {
        parse_endpoint_references(p, parsed_name);
    }

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// Parse flow usage (KerML item_flow and succession_item_flow)
/// Pattern: [abstract] [succession] flow [declaration] [of Type] [from X to Y] body
/// Per pest: item_flow = { flow_token ~ identification? ~ feature_specialization_part? ~ (...direct or declaration patterns...) }
/// ItemFlow can be: 'flow' X.y 'to' Z.w or 'flow' name ':' Type 'of' X 'from' Y 'to' Z
pub fn parse_flow_usage<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    consume_if(p, SyntaxKind::ABSTRACT_KW);
    consume_if(p, SyntaxKind::SUCCESSION_KW);
    expect_and_skip(p, SyntaxKind::FLOW_KW);

    let starts_with_all = consume_if(p, SyntaxKind::ALL_KW);

    // Determine which pattern to use
    let looks_like_direct = starts_with_all || {
        if p.at_name_token() {
            let next = p.peek_kind(1);
            matches!(next, SyntaxKind::DOT | SyntaxKind::TO_KW)
        } else {
            false
        }
    };

    if looks_like_direct {
        parse_flow_direct_pattern(p);
    } else {
        parse_flow_declaration_pattern(p);
    }

    p.skip_trivia();
    parse_body(p);
    p.finish_node();
}

/// Parse direct endpoint flow pattern: X.y to Z.w
fn parse_flow_direct_pattern<P: KerMLParser>(p: &mut P) {
    super::kerml_expressions::parse_expression(p);
    p.skip_trivia();

    if p.at(SyntaxKind::TO_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }
}

/// Parse declaration flow pattern: myFlow : Type of payload from X to Y
fn parse_flow_declaration_pattern<P: KerMLParser>(p: &mut P) {
    parse_optional_identification(p);
    parse_optional_multiplicity(p);
    parse_optional_typing(p);
    parse_specializations(p);
    p.skip_trivia();

    // Optional 'of' payload clause
    if p.at(SyntaxKind::OF_KW) {
        bump_and_skip(p);
        parse_qualified_name_and_skip(p);
    }

    // Parse optional from/to endpoints
    if p.at(SyntaxKind::FROM_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::TO_KW) {
        bump_and_skip(p);
        super::kerml_expressions::parse_expression(p);
        p.skip_trivia();
    }
}

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

pub fn parse_feature_relationships<P: KerMLParser>(p: &mut P) {
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
