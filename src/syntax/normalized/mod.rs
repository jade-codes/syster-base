//! Normalized syntax types for unified symbol extraction.
//!
//! This module provides a language-agnostic view of SysML and KerML syntax,
//! allowing the HIR layer to work with a single set of types instead of
//! duplicating logic for each language variant.
//!
//! The normalized types capture the essential structure needed for symbol
//! extraction while abstracting away language-specific details.

mod definition;
mod imports;
mod usage;

use crate::parser::{
    self, AstNode, Definition as RowanDefinition, DefinitionKind as RowanDefinitionKind, Direction,
    Expression, Import as RowanImport, NamespaceMember, Package as RowanPackage, SourceFile,
    SpecializationKind, Usage as RowanUsage, UsageKind as RowanUsageKind,
};
pub use rowan::TextRange;

// Re-export Direction for use by consumers
pub use crate::parser::Direction as FeatureDirection;

// ============================================================================
// Feature Chain - for dotted references like `engine.power.value`
// ============================================================================

/// A feature chain representing a dotted path like `engine.power.value`
#[derive(Debug, Clone)]
pub struct FeatureChain {
    pub parts: Vec<FeatureChainPart>,
    pub range: Option<TextRange>,
}

/// A single part of a feature chain
#[derive(Debug, Clone)]
pub struct FeatureChainPart {
    pub name: String,
    pub range: Option<TextRange>,
}

impl FeatureChain {
    /// Get the chain as a dotted string
    pub fn as_dotted_string(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

// ============================================================================
// RelTarget - relationship target
// ============================================================================

/// A normalized relationship target - either a simple name or a feature chain.
#[derive(Debug, Clone)]
pub enum RelTarget {
    /// A simple reference like `Vehicle`
    Simple(String),
    /// A feature chain like `engine.power.value`
    Chain(FeatureChain),
}

impl RelTarget {
    /// Get the target name (for simple refs) or the full dotted path (for chains)
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            RelTarget::Simple(s) => std::borrow::Cow::Borrowed(s),
            RelTarget::Chain(chain) => std::borrow::Cow::Owned(chain.as_dotted_string()),
        }
    }

    /// Check if this is a chain reference
    pub fn is_chain(&self) -> bool {
        matches!(self, RelTarget::Chain(_))
    }

    /// Get the chain if this is a chain reference
    pub fn chain(&self) -> Option<&FeatureChain> {
        match self {
            RelTarget::Chain(c) => Some(c),
            _ => None,
        }
    }
}

// ============================================================================
// Normalized Element Types
// ============================================================================

/// A normalized element that can appear in either SysML or KerML files.
#[derive(Debug, Clone)]
pub enum NormalizedElement {
    Package(NormalizedPackage),
    Definition(NormalizedDefinition),
    Usage(NormalizedUsage),
    Import(NormalizedImport),
    Alias(NormalizedAlias),
    Comment(NormalizedComment),
    Dependency(NormalizedDependency),
    Filter(NormalizedFilter),
    Expose(NormalizedExpose),
}

/// A normalized package with its children.
#[derive(Debug, Clone)]
pub struct NormalizedPackage {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub range: Option<TextRange>,
    /// Range of just the name identifier (for semantic tokens and hover)
    pub name_range: Option<TextRange>,
    pub doc: Option<String>,
    pub children: Vec<NormalizedElement>,
}

/// A normalized definition (SysML definition or KerML classifier).
#[derive(Debug, Clone)]
pub struct NormalizedDefinition {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub kind: NormalizedDefKind,
    pub range: Option<TextRange>,
    /// Range of just the name identifier (for semantic tokens and hover)
    pub name_range: Option<TextRange>,
    /// Range of the short name (for hover support on short names)
    pub short_name_range: Option<TextRange>,
    pub doc: Option<String>,
    pub relationships: Vec<NormalizedRelationship>,
    pub children: Vec<NormalizedElement>,
    // Modifiers
    /// Whether the definition has the `abstract` keyword
    pub is_abstract: bool,
    /// Whether the definition has the `variation` keyword
    pub is_variation: bool,
    /// Whether the definition has the `individual` keyword (singleton)
    pub is_individual: bool,
}

