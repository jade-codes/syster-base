use super::*;

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
        | SyntaxKind::ABOUT_KW => {
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
pub(super) fn parse_prefix_metadata<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::PREFIX_METADATA);
    expect_and_skip(p, SyntaxKind::HASH);
    if p.at_name_token() {
        p.bump();
    }
    p.finish_node();
}
