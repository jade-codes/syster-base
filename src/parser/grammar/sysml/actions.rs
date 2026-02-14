use super::*;

// =============================================================================
// Action Body Elements
// =============================================================================

/// Parse perform action usage
/// Per pest: perform_action_usage = { perform_token ~ perform_action_usage_declaration ~ action_body }
/// Per pest: perform_action_usage_declaration = { (action_declaration_header | qualified_name) ~ feature_specialization_part? }
/// Per pest: action_declaration_header = { action_token ~ usage_declaration? }
/// Per pest: usage_declaration = { identification ~ multiplicity_part? ~ feature_specialization_part }
pub fn parse_perform_action<P: SysMLParser>(p: &mut P) {
    // Wrap in USAGE so it's recognized as a NamespaceMember
    p.start_node(SyntaxKind::USAGE);
    p.start_node(SyntaxKind::PERFORM_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::PERFORM_KW);

    // Check if followed by 'action' keyword (action_declaration_header)
    if p.at(SyntaxKind::ACTION_KW) {
        bump_keyword(p); // consume 'action'

        // Parse optional usage_declaration (identification, multiplicity, specialization_part)
        parse_optional_identification(p);

        // Optional multiplicity [*], [1..*], etc.
        parse_optional_multiplicity(p);

        // Optional specializations
        parse_specializations_with_skip(p);
    } else {
        // Otherwise just a qualified name - parse as specialization for relationship extraction
        p.start_node(SyntaxKind::SPECIALIZATION);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        // Optional specializations (redefines, subsets, etc.)
        parse_specializations(p);
    }

    p.skip_trivia();
    p.parse_body();

    p.finish_node(); // PERFORM_ACTION_USAGE
    p.finish_node(); // USAGE
}

/// Parse frame usage
/// Per pest: Frame usage for requirement framing
/// Pattern: 'frame' [<keyword>] <name> ';'
pub fn parse_frame_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    expect_and_skip(p, SyntaxKind::FRAME_KW);

    // Check if followed by usage keyword (e.g., frame concern c1)
    if p.at_any(SYSML_USAGE_KEYWORDS) {
        bump_keyword(p);
    }

    // Parse identification
    parse_optional_identification(p);

    // Specializations
    parse_specializations_with_skip(p);

    p.parse_body();

    p.finish_node();
}

/// Parse render usage
/// Per pest: view_rendering_usage = { render_token ~ (rendering_usage_keyword ~ usage_declaration)? ~ semi_colon }
/// Pattern: 'render' [<keyword>] <name> [: Type] [multiplicity] ';'
pub fn parse_render_usage<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);

    p.expect(SyntaxKind::RENDER_KW);
    p.skip_trivia();

    // Check if followed by usage keyword (e.g., render rendering r1)
    if p.at_any(SYSML_USAGE_KEYWORDS) {
        p.bump();
        p.skip_trivia();
    }

    // Parse identification
    parse_optional_identification(p);

    // Typing
    parse_optional_typing(p);

    // Multiplicity
    parse_optional_multiplicity(p);

    // Specializations
    parse_specializations_with_skip(p);

    p.parse_body();

    p.finish_node();
}

