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

use crate::syntax::rowan_parser::syntax_kind::SyntaxKind;

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
}

/// Parse an expression, returning true if any tokens were consumed
pub fn parse_expression<P: ExpressionParser>(p: &mut P) -> bool {
    let start_pos = p.get_pos();
    parse_conditional_expression(p);
    p.get_pos() > start_pos
}

/// ConditionalExpression = NullCoalescingExpression ('?' Expression ':' Expression)?
pub fn parse_conditional_expression<P: ExpressionParser>(p: &mut P) {
    let start_pos = p.get_pos();
    p.start_node(SyntaxKind::EXPRESSION);
    
    parse_null_coalescing_expression(p);
    
    // Only continue if we consumed something
    if p.get_pos() > start_pos {
        p.skip_trivia();
        if p.at(SyntaxKind::QUESTION) && !p.at(SyntaxKind::QUESTION_QUESTION) {
            p.bump(); // ?
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
            p.expect(SyntaxKind::COLON);
            p.skip_trivia();
            parse_expression(p);
        }
    }
    
    p.finish_node();
}

/// NullCoalescingExpression = ImpliesExpression ('??' ImpliesExpression)*
pub fn parse_null_coalescing_expression<P: ExpressionParser>(p: &mut P) {
    parse_implies_expression(p);
    
    while p.at(SyntaxKind::QUESTION_QUESTION) {
        p.bump();
        p.skip_trivia();
        parse_implies_expression(p);
    }
}

/// ImpliesExpression = OrExpression ('implies' OrExpression)*
pub fn parse_implies_expression<P: ExpressionParser>(p: &mut P) {
    parse_or_expression(p);
    
    while p.at(SyntaxKind::IMPLIES_KW) {
        p.bump();
        p.skip_trivia();
        parse_or_expression(p);
    }
}

/// OrExpression = XorExpression (('|' | 'or') XorExpression)*
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
pub fn parse_classification_expression<P: ExpressionParser>(p: &mut P) {
    parse_relational_expression(p);
    
    p.skip_trivia();
    if p.at_any(&[SyntaxKind::HASTYPE_KW, SyntaxKind::ISTYPE_KW, SyntaxKind::AS_KW, SyntaxKind::META_KW, SyntaxKind::AT, SyntaxKind::AT_AT]) {
        p.bump();
        p.skip_trivia();
        p.parse_qualified_name();
    }
}

/// RelationalExpression = RangeExpression (('<' | '>' | '<=' | '>=') RangeExpression)*
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
pub fn parse_additive_expression<P: ExpressionParser>(p: &mut P) {
    parse_multiplicative_expression(p);
    
    while p.at(SyntaxKind::PLUS) || p.at(SyntaxKind::MINUS) {
        p.bump();
        p.skip_trivia();
        parse_multiplicative_expression(p);
    }
}

/// MultiplicativeExpression = ExponentiationExpression (('*' | '/' | '%') ExponentiationExpression)*
pub fn parse_multiplicative_expression<P: ExpressionParser>(p: &mut P) {
    parse_exponentiation_expression(p);
    
    while p.at_any(&[SyntaxKind::STAR, SyntaxKind::SLASH, SyntaxKind::PERCENT]) {
        p.bump();
        p.skip_trivia();
        parse_exponentiation_expression(p);
    }
}

/// ExponentiationExpression = UnaryExpression (('**' | '^') ExponentiationExpression)?
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
pub fn parse_unary_expression<P: ExpressionParser>(p: &mut P) {
    if p.at_any(&[SyntaxKind::PLUS, SyntaxKind::MINUS, SyntaxKind::TILDE, SyntaxKind::NOT_KW]) {
        p.bump();
        p.skip_trivia();
    }
    parse_extent_expression(p);
}

/// ExtentExpression = ('all')? PrimaryExpression
pub fn parse_extent_expression<P: ExpressionParser>(p: &mut P) {
    if p.at(SyntaxKind::ALL_KW) {
        p.bump();
        p.skip_trivia();
    }
    parse_primary_expression(p);
}

