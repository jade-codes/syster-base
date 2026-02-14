use super::*;

pub fn parse_usage<P: SysMLParser>(p: &mut P) {
    // Per pest: usage = { (usage_prefix* ~ metadata_prefix* ~ event_prefix? ~ usage_element) | owned_crossing_feature }
    // Per pest: usage_element = { keyword ~ usage_declaration ~ value_part? ~ (body | ";") }
    // Per pest: owned_crossing_feature = { "end" ~ (identifier ~ multiplicity?)? ~ keyword ~ usage_declaration }
    // Pattern: [prefixes] [#metadata] [event] <keyword> [<name>] [<mult>] [<typing>] [<specializations>] [<default>] <body>
    p.start_node(SyntaxKind::USAGE);

    // Prefixes - returns true if END_KW was seen
    let saw_end = parse_usage_prefix(p);
    p.skip_trivia();

    // Prefix metadata (after prefix keywords, before usage keyword)
    while p.at(SyntaxKind::HASH) {
        parse_prefix_metadata(p);
        p.skip_trivia();
    }

    // Event modifier (event occurrence pattern)
    if p.at(SyntaxKind::EVENT_KW) {
        bump_keyword(p);
    }

    // Check for owned crossing feature after END_KW: end name [mult] usage_kw name
    // If we see a name after END prefix (not a usage keyword), it's an owned_crossing_feature
    if saw_end && p.at_name_token() {
        // Parse: name [mult] usage_keyword name :> ... { }
        p.parse_identification();
        p.skip_trivia();

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Now we expect a usage keyword
        if p.at_any(SYSML_USAGE_KEYWORDS) {
            parse_usage_keyword(p);
            p.skip_trivia();

            // Parse the actual feature name
            if p.at_name_token() || p.at(SyntaxKind::LT) {
                p.parse_identification();
                p.skip_trivia();
            }

            // Continue with multiplicity, typing, specializations as normal
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

            // Ordering modifiers
            while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
                p.bump();
                p.skip_trivia();
            }

            // Body
            p.parse_body();
            p.finish_node();
            return;
        }
    }

    // Check for owned crossing feature: end [mult] keyword ...
    // If we see multiplicity before the usage keyword, parse it
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    let is_constraint = p.at(SyntaxKind::CONSTRAINT_KW);
    let is_action = p.at(SyntaxKind::ACTION_KW);
    let is_calc = p.at(SyntaxKind::CALC_KW);
    let is_state = p.at(SyntaxKind::STATE_KW);
    let is_analysis = p.at(SyntaxKind::ANALYSIS_KW);
    let is_verification = p.at(SyntaxKind::VERIFICATION_KW);
    let is_metadata = p.at(SyntaxKind::METADATA_KW);
    let is_message = p.at(SyntaxKind::MESSAGE_KW);
    let is_usecase = p.at(SyntaxKind::USE_KW); // use case usage
    let is_connection_kw = p.at(SyntaxKind::CONNECTION_KW);
    let is_interface_kw = p.at(SyntaxKind::INTERFACE_KW);

    // Usage keyword
    parse_usage_keyword(p);
    p.skip_trivia();

    // Per pest: constraint_usage_declaration is optional (usage_declaration? ~ value_part?)
    // So we can have just "requirement;" or "constraint;" with no name/typing/body content
    // Check if we're at body start immediately after keyword
    if p.at(SyntaxKind::SEMICOLON) || p.at(SyntaxKind::L_BRACE) {
        // Minimal usage: just keyword + body
        if is_constraint {
            parse_constraint_body(p);
        } else if is_calc {
            parse_sysml_calc_body(p);
        } else if is_action {
            parse_action_body(p);
        } else if is_state {
            parse_state_body(p);
        } else if is_analysis || is_verification || is_usecase {
            parse_case_body(p);
        } else if is_metadata {
            parse_metadata_body(p);
        } else {
            p.parse_body();
        }
        p.finish_node();
        return;
    }

    // For message usages, handle 'of' keyword before identification
    // Pattern: message of payload:Type from source to target;
    if is_message {
        consume_if(p, SyntaxKind::OF_KW);
    }

    // Handle shorthand redefines: 'attribute :>> name' (no identifier before :>>)
    if p.at(SyntaxKind::COLON_GT_GT)
        || p.at(SyntaxKind::COLON_GT)
        || p.at(SyntaxKind::REDEFINES_KW)
        || p.at(SyntaxKind::SUBSETS_KW)
    {
        // This is a shorthand feature member after a usage keyword
        // Wrap in SPECIALIZATION node so AST can extract the relationship
        p.start_node(SyntaxKind::SPECIALIZATION);
        bump_keyword(p); // the operator

        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
        p.finish_node(); // finish first SPECIALIZATION

        // Handle multiplicity after first name: :>> name[mult]
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Handle comma-separated references: :>> A, B, C
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p); // comma
            p.start_node(SyntaxKind::SPECIALIZATION);
            if p.at_name_token() {
                p.parse_qualified_name();
                p.skip_trivia();
            }
            p.finish_node(); // finish additional SPECIALIZATION
            // Multiplicity after each name
            if p.at(SyntaxKind::L_BRACKET) {
                p.parse_multiplicity();
                p.skip_trivia();
            }
        }

        // Additional specializations (including ::> references)
        parse_specializations(p);
        p.skip_trivia();

        // Typing after shorthand redefinition
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        // Ordering modifiers
        while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
            bump_keyword(p);
        }

        // Default value
        parse_optional_default_value(p);

        // Body (check type-specific bodies for shorthand redefines too)
        if is_constraint {
            parse_constraint_body(p);
        } else if is_calc {
            parse_sysml_calc_body(p);
        } else if is_action {
            parse_action_body(p);
        } else if is_state {
            parse_state_body(p);
        } else if is_analysis || is_verification || is_usecase {
            parse_case_body(p);
        } else {
            p.parse_body();
        }
        p.finish_node();
        return;
    }

    // Identification - but NOT if we're at CONNECT_KW (which is part of connector clause)
    // Check if this is a feature chain reference (name.member) vs a simple name
    // For patterns like `event sendSpeed.sourceEvent;`, the chain is a reference, not a name
    let has_chain = if (p.at_name_token() || p.at(SyntaxKind::LT)) && !p.at(SyntaxKind::CONNECT_KW)
    {
        // Look ahead to see if there's a dot after the name
        let mut lookahead = 0;
        if p.at(SyntaxKind::LT) {
            // Skip past short name <name>
            lookahead += 1; // <
            if is_name_kind(p.peek_kind(lookahead)) {
                lookahead += 1;
            }
            if p.peek_kind(lookahead) == SyntaxKind::GT {
                lookahead += 1;
            }
        }
        if is_name_kind(p.peek_kind(lookahead)) {
            lookahead += 1;
        }
        // Skip any whitespace/trivia in lookahead (simplified - just check next few)
        p.peek_kind(lookahead) == SyntaxKind::DOT
    } else {
        false
    };

    // For interface/connection usages, check if this looks like a connector pattern
    // Pattern: interface X.y to Z.w - the feature chain is a connector endpoint, not a specialization
    let looks_like_connector_endpoint = if (is_connection_kw || is_interface_kw) && has_chain {
        // Look ahead to see if there's a 'to' keyword after the chain
        let mut depth = 0;
        let mut found_to = false;
        for i in 0..30 {
            match p.peek_kind(i) {
                SyntaxKind::TO_KW if depth == 0 => {
                    found_to = true;
                    break;
                }
                SyntaxKind::DOT | SyntaxKind::IDENT => {}
                SyntaxKind::L_BRACKET => depth += 1,
                SyntaxKind::R_BRACKET => depth -= 1,
                SyntaxKind::WHITESPACE => {} // Skip whitespace in lookahead
                SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE | SyntaxKind::COLON => break,
                _ => break,
            }
        }
        found_to
    } else {
        false
    };

    if has_chain && !looks_like_connector_endpoint {
        // This is a feature chain reference like `sendSpeed.sourceEvent`
        // Parse as a SPECIALIZATION with a QUALIFIED_NAME containing the chain
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name(); // Parses the full chain including dots
        p.skip_trivia();
        p.finish_node();
    } else if (p.at_name_token() || p.at(SyntaxKind::LT))
        && !p.at(SyntaxKind::CONNECT_KW)
        && !looks_like_connector_endpoint
    {
        p.parse_identification();
    }
    p.skip_trivia();

    // For message usages: handle 'of' payload type after name
    // Pattern: message sendSensedSpeed of SensedSpeed from ... to ...
    if is_message && p.at(SyntaxKind::OF_KW) {
        bump_keyword(p);
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
    }

    // Handle feature chain continuation (e.g., producerBehavior.publish[1])
    // This handles cases where chain wasn't detected by lookahead
    while p.at(SyntaxKind::DOT) {
        bump_keyword(p); // .
        if p.at_name_token() {
            bump_keyword(p);
        }
        // Optional indexing after feature access
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }
    }

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
    }
    p.skip_trivia();

    // Typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity after specializations (e.g., port myPort :>> basePort [5])
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Ordering modifiers
    while p.at(SyntaxKind::ORDERED_KW) || p.at(SyntaxKind::NONUNIQUE_KW) {
        bump_keyword(p);
    }

    // More specializations
    parse_specializations(p);
    p.skip_trivia();

    // For connection/interface usage: n-ary endpoint syntax after typing
    // Pattern: connection : Type ( end1 ::> a, end2 ::> b );
    // Pattern: interface : Type ( end1 ::> a, end2 ::> b );
    if (is_connection_kw || is_interface_kw) && p.at(SyntaxKind::L_PAREN) {
        p.start_node(SyntaxKind::CONNECTOR_PART);
        bump_keyword(p); // (

        parse_connector_end(p);
        p.skip_trivia();

        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            parse_connector_end(p);
            p.skip_trivia();
        }

        p.expect(SyntaxKind::R_PAREN);
        p.finish_node(); // CONNECTOR_PART
        p.skip_trivia();
    }

    // For connection/interface usage: binary endpoint syntax with 'to'
    // Pattern: interface source.port to target.port;
    if (is_connection_kw || is_interface_kw) && p.at_name_token() && !p.at(SyntaxKind::CONNECT_KW) {
        // Check if there's a 'to' keyword ahead
        let has_to = {
            let mut depth = 0;
            let mut found_to = false;
            for i in 0..20 {
                match p.peek_kind(i) {
                    SyntaxKind::TO_KW if depth == 0 => {
                        found_to = true;
                        break;
                    }
                    SyntaxKind::DOT | SyntaxKind::IDENT => {}
                    SyntaxKind::L_BRACKET => depth += 1,
                    SyntaxKind::R_BRACKET => depth -= 1,
                    SyntaxKind::SEMICOLON | SyntaxKind::L_BRACE => break,
                    _ => break,
                }
            }
            found_to
        };

        if has_to {
            p.start_node(SyntaxKind::CONNECTOR_PART);

            // Parse source endpoint (chain like source.port)
            parse_connector_end(p);
            p.skip_trivia();

            // 'to' keyword
            if p.at(SyntaxKind::TO_KW) {
                bump_keyword(p);

                // Parse target endpoint
                parse_connector_end(p);
                p.skip_trivia();
            }

            p.finish_node(); // CONNECTOR_PART
        }
    }

    // For allocation usage: optional allocate clause
    let is_allocation = p.at(SyntaxKind::ALLOCATE_KW);
    if is_allocation {
        // Parse allocate keyword part: allocate <source> to <target>
        bump_keyword(p); // allocate

        // Check for n-ary or binary pattern
        if p.at(SyntaxKind::L_PAREN) {
            // N-ary: allocate (a, b ::> c, ...)
            bump_keyword(p); // (

            parse_allocate_end_member(p);

            while p.at(SyntaxKind::COMMA) {
                bump_keyword(p);
                parse_allocate_end_member(p);
            }

            p.expect(SyntaxKind::R_PAREN);
            p.skip_trivia();
        } else {
            // Binary: allocate source to target
            parse_allocate_end_member(p);

            if consume_if(p, SyntaxKind::TO_KW) {
                parse_allocate_end_member(p);
            }
        }
    }

    // For connection usage: optional connect clause
    let is_connection = p.at(SyntaxKind::CONNECT_KW);
    if is_connection {
        // Parse connect keyword part: connect <end> to <end> or connect (<ends>)
        p.start_node(SyntaxKind::CONNECTOR_PART);
        bump_keyword(p); // connect

        // Check for n-ary or binary pattern
        if p.at(SyntaxKind::L_PAREN) {
            // N-ary: connect (a ::> b, c ::> d, ...)
            bump_keyword(p); // (

            parse_connector_end(p);
            p.skip_trivia();

            while p.at(SyntaxKind::COMMA) {
                bump_keyword(p);
                parse_connector_end(p);
                p.skip_trivia();
            }

            p.expect(SyntaxKind::R_PAREN);
            p.skip_trivia();
        } else {
            // Binary: connect source to target
            parse_connector_end(p);
            p.skip_trivia();

            if consume_if(p, SyntaxKind::TO_KW) {
                parse_connector_end(p);
                p.skip_trivia();
            }
        }
        p.finish_node(); // CONNECTOR_PART
    }

    // For message: optional from/to clause
    parse_optional_from_to(p);

    // Default value: 'default' [expr] or '=' expr or ':=' expr
    parse_optional_default_value(p);

    // About clause (for metadata usages)
    // Pattern: about annotation ("," annotation)*
    if p.at(SyntaxKind::ABOUT_KW) {
        bump_keyword(p); // about

        // Parse first annotation (qualified name or identifier)
        parse_optional_qualified_name(p);

        // Parse additional annotations
        while p.at(SyntaxKind::COMMA) {
            bump_keyword(p);
            parse_optional_qualified_name(p);
        }
    }

    // Body
    if is_constraint {
        parse_constraint_body(p);
    } else if is_calc {
        parse_sysml_calc_body(p);
    } else if is_action {
        parse_action_body(p);
    } else if is_state {
        parse_state_body(p);
    } else if is_analysis || is_verification || is_usecase {
        parse_case_body(p);
    } else if is_metadata {
        parse_metadata_body(p);
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse allocate end member: [name ::>] qualified_name
fn parse_allocate_end_member<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() {
        // Check if this is "name ::> ref" pattern
        let lookahead = 1;
        if p.peek_kind(lookahead) == SyntaxKind::COLON_COLON_GT {
            // Pattern: name ::> qualified_name
            p.bump(); // name
            p.skip_trivia();
            p.bump(); // ::>
            p.skip_trivia();
            if p.at_name_token() {
                p.parse_qualified_name();
            }
        } else {
            // Just a qualified name
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }
}

pub fn parse_definition_keyword<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::USE_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::CASE_KW) {
            p.bump();
        }
        return;
    }

    if p.at_any(SYSML_DEFINITION_KEYWORDS) {
        p.bump();
    }
}

