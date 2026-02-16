use super::*;

// ============================================================================
// ValueExpression — typed literal extracted from an Expression AST node
// ============================================================================

/// A value expression assigned to a feature (e.g., `= 42`, `= "hello"`, `= true`).
#[derive(Debug, Clone, PartialEq)]
pub enum ValueExpression {
    /// Integer literal (e.g., `100`)
    LiteralInteger(i64),
    /// Real/decimal literal (e.g., `0.75`)
    LiteralReal(f64),
    /// String literal (e.g., `"temperature-01"`) — stored without quotes
    LiteralString(String),
    /// Boolean literal (`true` or `false`)
    LiteralBoolean(bool),
    /// Null literal
    Null,
    /// A non-literal expression, stored as raw source text
    Expression(String),
}

// Manual Eq impl because f64 doesn't implement Eq (NaN != NaN).
// We treat two LiteralReal values as equal when their bit patterns match.
impl Eq for ValueExpression {}

// Manual Hash impl consistent with the Eq impl above.
impl std::hash::Hash for ValueExpression {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ValueExpression::LiteralInteger(v) => v.hash(state),
            ValueExpression::LiteralReal(v) => v.to_bits().hash(state),
            ValueExpression::LiteralString(v) => v.hash(state),
            ValueExpression::LiteralBoolean(v) => v.hash(state),
            ValueExpression::Null => {}
            ValueExpression::Expression(v) => v.hash(state),
        }
    }
}

/// Extract a `ValueExpression` from a parser `Expression` node.
///
/// For simple literals (single token), returns a typed variant.
/// For complex expressions, falls back to storing the raw source text.
pub fn extract_value_expression(expr: &Expression) -> ValueExpression {
    let syntax = expr.syntax();
    // Collect non-trivia tokens from the expression
    let mut tokens = syntax
        .descendants_with_tokens()
        .filter_map(|el| el.into_token())
        .filter(|t| !t.kind().is_trivia());

    if let Some(token) = tokens.next() {
        // If there's only one non-trivia token, it's a simple literal
        let is_single = tokens.next().is_none();
        if is_single {
            match token.kind() {
                SyntaxKind::INTEGER => {
                    if let Ok(v) = token.text().parse::<i64>() {
                        return ValueExpression::LiteralInteger(v);
                    }
                }
                SyntaxKind::DECIMAL => {
                    if let Ok(v) = token.text().parse::<f64>() {
                        return ValueExpression::LiteralReal(v);
                    }
                }
                SyntaxKind::STRING => {
                    let text = token.text();
                    // Strip surrounding quotes
                    let inner = if (text.starts_with('"') && text.ends_with('"'))
                        || (text.starts_with('\'') && text.ends_with('\''))
                    {
                        &text[1..text.len() - 1]
                    } else {
                        text
                    };
                    return ValueExpression::LiteralString(inner.to_string());
                }
                SyntaxKind::TRUE_KW => return ValueExpression::LiteralBoolean(true),
                SyntaxKind::FALSE_KW => return ValueExpression::LiteralBoolean(false),
                SyntaxKind::NULL_KW => return ValueExpression::Null,
                _ => {}
            }
        }
    }
    // Fallback: store the full expression text
    ValueExpression::Expression(syntax.text().to_string().trim().to_string())
}

// ============================================================================
// Multiplicity bounds
// ============================================================================

/// Multiplicity bounds (lower, upper) where None means unbounded (*)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Multiplicity {
    pub lower: Option<u64>,
    pub upper: Option<u64>,
}

// Expression
// ============================================================================

ast_node!(Expression, EXPRESSION);

/// A feature chain like `fuelTank.mass` with individual part ranges
#[derive(Debug, Clone)]
pub struct FeatureChainRef {
    /// The parts of the chain (e.g., ["fuelTank", "mass"])
    pub parts: Vec<(String, rowan::TextRange)>,
    /// The full range of the chain
    pub full_range: rowan::TextRange,
}

impl Expression {
    /// Extract all identifier references from this expression
    /// Returns pairs of (identifier_name, text_range)
    pub fn references(&self) -> Vec<(String, rowan::TextRange)> {
        let mut refs = Vec::new();
        self.collect_references(&self.0, &mut refs);
        refs
    }

    /// Extract feature chains from this expression.
    /// A feature chain is a sequence of identifiers separated by `.` (e.g., `fuelTank.mass`).
    /// Returns each chain with its parts and their individual ranges.
    pub fn feature_chains(&self) -> Vec<FeatureChainRef> {
        let mut chains = Vec::new();
        self.collect_feature_chains(&self.0, &mut chains);
        chains
    }