/// Parse accept action usage
/// Per pest: accept_node = { occurrence_usage_prefix ~ accept_node_declaration ~ action_body }
/// Per pest: accept_node_declaration = { action_node_usage_declaration? ~ accept_token ~ accept_parameter_part }
/// Per pest: accept_parameter_part = { payload_parameter_member ~ (via_token ~ node_parameter_member)? }
/// Per pest: payload_parameter = { (identification? ~ payload_feature_specialization_part? ~ trigger_value_part) | payload }
/// Per pest: trigger_expression = { time_trigger_kind ~ argument_member | change_trigger_kind ~ argument_expression_member }
pub fn parse_accept_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::ACCEPT_ACTION_USAGE);

    // Optional 'action' keyword before 'accept'
    if p.at(SyntaxKind::ACTION_KW) {
        bump_keyword(p);
        // Optional name after 'action'
        if p.at_name_token() || p.at(SyntaxKind::LT) {
            p.parse_identification();
            p.skip_trivia();
        }
    }

    expect_and_skip(p, SyntaxKind::ACCEPT_KW);

    // Check for 'via' first (accept via port pattern - no payload)
    // Otherwise parse optional payload and trigger
    if !p.at(SyntaxKind::VIA_KW) {
        parse_accept_trigger(p);
    } else {
        // Just parse via port
        bump_keyword(p);
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Body (optional if followed by 'then' transition in state context)
    // Also skip if followed by 'do' (effect) or 'if' (guard) in target transition context
    if p.at(SyntaxKind::THEN_KW) || p.at(SyntaxKind::DO_KW) || p.at(SyntaxKind::IF_KW) {
        // In state bodies, accept can be followed by target transition without a body
        // The transition will be parsed by parse_state_body_element
    } else {
        p.parse_body();
    }

    p.finish_node();
}

/// Parse send action usage
/// Per pest: send_node = { occurrence_usage_prefix ~ send_node_declaration ~ action_body }
/// Per pest: send_node_declaration = { action_node_usage_declaration? ~ send_token ~ (action_body | (node_parameter_member ~ sender_receiver_part? | empty_parameter_member ~ sender_receiver_part) ~ action_body) }
/// Per pest: node_parameter_member = { owned_expression }
/// Per pest: sender_receiver_part = { via_token ~ ... | empty_parameter_member ~ to_token ~ ... }
pub fn parse_send_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SEND_ACTION_USAGE);

    p.expect(SyntaxKind::SEND_KW);
    p.skip_trivia();

    // Check if we have a body directly (pattern: send { ... })
    if p.at(SyntaxKind::L_BRACE) || p.at(SyntaxKind::SEMICOLON) {
        parse_action_body(p);
        p.finish_node();
        return;
    }

    // Check for sender_receiver_part directly (empty parameter member pattern)
    // Per pest: sender_receiver_part = { via_token ~ ... | empty_parameter_member ~ to_token ~ ... }
    // When via/to appears directly, skip the expression parsing
    if !p.at(SyntaxKind::VIA_KW) && !p.at(SyntaxKind::TO_KW) && p.can_start_expression() {
        // What to send (node_parameter_member = owned_expression)
        parse_expression(p);
        p.skip_trivia();
    }

    parse_optional_via(p);
    parse_optional_to(p);

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        parse_action_body(p);
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse if action usage
/// Per pest: if_node = { occurrence_usage_prefix ~ if_node_parameter_member ~ action_body ~ (else_token ~ action_body_parameter)? }
/// Per pest: if_node_parameter_member = { if_token ~ argument_expression_member ~ (then_token? ~ target_succession_member)? }
pub fn parse_if_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::IF_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::IF_KW);

    // Condition (parenthesized or not)
    if p.at(SyntaxKind::L_PAREN) {
        bump_keyword(p);
        parse_expression(p);
        p.skip_trivia();
        p.expect(SyntaxKind::R_PAREN);
    } else if p.can_start_expression() {
        parse_expression(p);
    }

    p.skip_trivia();

    // Check for 'then' keyword - this is a guarded succession, not a full if-action
    if p.at(SyntaxKind::THEN_KW) {
        // Pattern: if <expr> then <target>;
        bump_keyword(p); // then

        // Target (qualified name or inline action)
        if p.at(SyntaxKind::MERGE_KW)
            || p.at(SyntaxKind::DECIDE_KW)
            || p.at(SyntaxKind::JOIN_KW)
            || p.at(SyntaxKind::FORK_KW)
        {
            parse_control_node(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
            parse_accept_action(p);
        } else if p.at(SyntaxKind::SEND_KW) {
            parse_send_action(p);
        } else {
            p.parse_qualified_name();
            p.skip_trivia();
            expect_and_skip(p, SyntaxKind::SEMICOLON);
        }
    } else {
        // Standard if-action with body
        p.parse_body();

        p.skip_trivia();

        // Optional 'else'
        if consume_if(p, SyntaxKind::ELSE_KW) {
            p.parse_body();
        }
    }

    p.finish_node();
}

