use super::*;

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
pub(super) fn parse_textual_representation<P: SysMLParser>(p: &mut P) {
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

    parse_optional_typing(p);

    parse_specializations_with_skip(p);

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

    parse_optional_typing(p);

    parse_optional_multiplicity(p);

    parse_specializations_with_skip(p);

    // Default value or assignment
    parse_optional_default_value(p);

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
pub(super) fn parse_anonymous_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Parse typing (: Type or typed by Type)
    p.parse_typing();
    p.skip_trivia();

    // Optional specializations
    parse_specializations_with_skip(p);

    // Optional value assignment
    parse_optional_default_value(p);

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
    parse_optional_typing(p);

    parse_specializations_with_skip(p);

    parse_optional_default_value(p);

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
