//! Expression parsing for KerML and SysML
//!
//! This module implements the expression precedence chain from kerml_expressions.pest:
//!
//! ```text
//! OwnedExpression → ConditionalExpression → NullCoalescingExpression
//!     → ImpliesExpression → OrExpression → XorExpression → AndExpression
//!     → EqualityExpression → ClassificationExpression → RelationalExpression
//!     → RangeExpression → AdditiveExpression → MultiplicativeExpression
//!     → ExponentiationExpression → UnaryExpression → ExtentExpression
//!     → PrimaryExpression
//! ```

// Submodules
mod atoms;
mod body;
mod primary;

// Shared import — pub(super) so submodules get it via `use super::*;`
pub(super) use crate::parser::syntax_kind::SyntaxKind;

// Re-exports — submodules access siblings via `use super::*;`
// `pub use` so external callers (parser.rs, grammar/mod.rs) can reach submodule items
pub use self::atoms::*;
pub use self::body::*;
pub use self::primary::*;

/// Trait for expression parsing operations
///
/// This trait defines the interface between the expression parser and the main parser.
/// The main parser implements this trait to provide the necessary infrastructure.
pub trait ExpressionParser {
    // Token inspection
    fn current_kind(&self) -> SyntaxKind;
    fn at(&self, kind: SyntaxKind) -> bool;
    fn at_any(&self, kinds: &[SyntaxKind]) -> bool;
    fn at_name_token(&self) -> bool;

    // Position tracking
    fn get_pos(&self) -> usize;

    /// Peek at the kind of the nth token ahead (skipping trivia)
    fn peek_kind(&self, n: usize) -> SyntaxKind;

    // Token consumption
    fn bump(&mut self);
    fn bump_any(&mut self);
    fn expect(&mut self, kind: SyntaxKind);

    // Trivia handling
    fn skip_trivia(&mut self);

    // Node building
    fn start_node(&mut self, kind: SyntaxKind);
    fn finish_node(&mut self);

    // Shared parsing utilities
    fn parse_qualified_name(&mut self);

    // Argument parsing (with named argument handling)
    fn parse_argument(&mut self);
}

/// Parse an expression, returning true if any tokens were consumed
/// Per pest: owned_expression = { conditional_expression }
/// Entry point for all expressions
pub fn parse_expression<P: ExpressionParser>(p: &mut P) -> bool {
    let start_pos = p.get_pos();
    parse_conditional_expression(p);
    p.get_pos() > start_pos
}

/// ConditionalExpression per Pest:
/// Per pest: conditional_expression = {
///     if_token ~ null_coalescing_expression ~ question_mark ~ owned_expression_reference ~ else_token ~ owned_expression_reference
///     | null_coalescing_expression
/// }
///
/// We also support the SysML-style `if cond then expr else expr` with `then` keyword
///
/// Parse if ? then else - KerML style
fn parse_ternary_conditional<P: ExpressionParser>(p: &mut P) {
    p.bump(); // ?
    p.skip_trivia();
    parse_expression(p);
    p.skip_trivia();
    if p.at(SyntaxKind::ELSE_KW) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
    }
}

/// Parse if then else - SysML style
fn parse_keyword_conditional<P: ExpressionParser>(p: &mut P) {
    p.bump(); // then
    p.skip_trivia();
    parse_expression(p);
    p.skip_trivia();
    if p.at(SyntaxKind::ELSE_KW) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
    }
}

pub fn parse_conditional_expression<P: ExpressionParser>(p: &mut P) {
    p.start_node(SyntaxKind::EXPRESSION);

    if p.at(SyntaxKind::IF_KW) {
        // KerML if-expression: if cond ? then else | if cond then then else
        p.bump(); // if
        p.skip_trivia();
        parse_null_coalescing_expression(p); // condition
        p.skip_trivia();

        // Two forms: if cond ? then else | if cond then then else
        if p.at(SyntaxKind::QUESTION) {
            parse_ternary_conditional(p);
        } else if p.at(SyntaxKind::THEN_KW) {
            parse_keyword_conditional(p);
        }
    } else {
        // Standard ternary: cond ? then : else
        parse_null_coalescing_expression(p);
        p.skip_trivia();

        // Check for standard ternary operator (not ??)
        if p.at(SyntaxKind::QUESTION) && !p.at(SyntaxKind::QUESTION_QUESTION) {
            p.bump(); // ?
            p.skip_trivia();
            parse_expression(p); // then expression
            p.skip_trivia();
            p.expect(SyntaxKind::COLON);
            p.skip_trivia();
            parse_expression(p); // else expression
        }
    }

    p.finish_node();
}