/// Parse loop/while action usage
/// Per pest: while_loop_node = { occurrence_usage_prefix ~ (while_token ~ argument_expression_member? | loop_token) ~ action_body ~ (until_token ~ argument_expression_member ~ semi_colon)? }
pub fn parse_loop_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::WHILE_LOOP_ACTION_USAGE);

    // 'while' or 'loop'
    bump_keyword(p);

    // Optional condition for 'while'
    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    p.parse_body();

    p.skip_trivia();

    // Optional 'until'
    if p.at(SyntaxKind::UNTIL_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse for loop action usage
/// Per pest: for_loop_node = { occurrence_usage_prefix ~ for_token ~ for_variable_declaration_member ~ in_token ~ node_parameter_member ~ action_body }
/// Per pest: for_variable_declaration_member = { for_variable_declaration }
/// Per pest: for_variable_declaration = { identification? }
pub fn parse_for_loop<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::FOR_LOOP_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::FOR_KW);

    // Loop variable
    parse_optional_identification(p);

    // 'in' keyword
    if p.at(SyntaxKind::IN_KW) {
        bump_keyword(p);
    }

    // Collection expression
    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    p.parse_body();

    p.finish_node();
}

/// Parse first action usage (initial succession)
/// Per pest: empty_succession = { first_token ~ empty_succession_member ~ (then_token ~ empty_succession_member)? ~ semi_colon }
/// Pattern: 'first' [mult]? TargetRef ('then' [mult]? TargetRef)? (';' | '{' '}')
pub fn parse_first_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::FIRST_KW);

    // First endpoint wrapped in SUCCESSION_ITEM
    p.start_node(SyntaxKind::SUCCESSION_ITEM);

    // Optional multiplicity before first endpoint
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }

    p.parse_qualified_name();
    p.finish_node(); // SUCCESSION_ITEM
    p.skip_trivia();

    // Optional 'then' clause
    if p.at(SyntaxKind::THEN_KW) {
        p.bump();
        p.skip_trivia();

        // Second endpoint wrapped in SUCCESSION_ITEM
        p.start_node(SyntaxKind::SUCCESSION_ITEM);

        // Optional multiplicity before second endpoint
        if p.at(SyntaxKind::L_BRACKET) {
            p.parse_multiplicity();
            p.skip_trivia();
        }

        p.parse_qualified_name();
        p.finish_node(); // SUCCESSION_ITEM
        p.skip_trivia();
    }

    // Body (semicolon or braces)
    p.parse_body();

    p.finish_node();
}

