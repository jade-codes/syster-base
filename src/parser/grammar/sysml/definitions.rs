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
/// Per pest: package_member = { (definition | usage | alias_member | ...)
/// Pattern: Determines whether to parse as definition (has 'def') or usage (no 'def')
pub fn parse_definition_or_usage<P: SysMLParser>(p: &mut P) {
    let classification = classify_definition_or_usage(p);

    match classification {
        DefinitionClassification::SysmlDefinition => parse_definition(p),
        DefinitionClassification::KermlDefinition => parse_kerml_definition(p),
        DefinitionClassification::Usage => parse_usage(p),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionClassification {
    SysmlDefinition,
    KermlDefinition,
    Usage,
}

/// Check if kind is a KerML-only definition keyword (without 'def')
fn is_kerml_definition_keyword(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::CLASS_KW
            | SyntaxKind::STRUCT_KW
            | SyntaxKind::DATATYPE_KW
            | SyntaxKind::BEHAVIOR_KW
            | SyntaxKind::FUNCTION_KW
            | SyntaxKind::ASSOC_KW
            | SyntaxKind::INTERACTION_KW
            | SyntaxKind::PREDICATE_KW
            | SyntaxKind::METACLASS_KW
            | SyntaxKind::CLASSIFIER_KW
            | SyntaxKind::TYPE_KW
    )
}

fn classify_definition_or_usage<P: SysMLParser>(p: &P) -> DefinitionClassification {
    // Scan ahead (skipping trivia) to determine what we have
    // KerML definition: struct, class, datatype, etc. (no 'def' keyword)
    // SysML definition: has 'def' keyword
    // Usage: everything else (no 'def' keyword)

    // Check first token for KerML definition keywords
    // Skip over ABSTRACT_KW if present
    let first_non_prefix = if p.peek_kind(0) == SyntaxKind::ABSTRACT_KW {
        p.peek_kind(1)
    } else {
        p.peek_kind(0)
    };

    if is_kerml_definition_keyword(first_non_prefix) {
        return DefinitionClassification::KermlDefinition;
    }

    for i in 0..20 {
        // Look ahead up to 20 tokens
        let kind = p.peek_kind(i);

        // SysML definition: has 'def' keyword
        if kind == SyntaxKind::DEF_KW {
            return DefinitionClassification::SysmlDefinition;
        }

        // Stop scanning at statement-ending tokens
        if kind == SyntaxKind::SEMICOLON
            || kind == SyntaxKind::L_BRACE
            || kind == SyntaxKind::COLON
            || kind == SyntaxKind::COLON_GT
            || kind == SyntaxKind::COLON_GT_GT
            || kind == SyntaxKind::EQ
            || kind == SyntaxKind::ERROR
        {
            return DefinitionClassification::Usage;
        }
    }
    DefinitionClassification::Usage
}

fn parse_definition<P: SysMLParser>(p: &mut P) {
    // Per pest: definition = { prefix* ~ definition_declaration ~ definition_body }
    // Per pest: definition_declaration = { keyword ~ "def"? ~ (identifier ~ ";") | (usage_prefix ~ definition_declaration) }
    // Pattern: [abstract|variation|individual] <keyword> def <name> <specializations> <body>
    p.start_node(SyntaxKind::DEFINITION);

    // Prefixes (variation point and individual markers)
    while p.at(SyntaxKind::ABSTRACT_KW)
        || p.at(SyntaxKind::VARIATION_KW)
        || p.at(SyntaxKind::INDIVIDUAL_KW)
    {
        bump_keyword(p);
    }

    let is_constraint = p.at(SyntaxKind::CONSTRAINT_KW);
    let is_calc = p.at(SyntaxKind::CALC_KW);
    let is_action = p.at(SyntaxKind::ACTION_KW);
    let is_state = p.at(SyntaxKind::STATE_KW);
    let is_analysis = p.at(SyntaxKind::ANALYSIS_KW);
    let is_verification = p.at(SyntaxKind::VERIFICATION_KW);
    let is_metadata = p.at(SyntaxKind::METADATA_KW);
    let is_usecase = p.at(SyntaxKind::USE_KW); // use case def

    // Definition keyword
    parse_definition_keyword(p);
    p.skip_trivia();

    // 'def' keyword (or 'case def' for analysis/verification)
    consume_if(p, SyntaxKind::CASE_KW);
    expect_and_skip(p, SyntaxKind::DEF_KW);

    // Identification
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

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

/// Parse a KerML definition (class, struct, datatype, etc.)
/// These definitions don't use 'def' keyword like SysML definitions
/// Per pest: structure = { abstract? ~ struct_token ~ identification? ~ specializations ~ namespace_body }
fn parse_kerml_definition<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::DEFINITION);

    // Optional abstract prefix
    consume_if(p, SyntaxKind::ABSTRACT_KW);

    // KerML definition keyword (class, struct, datatype, etc.)
    if is_kerml_definition_keyword(p.current_kind()) {
        bump_keyword(p);
    }
    p.skip_trivia();

    // Identification (name)
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::LT) {
        p.parse_identification();
    }
    p.skip_trivia();

    // Specializations
    parse_specializations_with_skip(p);

    // Body
    p.parse_body();

    p.finish_node();
}