/// PrimaryExpression = BaseExpression (FeatureChain | ArrowInvocation)*
pub fn parse_primary_expression<P: ExpressionParser>(p: &mut P) {
    parse_base_expression(p);
    
    loop {
        p.skip_trivia();
        
        // Feature chains (.name or .name())
        if p.at(SyntaxKind::DOT) {
            p.bump(); // .
            p.skip_trivia();
            if p.at(SyntaxKind::IDENT) {
                p.bump();
            }
            p.skip_trivia();
            // Check for invocation
            if p.at(SyntaxKind::L_PAREN) {
                parse_argument_list(p);
            }
        }
        // Arrow invocation (->name or ->name{})
        else if p.at(SyntaxKind::ARROW) {
            p.bump(); // ->
            p.skip_trivia();
            // Method name (collect, select, forAll, exists, etc.)
            if p.at(SyntaxKind::IDENT) {
                p.bump();
            }
            p.skip_trivia();
            // Optional body {} or argument list ()
            if p.at(SyntaxKind::L_BRACE) {
                parse_arrow_body(p);
            } else if p.at(SyntaxKind::L_PAREN) {
                parse_argument_list(p);
            }
        }
        // Bracket index (#(n) or [n])
        else if p.at(SyntaxKind::HASH) {
            p.bump(); // #
            p.skip_trivia();
            if p.at(SyntaxKind::L_PAREN) {
                p.bump(); // (
                p.skip_trivia();
                parse_expression(p);
                p.skip_trivia();
                p.expect(SyntaxKind::R_PAREN);
            }
        }
        else if p.at(SyntaxKind::L_BRACKET) {
            p.bump(); // [
            p.skip_trivia();
            parse_expression(p);
            p.skip_trivia();
            p.expect(SyntaxKind::R_BRACKET);
        }
        else {
            break;
        }
    }
}

/// ArrowBody = '{' (Parameter* Expression)? '}'
/// Used for arrow invocation bodies like: ->collect { in x; x + 1 }
pub fn parse_arrow_body<P: ExpressionParser>(p: &mut P) {
    p.expect(SyntaxKind::L_BRACE);
    p.skip_trivia();
    
    let token_count = 10000; // Safety limit
    let mut iterations = 0;
    
    while !p.at(SyntaxKind::R_BRACE) && iterations < token_count {
        iterations += 1;
        let start_pos = p.get_pos();
        
        // Check for parameter keywords
        if p.at_any(&[SyntaxKind::IN_KW, SyntaxKind::OUT_KW, SyntaxKind::INOUT_KW]) {
            p.bump(); // in/out/inout
            p.skip_trivia();
            if p.at_name_token() {
                p.bump(); // parameter name
            }
            p.skip_trivia();
            // Optional type
            if p.at(SyntaxKind::COLON) {
                p.bump();
                p.skip_trivia();
                p.parse_qualified_name();
            }
            p.skip_trivia();
            if p.at(SyntaxKind::SEMICOLON) {
                p.bump();
                p.skip_trivia();
            }
        } else {
            // Body expression
            parse_expression(p);
            p.skip_trivia();
            break;
        }
        
        // Safety: if we didn't make progress, skip to avoid infinite loop
        if p.get_pos() == start_pos && !p.at(SyntaxKind::R_BRACE) {
            p.bump_any();
        }
    }
    
    p.expect(SyntaxKind::R_BRACE);
}

