use super::*;

// =============================================================================
// Helper Functions
// =============================================================================

/// Emit an error for missing body terminator with context
pub(super) fn error_missing_body_terminator<P: SysMLParser>(p: &mut P, context: &str) {
    let found = if let Some(text) = p.current_token_text() {
        format!("'{}' ({})", text, p.current_kind().display_name())
    } else {
        "end of file".to_string()
    };
    p.error(format!(
        "expected ';' to end {} or '{{' to start body, found {}",
        context, found
    ));
}

/// Helper to consume a keyword and skip trivia in one call
pub(super) fn bump_keyword<P: SysMLParser>(p: &mut P) {
    p.bump();
    p.skip_trivia();
}

/// Helper to expect a token and skip trivia
pub(super) fn expect_and_skip<P: SysMLParser>(p: &mut P, kind: SyntaxKind) {
    p.expect(kind);
    p.skip_trivia();
}

/// Helper to check, bump, and skip trivia for a specific token
pub(super) fn consume_if<P: SysMLParser>(p: &mut P, kind: SyntaxKind) -> bool {
    if p.at(kind) {
        bump_keyword(p);
        true
    } else {
        false
    }
}

/// Helper to skip trivia in lookahead
pub(super) fn skip_trivia_lookahead<P: SysMLParser>(p: &P, mut lookahead: usize) -> usize {
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }
    lookahead
}

/// Helper to peek past a name (possibly qualified with ::) and check if the following token is `target`
/// This is used to distinguish between:
/// - `binding myName bind x = y` (myName is identification)
/// - `binding payload = target` (payload is bind source, not identification)
pub(super) fn peek_past_name_for<P: SysMLParser>(p: &P, target: SyntaxKind) -> bool {
    let mut lookahead = 0;
    lookahead = skip_trivia_lookahead(p, lookahead);

    // We know we're at a name token, skip it
    if p.peek_kind(lookahead) == SyntaxKind::IDENT {
        lookahead += 1;
    } else {
        return false;
    }

    // Handle qualified names (A::B::C) and dotted chains (a.b.c)
    loop {
        lookahead = skip_trivia_lookahead(p, lookahead);
        let next = p.peek_kind(lookahead);

        if next == SyntaxKind::COLON_COLON || next == SyntaxKind::DOT {
            lookahead += 1;
            lookahead = skip_trivia_lookahead(p, lookahead);
            if p.peek_kind(lookahead) == SyntaxKind::IDENT {
                lookahead += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    lookahead = skip_trivia_lookahead(p, lookahead);
    p.peek_kind(lookahead) == target
}

/// Helper to peek past optional identifier and get next significant token
pub(super) fn peek_past_optional_name<P: SysMLParser>(p: &P, mut lookahead: usize) -> (usize, SyntaxKind) {
    lookahead = skip_trivia_lookahead(p, lookahead);
    let mut next = p.peek_kind(lookahead);
    if next == SyntaxKind::IDENT {
        lookahead += 1;
        lookahead = skip_trivia_lookahead(p, lookahead);
        next = p.peek_kind(lookahead);
    }
    (lookahead, next)
}

/// Helper to parse optional identification
pub(super) fn parse_optional_identification<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() || p.at(SyntaxKind::LT) {
        p.parse_identification();
        p.skip_trivia();
    }
}

/// Helper to parse optional qualified name
pub(super) fn parse_optional_qualified_name<P: SysMLParser>(p: &mut P) {
    if p.at_name_token() {
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Helper to parse qualified name and skip trivia
pub(super) fn parse_qualified_name_and_skip<P: SysMLParser>(p: &mut P) {
    p.parse_qualified_name();
    p.skip_trivia();
}

/// SysML-specific identification parsing.
/// Identification = '<' ShortName '>' Name? | Name
///
/// This is separate from KerML's parse_identification to allow SysML-specific
/// behavior if needed, though currently the grammar is the same.
/// Helper to parse body or semicolon (common pattern)
pub(super) fn parse_body_or_semicolon<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACE) {
        p.parse_body();
    } else {
        p.expect(SyntaxKind::SEMICOLON);
    }
}

/// Helper to parse optional default value
/// Pattern: default [=] expr or = expr or := expr
pub(super) fn parse_optional_default_value<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::DEFAULT_KW) {
        bump_keyword(p);
        // Optional '=' after default
        if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
            bump_keyword(p);
        }
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    } else if p.at(SyntaxKind::EQ) || p.at(SyntaxKind::COLON_EQ) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional multiplicity
pub(super) fn parse_optional_multiplicity<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::L_BRACKET) {
        p.parse_multiplicity();
        p.skip_trivia();
    }
}