/// Parse then succession
/// Per pest: action_target_succession = { target_succession | guarded_target_succession | default_target_succession }
/// Per pest: target_succession = { empty_succession_member ~ then_token ~ target_succession_member ~ usage_body }
/// Pattern: 'then' TargetRef ';'
pub fn parse_then_succession<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::THEN_KW);

    // After 'then', we can have:
    // 1. A control node: merge m;, decide;, join j;, fork f;
    // 2. An action node: accept ..., send ..., etc.
    // 3. An inline action: action <name> {...}
    // 4. An inline state: state <name> {...}
    // 5. A qualified name reference: then someAction;

    // Check for inline action or state: then action/state <name> {...}
    // Also handle prefixed actions and assign actions
    if p.at(SyntaxKind::ACTION_KW)
        || p.at(SyntaxKind::STATE_KW)
        || p.at(SyntaxKind::PRIVATE_KW)
        || p.at(SyntaxKind::PROTECTED_KW)
        || p.at(SyntaxKind::ABSTRACT_KW)
        || p.at(SyntaxKind::READONLY_KW)
        || p.at(SyntaxKind::DERIVED_KW)
        || p.at(SyntaxKind::ASSIGN_KW)
    {
        parse_package_body_element(p);
        p.skip_trivia();

        // After inline action/state, check for additional target successions (then X, then Y)
        // Per pest grammar: behavior_usage_member ~ target_succession_member*
        while p.at(SyntaxKind::THEN_KW) {
            bump_keyword(p); // then

            // Parse the target (send/accept/action/etc.)
            // NOTE: In succession chaining, semicolon comes after entire chain, not after each target
            if p.at(SyntaxKind::ACTION_KW) || p.at(SyntaxKind::STATE_KW) {
                // Inline action or state in chain: then action name;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::PRIVATE_KW)
                || p.at(SyntaxKind::PROTECTED_KW)
                || p.at(SyntaxKind::ABSTRACT_KW)
                || p.at(SyntaxKind::READONLY_KW)
                || p.at(SyntaxKind::DERIVED_KW)
            {
                // Prefix keyword followed by action: then private action name;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::ASSIGN_KW) {
                // Inline assign action: then assign x := y;
                parse_package_body_element(p);
            } else if p.at(SyntaxKind::SEND_KW) {
                // Parse send inline without semicolon expectation
                parse_inline_send_action(p);
            } else if p.at(SyntaxKind::ACCEPT_KW) {
                parse_accept_action(p);
            } else if p.at(SyntaxKind::PERFORM_KW) {
                parse_perform_action(p);
            } else if p.at_name_token() {
                p.start_node(SyntaxKind::SUCCESSION_ITEM);
                p.parse_qualified_name();
                p.finish_node();
                p.skip_trivia();
                p.expect(SyntaxKind::SEMICOLON);
            }
            p.skip_trivia();
        }
        // After the succession chain, expect a semicolon
        // (The semicolon comes after the entire chain, not after each element)
        if !p.at(SyntaxKind::SEMICOLON) {
            // Already consumed by the last element
        } else {
            p.expect(SyntaxKind::SEMICOLON);
        }
    }
    // Check for event occurrence: then event occurrence <name>
    else if p.at(SyntaxKind::EVENT_KW) {
        parse_package_body_element(p);
        p.skip_trivia();
    }
    // Check for control node keywords
    else if p.at(SyntaxKind::MERGE_KW)
        || p.at(SyntaxKind::DECIDE_KW)
        || p.at(SyntaxKind::JOIN_KW)
        || p.at(SyntaxKind::FORK_KW)
    {
        parse_control_node(p);
    }
    // Check for action nodes
    else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else if p.at(SyntaxKind::PERFORM_KW) {
        parse_perform_action(p);
    } else if p.at(SyntaxKind::IF_KW) {
        parse_if_action(p);
    } else if p.at(SyntaxKind::WHILE_KW) || p.at(SyntaxKind::LOOP_KW) {
        parse_loop_action(p);
    } else if p.at(SyntaxKind::FOR_KW) {
        parse_for_loop(p);
    } else if p.at(SyntaxKind::TERMINATE_KW) {
        bump_keyword(p); // terminate
        // Optional target name
        if p.at_name_token() {
            p.start_node(SyntaxKind::SUCCESSION_ITEM);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }
        p.expect(SyntaxKind::SEMICOLON);
    }
    // Otherwise it's a reference - wrap in SUCCESSION_ITEM
    else {
        p.start_node(SyntaxKind::SUCCESSION_ITEM);
        p.parse_qualified_name();
        p.finish_node();
        p.skip_trivia();

        // Handle optional 'after' clause: then step2 after trigger1;
        // This creates a guarded succession where step2 happens after trigger1 completes
        if p.at(SyntaxKind::AFTER_KW) {
            p.bump(); // after
            p.skip_trivia();
            // Parse the event/trigger reference (can be a chain like step1.done)
            p.start_node(SyntaxKind::SUCCESSION_ITEM);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }

        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse terminate action
/// Per pest: terminate_node = { terminate_token ~ target_succession_member ~ semi_colon }
/// Pattern: terminate [<target>] ;
pub fn parse_terminate_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONTROL_NODE); // or create TERMINATE_ACTION_USAGE if needed

    expect_and_skip(p, SyntaxKind::TERMINATE_KW);

    // Optional target name
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    p.expect(SyntaxKind::SEMICOLON);

    p.finish_node();
}

