use super::*;

pub fn parse_bind_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::BINDING_CONNECTOR);

    p.expect(SyntaxKind::BIND_KW);
    p.skip_trivia();

    // Optional multiplicity after bind keyword (connector_end can have multiplicity)
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // First end (left side)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // '=' separator
    if p.at(SyntaxKind::EQ) {
        p.bump();
        p.skip_trivia();
    }

    // Optional multiplicity before second end
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Second end (right side)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Body or semicolon
    p.parse_body();

    p.finish_node();
}

/// AssignAction = 'assign' target ':=' expr ';'
/// e.g., assign x := value;
/// Per pest: assignment_node = { "assign" ~ feature_reference ~ ":=" ~ owned_expression ~ (";"|action_body) }
/// Pattern: assign <feature> := <expression> <body|semicolon>
pub fn parse_assign_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::ASSIGN_KW);
    p.skip_trivia();

    // Assignment target (can be a feature chain like counter.count)
    if p.at_name_token() {
        p.parse_qualified_name(); // handles feature chains via dots
        p.skip_trivia();
    }

    // ':=' assignment operator
    if p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body or semicolon
    p.parse_body();

    p.finish_node();
}

/// ConnectUsage = 'connect' ...\n/// Per pest: binary_connection_usage = { \"connect\" ~ connector_part ~ (\";\"|connector_body) }\n/// Per pest: connector_part = { nary_connector_part | binary_connector_part }\n/// Per pest: binary_connector_part = { connector_end ~ \"to\" ~ connector_end }\n/// Per pest: nary_connector_part = { \"(\" ~ connector_end ~ (\",\" ~ connector_end)+ ~ \")\" }\n/// Pattern: connect (<end>, <end>) | connect <end> to <end> <body|semicolon>
pub fn parse_connect_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECT_USAGE);

    p.expect(SyntaxKind::CONNECT_KW);
    p.skip_trivia();

    // Per pest grammar: connect has connector_part which is either:
    // - binary: end to end
    // - nary: ( end, end, ... )

    if p.at(SyntaxKind::L_PAREN) {
        // N-ary connector part: ( end, end, ... )
        p.start_node(SyntaxKind::CONNECTOR_PART);
        p.bump(); // (
        p.skip_trivia();

        // Parse first connector end
        parse_connector_end(p);
        p.skip_trivia();

        // Parse remaining ends with commas
        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.expect(SyntaxKind::R_PAREN);
        p.finish_node(); // CONNECTOR_PART
        p.skip_trivia();
    } else {
        // Binary connector part: end to end
        p.start_node(SyntaxKind::CONNECTOR_PART);

        // First end
        parse_connector_end(p);
        p.skip_trivia();

        // 'to' keyword
        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();

            // Second end
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.finish_node(); // CONNECTOR_PART
    }

    p.parse_body();
    p.finish_node();
}

/// Parse a connector end
/// Per pest: connector_end = multiplicity? connector_end_reference
/// connector_end_reference = feature_chain | (identifier|quoted_name) ::> (feature_chain|reference) | reference
pub fn parse_connector_end<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR_END);

    // Optional multiplicity (e.g., [1..3])
    parse_optional_multiplicity(p);

    // connector_end_reference
    parse_connector_end_reference(p);

    p.finish_node();
}

/// Parse connector end reference
/// identifier ::> reference | identifier references reference | qualified_name
fn parse_connector_end_reference<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR_END_REFERENCE);

    if p.at_name_token() {
        // Parse the first identifier or qualified name
        parse_qualified_name_and_skip(p);

        // Check for ::> or 'references' (references operator)
        if p.at(SyntaxKind::COLON_COLON_GT) || p.at(SyntaxKind::REFERENCES_KW) {
            bump_keyword(p);

            // Parse target (qualified name or feature chain)
            parse_qualified_name_and_skip(p);
        }
    }

    p.finish_node();
}

