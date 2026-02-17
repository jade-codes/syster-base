use super::*;

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
                p.current_kind().display_name().to_string()
            };
            p.error(format!("unexpected {} in top level", got));
            p.bump();
        }
    }

    p.finish_node();
}

// =============================================================================
// Namespace Element Dispatch
// =============================================================================

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
        | SyntaxKind::REF_KW
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
        | SyntaxKind::IF_KW
        | SyntaxKind::INTEGER
        | SyntaxKind::DECIMAL
        | SyntaxKind::STRING
        | SyntaxKind::L_PAREN => {
            kerml_expressions::parse_expression(p);
            p.skip_trivia();
            if p.at(SyntaxKind::SEMICOLON) {
                p.bump();
            }
        }

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
                    SyntaxKind::CLASS_KW,
                    SyntaxKind::R_BRACE,
                ],
            );
        }
    }
}

// =============================================================================
// Prefix Metadata
// =============================================================================

/// Parse prefix metadata (#name)
/// Per pest: prefix_metadata = { user_defined_keyword+ }
/// Per pest: user_defined_keyword = { "#" ~ (identifier ~ ("::" ~ identifier)*) }
pub fn parse_prefix_metadata<P: KerMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PREFIX_METADATA);
    expect_and_skip(p, SyntaxKind::HASH);
    if p.at_name_token() {
        p.bump();
    }
    p.finish_node();
}

// =============================================================================
// Prefix Dispatch Handlers
// =============================================================================

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