fn parse_usage<P: SysMLParser>(p: &mut P) {
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

fn parse_definition_keyword<P: SysMLParser>(p: &mut P) {
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

/// Dependency = 'dependency' (identification 'from' | 'from')? source (',' source)* 'to' target (',' target)*
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
fn parse_connector_end<P: SysMLParser>(p: &mut P) {
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

/// Parse multiplicity: [expression] or [lower..upper] or [lower..expr()]
/// Supports expressions including function calls as bounds
fn parse_multiplicity<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::MULTIPLICITY);
    p.expect(SyntaxKind::L_BRACKET);
    p.skip_trivia();

    // Parse multiplicity bounds - can be:
    // - Single value: [5], [*]
    // - Range: [0..*], [1..10], [0..size(items)]
    // - Expression: [size(items)]

    // Parse first bound (could be expression)
    if !p.at(SyntaxKind::R_BRACKET) {
        parse_multiplicity_bound(p);
        p.skip_trivia();

        // Check for range operator (..)
        if p.at(SyntaxKind::DOT_DOT) {
            p.bump();
            p.skip_trivia();

            // Parse upper bound (could be expression or *)
            if !p.at(SyntaxKind::R_BRACKET) {
                parse_multiplicity_bound(p);
                p.skip_trivia();
            }
        }
    }

    p.expect(SyntaxKind::R_BRACKET);
    p.finish_node();
}

/// Parse a single multiplicity bound (number, *, or expression including function calls)
/// Per spec: multiplicity_bound = { inline_expression | number | "*" }
/// Bounds are typed as Expression in the metamodel
fn parse_multiplicity_bound<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::STAR) {
        p.bump();
    } else if p.at(SyntaxKind::INTEGER) {
        // Integers are literals - parse as expression for consistency
        parse_expression(p);
    } else if p.at_name_token() || p.at(SyntaxKind::L_PAREN) {
        // Parse as full expression (handles identifiers, function calls, etc.)
        parse_expression(p);
    }
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

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

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

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    // Default value or assignment
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

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
    parse_specializations(p);
    p.skip_trivia();

    // Optional value assignment
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        if p.at(SyntaxKind::DEFAULT_KW) {
            p.bump();
            p.skip_trivia();
        }
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    }

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
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) || p.at(SyntaxKind::DEFAULT_KW) {
        if p.at(SyntaxKind::DEFAULT_KW) {
            p.bump();
            p.skip_trivia();
        }
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        // Parse expression after '=' or 'default' (default can omit '=')
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    }

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
fn parse_case_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "case definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Parse case body items: objective, subject, actor, case_calculation_body_item
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        if p.at(SyntaxKind::OBJECTIVE_KW) {
            parse_objective_member(p);
        } else if p.at(SyntaxKind::SUBJECT_KW) {
            parse_subject_member(p);
        } else if p.at(SyntaxKind::ACTOR_KW) {
            parse_actor_member(p);
        } else if p.at(SyntaxKind::STAKEHOLDER_KW) {
            parse_stakeholder_member(p);
        } else if p.at(SyntaxKind::RETURN_KW) {
            parse_sysml_parameter(p);
        } else if p.at(SyntaxKind::IDENT) {
            // Check if this looks like an expression (followed by operator) or a feature member
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);

            // If followed by expression operators, parse as expression
            if matches!(
                next,
                SyntaxKind::DOT
                    | SyntaxKind::COLON_COLON
                    | SyntaxKind::L_BRACKET
                    | SyntaxKind::L_PAREN
                    | SyntaxKind::PLUS
                    | SyntaxKind::MINUS
                    | SyntaxKind::STAR
                    | SyntaxKind::SLASH
                    | SyntaxKind::PERCENT
                    | SyntaxKind::GT
                    | SyntaxKind::LT
                    | SyntaxKind::EQ_EQ
                    | SyntaxKind::BANG_EQ
            ) {
                // Parse as expression (shared expression grammar from kerml_expressions.pest)
                parse_expression(p);
            } else {
                // Parse as package body element (feature member)
                parse_package_body_element(p);
            }
        } else {
            // Other case body items (calculation body, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body (for metadata definitions)
fn parse_metadata_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "metadata definition");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Metadata body can contain metadata_body_usage items
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        // Metadata body usage pattern: [ref] [:>>] identifier
        // Need to distinguish from other body elements
        let is_metadata_usage = if p.at(SyntaxKind::REF_KW) {
            // Check if ref is followed by :>> or identifier (not a usage keyword)
            let lookahead = skip_trivia_lookahead(p, 1);
            let next = p.peek_kind(lookahead);
            matches!(
                next,
                SyntaxKind::COLON_GT_GT
                    | SyntaxKind::COLON_GT
                    | SyntaxKind::REDEFINES_KW
                    | SyntaxKind::IDENT
            )
        } else if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::REDEFINES_KW) {
            // Starts with redefines operator
            true
        } else if p.at(SyntaxKind::IDENT) {
            // Just an identifier - could be metadata usage or other element
            // In metadata body, bare identifiers are metadata body usages
            true
        } else {
            false
        };

        if is_metadata_usage {
            parse_metadata_body_usage(p);
        } else {
            // Other body elements (imports, relationships, annotations)
            parse_package_body_element(p);
        }
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