/// Multiplicity bounds (lower, upper) where None means unbounded (*)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Multiplicity {
    pub lower: Option<u64>,
    pub upper: Option<u64>,
}

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

/// A normalized usage (SysML usage or KerML feature).
#[derive(Debug, Clone)]
pub struct NormalizedUsage {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub kind: NormalizedUsageKind,
    pub range: Option<TextRange>,
    /// Range of just the name identifier (for semantic tokens and hover)
    pub name_range: Option<TextRange>,
    /// Range of the short name (for hover support on short names)
    pub short_name_range: Option<TextRange>,
    pub doc: Option<String>,
    pub relationships: Vec<NormalizedRelationship>,
    pub children: Vec<NormalizedElement>,
    // Modifiers
    /// Whether the usage has the `abstract` keyword
    pub is_abstract: bool,
    /// Whether the usage has the `variation` keyword  
    pub is_variation: bool,
    /// Whether the usage has the `readonly` keyword
    pub is_readonly: bool,
    /// Whether the usage has the `derived` keyword
    pub is_derived: bool,
    /// Whether the usage (for state) has the `parallel` keyword
    pub is_parallel: bool,
    /// Whether the usage has the `individual` keyword (singleton)
    pub is_individual: bool,
    /// Whether the usage has the `end` keyword (connector end)
    pub is_end: bool,
    /// Whether the usage has the `default` keyword
    pub is_default: bool,
    /// Whether the usage has the `ordered` keyword
    pub is_ordered: bool,
    /// Whether the usage has the `nonunique` keyword
    pub is_nonunique: bool,
    /// Whether the usage has the `portion` keyword
    pub is_portion: bool,
    /// Direction (in, out, inout) for ports and parameters
    pub direction: Option<Direction>,
    /// Multiplicity bounds [lower..upper]
    pub multiplicity: Option<Multiplicity>,
    /// Value expression (e.g., `= 42` or `default "hello"`)
    pub value: Option<ValueExpression>,
}

/// A normalized import statement.
#[derive(Debug, Clone)]
pub struct NormalizedImport {
    pub path: String,
    pub path_range: Option<TextRange>,
    pub range: Option<TextRange>,
    pub is_public: bool,
    /// Filter metadata names from bracket syntax, e.g., `import X::*[@Safety]`
    pub filters: Vec<String>,
}

/// A normalized alias.
#[derive(Debug, Clone)]
pub struct NormalizedAlias {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub target: String,
    pub target_range: Option<TextRange>,
    pub name_range: Option<TextRange>,
    pub range: Option<TextRange>,
}

/// A normalized comment.
#[derive(Debug, Clone)]
pub struct NormalizedComment {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub content: String,
    /// References in the `about` clause
    pub about: Vec<NormalizedRelationship>,
    pub range: Option<TextRange>,
}

/// A normalized dependency (relationships like refinement, derivation, etc.).
#[derive(Debug, Clone)]
pub struct NormalizedDependency {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub sources: Vec<NormalizedRelationship>,
    pub targets: Vec<NormalizedRelationship>,
    /// Additional relationships like prefix metadata (e.g., #refinement, #derivation)
    pub relationships: Vec<NormalizedRelationship>,
    pub range: Option<TextRange>,
}

/// A normalized filter statement (e.g., `filter @Safety;`).
/// Filters restrict which elements are visible from wildcard imports.
#[derive(Debug, Clone)]
pub struct NormalizedFilter {
    /// Simple metadata type names that elements must have (e.g., ["Safety", "Approved"])
    pub metadata_refs: Vec<String>,
    /// All qualified name references in the filter expression with their ranges.
    /// Used for IDE features (hover, go-to-def) on filter expressions.
    pub all_refs: Vec<(String, TextRange)>,
    pub range: Option<TextRange>,
}

/// A normalized expose statement for views (e.g., `expose Vehicle::*;`).
#[derive(Debug, Clone)]
pub struct NormalizedExpose {
    /// The import path
    pub import_path: String,
    /// Whether this is a recursive expose (e.g., `Vehicle::**`)
    pub is_recursive: bool,
    pub range: Option<TextRange>,
}

