use super::*;

// =============================================================================
// State Body Elements
// =============================================================================

/// StateSubaction = ('entry' | 'do' | 'exit') Identification? Body\n/// Per pest: entry_transition_member = { \"entry\" ~ (entry_transition_member_declaration|semi_colon) }\n/// Per pest: do_behavior_member = { \"do\" ~ (behavior_usage_member_declaration|semi_colon) }\n/// Per pest: exit_transition_member = { \"exit\" ~ (exit_transition_member_declaration|semi_colon) }\n/// Pattern: entry|do|exit [assign|send|accept|action|<name>] [body|semicolon]
pub fn parse_state_subaction<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::STATE_SUBACTION);

    // entry/do/exit keyword
    bump_keyword(p);

    // State action usage can be:
    // - assignment: assign target := expr ;
    // - send: send expr [via expr] [to expr] ;
    // - accept: accept ...
    // - action: action [name] { ... }
    // - identifier [body or ;]
    // - qualified_name ;
    // - ;

    if p.at(SyntaxKind::ASSIGN_KW) {
        parse_assign_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::ACTION_KW) {
        // action [name] [: Type] [:>> ref, ...] body
        // Per pest: action_keyword ~ (identifier ~ semi_colon | usage_declaration? ~ action_body)
        // where usage_declaration includes typing and specializations
        p.bump(); // action
        p.skip_trivia();
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
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

        // Multiplicity (rarely, but per spec)
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        parse_action_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
    } else if p.at_name_token() {
        // Could be: identifier ; or identifier [: Type] [:> ref, ...] { ... } or semicolon
        // Pattern: do myAction : ActionType { ... }
        p.parse_identification();
        p.skip_trivia();

        // Optional typing (: Type)
        if p.at(SyntaxKind::COLON) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Optional specializations (:>, :>>, etc.)
        parse_specializations(p);
        p.skip_trivia();

        // Optional multiplicity
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::L_BRACE) {
            parse_action_body(p);
        } else if p.at(SyntaxKind::SEMICOLON) {
            p.bump();
        }
        // else: no body, no semicolon - might be valid shorthand
    }

    p.finish_node();
}

/// TransitionUsage per Pest grammar:
/// transition_usage = transition_usage_keyword
///   ~ (usage_declaration ~ (first_token ~ transition_source_member | transition_source_member)
///     | first_token ~ transition_source_member
///     | transition_source_member)
///   ~ empty_parameter_member
///   ~ (empty_parameter_member ~ trigger_action_member)?  // accept trigger
///   ~ guard_expression_member?                          // if guard
///   ~ effect_behavior_member?                           // do effect
///   ~ then_token ~ transition_succession_member
///   ~ action_body
/// Per pest: transition_usage = { (transition_usage_declaration | first_node) ~ transition_succession_block }
/// Per pest: transition_succession = { succession_as_usage | transition_feature_membership }
/// Pattern: transition [name] [first] <source>? accept [trigger] [if guard] [do effect] then <target> body
pub fn parse_transition<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE so it gets extracted by NamespaceMember::cast
    p.start_node(SyntaxKind::USAGE);
    p.start_node(SyntaxKind::TRANSITION_USAGE);

    p.expect(SyntaxKind::TRANSITION_KW);
    p.skip_trivia();

    // Optional usage declaration (transition name)
    // Per pest: usage_declaration ~ (first_token ~ transition_source_member | transition_source_member)
    // Heuristic: if we see a name that's NOT 'first', and peek shows 'first' or newline after it, it's a name
    if p.at_name_token() && !p.at(SyntaxKind::FIRST_KW) {
        // Check if next token (after skipping this name) is 'first'
        // If so, this is a transition name, not the source
        let is_transition_name = p.peek_kind(1) == SyntaxKind::FIRST_KW
            || p.peek_kind(1) == SyntaxKind::WHITESPACE && p.peek_kind(2) == SyntaxKind::FIRST_KW;

        if is_transition_name {
            p.parse_identification();
            p.skip_trivia();
        }
    }

    // Optional 'first' keyword
    if p.at(SyntaxKind::FIRST_KW) {
        bump_keyword(p);
    }

    // Source state (transition_source_member) - wrap in SPECIALIZATION for type_ref extraction
    if p.at_name_token()
        && !p.at(SyntaxKind::ACCEPT_KW)
        && !p.at(SyntaxKind::IF_KW)
        && !p.at(SyntaxKind::DO_KW)
        && !p.at(SyntaxKind::THEN_KW)
    {
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();
    }

    // Optional trigger: accept <payload> [at/after/when <expr>] [via <port>]
    if p.at(SyntaxKind::ACCEPT_KW) {
        p.bump(); // accept
        p.skip_trivia();

        // Payload name (but not if it's a trigger keyword)
        if (p.at_name_token() || p.at(SyntaxKind::LT))
            && !p.at(SyntaxKind::AT_KW)
            && !p.at(SyntaxKind::AFTER_KW)
            && !p.at(SyntaxKind::WHEN_KW)
            && !p.at(SyntaxKind::VIA_KW)
        {
            p.parse_identification();
            p.skip_trivia();
        }

        // Optional typing
        if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) {
            p.parse_typing();
            p.skip_trivia();
        }

        // Optional trigger expression (at/after/when)
        if p.at(SyntaxKind::AT_KW) || p.at(SyntaxKind::AFTER_KW) || p.at(SyntaxKind::WHEN_KW) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }

        // Optional via
        if p.at(SyntaxKind::VIA_KW) {
            p.bump();
            p.skip_trivia();
            p.parse_qualified_name();
            p.skip_trivia();
        }
    }

    // Optional guard: if <expression>
    if consume_if(p, SyntaxKind::IF_KW) {
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional effect: do <action>
    if consume_if(p, SyntaxKind::DO_KW) {
        // Effect can be a performed action, send, accept, or assignment
        // NOTE: In transition context, these don't have semicolons - the semicolon comes after 'then'
        if p.at(SyntaxKind::SEND_KW) {
            parse_inline_send_action(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
            // parse_accept_action handles no-semicolon case already
            parse_accept_action(p);
        } else if p.at(SyntaxKind::ASSIGN_KW) {
            bump_keyword(p);
            p.parse_qualified_name();
            p.skip_trivia();
            if p.at(SyntaxKind::COLON_EQ) {
                bump_keyword(p);
                parse_expression(p);
            }
        } else if p.at(SyntaxKind::ACTION_KW) {
            parse_inline_action(p);
        } else if p.at_name_token() {
            // Typed reference (action name)
            p.parse_qualified_name();
        }
        p.skip_trivia();
    }

    // 'then' target state - wrap in SPECIALIZATION for type_ref extraction
    if p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.start_node(SyntaxKind::SPECIALIZATION);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
    }

    p.finish_node(); // TRANSITION_USAGE

    // Body (action_body)
    p.parse_body();

    p.finish_node(); // USAGE
}