// Parse metadata body usage: ref? :>>? identifier typing? specializations? default? meta? value? body
// Handles patterns like:
//   - `ref :>> annotatedElement : SysML::Usage;`
//   - `:>> baseType default global_sd meta SysML::PortUsage;`
fn parse_metadata_body_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    // Optional 'ref'
    if p.at(SyntaxKind::REF_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Optional redefines operator - wrap in SPECIALIZATION node for AST extraction
    if p.at(SyntaxKind::COLON_GT_GT) || p.at(SyntaxKind::COLON_GT) || p.at(SyntaxKind::REDEFINES_KW)
    {
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.bump(); // :>> or :> or redefines
        p.skip_trivia();

        // Required identifier (as qualified name)
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
        p.finish_node(); // SPECIALIZATION
    } else {
        // No redefines - parse name directly as NAME node for hoverable symbol
        if p.at(SyntaxKind::IDENT) {
            p.start_node(SyntaxKind::NAME);
            p.bump();
            p.finish_node();
            p.skip_trivia();
        } else {
            p.error("expected identifier in metadata body usage");
        }
    }

    // Optional typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Optional specializations
    parse_specializations(p);
    p.skip_trivia();

    // Optional 'default' clause with expression
    // Pattern: `default <expression>`
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump(); // default
        p.skip_trivia();
        // The default value is an expression (usually an identifier reference)
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }

    // Optional 'meta' clause with type reference
    // Pattern: `meta <qualified_name>`
    // We use TYPING node to wrap this since it functions similarly
    if p.at(SyntaxKind::META_KW) {
        p.start_node(SyntaxKind::TYPING);
        p.bump(); // meta
        p.skip_trivia();
        if p.at_name_token() {
            p.parse_qualified_name();
            p.skip_trivia();
        }
        p.finish_node();
    }

    // Optional value (= expression or 'as' cast)
    if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body (semicolon or nested metadata body)
    if p.at(SyntaxKind::L_BRACE) {
        parse_metadata_body(p);
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse objective member: 'objective' [name] ':' type [:>> ref, ...] body
fn parse_objective_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::OBJECTIVE_USAGE);
    p.bump(); // objective
    p.skip_trivia();

    // Optional identifier (wrapped in NAME)
    parse_optional_identification(p);

    // Optional typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Specializations (per pest: constraint_usage_declaration includes usage_declaration)
    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Body (requirement body)
    parse_requirement_body(p);

    p.finish_node();
}

// Parse subject member: 'subject' usage_declaration
fn parse_subject_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUBJECT_USAGE);
    p.bump(); // subject
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME, typing, etc.)
    parse_optional_identification(p);

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        parse_multiplicity(p);
        p.skip_trivia();
    }

    // Typing
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // Specializations
    parse_specializations(p);
    p.skip_trivia();

    // Default value or assignment
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse actor member: 'actor' usage_declaration
fn parse_actor_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACTOR_USAGE);
    p.bump(); // actor
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME)
    parse_optional_identification(p);

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    // Multiplicity
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    // Default value
    if p.at(SyntaxKind::DEFAULT_KW) {
        p.bump();
        p.skip_trivia();
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            p.bump();
            p.skip_trivia();
        }
        if p.can_start_expression() {
            parse_expression(p);
        }
        p.skip_trivia();
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse stakeholder member: 'stakeholder' usage_declaration
fn parse_stakeholder_member<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STAKEHOLDER_USAGE);
    p.bump(); // stakeholder
    p.skip_trivia();

    // Usage declaration (identifier wrapped in NAME)
    parse_optional_identification(p);

    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    parse_specializations(p);
    p.skip_trivia();

    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

