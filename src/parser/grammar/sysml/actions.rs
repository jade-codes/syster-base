use super::*;

// =============================================================================
// Action Body Elements
// =============================================================================

// tag::parse_perform_action[]
/// Parse perform action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_perform_action
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
// end::parse_perform_action[]

// tag::parse_frame_usage[]
/// Parse frame usage
/// Pattern: 'frame' [<keyword>] <name> ';'
/// Grammar: see docs/grammar-mapping.adoc#parse_frame_usage
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
// end::parse_frame_usage[]

// tag::parse_render_usage[]
/// Parse render usage
/// Pattern: 'render' [<keyword>] <name> [: Type] [multiplicity] ';'
/// Grammar: see docs/grammar-mapping.adoc#parse_render_usage
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
// end::parse_render_usage[]

// tag::parse_accept_action[]
/// Parse accept action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_accept_action
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
// end::parse_accept_action[]

// tag::parse_send_action[]
/// Parse send action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_send_action
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

    // Check for a bare via/to clause with no payload expression.
    // When via/to appears directly, skip the expression parsing
    if !p.at(SyntaxKind::VIA_KW) && !p.at(SyntaxKind::TO_KW) && p.can_start_expression() {
        // What to send
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
// end::parse_send_action[]

// tag::parse_if_action[]
/// Parse if action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_if_action
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

        // Optional 'else' or 'else if'
        if consume_if(p, SyntaxKind::ELSE_KW) {
            if p.at(SyntaxKind::IF_KW) {
                // Chained else-if: else if <expr> { ... } else { ... }
                parse_if_action(p);
            } else {
                p.parse_body();
            }
        }
    }

    p.finish_node();
}
// end::parse_if_action[]

// tag::parse_loop_action[]
/// Parse loop/while action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_loop_action
pub fn parse_loop_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::WHILE_LOOP_ACTION_USAGE);

    // 'while' or 'loop'
    let is_while = p.at(SyntaxKind::WHILE_KW);
    bump_keyword(p);

    // Optional condition for 'while' only — 'loop' has no pre-condition
    // (its body is followed by an optional 'until' post-condition)
    if is_while && p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    // Use action body since loop bodies contain action statements (assign, if, etc.)
    parse_action_body(p);

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
// end::parse_loop_action[]

// tag::parse_for_loop[]
/// Parse for loop action usage
/// Grammar: see docs/grammar-mapping.adoc#parse_for_loop
pub fn parse_for_loop<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::FOR_LOOP_ACTION_USAGE);

    expect_and_skip(p, SyntaxKind::FOR_KW);

    // Loop variable: name [: Type]
    parse_optional_identification(p);

    // Optional typing for loop variable: for n : Integer in ...
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }

    // 'in' keyword
    if p.at(SyntaxKind::IN_KW) {
        bump_keyword(p);
    }

    // Collection expression
    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    // Use action body since for-loop bodies contain action statements
    parse_action_body(p);

    p.finish_node();
}
// end::parse_for_loop[]

// tag::parse_first_action[]
/// Parse first action usage (initial succession)
/// Pattern: 'first' [mult]? TargetRef ('then' [mult]? TargetRef)? (';' | '{' '}')
/// Grammar: see docs/grammar-mapping.adoc#parse_first_action
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
// end::parse_first_action[]

// tag::parse_then_succession[]
/// Parse then succession
/// Pattern: 'then' TargetRef ';'
/// Grammar: see docs/grammar-mapping.adoc#parse_then_succession
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
// end::parse_then_succession[]

// tag::parse_terminate_action[]
/// Parse terminate action
/// Pattern: terminate [<target>] ;
/// Grammar: see docs/grammar-mapping.adoc#parse_terminate_action
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
// end::parse_terminate_action[]

// tag::parse_else_succession[]
/// Parse else succession (default target succession)
/// Pattern: else <target>;
/// Grammar: see docs/grammar-mapping.adoc#parse_else_succession
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
// end::parse_else_succession[]

// tag::parse_control_node[]
/// Parse control node (fork, join, merge, decide)
///
/// Per the official SysML v2 KEBNF grammar, a control node is one of four
/// kinds sharing a common `ControlNodePrefix`:
///
///   ControlNode = MergeNode | DecisionNode | JoinNode | ForkNode
///   ControlNodePrefix : OccurrenceUsage =
///       RefPrefix ( isIndividual ?= 'individual' )?
///       ( portionKind = PortionKind { isPortion = true } )?
///       UsageExtensionKeyword*
///   MergeNode    = ControlNodePrefix isComposite ?= 'merge'  UsageDeclaration ActionBody
///   DecisionNode = ControlNodePrefix isComposite ?= 'decide' UsageDeclaration ActionBody
///   JoinNode     = ControlNodePrefix isComposite ?= 'join'   UsageDeclaration ActionBody
///   ForkNode     = ControlNodePrefix isComposite ?= 'fork'   UsageDeclaration ActionBody
///
/// Pattern: ('fork' | 'join' | 'merge' | 'decide') Identification? Body
/// Grammar: see docs/grammar-mapping.adoc#parse_control_node
pub fn parse_control_node<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::CONTROL_NODE);

    // Control keyword
    bump_keyword(p);

    // Optional name
    parse_optional_identification(p);

    p.parse_body();

    p.finish_node();
}
// end::parse_control_node[]

// tag::parse_action_body[]
/// Parse action body (for action definitions and action usages)
/// Grammar: see docs/grammar-mapping.adoc#parse_action_body
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
// end::parse_action_body[]

// tag::parse_state_body[]
/// Parse state body (for state usages)
/// Pattern: ";" | parallel? "{" state_body_part "}"
/// Grammar: see docs/grammar-mapping.adoc#parse_state_body
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
// end::parse_state_body[]

// tag::parse_state_body_element[]
/// Parse a state body element
/// This means after accept/action/etc., we can have "then target;" transitions
/// BUT: entry/do/exit subactions are standalone and don't have transitions after
/// Grammar: see docs/grammar-mapping.adoc#parse_state_body_element
fn parse_state_body_element<P: SysMLParser>(p: &mut P) {
    // Check if this is a standalone state subaction (entry/do/exit)
    // These are complete statements and should NOT be followed by target transitions
    let is_state_subaction =
        p.at(SyntaxKind::ENTRY_KW) || p.at(SyntaxKind::DO_KW) || p.at(SyntaxKind::EXIT_KW);

    // Parse the main element (could be transition, state, accept, do, etc.)
    parse_package_body_element(p);
    p.skip_trivia();

    // Only check for target transitions if this was NOT a state subaction
    // State subactions (entry/do/exit) are standalone, not followed by a transition
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
// end::parse_state_body_element[]

// tag::parse_target_transition[]
/// Parse target transition usage
/// This handles: [accept ...] [if expr] [do action] then target;
/// Pattern: [accept <trigger>] [if <guard>] [do <effect>] then <target> [body]
/// Grammar: see docs/grammar-mapping.adoc#parse_target_transition
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
// end::parse_target_transition[]