// ============================================================================
// Normalized Kind Enums
// ============================================================================

/// Normalized definition kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedDefKind {
    Part,
    Item,
    Action,
    Port,
    Attribute,
    Connection,
    Interface,
    Allocation,
    Requirement,
    Constraint,
    State,
    Calculation,
    UseCase,
    AnalysisCase,
    Concern,
    View,
    Viewpoint,
    Rendering,
    Enumeration,
    // KerML specific
    DataType,
    Class,
    Structure,
    Behavior,
    Function,
    Association,
    Metaclass,
    Interaction,
    // Fallback
    Other,
}

/// Normalized usage kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedUsageKind {
    Part,
    Item,
    Action,
    Port,
    Attribute,
    Connection,
    Interface,
    Allocation,
    Requirement,
    Constraint,
    State,
    Calculation,
    Reference,
    Occurrence,
    Flow,
    Transition,
    Accept,
    End, // Connection/interface endpoint
    // Control nodes
    Fork,
    Join,
    Merge,
    Decide,
    // View-related
    View,
    Viewpoint,
    Rendering,
    // KerML: features are treated as usages
    Feature,
    // Fallback
    Other,
}

// ============================================================================
// Normalized Relationship
// ============================================================================

/// A normalized relationship (specialization, typing, subsetting, etc.).
#[derive(Debug, Clone)]
pub struct NormalizedRelationship {
    pub kind: NormalizedRelKind,
    pub target: RelTarget,
    pub range: Option<TextRange>,
}

/// Kinds of relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedRelKind {
    // Core KerML relationships
    Specializes,
    Redefines,
    Subsets,
    TypedBy,
    References,
    Conjugates,
    FeatureChain,
    Expression,

    // State/Transition relationships
    TransitionSource,
    TransitionTarget,
    SuccessionSource,
    SuccessionTarget,

    // Message relationships
    AcceptedMessage,
    AcceptVia,
    SentMessage,
    SendVia,
    SendTo,
    MessageSource,
    MessageTarget,

    // Requirement/Constraint relationships
    Satisfies,
    Verifies,
    Asserts,
    Assumes,
    Requires,

    // Allocation/Connection relationships
    AllocateSource,
    AllocateTo,
    BindSource,
    BindTarget,
    ConnectSource,
    ConnectTarget,
    FlowItem,
    FlowSource,
    FlowTarget,
    InterfaceEnd,

    // Action/Behavior relationships
    Performs,
    Exhibits,
    Includes,

    // Metadata/Documentation relationships
    About,
    Meta,

    // View relationships
    Exposes,
    Renders,
    Filters,

    // Dependency relationships
    DependencySource,
    DependencyTarget,

    // Other
    Crosses,
}

// ============================================================================
// Adapters from Rowan AST
// ============================================================================

/// Extract feature chains from an expression into normalized relationships.
pub(super) fn extract_expression_chains(
    expr: &crate::parser::Expression,
    relationships: &mut Vec<NormalizedRelationship>,
) {
    for chain in expr.feature_chains() {
        if chain.parts.len() == 1 {
            let (name, range) = &chain.parts[0];
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Expression,
                target: RelTarget::Simple(name.clone()),
                range: Some(*range),
            });
        } else {
            let parts: Vec<FeatureChainPart> = chain
                .parts
                .iter()
                .map(|(name, range)| FeatureChainPart {
                    name: name.clone(),
                    range: Some(*range),
                })
                .collect();
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Expression,
                target: RelTarget::Chain(FeatureChain {
                    parts,
                    range: Some(chain.full_range),
                }),
                range: Some(chain.full_range),
            });
        }
    }
}

