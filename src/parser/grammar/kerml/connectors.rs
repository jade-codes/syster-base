use super::*;

// =============================================================================
// Connectors — connector usage, bindings, successions, flows
// =============================================================================

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
/// Extended pattern: of [mult] X = [mult] Y
fn parse_binding_of_clause<P: KerMLParser>(p: &mut P) {
    if p.at(SyntaxKind::OF_KW) {
        p.bump();
        p.skip_trivia();

        // Optional multiplicity before the first endpoint
        if p.at(SyntaxKind::L_BRACKET) {
            parse_multiplicity(p);
            p.skip_trivia();
        }

        // Feature chain (can use . separator)
        if p.at_name_token() {
            parse_feature_chain_or_qualified_name(p);
            p.skip_trivia();
        }

        // Handle second endpoint: = [mult] Y
        if p.at(SyntaxKind::EQ) {
            p.bump();
            p.skip_trivia();

            // Optional multiplicity before the second endpoint
            if p.at(SyntaxKind::L_BRACKET) {
                parse_multiplicity(p);
                p.skip_trivia();
            }

            // Feature chain for second endpoint
            if p.at_name_token() {
                parse_feature_chain_or_qualified_name(p);
                p.skip_trivia();
            }
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

    // Handle 'all' keyword for sufficient successions (can appear after succession keyword)
    // Per SysML v2 Spec: sufficient successions use 'all' to indicate all instances
    consume_if(p, SyntaxKind::ALL_KW);

    let parsed_name = if should_parse_first_pattern(p, is_succession) {
        false // FIRST indicates direct endpoint syntax
    } else {
        // For successions: `succession X then Y` should NOT parse X as the identification
        // X is a direct endpoint reference. Only parse as identification if there's something
        // else between the name and 'then' (like typing, specialization, etc.)
        //
        // For bindings: `binding payload = target` should NOT parse `payload` as identification
        // `payload` is the source endpoint. Only parse as identification if there's something
        // else between the name and '=' (like typing, or another name for bind keyword)
        let looks_like_direct = if is_succession {
            // Check if the pattern is simply `name then` (direct endpoint)
            // vs `name : Type then` or `name someName then` (has identification)
            let next_after_name = p.peek_kind(1);
            next_after_name == SyntaxKind::THEN_KW
        } else {
            // For bindings: check if the name is followed by `=` (after skipping optional qualifiers)
            // `binding payload = target` -> direct endpoint (payload is source)
            // `binding myName bind x = y` -> myName is identification, x is source
            looks_like_name_then(p, SyntaxKind::EQ)
        };
        parse_binding_succession_prefix(p, looks_like_direct)
    };

    // Handle optional multiplicity after binding name (e.g., binding instant[1] of ...)
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

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