fn parse_usage_keyword<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::USE_KW) {
        bump_keyword(p);
        if p.at(SyntaxKind::CASE_KW) {
            p.bump();
        }
        return;
    }

    if p.at_any(SYSML_USAGE_KEYWORDS) {
        // Don't consume a keyword if it's actually being used as a name.
        // Check if the next non-trivia token indicates this is a name (followed by : or :> or [ etc.)
        // This handles cases like `in frame : Integer` where `frame` is a name, not a usage keyword.
        if p.at_name_token() {
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);
            if matches!(
                next,
                SyntaxKind::COLON
                    | SyntaxKind::COLON_GT
                    | SyntaxKind::COLON_GT_GT
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::SEMICOLON
                    | SyntaxKind::L_BRACE
                    | SyntaxKind::REDEFINES_KW
                    | SyntaxKind::SUBSETS_KW
                    | SyntaxKind::REFERENCES_KW
            ) {
                // This looks like a name followed by typing/specialization, not a usage keyword
                return;
            }
        }
        p.bump();
    }
}

fn parse_usage_prefix<P: SysMLParser>(p: &mut P) -> bool {
    let mut saw_end = false;
    while p.at_any(USAGE_PREFIX_KEYWORDS) {
        if p.at(SyntaxKind::END_KW) {
            saw_end = true;
        }
        bump_keyword(p);
    }
    saw_end
}