/// Helper to parse optional typing
pub(super) fn parse_optional_typing<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::COLON) {
        p.parse_typing();
        p.skip_trivia();
    }
}

/// Helper to parse specializations followed by skip_trivia
pub(super) fn parse_specializations_with_skip<P: SysMLParser>(p: &mut P) {
    parse_specializations(p);
    p.skip_trivia();
}

/// Helper to parse optional via clause: via <expr>
pub(super) fn parse_optional_via<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::VIA_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional to clause: to <expr>
pub(super) fn parse_optional_to<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::TO_KW) {
        bump_keyword(p);
        if p.can_start_expression() {
            parse_expression(p);
            p.skip_trivia();
        }
    }
}

/// Helper to parse optional from/to clause: from <name> to <name>
/// Creates FROM_TO_CLAUSE, FROM_TO_SOURCE, and FROM_TO_TARGET nodes for AST extraction
pub(super) fn parse_optional_from_to<P: SysMLParser>(p: &mut P) {
    if p.at(SyntaxKind::FROM_KW) {
        p.start_node(SyntaxKind::FROM_TO_CLAUSE);
        p.bump(); // from
        p.skip_trivia();

        // Parse source wrapped in FROM_TO_SOURCE
        if p.at_name_token() {
            p.start_node(SyntaxKind::FROM_TO_SOURCE);
            p.parse_qualified_name();
            p.finish_node();
            p.skip_trivia();
        }

        if p.at(SyntaxKind::TO_KW) {
            p.bump(); // to
            p.skip_trivia();

            // Parse target wrapped in FROM_TO_TARGET
            if p.at_name_token() {
                p.start_node(SyntaxKind::FROM_TO_TARGET);
                p.parse_qualified_name();
                p.finish_node();
                p.skip_trivia();
            }
        }
        p.finish_node(); // FROM_TO_CLAUSE
    }
}

/// Helper to parse comma-separated list with a parser function
#[allow(dead_code)]
pub(super) fn parse_comma_separated_list<P: SysMLParser, F>(p: &mut P, mut parse_item: F)
where
    F: FnMut(&mut P),
{
    parse_item(p);

    while p.at(SyntaxKind::COMMA) {
        bump_keyword(p);
        parse_item(p);
    }
}

/// Helper to parse inline send action (without final semicolon/body)
/// Used in contexts where send appears inside transitions, successions, etc.
pub(super) fn parse_inline_send_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::SEND_ACTION_USAGE);
    p.expect(SyntaxKind::SEND_KW);
    p.skip_trivia();

    if p.can_start_expression() {
        parse_expression(p);
        p.skip_trivia();
    }

    parse_optional_via(p);
    parse_optional_to(p);

    p.finish_node();
}

/// Helper to parse accept trigger (used in transitions)
/// Parses: [payload [:Type]] [at/after/when <expr>] [via <port>]
pub(super) fn parse_accept_trigger<P: SysMLParser>(p: &mut P) {
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
        bump_keyword(p);
        parse_expression(p);
        p.skip_trivia();
    }

    // Optional via
    if p.at(SyntaxKind::VIA_KW) {
        bump_keyword(p);
        p.parse_qualified_name();
        p.skip_trivia();
    }
}

/// Helper to parse inline action declaration: action <name> {...}
pub(super) fn parse_inline_action<P: SysMLParser>(p: &mut P) {
    p.start_node(SyntaxKind::USAGE);
    p.expect(SyntaxKind::ACTION_KW);
    p.skip_trivia();
    if p.at_name_token() {
        p.parse_identification();
        p.skip_trivia();
    }
    parse_action_body(p);
    p.finish_node();
}