/// Parse connector usage (standalone connector keyword)
/// connector [name] [:> Type] [from source to target] body
pub fn parse_connector_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONNECTOR);

    expect_and_skip(p, SyntaxKind::CONNECTOR_KW);

    // Optional identification
    parse_optional_identification(p);

    // Optional typing
    parse_optional_typing(p);

    // Optional specializations
    parse_specializations_with_skip(p);

    // Optional from...to clause
    if p.at(SyntaxKind::FROM_KW) {
        bump_keyword(p);
        parse_optional_qualified_name(p);

        if p.at(SyntaxKind::TO_KW) {
            bump_keyword(p);
            parse_optional_qualified_name(p);
        }
    }

    p.parse_body();
    p.finish_node();
}

/// Binding or Succession
/// succession [identification] [typing] [multiplicity] first [mult] source then [mult] target;
/// binding [identification] source = target;
pub fn parse_binding_or_succession<P: SysMLParser>(p: &mut P) {
    let is_succession = p.at(SyntaxKind::SUCCESSION_KW);

    // Check for succession flow pattern
    if is_succession && p.peek_kind(1) == SyntaxKind::FLOW_KW {
        // Delegate to SysML-specific flow parser
        parse_flow_usage(p);
        return;
    }

    if is_succession {
        p.start_node(SyntaxKind::SUCCESSION);
    } else {
        p.start_node(SyntaxKind::BINDING_CONNECTOR);
    }

    p.bump(); // binding or succession
    p.skip_trivia();

    // Optional multiplicity (for both binding and succession)
    // Examples: binding [1] bind ..., succession [0..*] first ...
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Check for 'bind' keyword (binding_connector_as_usage pattern)
    // Pattern: binding [mult]? name? bind [mult]? x = [mult]? y;
    if !is_succession && p.at(SyntaxKind::BIND_KW) {
        p.bump(); // bind
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // First end (left side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // '=' separator
        if p.at(SyntaxKind::EQ) {
            p.bump();
            p.skip_trivia();
        }

        // Optional multiplicity before second end
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Second end (right side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // Optional redefines
    let mut parsed_name = false;
    if p.at(SyntaxKind::REDEFINES_KW) || p.at(SyntaxKind::COLON_GT_GT) {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
            parsed_name = true;
        }
    // Optional identification (name) - but NOT for binding `name = target` pattern
    // In `binding payload = target`, `payload` is the source endpoint, not the name
    } else if p.at_name_token() && !p.at(SyntaxKind::FIRST_KW) && !p.at(SyntaxKind::BIND_KW) {
        // For bindings, check if token after name is '=' - if so, it's the source endpoint
        // For successions, check if token after name is 'then' - if so, it's the source endpoint
        // Peek ahead: name might be qualified (A::B) so look for EQ/THEN_KW after names
        let is_binding_source = !is_succession && peek_past_name_for(p, SyntaxKind::EQ);
        let is_succession_source = is_succession && peek_past_name_for(p, SyntaxKind::THEN_KW);

        if !is_binding_source && !is_succession_source {
            // It's an identification, not a source endpoint
            p.parse_identification();
            p.skip_trivia();
            parsed_name = true;
        }
    }

    // Check for 'bind' keyword AFTER optional identification
    // Pattern: binding myBinding bind [mult]? x = [mult]? y;
    if !is_succession && p.at(SyntaxKind::BIND_KW) {
        p.bump(); // bind
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // First end (left side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // '=' separator
        if p.at(SyntaxKind::EQ) {
            p.bump();
            p.skip_trivia();
        }

        // Optional multiplicity before second end
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Second end (right side of =)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        p.parse_body();
        p.finish_node();
        return;
    }

    // For binding: 'of' keyword
    if !is_succession && p.at(SyntaxKind::OF_KW) {
        p.bump();
        p.skip_trivia();
    }

    // For succession: optional typing
    if is_succession && p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Succession with first/then
    if is_succession && p.at(SyntaxKind::FIRST_KW) {
        p.bump(); // first
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Source feature
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        // One transition target: then X | if guard then X | else X
        if p.at(SyntaxKind::THEN_KW) {
            // then target
            p.bump(); // then
            p.skip_trivia();

            // Optional multiplicity
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }

            // Target feature
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        } else if p.at(SyntaxKind::IF_KW) {
            // if guard then target
            p.bump(); // if
            p.skip_trivia();

            // Guard expression
            if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
            }

            // then
            if p.at(SyntaxKind::THEN_KW) {
                p.bump();
                p.skip_trivia();

                // Target
                if p.at_name_token() {
                    p.parse_qualified_name();
                    p.skip_trivia();
                }
            }
        } else if p.at(SyntaxKind::ELSE_KW) {
            // else target
            p.bump(); // else
            p.skip_trivia();

            // Target
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
        }
    } else {
        // Simple succession/binding: source = target or source then target
        // Only parse the source name if we didn't already parse it via identification
        if !parsed_name && p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::THEN_KW) {
            p.bump();
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
        }
    }

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// VariantUsage = 'variant' ...

