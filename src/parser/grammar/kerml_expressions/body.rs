use super::*;

/// ArrowBody = '{' (Parameter* Expression)? '}'
/// Used for arrow invocation bodies like: ->collect { in x; x + 1 }
/// Per Pest body_expression_body: only 'in' parameters are allowed (not out/inout)
/// Per pest: Arrow operation bodies are grammar-specific implementations

/// Check if current position looks like a typed parameter (name : Type;)
fn looks_like_typed_parameter<P: ExpressionParser>(p: &P) -> bool {
    let mut lookahead = 1;
    // Skip trivia
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }

    if p.peek_kind(lookahead) != SyntaxKind::COLON {
        return false;
    }

    // Skip past colon
    lookahead += 1;
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }

    // Check for type name
    if !matches!(p.peek_kind(lookahead), SyntaxKind::IDENT) {
        return false;
    }

    lookahead += 1;
    // Skip qualified name parts (::Part)
    loop {
        while matches!(
            p.peek_kind(lookahead),
            SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
        ) {
            lookahead += 1;
        }
        if p.peek_kind(lookahead) == SyntaxKind::COLON_COLON {
            lookahead += 1;
            while matches!(
                p.peek_kind(lookahead),
                SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
            ) {
                lookahead += 1;
            }
            if matches!(p.peek_kind(lookahead), SyntaxKind::IDENT) {
                lookahead += 1;
            }
        } else {
            break;
        }
    }

    // Check for semicolon
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }
    p.peek_kind(lookahead) == SyntaxKind::SEMICOLON
}

/// Handle visibility keyword in arrow body
fn handle_visibility_prefix<P: ExpressionParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::PRIVATE_KW,
        SyntaxKind::PUBLIC_KW,
        SyntaxKind::PROTECTED_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }
}

/// Parse nested body members (doc/comment annotations)
fn parse_nested_body_members<P: ExpressionParser>(p: &mut P) {
    while !p.at(SyntaxKind::R_BRACE) && p.current_kind() != SyntaxKind::__LAST {
        if p.at(SyntaxKind::DOC_KW) || p.at(SyntaxKind::COMMENT_KW) {
            p.bump();
            p.skip_trivia();
            // Skip content
            while !p.at(SyntaxKind::R_BRACE)
                && p.current_kind() != SyntaxKind::__LAST
                && !p.at(SyntaxKind::DOC_KW)
                && !p.at(SyntaxKind::COMMENT_KW)
            {
                p.bump_any();
                p.skip_trivia();
            }
        } else {
            break;
        }
    }
}

/// Parse parameter body (for parameters with { ... } bodies)
fn parse_parameter_body<P: ExpressionParser>(p: &mut P) {
    p.bump(); // {
    p.skip_trivia();
    parse_nested_body_members(p);
    p.expect(SyntaxKind::R_BRACE);
    p.skip_trivia();
}

/// Handle usage keyword declarations (attribute, part, item)
fn handle_usage_keyword<P: ExpressionParser>(p: &mut P) {
    p.bump(); // usage keyword
    p.skip_trivia();

    // Name
    if p.at_name_token() {
        p.bump();
        p.skip_trivia();
    }

    // Optional typing
    if p.at(SyntaxKind::COLON) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name();
        p.skip_trivia();
    }

    // Optional value assignment
    if p.at(SyntaxKind::EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    // Body or semicolon
    if p.at(SyntaxKind::L_BRACE) {
        // Nested body - simplified parsing
        p.bump();
        p.skip_trivia();
        while !p.at(SyntaxKind::R_BRACE) && p.current_kind() != SyntaxKind::__LAST {
            if p.at_any(&[
                SyntaxKind::DOC_KW,
                SyntaxKind::COMMENT_KW,
                SyntaxKind::COLON_GT_GT,
                SyntaxKind::COLON_GT,
            ]) {
                p.bump();
                p.skip_trivia();
                if p.at_name_token() {
                    p.bump();
                    p.skip_trivia();
                }
                if p.at(SyntaxKind::SEMICOLON) {
                    p.bump();
                    p.skip_trivia();
                }
            } else {
                p.bump_any();
                p.skip_trivia();
            }
        }
        p.expect(SyntaxKind::R_BRACE);
        p.skip_trivia();
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        p.skip_trivia();
    }
}

