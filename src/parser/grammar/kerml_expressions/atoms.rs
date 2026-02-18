use super::*;

/// BaseExpression = LiteralExpression | FeatureReferenceExpression | InvocationExpression | '(' SequenceExpression ')' | NewExpression | IfExpression
/// Per pest: primary_expression defined in each grammar - this is the base/atomic expression parsing
/// Handle literal values (integers, strings, booleans, null)
fn parse_literal<P: ExpressionParser>(p: &mut P) -> bool {
    if p.at_any(&[
        SyntaxKind::INTEGER,
        SyntaxKind::DECIMAL,
        SyntaxKind::STRING,
        SyntaxKind::TRUE_KW,
        SyntaxKind::FALSE_KW,
        SyntaxKind::NULL_KW,
    ]) {
        p.bump();
        true
    } else {
        false
    }
}

/// Handle instantiation: new Type() or new Type(args)
fn parse_instantiation<P: ExpressionParser>(p: &mut P) {
    p.bump(); // new
    p.skip_trivia();
    p.parse_qualified_name();
    p.skip_trivia();
    if p.at(SyntaxKind::L_PAREN) {
        parse_argument_list(p);
    }
}

/// Handle block expression: { expr }
fn parse_block_expression<P: ExpressionParser>(p: &mut P) {
    p.bump(); // {
    p.skip_trivia();
    if !p.at(SyntaxKind::R_BRACE) {
        parse_expression(p);
    }
    p.skip_trivia();
    p.expect(SyntaxKind::R_BRACE);
}

/// Handle parenthesized expression or sequence: (expr) or (expr1, expr2, ...)
fn parse_parenthesized_expression<P: ExpressionParser>(p: &mut P) {
    p.bump(); // (
    p.skip_trivia();

    if !p.at(SyntaxKind::R_PAREN) {
        parse_expression(p);

        // Check for sequence (comma-separated)
        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
        }
    }

    p.skip_trivia();
    p.expect(SyntaxKind::R_PAREN);
}

/// Check if current token can start a feature reference
/// In SysML/KerML, most keywords can also be used as identifiers in expression context
fn is_feature_reference_token(kind: SyntaxKind) -> bool {
    // Exclude tokens that definitely cannot be names
    !matches!(
        kind,
        SyntaxKind::ERROR
            | SyntaxKind::WHITESPACE
            | SyntaxKind::LINE_COMMENT
            | SyntaxKind::BLOCK_COMMENT
            | SyntaxKind::L_BRACE
            | SyntaxKind::R_BRACE
            | SyntaxKind::L_BRACKET
            | SyntaxKind::R_BRACKET
            | SyntaxKind::L_PAREN
            | SyntaxKind::R_PAREN
            | SyntaxKind::SEMICOLON
            | SyntaxKind::COLON
            | SyntaxKind::COLON_COLON
            | SyntaxKind::COLON_GT
            | SyntaxKind::COLON_GT_GT
            | SyntaxKind::COLON_COLON_GT
            | SyntaxKind::DOT
            | SyntaxKind::DOT_DOT
            | SyntaxKind::COMMA
            | SyntaxKind::EQ
            | SyntaxKind::EQ_EQ
            | SyntaxKind::EQ_EQ_EQ
            | SyntaxKind::BANG_EQ
            | SyntaxKind::BANG_EQ_EQ
            | SyntaxKind::LT
            | SyntaxKind::GT
            | SyntaxKind::LT_EQ
            | SyntaxKind::GT_EQ
            | SyntaxKind::AT
            | SyntaxKind::AT_AT
            | SyntaxKind::HASH
            | SyntaxKind::STAR
            | SyntaxKind::STAR_STAR
            | SyntaxKind::PLUS
            | SyntaxKind::MINUS
            | SyntaxKind::SLASH
            | SyntaxKind::PERCENT
            | SyntaxKind::CARET
            | SyntaxKind::AMP
            | SyntaxKind::AMP_AMP
            | SyntaxKind::PIPE
            | SyntaxKind::PIPE_PIPE
            | SyntaxKind::BANG
            | SyntaxKind::TILDE
            | SyntaxKind::QUESTION
            | SyntaxKind::QUESTION_QUESTION
            | SyntaxKind::ARROW
            | SyntaxKind::FAT_ARROW
            | SyntaxKind::INTEGER
            | SyntaxKind::DECIMAL
            | SyntaxKind::STRING
            | SyntaxKind::TRUE_KW
            | SyntaxKind::FALSE_KW
            | SyntaxKind::NULL_KW
    )
}

/// Handle feature reference or invocation: name or name(args)
fn parse_feature_reference<P: ExpressionParser>(p: &mut P) {
    p.parse_qualified_name();
    p.skip_trivia();

    // Check for invocation
    if p.at(SyntaxKind::L_PAREN) {
        parse_argument_list(p);
    }
}

/// Handle metadata access: @name
fn parse_metadata_access<P: ExpressionParser>(p: &mut P) {
    p.bump(); // @
    p.skip_trivia();
    p.parse_qualified_name();
}

pub fn parse_base_expression<P: ExpressionParser>(p: &mut P) {
    p.skip_trivia();

    match p.current_kind() {
        kind if parse_literal(p) => {}
        SyntaxKind::NEW_KW => parse_instantiation(p),
        SyntaxKind::L_BRACE => parse_block_expression(p),
        SyntaxKind::L_PAREN => parse_parenthesized_expression(p),
        kind if is_feature_reference_token(kind) => parse_feature_reference(p),
        SyntaxKind::AT => parse_metadata_access(p),
        _ => {}
    }
}

/// ArgumentList = '(' (Argument (',' Argument)*)? ')'
/// Per pest: Argument list parsing is grammar-specific
/// Per pest: argument = { (name ~ "=")? ~ expression } for named arguments
pub fn parse_argument_list<P: ExpressionParser>(p: &mut P) {
    p.start_node(SyntaxKind::ARGUMENT_LIST);

    p.expect(SyntaxKind::L_PAREN);
    p.skip_trivia();

    if !p.at(SyntaxKind::R_PAREN) {
        parse_argument_via_trait(p);
        p.skip_trivia();

        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_argument_via_trait(p);
            p.skip_trivia();
        }
    }

    p.expect(SyntaxKind::R_PAREN);

    p.finish_node();
}

/// Argument = (Name '=')? Expression
/// Delegates to the main parser via ExpressionParser trait for named argument handling
fn parse_argument_via_trait<P: ExpressionParser>(p: &mut P) {
    p.parse_argument();
}

/// Parse a single argument (potentially named)
/// Argument = (Name '=')? Expression
pub fn parse_argument<P: ExpressionParser>(p: &mut P) {
    p.start_node(SyntaxKind::ARGUMENT_LIST);

    // Check for named argument: name = value
    if p.at(SyntaxKind::IDENT) {
        let next = p.peek_kind(1);
        if next == SyntaxKind::EQ {
            p.bump(); // name
            p.skip_trivia();
            p.bump(); // =
            p.skip_trivia();
        }
    }

    // Parse the expression value
    parse_expression(p);

    p.finish_node();
}
