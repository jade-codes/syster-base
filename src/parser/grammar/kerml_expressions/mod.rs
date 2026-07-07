//! Expression parsing for KerML and SysML
//!
//! This module implements the following expression precedence chain (highest to
//! lowest binding, i.e. `PrimaryExpression` binds tightest):
//!
//! ```text
//! OwnedExpression → ConditionalExpression → NullCoalescingExpression
//!     → ImpliesExpression → OrExpression → XorExpression → AndExpression
//!     → EqualityExpression → ClassificationExpression → RelationalExpression
//!     → RangeExpression → AdditiveExpression → MultiplicativeExpression
//!     → ExponentiationExpression → UnaryExpression → ExtentExpression
//!     → PrimaryExpression
//! ```
//!
//! The official KEBNF grammar (`docs/grammar/KerML-textual-bnf.kebnf`) doesn't name
//! each precedence level as its own rule the way this chain does -- it collapses
//! almost all of them into three generic rules, `ConditionalBinaryOperatorExpression`,
//! `BinaryOperatorExpression`, and `UnaryOperatorExpression`, and expresses precedence
//! via a prose table rather than the grammar productions themselves. See
//! `docs/grammar-mapping.adoc` for the per-function mapping to those rules.

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

    /// Record a position to retroactively wrap in a node once its kind is
    /// known (e.g. after parsing prefixes to determine whether a definition
    /// is an `ActionDef`, `CalcDef`, etc.). Pairs with `start_node_at`.
    fn checkpoint(&self) -> rowan::Checkpoint;

    /// Start a node at a previously recorded `checkpoint`, wrapping
    /// everything parsed since that checkpoint. Must be paired with a
    /// regular `finish_node` call once the wrapped content is complete.
    fn start_node_at(&mut self, checkpoint: rowan::Checkpoint, kind: SyntaxKind);

    // Shared parsing utilities
    fn parse_qualified_name(&mut self);

    // Argument parsing (with named argument handling)
    fn parse_argument(&mut self);
}

// tag::parse_expression[]
/// Parse an expression, returning true if any tokens were consumed
/// Entry point for all expressions
/// Grammar: see docs/grammar-mapping.adoc#parse_expression
pub fn parse_expression<P: ExpressionParser>(p: &mut P) -> bool {
    let start_pos = p.get_pos();
    parse_conditional_expression(p);
    p.get_pos() > start_pos
}
// end::parse_expression[]