pub fn parse_flow_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    if p.at(SyntaxKind::ABSTRACT_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Handle optional succession keyword (succession flow)
    if p.at(SyntaxKind::SUCCESSION_KW) {
        p.bump();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::FLOW_KW);
    p.skip_trivia();

    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Check for direct flow pattern first (e.g., "flow X.Y to A.B")
    let is_direct_flow = peek_for_direct_flow(p);

    // Check for "flow of Type" pattern (no name, just typing)
    let has_of_clause = p.at(SyntaxKind::OF_KW);

    if is_direct_flow {
        p.parse_qualified_name();
        p.skip_trivia();

        if p.at(SyntaxKind::TO_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
        }
    } else if has_of_clause {
        // Pattern: flow of Type [mult] from X to Y
        p.bump(); // of
        p.skip_trivia();
        p.parse_qualified_name(); // Type
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Flow part: from X to Y or X to Y - wrap in FROM_TO_CLAUSE
        parse_optional_from_to(p);
    } else {
        // Pattern: flow [name] [: Type] [...] [from X to Y]
        // But skip identification if we're directly at FROM_KW (pattern: flow from X to Y)
        if (p.at_name_token() || p.at(SyntaxKind::LT)) && !p.at(SyntaxKind::FROM_KW) {
            p.parse_identification();
            p.skip_trivia();
        }

        // Parse multiplicity bounds (e.g., [1])
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        parse_specializations(p);
        p.skip_trivia();

        // Default value assignment (per sysml.pest: value_part in flow declarations)
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }

        // Optional 'of Type' for named flows
        if p.at(SyntaxKind::OF_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
            p.skip_trivia();

            // Multiplicity after of clause
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }
        }

        // Flow part: from X to Y - wrap in FROM_TO_CLAUSE
        parse_optional_from_to(p);
    }

    p.skip_trivia();
    p.parse_body();
    p.finish_node();
}

/// Helper to detect direct flow pattern (flow X.Y to A.B) vs named flow (flow name from X to Y)
fn peek_for_direct_flow<P: SysMLParser>(p: &P) -> bool {
    // Check if we see "name [.name]* to ..." pattern (direct flow endpoints)
    // vs "name : Type ..." pattern (declaration)

    // If we're currently at FROM_KW, this is definitely a from/to pattern, not direct
    if p.current_kind() == SyntaxKind::FROM_KW {
        return false;
    }

    // If we see a colon, it's a typed declaration
    if p.peek_kind(1) == SyntaxKind::COLON {
        return false;
    }

    // If we see FROM_KW before TO_KW, it's a named flow with from/to pattern, not direct
    // Pattern: "flow name from X to Y" vs "flow X to Y"
    let mut saw_from = false;

    // Look ahead for 'to' keyword within first few tokens
    for i in 1..9 {
        let kind = p.peek_kind(i);

        if kind == SyntaxKind::FROM_KW {
            saw_from = true;
        }

        if kind == SyntaxKind::TO_KW {
            // If we saw FROM before TO, it's a from/to pattern with a name, not direct flow
            if saw_from {
                return false;
            }
            return true;
        }

        // Stop if we hit something that indicates declaration (colon, equals, specialization)
        if matches!(
            kind,
            SyntaxKind::COLON
                | SyntaxKind::EQ
                | SyntaxKind::COLON_EQ
                | SyntaxKind::COLON_GT
                | SyntaxKind::COLON_GT_GT
                | SyntaxKind::SPECIALIZES_KW
        ) {
            return false;
        }
        // Stop if we hit end of statement
        if matches!(
            kind,
            SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE | SyntaxKind::ERROR
        ) {
            return false;
        }
    }

    false
}