/// NullCoalescingExpression = ImpliesExpression ('??' ImpliesExpression)*
/// Per pest: null_coalescing_expression = { implies_expression ~ (double_question_mark ~ implies_expression_reference)* }
pub fn parse_null_coalescing_expression<P: ExpressionParser>(p: &mut P) {
    parse_implies_expression(p);

    while p.at(SyntaxKind::QUESTION_QUESTION) {
        p.bump();
        p.skip_trivia();
        parse_implies_expression(p);
    }
}

/// ImpliesExpression = OrExpression ('implies' OrExpression)*
/// Per pest: implies_expression = { or_expression ~ (implies_token ~ or_expression_reference)* }
pub fn parse_implies_expression<P: ExpressionParser>(p: &mut P) {
    parse_or_expression(p);

    while p.at(SyntaxKind::IMPLIES_KW) {
        p.bump();
        p.skip_trivia();
        parse_or_expression(p);
    }
}

/// OrExpression = XorExpression (('|' | 'or') XorExpression)*
/// Per pest: or_expression = { xor_expression ~ ((or_token ~ xor_expression_reference) | ("|" ~ xor_expression))* }
pub fn parse_or_expression<P: ExpressionParser>(p: &mut P) {
    parse_xor_expression(p);
    p.skip_trivia();

    while p.at(SyntaxKind::PIPE) || p.at(SyntaxKind::OR_KW) {
        p.bump();
        p.skip_trivia();
        parse_xor_expression(p);
        p.skip_trivia();
    }
}

/// XorExpression = AndExpression ('xor' AndExpression)*
/// Per pest: xor_expression = { and_expression ~ (xor_token ~ and_expression)* }
pub fn parse_xor_expression<P: ExpressionParser>(p: &mut P) {
    parse_and_expression(p);
    p.skip_trivia();

    while p.at(SyntaxKind::XOR_KW) {
        p.bump();
        p.skip_trivia();
        parse_and_expression(p);
        p.skip_trivia();
    }
}

/// AndExpression = EqualityExpression (('&' | 'and') EqualityExpression)*
/// Per pest: and_expression = { equality_expression ~ ((and_token ~ equality_expression_reference) | ("&" ~ equality_expression))* }
pub fn parse_and_expression<P: ExpressionParser>(p: &mut P) {
    parse_equality_expression(p);
    p.skip_trivia();

    while p.at(SyntaxKind::AMP) || p.at(SyntaxKind::AND_KW) {
        p.bump();
        p.skip_trivia();
        parse_equality_expression(p);
        p.skip_trivia();
    }
}

/// EqualityExpression = ClassificationExpression (('==' | '!=' | '===' | '!==') ClassificationExpression)*
/// Per pest: equality_expression = { classification_expression ~ (equality_operator ~ classification_expression)* }
/// Per pest: equality_operator is defined in parent grammar (KerML/SysML)
pub fn parse_equality_expression<P: ExpressionParser>(p: &mut P) {
    parse_classification_expression(p);
    p.skip_trivia();

    while p.at_any(&[
        SyntaxKind::EQ_EQ,
        SyntaxKind::BANG_EQ,
        SyntaxKind::EQ_EQ_EQ,
        SyntaxKind::BANG_EQ_EQ,
    ]) {
        p.bump();
        p.skip_trivia();
        parse_classification_expression(p);
        p.skip_trivia();
    }
}

/// ClassificationExpression = RelationalExpression (('hastype' | 'istype' | 'as' | 'meta' | '@' | '@@') TypeReference)?
/// Per pest: classification_expression defined in each grammar - handles type operators
/// KerML/SysML define their own classification operators
/// Also handles prefix forms: 'hastype T' and 'istype T' (implicit self operand)
pub fn parse_classification_expression<P: ExpressionParser>(p: &mut P) {
    // Handle prefix hastype/istype with implicit self operand
    if p.at_any(&[SyntaxKind::HASTYPE_KW, SyntaxKind::ISTYPE_KW]) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name();
        return;
    }

    parse_relational_expression(p);

    p.skip_trivia();
    if p.at_any(&[
        SyntaxKind::HASTYPE_KW,
        SyntaxKind::ISTYPE_KW,
        SyntaxKind::AS_KW,
        SyntaxKind::META_KW,
        SyntaxKind::AT,
        SyntaxKind::AT_AT,
    ]) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name();
    }
}