    /// Extract named constructor arguments from `new Type(argName = value)` patterns.
    /// Returns tuples of (type_name, arg_name, arg_name_range).
    /// The arg_name should resolve as Type.argName (a feature of the constructed type).
    pub fn named_constructor_args(&self) -> Vec<(String, String, rowan::TextRange)> {
        let mut results = Vec::new();
        self.collect_named_constructor_args(&self.0, &mut results);
        results
    }

    fn collect_named_constructor_args(
        &self,
        node: &SyntaxNode,
        results: &mut Vec<(String, String, rowan::TextRange)>,
    ) {
        // Look for pattern: NEW_KW followed by QUALIFIED_NAME then ARGUMENT_LIST
        let children: Vec<_> = node.children_with_tokens().collect();

        for (i, child) in children.iter().enumerate() {
            if child.as_token().map(|t| t.kind()) == Some(SyntaxKind::NEW_KW) {
                // Find the type name and argument list after NEW_KW
                let rest = &children[i + 1..];
                let type_name = rest
                    .iter()
                    .filter_map(|c| c.as_node())
                    .find(|n| n.kind() == SyntaxKind::QUALIFIED_NAME)
                    .map(|n| n.text().to_string());

                if let Some(type_name) = type_name {
                    for arg_list in rest
                        .iter()
                        .filter_map(|c| c.as_node())
                        .filter(|n| n.kind() == SyntaxKind::ARGUMENT_LIST)
                    {
                        self.extract_named_args_from_list(arg_list, &type_name, results);
                    }
                }
            }
        }

        // Recurse into child nodes
        for child in node.children() {
            self.collect_named_constructor_args(&child, results);
        }
    }

    fn extract_named_args_from_list(
        &self,
        arg_list: &SyntaxNode,
        type_name: &str,
        results: &mut Vec<(String, String, rowan::TextRange)>,
    ) {
        for child in arg_list
            .children()
            .filter(|c| c.kind() == SyntaxKind::ARGUMENT_LIST)
        {
            // Look for IDENT followed by EQ (named argument pattern)
            let tokens: Vec<_> = child.children_with_tokens().collect();

            for (idx, elem) in tokens.iter().enumerate() {
                if let Some(token) = elem.as_token().filter(|t| t.kind() == SyntaxKind::IDENT) {
                    // Check if next non-whitespace is EQ
                    let has_eq = tokens[idx + 1..]
                        .iter()
                        .filter_map(|e| e.as_token())
                        .find(|t| t.kind() != SyntaxKind::WHITESPACE)
                        .map(|t| t.kind() == SyntaxKind::EQ)
                        .unwrap_or(false);

                    if has_eq {
                        results.push((
                            type_name.to_string(),
                            token.text().to_string(),
                            token.text_range(),
                        ));
                    }
                }
            }

            // Recurse into nested argument lists
            self.extract_named_args_from_list(&child, type_name, results);
        }
    }

    fn collect_feature_chains(&self, node: &SyntaxNode, chains: &mut Vec<FeatureChainRef>) {
        if node.kind() == SyntaxKind::QUALIFIED_NAME {
            let parts: Vec<_> = node
                .children_with_tokens()
                .filter_map(|c| c.into_token())
                .filter(|t| t.kind() == SyntaxKind::IDENT)
                .map(|t| (strip_unrestricted_name(t.text()), t.text_range()))
                .collect();

            if !parts.is_empty() {
                chains.push(FeatureChainRef {
                    parts,
                    full_range: node.text_range(),
                });
            }
            return;
        }

        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::IDENT => {
                    chains.push(FeatureChainRef {
                        parts: vec![(strip_unrestricted_name(t.text()), t.text_range())],
                        full_range: t.text_range(),
                    });
                }
                rowan::NodeOrToken::Node(n) => self.collect_feature_chains(&n, chains),
                _ => {}
            }
        }
    }

    fn collect_references(&self, node: &SyntaxNode, refs: &mut Vec<(String, rowan::TextRange)>) {
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::IDENT => {
                    refs.push((strip_unrestricted_name(t.text()), t.text_range()));
                }
                rowan::NodeOrToken::Node(n) => self.collect_references(&n, refs),
                _ => {}
            }
        }
    }
}

// ============================================================================