/// Helper to create a feature chain or simple target from a qualified name
/// Extract a `ValueExpression` from a parser `Expression` node.
///
/// For simple literals (single token), returns a typed variant.
/// For complex expressions, falls back to storing the raw source text.
pub(super) fn extract_value_expression(expr: &crate::parser::Expression) -> ValueExpression {
    use crate::parser::SyntaxKind;

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

pub(super) fn make_chain_or_simple(target_str: &str, qn: &crate::parser::QualifiedName) -> RelTarget {
    if target_str.contains('.') {
        // Get segments with their ranges for proper hover resolution
        let segments_with_ranges = qn.segments_with_ranges();
        let parts: Vec<FeatureChainPart> = segments_with_ranges
            .into_iter()
            .map(|(name, range)| FeatureChainPart {
                name,
                range: Some(range),
            })
            .collect();
        RelTarget::Chain(FeatureChain {
            parts,
            range: Some(qn.syntax().text_range()),
        })
    } else {
        RelTarget::Simple(target_str.to_string())
    }
}

impl NormalizedElement {
    /// Create a normalized element from a rowan NamespaceMember
    pub fn from_rowan(member: &NamespaceMember) -> Self {
        match member {
            NamespaceMember::Package(pkg) => {
                NormalizedElement::Package(NormalizedPackage::from_rowan(pkg))
            }
            NamespaceMember::LibraryPackage(pkg) => {
                // Library packages are treated as regular packages
                NormalizedElement::Package(NormalizedPackage {
                    name: pkg.name().and_then(|n| n.text()),
                    short_name: pkg
                        .name()
                        .and_then(|n| n.short_name())
                        .and_then(|sn| sn.text()),
                    range: Some(pkg.syntax().text_range()),
                    name_range: pkg.name().map(|n| n.syntax().text_range()),
                    doc: parser::extract_doc_comment(pkg.syntax()),
                    children: pkg
                        .body()
                        .map(|b| {
                            b.members()
                                .map(|m| NormalizedElement::from_rowan(&m))
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            }
            NamespaceMember::Definition(def) => {
                NormalizedElement::Definition(NormalizedDefinition::from_rowan(def))
            }
            NamespaceMember::Usage(usage) => {
                NormalizedElement::Usage(NormalizedUsage::from_rowan(usage))
            }
            NamespaceMember::Import(import) => {
                NormalizedElement::Import(NormalizedImport::from_rowan(import))
            }
            NamespaceMember::Alias(alias) => {
                NormalizedElement::Alias(NormalizedAlias::from_rowan(alias))
            }
            NamespaceMember::Dependency(dep) => {
                NormalizedElement::Dependency(NormalizedDependency::from_rowan(dep))
            }
            NamespaceMember::Filter(filter) => NormalizedElement::Filter(NormalizedFilter {
                metadata_refs: filter.metadata_refs(),
                all_refs: filter.all_qualified_refs(),
                range: Some(filter.syntax().text_range()),
            }),
            NamespaceMember::Metadata(meta) => {
                // Convert metadata usage (@Type) to a normalized usage with TypedBy relationship
                // This allows filter imports to match on metadata annotations
                let type_name = meta.target().map(|t| t.to_string()).unwrap_or_default();
                let mut relationships = Vec::new();

                // Add TypedBy for the metadata type (e.g., Rationale, Risk)
                if !type_name.is_empty() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(type_name),
                        range: meta.target().map(|t| t.syntax().text_range()),
                    });
                }

                // Add About relationships for each target in the about clause
                // e.g., `@Rationale about vehicle::engine` -> About(vehicle::engine)
                for qn in meta.about_targets() {
                    let target_str = qn.to_string();
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::About,
                        target: make_chain_or_simple(&target_str, &qn),
                        range: Some(qn.syntax().text_range()),
                    });
                }

                // Extract children from the metadata body (if any)
                let children: Vec<NormalizedElement> = meta
                    .body()
                    .map(|b| {
                        b.members()
                            .map(|m| NormalizedElement::from_rowan(&m))
                            .collect()
                    })
                    .unwrap_or_default();

                NormalizedElement::Usage(NormalizedUsage {
                    name: None, // Metadata usages are anonymous
                    short_name: None,
                    kind: NormalizedUsageKind::Attribute, // Use Attribute for metadata
                    relationships,
                    range: Some(meta.syntax().text_range()),
                    name_range: None,
                    short_name_range: None,
                    doc: None,
                    children,
                    is_abstract: false,
                    is_variation: false,
                    is_readonly: false,
                    is_derived: false,
                    is_parallel: false,
                    is_individual: false,
                    is_end: false,
                    is_default: false,
                    is_ordered: false,
                    is_nonunique: false,
                    is_portion: false,
                    direction: None,
                    multiplicity: None,
                    value: None,
                })
            }
            NamespaceMember::Comment(comment) => {
                // Extract about references
                let mut about = Vec::new();
                for qn in comment.about_targets() {
                    let target_str = qn.to_string();
                    about.push(NormalizedRelationship {
                        kind: NormalizedRelKind::About,
                        target: RelTarget::Simple(target_str),
                        range: Some(qn.syntax().text_range()),
                    });
                }

                NormalizedElement::Comment(NormalizedComment {
                    name: comment.name().and_then(|n| n.text()),
                    short_name: comment
                        .name()
                        .and_then(|n| n.short_name())
                        .and_then(|sn| sn.text()),
                    content: String::new(), // TODO: Extract comment content
                    about,
                    range: Some(comment.syntax().text_range()),
                })
            }
            NamespaceMember::Bind(bind) => {
                // Convert standalone bind to a usage with bind relationships
                NormalizedElement::Usage(NormalizedUsage::from_bind(bind))
            }
            NamespaceMember::Succession(succ) => {
                // Convert standalone succession to a usage with succession relationships
                NormalizedElement::Usage(NormalizedUsage::from_succession(succ))
            }
            NamespaceMember::Transition(trans) => {
                // Convert standalone transition to a usage with transition relationships
                NormalizedElement::Usage(NormalizedUsage::from_transition(trans))
            }
            NamespaceMember::Connector(conn) => {
                // Convert KerML connector to a usage
                NormalizedElement::Usage(NormalizedUsage::from_connector(conn))
            }
            NamespaceMember::ConnectUsage(conn) => {
                // Convert connect usage to a normalized usage with connection relationships
                NormalizedElement::Usage(NormalizedUsage::from_connect_usage(conn))
            }
            NamespaceMember::SendAction(send) => {
                // Convert send action to a usage with its children
                NormalizedElement::Usage(NormalizedUsage::from_send_action(send))
            }
            NamespaceMember::AcceptAction(accept) => {
                // Convert accept action to a usage
                NormalizedElement::Usage(NormalizedUsage::from_accept_action(accept))
            }
            NamespaceMember::StateSubaction(subaction) => {
                // Convert state subaction (entry/do/exit) to a usage
                NormalizedElement::Usage(NormalizedUsage::from_state_subaction(subaction))
            }
            NamespaceMember::ControlNode(node) => {
                // Convert control node (fork/join/merge/decide) to a usage
                NormalizedElement::Usage(NormalizedUsage::from_control_node(node))
            }
            NamespaceMember::ForLoop(for_loop) => {
                // Convert for loop to a usage with loop variable as a child
                NormalizedElement::Usage(NormalizedUsage::from_for_loop(for_loop))
            }
            NamespaceMember::IfAction(if_action) => {
                // Convert if action to a usage with expression refs
                NormalizedElement::Usage(NormalizedUsage::from_if_action(if_action))
            }
            NamespaceMember::WhileLoop(while_loop) => {
                // Convert while loop to a usage with expression refs
                NormalizedElement::Usage(NormalizedUsage::from_while_loop(while_loop))
            }
        }
    }
}


// Iteration helpers for normalized files
// ============================================================================

/// An iterator over normalized elements from a rowan SourceFile.
pub struct RowanNormalizedIter {
    members: Vec<NamespaceMember>,
    index: usize,
}

impl RowanNormalizedIter {
    pub fn new(file: &SourceFile) -> Self {
        Self {
            members: file.members().collect(),
            index: 0,
        }
    }
}

impl Iterator for RowanNormalizedIter {
    type Item = NormalizedElement;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.members.len() {
            let member = &self.members[self.index];
            self.index += 1;
            Some(NormalizedElement::from_rowan(member))
        } else {
            None
        }
    }
}

// Legacy type aliases for backwards compatibility during migration
pub type SysMLNormalizedIter = RowanNormalizedIter;
pub type KerMLNormalizedIter = RowanNormalizedIter;
