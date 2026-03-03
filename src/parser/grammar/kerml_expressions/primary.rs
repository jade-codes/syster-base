use super::*;

/// PrimaryExpression = BaseExpression (FeatureChain | ArrowInvocation)*
/// Per pest: primary_expression defined in each grammar - handles feature chains, arrow ops, indexing
/// Handle .name feature chain
fn handle_feature_chain<P: ExpressionParser>(p: &mut P) {
    p.bump(); // .
    p.skip_trivia();

    // Handle identifier or 'this' keyword in feature chain
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::THIS_KW) {
        p.bump();
        p.skip_trivia();
        // Check for invocation
        if p.at(SyntaxKind::L_PAREN) {
            parse_argument_list(p);
        }
    }
}

/// Handle .?{...} shorthand select
fn handle_shorthand_select<P: ExpressionParser>(p: &mut P) {
    p.bump(); // .
    p.skip_trivia();
    p.bump(); // ?
    p.skip_trivia();
    if p.at(SyntaxKind::L_BRACE) {
        parse_arrow_body(p);
    }
}

/// Handle .{...} shorthand collect
fn handle_shorthand_collect<P: ExpressionParser>(p: &mut P) {
    p.bump(); // .
    p.skip_trivia();
    parse_arrow_body(p);
}

/// Handle -> arrow invocation
fn handle_arrow_invocation<P: ExpressionParser>(p: &mut P) {
    p.bump(); // ->
    p.skip_trivia();

    // Method name
    if p.at(SyntaxKind::IDENT) {
        p.bump();
    }
    p.skip_trivia();

    // Arguments: { body } | ( args ) | bare expression
    if p.at(SyntaxKind::L_BRACE) {
        parse_arrow_body(p);
    } else if p.at(SyntaxKind::L_PAREN) {
        parse_argument_list(p);
    } else if !p.at(SyntaxKind::SEMICOLON)
        && !p.at(SyntaxKind::R_BRACE)
        && !p.at(SyntaxKind::R_PAREN)
    {
        // Bare expression: ->reduce '+'
        if p.at(SyntaxKind::STRING) || p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::INTEGER) {
            parse_expression(p);
        }
    }
}

/// Handle #(n, m) or #n bracket index
fn handle_bracket_index<P: ExpressionParser>(p: &mut P) {
    p.bump(); // #
    p.skip_trivia();
    if p.at(SyntaxKind::L_PAREN) {
        p.bump(); // (
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
        // Support multiple indices
        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }
        p.expect(SyntaxKind::R_PAREN);
    }
}

/// Handle [n] array index
fn handle_array_index<P: ExpressionParser>(p: &mut P) {
    p.bump(); // [
    p.skip_trivia();
    parse_expression(p);
    p.skip_trivia();
    p.expect(SyntaxKind::R_BRACKET);
}

pub fn parse_primary_expression<P: ExpressionParser>(p: &mut P) {
    parse_base_expression(p);

    loop {
        p.skip_trivia();

        match p.current_kind() {
            SyntaxKind::DOT => {
                // Check for shorthand operations
                if p.peek_kind(1) == SyntaxKind::QUESTION {
                    handle_shorthand_select(p);
                } else if p.peek_kind(1) == SyntaxKind::L_BRACE {
                    handle_shorthand_collect(p);
                } else {
                    handle_feature_chain(p);
                }
            }
            SyntaxKind::ARROW => handle_arrow_invocation(p),
            SyntaxKind::HASH => handle_bracket_index(p),
            SyntaxKind::L_BRACKET => handle_array_index(p),
            _ => break,
        }
    }
}