// Parse requirement body (for objective members and requirements)
fn parse_requirement_body<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        error_missing_body_terminator(p, "requirement");
        return;
    }

    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.bump(); // {
    p.skip_trivia();

    // Requirement body can contain definition_body_items, subject members, constraints, etc.
    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_package_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node();
}

/// Parse calc body for SysML (extends KerML calc body with behavior usages)
/// Per pest: calculation_body_item includes behavior_usage_member (perform, send, etc.)
/// and result_expression_member (final expression without semicolon)
pub fn parse_sysml_calc_body<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at(SyntaxKind::L_BRACE) {
        p.bump();
        p.skip_trivia();

        while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
            let start_pos = p.get_pos();

            // Parameters (in, out) - treat as SysML usages to handle ref prefix
            if p.at_any(&[SyntaxKind::IN_KW, SyntaxKind::OUT_KW, SyntaxKind::INOUT_KW]) {
                parse_usage(p);
            }
            // RETURN_KW can be either a return parameter or return expression
            else if p.at(SyntaxKind::RETURN_KW) {
                // Look ahead to distinguish: return <name>? : ... vs return <expr>
                let lookahead = skip_trivia_lookahead(p, 1);
                let after_return = p.peek_kind(lookahead);

                // If return is followed directly by colon, it's a parameter: return : Type
                if after_return == SyntaxKind::COLON || after_return == SyntaxKind::TYPED_KW {
                    parse_usage(p);
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
                        parse_usage(p);
                    } else {
                        // return expression statement
                        parse_return_expression(p);
                    }
                } else if is_usage_keyword(after_return) {
                    // return part x; or return attribute y;
                    parse_usage(p);
                } else {
                    // return expression statement
                    parse_return_expression(p);
                }
            }
            // Behavior usages (perform, send, accept, etc.)
            else if p.at(SyntaxKind::PERFORM_KW) {
                parse_perform_action(p);
            } else if p.at(SyntaxKind::SEND_KW) {
                parse_send_action(p);
            } else if p.at(SyntaxKind::ACCEPT_KW) {
                parse_accept_action(p);
            }
            // General namespace elements (definitions, usages, etc.)
            else if p.at_any(&[
                SyntaxKind::ATTRIBUTE_KW,
                SyntaxKind::PART_KW,
                SyntaxKind::ITEM_KW,
                SyntaxKind::CALC_KW,
                SyntaxKind::CONSTRAINT_KW,
                SyntaxKind::ACTION_KW,
                SyntaxKind::DOC_KW,
                SyntaxKind::COMMENT_KW,
                SyntaxKind::PRIVATE_KW,
                SyntaxKind::PUBLIC_KW,
                SyntaxKind::PROTECTED_KW,
            ]) {
                parse_package_body_element(p);
            }
            // Result expression (identifier, new, literal, or any expression start)
            // Per sysml.pest: calculation_body_item includes result_expression_member
            else if p.can_start_expression() {
                parse_expression(p);
                p.skip_trivia();
                // Optional semicolon for expression statements
                if p.at(SyntaxKind::SEMICOLON) {
                    p.bump();
                }
            } else {
                parse_package_body_element(p);
            }

            p.skip_trivia();

            if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
                let got = if let Some(text) = p.current_token_text() {
                    format!("'{}'", text)
                } else {
                    p.current_kind().display_name().to_string()
                };
                p.error(format!("unexpected {} in calc body", got));
                p.bump();
            }
        }

        p.expect(SyntaxKind::R_BRACE);
    } else {
        error_missing_body_terminator(p, "calc definition");
    }

    p.finish_node();
}

/// Parse flow usage (SysML-specific)\n/// Pattern: [succession] flow [name] [of Type] [from X] to Y [body]\n/// Per pest: flow_connection_usage = { succession_flow_connection_usage | item_flow }\n/// Per pest: item_flow = { \"flow\" ~ (item_flow_end ~ \"to\" ~ item_flow_end | \"of\" ~ item_feature ~ item_flow_end?) }\n/// Per pest: succession_flow_connection_usage = { \"succession\" ~ \"flow\" ~ (succession_item_flow | flow_usage_declaration ~ succession_flow_connection_block) }\n/// Pattern: [succession] flow [all] [<name>|of <type>] [from <source>] to <target> <body|semicolon>
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