// tag::parse_ternary_conditional[]
/// We also support the SysML-style `if cond then expr else expr` with `then` keyword
///
/// Parse if ? then else - KerML style
/// Grammar: see docs/grammar-mapping.adoc#parse_ternary_conditional
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
// end::parse_ternary_conditional[]

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
    } else if p.at(SyntaxKind::EXISTS_KW) {
        parse_exists_expression(p);
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

/// ExistsExpression = 'exists' Name (',' Name)* ':' Expression
///
/// MontiCore `SysMLExpressions.mc4` extension of `OCLExpressions` -- not part
/// of the official OMG KEBNF grammar. "exists" is a contextual keyword, not
/// reserved: it's used as a plain function name in the standard library
/// (`ControlFunctions::exists`, `collection->exists {...}`), so it stays in
/// `at_name_token()`'s allowlist and this form is only recognized when
/// `exists` starts a new expression.
///
/// The bound names are untyped (no `Name : Type` form): MontiCore's own
/// grammar declares this as `key("exists") (InDeclaration || ",")+ ":" Expression`,
/// but `InDeclaration`'s exact structure isn't available (it's inherited from
/// a third-party OCL grammar this project doesn't vendor), and a per-name
/// `: Type` clause would be genuinely ambiguous with the single terminal `:`
/// before the predicate (e.g. `exists a : a == a` -- is the second `a` a type
/// or the start of the predicate?). This matches the single-colon form the
/// punch list itself shows (`exists ... :`).
fn parse_exists_expression<P: ExpressionParser>(p: &mut P) {
    p.bump(); // exists
    p.skip_trivia();

    if p.at_name_token() {
        p.bump();
        p.skip_trivia();
    }
    while p.at(SyntaxKind::COMMA) {
        p.bump();
        p.skip_trivia();
        if p.at_name_token() {
            p.bump();
            p.skip_trivia();
        }
    }

    if p.at(SyntaxKind::COLON) {
        p.bump();
        p.skip_trivia();
        parse_expression(p);
    }
}

// tag::parse_null_coalescing_expression[]
/// NullCoalescingExpression = ImpliesExpression ('??' ImpliesExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_null_coalescing_expression
pub fn parse_null_coalescing_expression<P: ExpressionParser>(p: &mut P) {
    parse_implies_expression(p);

    while p.at(SyntaxKind::QUESTION_QUESTION) {
        p.bump();
        p.skip_trivia();
        parse_implies_expression(p);
    }
}
// end::parse_null_coalescing_expression[]

// tag::parse_implies_expression[]
/// ImpliesExpression = OrExpression ('implies' OrExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_implies_expression
pub fn parse_implies_expression<P: ExpressionParser>(p: &mut P) {
    parse_or_expression(p);

    while p.at(SyntaxKind::IMPLIES_KW) {
        p.bump();
        p.skip_trivia();
        parse_or_expression(p);
    }
}
// end::parse_implies_expression[]

// tag::parse_or_expression[]
/// OrExpression = XorExpression (('|' | 'or') XorExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_or_expression
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
// end::parse_or_expression[]

// tag::parse_xor_expression[]
/// XorExpression = AndExpression ('xor' AndExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_xor_expression
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
// end::parse_xor_expression[]

// tag::parse_and_expression[]
/// AndExpression = UnionExpression (('&' | 'and') UnionExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_and_expression
pub fn parse_and_expression<P: ExpressionParser>(p: &mut P) {
    parse_union_expression(p);
    p.skip_trivia();

    while p.at(SyntaxKind::AMP) || p.at(SyntaxKind::AND_KW) {
        p.bump();
        p.skip_trivia();
        parse_union_expression(p);
        p.skip_trivia();
    }
}
// end::parse_and_expression[]

/// UnionExpression = EqualityExpression ('union' EqualityExpression)*
///
/// MontiCore `SysMLExpressions.mc4` extension of `de.monticore.ocl.SetExpressions`
/// -- not part of the official OMG KEBNF grammar. "union" is a contextual
/// keyword, not reserved: it's used as a plain function/feature name
/// throughout the standard library (`SequenceFunctions::union`, the
/// `union(a, b)` invocation form used everywhere instead of this infix
/// operator, `feature union: Occurrence[0..1]`), so it stays in
/// `at_name_token()`'s allowlist and this operator is only recognized
/// between two already-parsed operands.
pub fn parse_union_expression<P: ExpressionParser>(p: &mut P) {
    parse_equality_expression(p);
    p.skip_trivia();

    while p.at(SyntaxKind::UNION_KW) {
        p.bump();
        p.skip_trivia();
        parse_equality_expression(p);
        p.skip_trivia();
    }
}
// end::parse_and_expression[]

// tag::parse_equality_expression[]
/// EqualityExpression = ClassificationExpression (('==' | '!=' | '===' | '!==') ClassificationExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_equality_expression
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
// end::parse_equality_expression[]

// tag::parse_classification_expression[]
/// ClassificationExpression = RelationalExpression (('hastype' | 'istype' | 'as' | 'meta' | '@' | '@@') TypeReference)?
/// KerML/SysML define their own classification operators
/// Also handles prefix forms: 'hastype T', 'istype T', and '@ T' (implicit self operand,
/// per KerMLHasTypeSelfExpression = ("hastype" | "@") MCType). Without this, a leading '@'
/// falls through to the base-expression level's metadata-access parsing instead, which
/// happens to consume the same tokens but doesn't short-circuit the way a bare MCType should
/// (unlike the keyword form, it would otherwise allow further postfix/binary continuation
/// onto the type reference).
/// Grammar: see docs/grammar-mapping.adoc#parse_classification_expression
pub fn parse_classification_expression<P: ExpressionParser>(p: &mut P) {
    // Handle prefix hastype/istype/@ with implicit self operand
    if p.at_any(&[SyntaxKind::HASTYPE_KW, SyntaxKind::ISTYPE_KW, SyntaxKind::AT]) {
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
// end::parse_classification_expression[]

// tag::parse_relational_expression[]
/// RelationalExpression = RangeExpression (('<' | '>' | '<=' | '>=') RangeExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_relational_expression
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
// end::parse_relational_expression[]

// tag::parse_range_expression[]
/// RangeExpression = AdditiveExpression ('..' AdditiveExpression)?
/// Grammar: see docs/grammar-mapping.adoc#parse_range_expression
pub fn parse_range_expression<P: ExpressionParser>(p: &mut P) {
    parse_additive_expression(p);

    p.skip_trivia();
    if p.at(SyntaxKind::DOT_DOT) {
        p.bump();
        p.skip_trivia();
        parse_additive_expression(p);
    }
}
// end::parse_range_expression[]

// tag::parse_additive_expression[]
/// AdditiveExpression = MultiplicativeExpression (('+' | '-') MultiplicativeExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_additive_expression
pub fn parse_additive_expression<P: ExpressionParser>(p: &mut P) {
    parse_multiplicative_expression(p);

    while p.at(SyntaxKind::PLUS) || p.at(SyntaxKind::MINUS) {
        p.bump();
        p.skip_trivia();
        parse_multiplicative_expression(p);
    }
}
// end::parse_additive_expression[]

// tag::parse_multiplicative_expression[]
/// MultiplicativeExpression = ExponentiationExpression (('*' | '/' | '%') ExponentiationExpression)*
/// Grammar: see docs/grammar-mapping.adoc#parse_multiplicative_expression
pub fn parse_multiplicative_expression<P: ExpressionParser>(p: &mut P) {
    parse_exponentiation_expression(p);

    while p.at_any(&[SyntaxKind::STAR, SyntaxKind::SLASH, SyntaxKind::PERCENT]) {
        p.bump();
        p.skip_trivia();
        parse_exponentiation_expression(p);
    }
}
// end::parse_multiplicative_expression[]

// tag::parse_exponentiation_expression[]
/// ExponentiationExpression = UnaryExpression (('**' | '^') ExponentiationExpression)?
/// Note: Right-associative by recursing on right side
/// Grammar: see docs/grammar-mapping.adoc#parse_exponentiation_expression
pub fn parse_exponentiation_expression<P: ExpressionParser>(p: &mut P) {
    parse_unary_expression(p);

    p.skip_trivia();
    if p.at(SyntaxKind::STAR_STAR) || p.at(SyntaxKind::CARET) {
        p.bump();
        p.skip_trivia();
        parse_exponentiation_expression(p);
    }
}
// end::parse_exponentiation_expression[]

// tag::parse_unary_expression[]
/// UnaryExpression = ('+' | '-' | '~' | 'not')? ExtentExpression
/// Grammar: see docs/grammar-mapping.adoc#parse_unary_expression
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
// end::parse_unary_expression[]

// tag::parse_extent_expression[]
/// ExtentExpression = ('all')? PrimaryExpression
/// Grammar: see docs/grammar-mapping.adoc#parse_extent_expression
pub fn parse_extent_expression<P: ExpressionParser>(p: &mut P) {
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }
    parse_primary_expression(p);
}
// end::parse_extent_expression[]
