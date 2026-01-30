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

use crate::parser::syntax_kind::SyntaxKind;

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
        parse_null_coalescing_expression(p);
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
    
    while p.at_any(&[SyntaxKind::EQ_EQ, SyntaxKind::BANG_EQ, SyntaxKind::EQ_EQ_EQ, SyntaxKind::BANG_EQ_EQ]) {
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
    if p.at_any(&[SyntaxKind::HASTYPE_KW, SyntaxKind::ISTYPE_KW, SyntaxKind::AS_KW, SyntaxKind::META_KW, SyntaxKind::AT, SyntaxKind::AT_AT]) {
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
    
    while p.at_any(&[SyntaxKind::LT, SyntaxKind::GT, SyntaxKind::LT_EQ, SyntaxKind::GT_EQ]) {
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
    if p.at_any(&[SyntaxKind::PLUS, SyntaxKind::MINUS, SyntaxKind::TILDE, SyntaxKind::NOT_KW]) {
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

/// PrimaryExpression = BaseExpression (FeatureChain | ArrowInvocation)*
/// Per pest: primary_expression defined in each grammar - handles feature chains, arrow ops, indexing
/// Handle .name feature chain
fn handle_feature_chain<P: ExpressionParser>(p: &mut P) {
    p.bump(); // .
    p.skip_trivia();
    
    if p.at(SyntaxKind::IDENT) {
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
    } else if !p.at(SyntaxKind::SEMICOLON) && !p.at(SyntaxKind::R_BRACE) && !p.at(SyntaxKind::R_PAREN) {
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

/// ArrowBody = '{' (Parameter* Expression)? '}'
/// Used for arrow invocation bodies like: ->collect { in x; x + 1 }
/// Per Pest body_expression_body: only 'in' parameters are allowed (not out/inout)
/// Per pest: Arrow operation bodies are grammar-specific implementations

/// Check if current position looks like a typed parameter (name : Type;)
fn looks_like_typed_parameter<P: ExpressionParser>(p: &P) -> bool {
    let mut lookahead = 1;
    // Skip trivia
    while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
        lookahead += 1;
    }
    
    if p.peek_kind(lookahead) != SyntaxKind::COLON {
        return false;
    }
    
    // Skip past colon
    lookahead += 1;
    while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
        lookahead += 1;
    }
    
    // Check for type name
    if !matches!(p.peek_kind(lookahead), SyntaxKind::IDENT) {
        return false;
    }
    
    lookahead += 1;
    // Skip qualified name parts (::Part)
    loop {
        while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
            lookahead += 1;
        }
        if p.peek_kind(lookahead) == SyntaxKind::COLON_COLON {
            lookahead += 1;
            while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
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
    while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
        lookahead += 1;
    }
    p.peek_kind(lookahead) == SyntaxKind::SEMICOLON
}

/// Handle visibility keyword in arrow body
fn handle_visibility_prefix<P: ExpressionParser>(p: &mut P) {
    if p.at_any(&[SyntaxKind::PRIVATE_KW, SyntaxKind::PUBLIC_KW, SyntaxKind::PROTECTED_KW]) {
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
                && !p.at(SyntaxKind::COMMENT_KW) {
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
            if p.at_any(&[SyntaxKind::DOC_KW, SyntaxKind::COMMENT_KW, SyntaxKind::COLON_GT_GT, SyntaxKind::COLON_GT]) {
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
    while matches!(p.peek_kind(lookahead), SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT | SyntaxKind::BLOCK_COMMENT) {
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
        let start_pos = p.get_pos();
        
        // Handle doc/comment annotations
        if p.at(SyntaxKind::DOC_KW) || p.at(SyntaxKind::COMMENT_KW) {
            p.bump();
            p.skip_trivia();
            continue;
        }
        
        // Handle visibility prefix
        handle_visibility_prefix(p);
        
        // Handle usage keywords (attribute, part, item)
        if p.at_any(&[SyntaxKind::ATTRIBUTE_KW, SyntaxKind::ITEM_KW, SyntaxKind::PART_KW]) {
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
        break;
        
        // Safety: if no progress, skip to avoid infinite loop
        if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
            p.bump_any();
        }
    }
    
    p.expect(SyntaxKind::R_BRACE);
}

/// BaseExpression = LiteralExpression | FeatureReferenceExpression | InvocationExpression | '(' SequenceExpression ')' | NewExpression | IfExpression
/// Per pest: primary_expression defined in each grammar - this is the base/atomic expression parsing

/// Handle literal values (integers, strings, booleans, null)
fn parse_literal<P: ExpressionParser>(p: &mut P) -> bool {
    if p.at_any(&[
        SyntaxKind::INTEGER, SyntaxKind::DECIMAL, SyntaxKind::STRING,
        SyntaxKind::TRUE_KW, SyntaxKind::FALSE_KW, SyntaxKind::NULL_KW
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
    !matches!(kind,
        SyntaxKind::ERROR | SyntaxKind::WHITESPACE | SyntaxKind::LINE_COMMENT |
        SyntaxKind::BLOCK_COMMENT | SyntaxKind::L_BRACE | SyntaxKind::R_BRACE |
        SyntaxKind::L_BRACKET | SyntaxKind::R_BRACKET | SyntaxKind::L_PAREN |
        SyntaxKind::R_PAREN | SyntaxKind::SEMICOLON | SyntaxKind::COLON |
        SyntaxKind::COLON_COLON | SyntaxKind::COLON_GT | SyntaxKind::COLON_GT_GT |
        SyntaxKind::COLON_COLON_GT | SyntaxKind::DOT | SyntaxKind::DOT_DOT |
        SyntaxKind::COMMA | SyntaxKind::EQ | SyntaxKind::EQ_EQ | SyntaxKind::EQ_EQ_EQ |
        SyntaxKind::BANG_EQ | SyntaxKind::BANG_EQ_EQ | SyntaxKind::LT | SyntaxKind::GT |
        SyntaxKind::LT_EQ | SyntaxKind::GT_EQ | SyntaxKind::AT | SyntaxKind::AT_AT |
        SyntaxKind::HASH | SyntaxKind::STAR | SyntaxKind::STAR_STAR | SyntaxKind::PLUS |
        SyntaxKind::MINUS | SyntaxKind::SLASH | SyntaxKind::PERCENT | SyntaxKind::CARET |
        SyntaxKind::AMP | SyntaxKind::AMP_AMP | SyntaxKind::PIPE | SyntaxKind::PIPE_PIPE |
        SyntaxKind::BANG | SyntaxKind::TILDE | SyntaxKind::QUESTION |
        SyntaxKind::QUESTION_QUESTION | SyntaxKind::ARROW | SyntaxKind::FAT_ARROW |
        SyntaxKind::INTEGER | SyntaxKind::DECIMAL | SyntaxKind::STRING |
        SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NULL_KW
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
        kind if parse_literal(p) => {},
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