/// Parse else succession (default target succession)
/// Per pest: default_target_succession = { else_token ~ target_succession_member ~ usage_body }
/// Pattern: else <target>;
pub fn parse_else_succession<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SUCCESSION);

    expect_and_skip(p, SyntaxKind::ELSE_KW);

    // Target (qualified name or inline action/control node)
    if p.at(SyntaxKind::MERGE_KW)
        || p.at(SyntaxKind::DECIDE_KW)
        || p.at(SyntaxKind::JOIN_KW)
        || p.at(SyntaxKind::FORK_KW)
    {
        parse_control_node(p);
    } else if p.at(SyntaxKind::ACCEPT_KW) {
        parse_accept_action(p);
    } else if p.at(SyntaxKind::SEND_KW) {
        parse_send_action(p);
    } else {
        p.parse_qualified_name();
        p.skip_trivia();
        p.expect(SyntaxKind::SEMICOLON);
    }

    p.finish_node();
}

/// Parse control node (fork, join, merge, decide)
/// Per pest: control_node = { control_node_prefix? ~ (merge_node | decision_node | join_node | fork_node) }
/// Per pest: merge_node = { merge_token ~ identification? ~ action_body }
/// Per pest: decision_node = { decide_token ~ identification? ~ action_body }
/// Per pest: join_node = { join_token ~ identification? ~ action_body }
/// Per pest: fork_node = { fork_token ~ identification? ~ action_body }
/// Pattern: ('fork' | 'join' | 'merge' | 'decide') Identification? Body
pub fn parse_control_node<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONTROL_NODE);

    // Control keyword
    bump_keyword(p);

    // Optional name
    parse_optional_identification(p);

    p.parse_body();

    p.finish_node();
}

/// Parse action body (for action definitions and action usages)
/// Per pest: action_body = { semi_colon | (forward_curl_brace ~ action_body_item* ~ backward_curl_brace) }
/// Per pest: action_body_item can include: directed_parameter_member, structure_usage_member, behavior_usage_member,
///           action_node_member, initial_node_member, etc.
pub fn parse_action_body<P: SysMLParser>(p: &mut P) {
    p.skip_trivia();

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    if !p.at(SyntaxKind::L_BRACE) {
        return;
    }

    // Start NAMESPACE_BODY node so members can be extracted
    p.start_node(SyntaxKind::NAMESPACE_BODY);
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_package_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);
    p.finish_node(); // NAMESPACE_BODY
}

/// Parse state body (for state usages)
/// Per pest: state_usage_body = { semi_colon | (parallel_marker? ~ forward_curl_brace ~ state_body_part ~ backward_curl_brace) }
/// Per pest: state_body_part = { state_body_item* }
/// Pattern: ";" | parallel? "{" state_body_part "}"
pub fn parse_state_body<P: SysMLParser>(p: &mut P) {
    p.skip_trivia();

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        return;
    }

    // Optional 'parallel' marker before body
    if p.at(SyntaxKind::PARALLEL_KW) {
        p.bump();
        p.skip_trivia();
    }

    if !p.at(SyntaxKind::L_BRACE) {
        return;
    }

    // Start NAMESPACE_BODY node so members can be extracted
    p.start_node(SyntaxKind::NAMESPACE_BODY);

    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    while !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::ERROR) {
        parse_state_body_element(p);
        p.skip_trivia();
    }

    p.expect(SyntaxKind::R_BRACE);

    p.finish_node(); // NAMESPACE_BODY
}