/// BaseExpression = LiteralExpression | FeatureReferenceExpression | InvocationExpression | '(' SequenceExpression ')' | NewExpression | IfExpression
pub fn parse_base_expression<P: ExpressionParser>(p: &mut P) {
    p.skip_trivia();
    
    match p.current_kind() {
        // Literals
        SyntaxKind::INTEGER | SyntaxKind::DECIMAL | SyntaxKind::STRING => {
            p.bump();
        }
        SyntaxKind::TRUE_KW | SyntaxKind::FALSE_KW | SyntaxKind::NULL_KW => {
            p.bump();
        }
        
        // Instantiation expression: new Foo() or new Foo(x, y)
        SyntaxKind::NEW_KW => {
            p.bump(); // new
            p.skip_trivia();
            p.parse_qualified_name(); // Type name
            p.skip_trivia();
            // Optional argument list
            if p.at(SyntaxKind::L_PAREN) {
                parse_argument_list(p);
            }
        }
        
        // If expression: if cond then expr else expr
        SyntaxKind::IF_KW => {
            p.bump(); // if
            p.skip_trivia();
            parse_expression(p); // condition
            p.skip_trivia();
            if p.at(SyntaxKind::THEN_KW) {
                p.bump(); // then
                p.skip_trivia();
                parse_expression(p); // then branch
                p.skip_trivia();
                if p.at(SyntaxKind::ELSE_KW) {
                    p.bump(); // else
                    p.skip_trivia();
                    parse_expression(p); // else branch
                }
            }
        }
        
        // Parenthesized expression or sequence
        SyntaxKind::L_PAREN => {
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
        
        // Feature reference or invocation (includes keywords that can be used as identifiers in expressions)
        SyntaxKind::IDENT | SyntaxKind::END_KW |
        SyntaxKind::PART_KW | SyntaxKind::ATTRIBUTE_KW | SyntaxKind::PORT_KW |
        SyntaxKind::ITEM_KW | SyntaxKind::ACTION_KW | SyntaxKind::STATE_KW |
        SyntaxKind::CONSTRAINT_KW | SyntaxKind::REQUIREMENT_KW | SyntaxKind::CASE_KW |
        SyntaxKind::CALC_KW | SyntaxKind::CONNECTION_KW | SyntaxKind::INTERFACE_KW |
        SyntaxKind::ALLOCATION_KW | SyntaxKind::FLOW_KW | SyntaxKind::VIEW_KW |
        SyntaxKind::OCCURRENCE_KW | SyntaxKind::INDIVIDUAL_KW | SyntaxKind::METADATA_KW => {
            p.parse_qualified_name();
            p.skip_trivia();
            
            // Check for invocation
            if p.at(SyntaxKind::L_PAREN) {
                parse_argument_list(p);
            }
        }
        
        // Metadata access expression
        SyntaxKind::AT => {
            p.bump(); // @
            p.skip_trivia();
            p.parse_qualified_name();
        }
        
        _ => {
            // Unknown - don't consume
        }
    }
}

/// ArgumentList = '(' (Argument (',' Argument)*)? ')'
pub fn parse_argument_list<P: ExpressionParser>(p: &mut P) {
    p.start_node(SyntaxKind::ARGUMENT_LIST);
    
    p.expect(SyntaxKind::L_PAREN);
    p.skip_trivia();
    
    if !p.at(SyntaxKind::R_PAREN) {
        parse_argument(p);
        p.skip_trivia();
        
        while p.at(SyntaxKind::COMMA) {
            p.bump();
            p.skip_trivia();
            parse_argument(p);
            p.skip_trivia();
        }
    }
    
    p.expect(SyntaxKind::R_PAREN);
    
    p.finish_node();
}

/// Argument = (Name '=')? Expression
/// Note: Named argument detection requires lookahead which needs access to token stream
/// This is handled by the main parser which implements ExpressionParser
pub fn parse_argument<P: ExpressionParser>(p: &mut P) {
    // The named argument check (name = value) requires lookahead,
    // which we delegate to the main parser via parse_argument_with_lookahead
    // For now, just parse the expression
    parse_expression(p);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::rowan_parser::parser::parse;
    
    #[test]
    fn test_simple_expression() {
        let result = parse("constraint { 1 + 2 }");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_comparison_expression() {
        let result = parse("constraint { x > 0 }");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_conditional_expression() {
        let result = parse("constraint { x > 0 ? y : z }");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
    
    #[test]
    fn test_arrow_invocation() {
        let result = parse("constraint { items->collect { in x; x + 1 } }");
        assert!(result.ok(), "errors: {:?}", result.errors);
    }
}