/// Handle shorthand feature declaration: name : Type = value;
fn handle_feature_declaration<P: ExpressionParser>(p: &mut P) -> bool {
    if !p.at_name_token() {
        return false;
    }

    let mut lookahead = 1;
    while matches!(
        p.peek_kind(lookahead),
        SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT
    ) {
        lookahead += 1;
    }

    if p.peek_kind(lookahead) != SyntaxKind::COLON {
        return false;
    }

    // Parse: name : Type = value;
    p.bump(); // name
    p.skip_trivia();
    p.bump(); // :
    p.skip_trivia();
    p.parse_qualified_name();
    p.skip_trivia();

    if p.at(SyntaxKind::EQ) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
        p.skip_trivia();
    }

    if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        p.skip_trivia();
    }
    true
}

/// Parse 'in' parameter: in [ref] name [: Type] [{ ... }]
fn parse_in_parameter<P: ExpressionParser>(p: &mut P) {
    p.bump(); // in
    p.skip_trivia();

    // Optional 'ref' prefix
    if p.at(SyntaxKind::REF_KW) {
        p.bump();
        p.skip_trivia();
    }

    if p.at_name_token() {
        p.bump();
    }
    p.skip_trivia();

    // Optional type
    if p.at(SyntaxKind::COLON) || p.at(SyntaxKind::COLON_GT) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name();
    }
    p.skip_trivia();

    // Optional body
    if p.at(SyntaxKind::L_BRACE) {
        parse_parameter_body(p);
    } else if p.at(SyntaxKind::SEMICOLON) {
        p.bump();
        p.skip_trivia();
    }
}

/// Parse parameter without 'in' keyword: name : Type;
fn parse_implicit_parameter<P: ExpressionParser>(p: &mut P) -> bool {
    if !looks_like_typed_parameter(p) {
        return false;
    }

    p.bump(); // name
    p.skip_trivia();
    p.bump(); // :
    p.skip_trivia();
    p.parse_qualified_name();
    p.skip_trivia();
    p.bump(); // ;
    p.skip_trivia();
    true
}

pub fn parse_arrow_body<P: ExpressionParser>(p: &mut P) {
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();

    let token_count = 10000;
    let mut iterations = 0;

    while !p.at(SyntaxKind::R_BRACE) && iterations < token_count {
        iterations += 1;
        let _start_pos = p.get_pos();

        // Handle doc/comment annotations
        if p.at(SyntaxKind::DOC_KW) || p.at(SyntaxKind::COMMENT_KW) {
            p.bump();
            p.skip_trivia();
            continue;
        }

        // Handle visibility prefix
        handle_visibility_prefix(p);

        // Handle usage keywords (attribute, part, item)
        if p.at_any(&[
            SyntaxKind::ATTRIBUTE_KW,
            SyntaxKind::ITEM_KW,
            SyntaxKind::PART_KW,
        ]) {
            handle_usage_keyword(p);
            continue;
        }

        // Handle shorthand feature declaration
        if handle_feature_declaration(p) {
            continue;
        }

        // Handle 'in' parameters
        if p.at(SyntaxKind::IN_KW) {
            parse_in_parameter(p);
            continue;
        }

        // Handle implicit parameters (name : Type;)
        if p.at_name_token() && parse_implicit_parameter(p) {
            continue;
        }

        // Body expression
        parse_expression(p);
        p.skip_trivia();

        // Safety: if no progress, skip to avoid infinite loop
        if p.get_pos() == _start_pos && !p.at(SyntaxKind::R_BRACE) {
            p.bump_any();
        }
    }

    p.expect(SyntaxKind::R_BRACE);
}