/// RelationalExpression = RangeExpression (('<' | '>' | '<=' | '>=') RangeExpression)*
/// Per pest: relational_expression = { range_expression ~ (relational_operator ~ range_expression)* }
/// Per pest: relational_operator is defined in parent grammar
pub fn parse_relational_expression<P: ExpressionParser>(p: &mut P) {
    parse_range_expression(p);
    p.skip_trivia();

    while p.at_any(&[
        SyntaxKind::LT,
        SyntaxKind::GT,
        SyntaxKind::LT_EQ,
        SyntaxKind::GT_EQ,
    ]) {
        p.bump();
        p.skip_trivia();
        parse_range_expression(p);
        p.skip_trivia();
    }
}

/// RangeExpression = AdditiveExpression ('..' AdditiveExpression)?
/// Per pest: range_expression = { additive_expression ~ (".." ~ additive_expression)? }
pub fn parse_range_expression<P: ExpressionParser>(p: &mut P) {
    parse_additive_expression(p);

    p.skip_trivia();
    if p.at(SyntaxKind::DOT_DOT) {
        p.bump();
        p.skip_trivia();
        parse_additive_expression(p);
    }
}

/// AdditiveExpression = MultiplicativeExpression (('+' | '-') MultiplicativeExpression)*
/// Per pest: additive_expression = { multiplicative_expression ~ (additive_operator ~ multiplicative_expression)* }
pub fn parse_additive_expression<P: ExpressionParser>(p: &mut P) {
    parse_multiplicative_expression(p);

    while p.at(SyntaxKind::PLUS) || p.at(SyntaxKind::MINUS) {
        p.bump();
        p.skip_trivia();
        parse_multiplicative_expression(p);
    }
}

/// MultiplicativeExpression = ExponentiationExpression (('*' | '/' | '%') ExponentiationExpression)*
/// Per pest: multiplicative_expression = { exponentiation_expression ~ (multiplicative_operator ~ exponentiation_expression)* }
pub fn parse_multiplicative_expression<P: ExpressionParser>(p: &mut P) {
    parse_exponentiation_expression(p);

    while p.at_any(&[SyntaxKind::STAR, SyntaxKind::SLASH, SyntaxKind::PERCENT]) {
        p.bump();
        p.skip_trivia();
        parse_exponentiation_expression(p);
    }
}

/// ExponentiationExpression = UnaryExpression (('**' | '^') ExponentiationExpression)?
/// Per pest: exponentiation_expression = { unary_expression ~ (exponentiation_operator ~ exponentiation_expression)? }
/// Note: Right-associative by recursing on right side
pub fn parse_exponentiation_expression<P: ExpressionParser>(p: &mut P) {
    parse_unary_expression(p);

    p.skip_trivia();
    if p.at(SyntaxKind::STAR_STAR) || p.at(SyntaxKind::CARET) {
        p.bump();
        p.skip_trivia();
        parse_exponentiation_expression(p);
    }
}

/// UnaryExpression = ('+' | '-' | '~' | 'not')? ExtentExpression
/// Per pest: unary_expression = { unary_operator ~ extent_expression | extent_expression }
pub fn parse_unary_expression<P: ExpressionParser>(p: &mut P) {
    if p.at_any(&[
        SyntaxKind::PLUS,
        SyntaxKind::MINUS,
        SyntaxKind::TILDE,
        SyntaxKind::NOT_KW,
    ]) {
        p.bump();
        p.skip_trivia();
    }
    parse_extent_expression(p);
}

/// ExtentExpression = ('all')? PrimaryExpression
/// Per pest: extent_expression defined in each grammar - handles 'all' and collection ops
pub fn parse_extent_expression<P: ExpressionParser>(p: &mut P) {
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }
    parse_primary_expression(p);
}