/// Parse a state body element
/// Per pest: state_body_item includes: entry_action_member, do_action_member, exit_action_member,
///           entry_transition_member, transition_usage_member, target_transition_usage_member,
///           behavior_usage_member, and more
/// Per pest: behavior_usage_member ~ target_transition_usage_member*
/// This means after accept/action/etc., we can have "then target;" transitions
/// BUT: entry/do/exit subactions are standalone and don't have transitions after
fn parse_state_body_element<P: SysMLParser>(p: &mut P) {
    // Check if this is a standalone state subaction (entry/do/exit)
    // These are complete statements and should NOT be followed by target transitions
    let is_state_subaction =
        p.at(SyntaxKind::ENTRY_KW) || p.at(SyntaxKind::DO_KW) || p.at(SyntaxKind::EXIT_KW);

    // Parse the main element (could be transition, state, accept, do, etc.)
    parse_package_body_element(p);
    p.skip_trivia();

    // Only check for target transitions if this was NOT a state subaction
    // State subactions (entry/do/exit) are standalone per the pest grammar
    if !is_state_subaction {
        // After behavior usages, check for target transitions
        // Target transitions can start with: accept, if, do, or then
        while p.at(SyntaxKind::THEN_KW)
            || p.at(SyntaxKind::ACCEPT_KW)
            || p.at(SyntaxKind::IF_KW)
            || p.at(SyntaxKind::DO_KW)
        {
            parse_target_transition(p);
            p.skip_trivia();
        }
    }
}

/// Parse target transition usage
/// Per pest grammar:
/// target_transition_usage = empty_parameter_member
///   ~ (transition_usage_keyword ~ ... | trigger_action_member ~ ... | guard_expression_member ~ ...)?\n///   ~ then_token ~ transition_succession_member ~ action_body\n/// This handles: [accept ...] [if expr] [do action] then target;\n/// Per pest: target_transition_usage = { target_transition_usage_declaration ~ transition_succession_block }\n/// Per pest: target_transition_usage_declaration = { (trigger_action_member? ~ guard_expression_member? ~ effect_behavior_member?)? ~ then_token ~ transition_succession_member }\n/// Pattern: [accept <trigger>] [if <guard>] [do <effect>] then <target> [body]
fn parse_target_transition<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::TRANSITION_USAGE);

    let has_prefix_keywords =
        p.at(SyntaxKind::ACCEPT_KW) || p.at(SyntaxKind::IF_KW) || p.at(SyntaxKind::DO_KW);

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
    if p.at(SyntaxKind::IF_KW) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional effect: do <action>
    if consume_if(p, SyntaxKind::DO_KW) {
        // Effect can be a performed action reference, send, accept, or assignment
        // NOTE: In target transition context, these don't have semicolons
        if p.at(SyntaxKind::SEND_KW) {
            parse_inline_send_action(p);
        } else if p.at(SyntaxKind::ACCEPT_KW) {
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

    // 'then' target is required per grammar
    // If we don't have it but we had prefix keywords, it's a malformed target transition
    // If we don't have prefix keywords and no THEN, this shouldn't have been called
    if !p.at(SyntaxKind::THEN_KW) {
        if has_prefix_keywords {
            p.error("expected 'then' after transition trigger/guard/effect");
        }
        // Finish early if malformed
        p.finish_node();
        return;
    }

    p.expect(SyntaxKind::THEN_KW);
    p.skip_trivia();

    // Optional 'state' keyword before target
    if p.at(SyntaxKind::STATE_KW) {
        p.bump();
        p.skip_trivia();
    }

    // Target (succession member - can be a state name or qualified name)
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Semicolon or body
    p.parse_body();

    p.finish_node();
}
