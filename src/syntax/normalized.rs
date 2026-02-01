//! Normalized syntax types for unified symbol extraction.
//!
//! This module provides a language-agnostic view of SysML and KerML syntax,
//! allowing the HIR layer to work with a single set of types instead of
//! duplicating logic for each language variant.
//!
//! The normalized types capture the essential structure needed for symbol
//! extraction while abstracting away language-specific details.

use crate::parser::{
    self, AstNode, Definition as RowanDefinition, DefinitionKind as RowanDefinitionKind,
    Expression, Import as RowanImport, NamespaceMember, Package as RowanPackage, SourceFile,
    SpecializationKind, Usage as RowanUsage, UsageKind as RowanUsageKind,
};
pub use rowan::TextRange;

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
    // Control nodes
    Fork,
    Join,
    Merge,
    Decide,
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

/// Helper to create a feature chain or simple target from a qualified name
fn make_chain_or_simple(target_str: &str, qn: &crate::parser::QualifiedName) -> RelTarget {
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
                    short_name: pkg.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
                    range: Some(pkg.syntax().text_range()),
                    name_range: pkg.name().map(|n| n.syntax().text_range()),
                    doc: parser::extract_doc_comment(pkg.syntax()),
                    children: pkg
                        .body()
                        .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
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
                NormalizedElement::Dependency(NormalizedDependency::from_rowan(&dep))
            }
            NamespaceMember::Filter(filter) => {
                NormalizedElement::Filter(NormalizedFilter {
                    metadata_refs: filter.metadata_refs(),
                    all_refs: filter.all_qualified_refs(),
                    range: Some(filter.syntax().text_range()),
                })
            }
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
                let children: Vec<NormalizedElement> = meta.body()
                    .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
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
                    short_name: comment.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
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
                NormalizedElement::Usage(NormalizedUsage::from_connector(&conn))
            }
            NamespaceMember::ConnectUsage(conn) => {
                // Convert connect usage to a normalized usage with connection relationships
                NormalizedElement::Usage(NormalizedUsage::from_connect_usage(&conn))
            }
            NamespaceMember::SendAction(send) => {
                // Convert send action to a usage with its children
                NormalizedElement::Usage(NormalizedUsage::from_send_action(&send))
            }
            NamespaceMember::AcceptAction(accept) => {
                // Convert accept action to a usage
                NormalizedElement::Usage(NormalizedUsage::from_accept_action(&accept))
            }
            NamespaceMember::StateSubaction(subaction) => {
                // Convert state subaction (entry/do/exit) to a usage
                NormalizedElement::Usage(NormalizedUsage::from_state_subaction(&subaction))
            }
            NamespaceMember::ControlNode(node) => {
                // Convert control node (fork/join/merge/decide) to a usage
                NormalizedElement::Usage(NormalizedUsage::from_control_node(&node))
            }
        }
    }
}

impl NormalizedPackage {
    fn from_rowan(pkg: &RowanPackage) -> Self {
        Self {
            name: pkg.name().and_then(|n| n.text()),
            short_name: pkg.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
            range: Some(pkg.syntax().text_range()),
            name_range: pkg.name().map(|n| n.syntax().text_range()),
            doc: parser::extract_doc_comment(pkg.syntax()),
            children: pkg
                .body()
                .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
                .unwrap_or_default(),
        }
    }
}

impl NormalizedDefinition {
    fn from_rowan(def: &RowanDefinition) -> Self {
        let kind = match def.definition_kind() {
            Some(RowanDefinitionKind::Part) => NormalizedDefKind::Part,
            Some(RowanDefinitionKind::Item) => NormalizedDefKind::Item,
            Some(RowanDefinitionKind::Action) => NormalizedDefKind::Action,
            Some(RowanDefinitionKind::Port) => NormalizedDefKind::Port,
            Some(RowanDefinitionKind::Attribute) => NormalizedDefKind::Attribute,
            Some(RowanDefinitionKind::Connection) => NormalizedDefKind::Connection,
            Some(RowanDefinitionKind::Interface) => NormalizedDefKind::Interface,
            Some(RowanDefinitionKind::Allocation) => NormalizedDefKind::Allocation,
            Some(RowanDefinitionKind::Requirement) => NormalizedDefKind::Requirement,
            Some(RowanDefinitionKind::Constraint) => NormalizedDefKind::Constraint,
            Some(RowanDefinitionKind::State) => NormalizedDefKind::State,
            Some(RowanDefinitionKind::Calc) => NormalizedDefKind::Calculation,
            Some(RowanDefinitionKind::Case) | Some(RowanDefinitionKind::UseCase) => {
                NormalizedDefKind::UseCase
            }
            Some(RowanDefinitionKind::Analysis) | Some(RowanDefinitionKind::Verification) => {
                NormalizedDefKind::AnalysisCase
            }
            Some(RowanDefinitionKind::Concern) => NormalizedDefKind::Concern,
            Some(RowanDefinitionKind::View) => NormalizedDefKind::View,
            Some(RowanDefinitionKind::Viewpoint) => NormalizedDefKind::Viewpoint,
            Some(RowanDefinitionKind::Rendering) => NormalizedDefKind::Rendering,
            Some(RowanDefinitionKind::Enum) => NormalizedDefKind::Enumeration,
            Some(RowanDefinitionKind::Flow) => NormalizedDefKind::Other, // Map flow def to Other
            Some(RowanDefinitionKind::Metadata) => NormalizedDefKind::Other,
            Some(RowanDefinitionKind::Occurrence) => NormalizedDefKind::Other,
            // KerML mappings to SysML equivalents
            Some(RowanDefinitionKind::Class) => NormalizedDefKind::Part,  // class -> part def
            Some(RowanDefinitionKind::Struct) => NormalizedDefKind::Part, // struct -> part def
            Some(RowanDefinitionKind::Datatype) => NormalizedDefKind::Attribute, // datatype -> attribute def
            Some(RowanDefinitionKind::Assoc) => NormalizedDefKind::Connection, // assoc -> connection def
            Some(RowanDefinitionKind::Behavior) => NormalizedDefKind::Action, // behavior -> action def
            Some(RowanDefinitionKind::Function) => NormalizedDefKind::Calculation, // function -> calc def
            Some(RowanDefinitionKind::Predicate) => NormalizedDefKind::Constraint, // predicate -> constraint def
            Some(RowanDefinitionKind::Interaction) => NormalizedDefKind::Action, // interaction -> action def
            Some(RowanDefinitionKind::Classifier) => NormalizedDefKind::Part, // classifier -> part def
            Some(RowanDefinitionKind::Type) => NormalizedDefKind::Other, // type -> other
            Some(RowanDefinitionKind::Metaclass) => NormalizedDefKind::Metaclass, // metaclass -> metaclass
            None => NormalizedDefKind::Other,
        };

        // Extract relationships from specializations
        let mut relationships: Vec<NormalizedRelationship> = def
            .specializations()
            .filter_map(|spec| {
                // If kind is None but target exists, it's a comma-separated continuation
                // Default to Specializes since `:> A, B, C` means A, B, C all specialize
                let rel_kind = match spec.kind() {
                    Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                    Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                    Some(SpecializationKind::References) => NormalizedRelKind::References,
                    Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::FeatureChain) => NormalizedRelKind::Specializes,
                    None => NormalizedRelKind::Specializes, // Comma-continuation inherits Specializes
                };
                let target_node = spec.target()?;
                let target = target_node.to_string();
                Some(NormalizedRelationship {
                    kind: rel_kind,
                    target: RelTarget::Simple(target),
                    range: Some(target_node.syntax().text_range()),
                })
            })
            .collect();
        
        // Extract expression references from ALL expressions in this definition
        // (e.g., constraint def bodies)
        for expr in def.descendants::<Expression>() {
            let chains = expr.feature_chains();
            for chain in chains {
                if chain.parts.len() == 1 {
                    let (name, range) = &chain.parts[0];
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Expression,
                        target: RelTarget::Simple(name.clone()),
                        range: Some(*range),
                    });
                } else {
                    let parts: Vec<FeatureChainPart> = chain.parts.iter()
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

        // Extract children from body
        let children: Vec<NormalizedElement> = def
            .body()
            .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
            .unwrap_or_default();

        Self {
            name: def.name().and_then(|n| n.text()),
            short_name: def.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
            kind,
            range: Some(def.syntax().text_range()),
            name_range: def.name().map(|n| n.syntax().text_range()),
            short_name_range: def
                .name()
                .and_then(|n| n.short_name())
                .map(|sn| sn.syntax().text_range()),
            doc: parser::extract_doc_comment(def.syntax()),
            relationships,
            children,
        }
    }
}

impl NormalizedUsage {
    fn from_rowan(usage: &RowanUsage) -> Self {
        // Determine usage kind based on keyword tokens
        // Check for nested transition first, then perform action
        let kind = if usage.transition_usage().is_some() {
            NormalizedUsageKind::Transition
        } else if usage.perform_action_usage().is_some() {
            // perform action => Action kind
            NormalizedUsageKind::Action
        } else {
            match usage.usage_kind() {
                Some(RowanUsageKind::Part) => NormalizedUsageKind::Part,
                Some(RowanUsageKind::Attribute) => NormalizedUsageKind::Attribute,
                Some(RowanUsageKind::Port) => NormalizedUsageKind::Port,
                Some(RowanUsageKind::Item) => NormalizedUsageKind::Item,
                Some(RowanUsageKind::Action) => NormalizedUsageKind::Action,
                Some(RowanUsageKind::State) => NormalizedUsageKind::State,
                Some(RowanUsageKind::Constraint) => NormalizedUsageKind::Constraint,
                Some(RowanUsageKind::Requirement) => NormalizedUsageKind::Requirement,
                Some(RowanUsageKind::Calc) => NormalizedUsageKind::Calculation,
                Some(RowanUsageKind::Connection) => NormalizedUsageKind::Connection,
                Some(RowanUsageKind::Interface) => NormalizedUsageKind::Interface,
                Some(RowanUsageKind::Allocation) => NormalizedUsageKind::Allocation,
                Some(RowanUsageKind::Flow) => NormalizedUsageKind::Flow,
                Some(RowanUsageKind::Occurrence) => NormalizedUsageKind::Occurrence,
                Some(RowanUsageKind::Ref) => NormalizedUsageKind::Reference,
                // KerML mappings
                Some(RowanUsageKind::Feature) => NormalizedUsageKind::Attribute, // feature -> attribute
                Some(RowanUsageKind::Step) => NormalizedUsageKind::Action, // step -> action
                Some(RowanUsageKind::Expr) => NormalizedUsageKind::Calculation, // expr -> calc
                Some(RowanUsageKind::Connector) => NormalizedUsageKind::Connection, // connector -> connection
                Some(RowanUsageKind::Case) => NormalizedUsageKind::Other,
                None => NormalizedUsageKind::Part, // Default to Part for usages without keyword
            }
        };

        // Extract typing as a relationship
        let mut relationships: Vec<NormalizedRelationship> = Vec::new();

        // Check for typing on the usage itself, or on nested perform action
        let typing = usage.typing().or_else(|| {
            usage.perform_action_usage().and_then(|p| p.typing())
        });
        if let Some(typing) = typing {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }
        
        // Extract "of Type" clause (for messages, items, etc.)
        // e.g., `message sendCmd of SensedSpeed`
        if let Some(of_type) = usage.of_type() {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::TypedBy,
                target: RelTarget::Simple(of_type.to_string()),
                range: Some(of_type.syntax().text_range()),
            });
        }

        // Extract specializations
        for spec in usage.specializations() {
            // If kind is None but target exists, it's a comma-separated continuation
            // Default to Subsets since `:> A, B, C` in usages means subsetting
            let rel_kind = match spec.kind() {
                Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                Some(SpecializationKind::References) => NormalizedRelKind::References,
                Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::FeatureChain) => NormalizedRelKind::FeatureChain,
                None => NormalizedRelKind::Subsets, // Comma-continuation inherits Subsets for usages
            };
            if let Some(target) = spec.target() {
                let target_str = target.to_string();
                let target_range = target.syntax().text_range();
                // Check if this is a feature chain (contains .)
                let rel_target = if target_str.contains('.') {
                    // Parse as chain
                    let parts: Vec<FeatureChainPart> = target_str
                        .split('.')
                        .map(|s| FeatureChainPart {
                            name: s.to_string(),
                            range: None, // TODO: compute individual ranges
                        })
                        .collect();
                    RelTarget::Chain(FeatureChain {
                        parts,
                        range: Some(target_range),
                    })
                } else {
                    RelTarget::Simple(target_str)
                };
                relationships.push(NormalizedRelationship {
                    kind: rel_kind,
                    target: rel_target,
                    range: Some(target_range),
                });
            }
        }

        // Extract expression references from ALL expressions in this usage
        // This covers: value expressions, constraint bodies, and any other nested expressions
        for expr in usage.descendants::<Expression>() {
            // Use feature_chains() to properly extract chains like `fuelTank.mass`
            for chain in expr.feature_chains() {
                if chain.parts.len() == 1 {
                    // Single identifier - add as Simple
                    let (name, range) = &chain.parts[0];
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Expression,
                        target: RelTarget::Simple(name.clone()),
                        range: Some(*range),
                    });
                } else {
                    // Multi-part chain - add as Chain
                    let parts: Vec<FeatureChainPart> = chain.parts.iter()
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

        // Extract from-to clause for message/flow usages (e.g., `from driver.turnVehicleOn to vehicle.trigger1`)
        if let Some(from_to) = usage.from_to_clause() {
            // Extract source chain
            if let Some(source) = from_to.source() {
                if let Some(qn) = source.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FlowSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            
            // Extract target chain
            if let Some(target) = from_to.target() {
                if let Some(qn) = target.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FlowTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }

        // Extract transition source/target (e.g., `transition initial then off`)
        if let Some(transition) = usage.transition_usage() {
            if let Some(source_spec) = transition.source() {
                if let Some(qn) = source_spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TransitionSource,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            if let Some(target_spec) = transition.target() {
                if let Some(qn) = target_spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TransitionTarget,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract succession source/target (e.g., `first start then run then stop`)
        if let Some(succession) = usage.succession() {
            let items: Vec<_> = succession.items().collect();
            if let Some(first) = items.first() {
                if let Some(qn) = first.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::SuccessionSource,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            // All subsequent items are targets
            for item in items.iter().skip(1) {
                if let Some(qn) = item.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::SuccessionTarget,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract perform action (e.g., `perform engineStart` or `perform takePicture.focus`)
        if let Some(perform) = usage.perform_action_usage() {
            if let Some(spec) = perform.performed() {
                if let Some(qn) = spec.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Performs,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            
            // Also extract redefines/subsets from the perform action
            // e.g., `perform X redefines foo` should capture the redefines relationship
            for spec in perform.specializations().skip(1) {
                // Skip the first one (performed action), get the rest
                let rel_kind = match spec.kind() {
                    Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                    Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                    Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::References) => NormalizedRelKind::References,
                    _ => continue,
                };
                if let Some(qn) = spec.target() {
                    let target_str = qn.to_string();
                    relationships.push(NormalizedRelationship {
                        kind: rel_kind,
                        target: make_chain_or_simple(&target_str, &qn),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract satisfy/verify (e.g., `satisfy speedRequirement`, `verify SafetyReq by TestCase`)
        if let Some(req_ver) = usage.requirement_verification() {
            let kind = if req_ver.is_satisfy() {
                NormalizedRelKind::Satisfies
            } else {
                NormalizedRelKind::Verifies
            };
            
            if let Some(qn) = req_ver.requirement() {
                relationships.push(NormalizedRelationship {
                    kind,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            } else if let Some(typing) = req_ver.typing() {
                if let Some(target) = typing.target() {
                    relationships.push(NormalizedRelationship {
                        kind,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract connect endpoints (e.g., `connect engine.output to wheel.input`)
        // First check for nested connect_usage, then direct connector_part on the usage
        let connector_part = if let Some(connect) = usage.connect_usage() {
            connect.connector_part()
        } else {
            // For connection usages with inline connect: `connection x connect (...)`
            usage.connector_part()
        };
        
        if let Some(part) = connector_part {
            // Extract all connector ends, not just source/target
            for end in part.ends() {
                if let Some(qn) = end.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract bind endpoints (e.g., `bind port1 = port2`)
        if let Some(bind) = usage.binding_connector() {
            if let Some(qn) = bind.source() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::BindSource,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            }
            if let Some(qn) = bind.target() {
                let target_str = qn.to_string();
                let rel_target = make_chain_or_simple(&target_str, &qn);
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::BindTarget,
                    target: rel_target,
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        
        // Extract exhibit (e.g., `exhibit runningState`)
        if usage.is_exhibit() {
            // Look for qualified name that's the exhibited element
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Exhibits,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract include (e.g., `include useCase`)
        if usage.is_include() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Includes,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract assert (e.g., `assert constraint`)
        if usage.is_assert() && usage.requirement_verification().is_none() {
            // assert without satisfy/verify - standalone constraint assertion
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Asserts,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract assume (e.g., `assume precondition`)
        if usage.is_assume() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Assumes,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract require (e.g., `require constraint`)
        if usage.is_require() {
            for spec in usage.specializations() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Requires,
                        target: RelTarget::Simple(qn.to_string()),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract allocate (e.g., `allocate function to component`)
        // Allocations use qualified names directly - may be feature chains like `a.b.c`
        if usage.is_allocate() {
            let qnames: Vec<_> = usage.syntax().children()
                .filter_map(|n| crate::parser::QualifiedName::cast(n))
                .collect();
            if qnames.len() >= 1 {
                let source_str = qnames[0].to_string();
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateSource,
                    target: make_chain_or_simple(&source_str, &qnames[0]),
                    range: Some(qnames[0].syntax().text_range()),
                });
            }
            if qnames.len() >= 2 {
                let target_str = qnames[1].to_string();
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateTo,
                    target: make_chain_or_simple(&target_str, &qnames[1]),
                    range: Some(qnames[1].syntax().text_range()),
                });
            }
        }

        // Extract children from body
        // For perform actions, the body is inside the PERFORM_ACTION_USAGE
        let mut children: Vec<NormalizedElement> = if let Some(perform) = usage.perform_action_usage() {
            perform
                .body()
                .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
                .unwrap_or_default()
        } else {
            usage
                .body()
                .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
                .unwrap_or_default()
        };
        
        // For transitions, also add accept payload as a child if present
        if let Some(trans) = usage.transition_usage() {
            if let Some(accept_name) = trans.accept_payload_name() {
                let payload_text = accept_name.text();
                let payload_short = accept_name.short_name().and_then(|sn| sn.text());
                let payload_range = Some(accept_name.syntax().text_range());
                let payload_short_range = accept_name
                    .short_name()
                    .map(|sn| sn.syntax().text_range());
                
                // Get accept typing if present (e.g., `accept sig : Signal`)
                let mut payload_rels = Vec::new();
                if let Some(typing) = trans.accept_typing() {
                    if let Some(target) = typing.target() {
                        payload_rels.push(NormalizedRelationship {
                            kind: NormalizedRelKind::TypedBy,
                            target: RelTarget::Simple(target.to_string()),
                            range: Some(target.syntax().text_range()),
                        });
                    }
                }
                
                children.push(NormalizedElement::Usage(NormalizedUsage {
                    name: payload_text,
                    short_name: payload_short,
                    kind: NormalizedUsageKind::Accept,
                    range: payload_range,
                    name_range: payload_range,
                    short_name_range: payload_short_range,
                    doc: None,
                    relationships: payload_rels,
                    children: Vec::new(),
                }));
            }
        }

        // Get name from nested transition or perform action if present, otherwise from usage itself
        let (name, short_name, name_range, short_name_range) = if let Some(trans) = usage.transition_usage() {
            let trans_name = trans.name();
            (
                trans_name.as_ref().and_then(|n| n.text()),
                trans_name.as_ref().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
                trans_name.as_ref().map(|n| n.syntax().text_range()),
                trans_name.and_then(|n| n.short_name()).map(|sn| sn.syntax().text_range()),
            )
        } else if let Some(perform) = usage.perform_action_usage() {
            let perform_name = perform.name();
            (
                perform_name.as_ref().and_then(|n| n.text()),
                perform_name.as_ref().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
                perform_name.as_ref().map(|n| n.syntax().text_range()),
                perform_name.and_then(|n| n.short_name()).map(|sn| sn.syntax().text_range()),
            )
        } else {
            let usage_name = usage.name();
            (
                usage_name.as_ref().and_then(|n| n.text()),
                usage_name.as_ref().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
                usage_name.as_ref().map(|n| n.syntax().text_range()),
                usage_name.and_then(|n| n.short_name()).map(|sn| sn.syntax().text_range()),
            )
        };

        Self {
            name,
            short_name,
            kind,
            range: Some(usage.syntax().text_range()),
            name_range,
            short_name_range,
            doc: parser::extract_doc_comment(usage.syntax()),
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a standalone BindingConnector
    fn from_bind(bind: &parser::BindingConnector) -> Self {
        let mut relationships = Vec::new();
        
        if let Some(qn) = bind.source() {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::BindSource,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }
        if let Some(qn) = bind.target() {
            let target_str = qn.to_string();
            let rel_target = make_chain_or_simple(&target_str, &qn);
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::BindTarget,
                target: rel_target,
                range: Some(qn.syntax().text_range()),
            });
        }
        
        Self {
            name: None, // Bind statements are anonymous
            short_name: None,
            kind: NormalizedUsageKind::Connection, // Bind is a kind of connection
            range: Some(bind.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: None,
            relationships,
            children: Vec::new(),
        }
    }
    
    /// Create a NormalizedUsage from a standalone Succession
    fn from_succession(succ: &parser::Succession) -> Self {
        let mut relationships = Vec::new();
        let mut children = Vec::new();
        
        // Handle succession items (wrapped in SUCCESSION_ITEM)
        let items: Vec<_> = succ.items().collect();
        if !items.is_empty() {
            // First item is the source
            if let Some(qn) = items[0].target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionSource,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            } else if let Some(usage) = items[0].usage() {
                // Inline usage definition - add as child
                children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(&usage)));
            }
        }
        
        // Remaining wrapped items are targets
        for item in items.iter().skip(1) {
            if let Some(qn) = item.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionTarget,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            } else if let Some(usage) = item.usage() {
                // Inline usage definition (like `then action a { ... }`)
                children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(&usage)));
            }
        }
        
        // Handle inline usages directly inside succession (not wrapped in SUCCESSION_ITEM)
        // These come from `then action a { ... }` style successions
        for usage in succ.inline_usages() {
            children.push(NormalizedElement::Usage(NormalizedUsage::from_rowan(&usage)));
        }
        
        Self {
            name: None, // Succession statements are anonymous
            short_name: None,
            kind: NormalizedUsageKind::Other, // Succession as "other"
            range: Some(succ.syntax().text_range()),
            name_range: None,
            short_name_range: None,
            doc: None,
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a standalone TransitionUsage
    fn from_transition(trans: &parser::TransitionUsage) -> Self {
        let mut relationships = Vec::new();
        
        // Extract transition source (first specialization)
        if let Some(source_spec) = trans.source() {
            if let Some(qn) = source_spec.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TransitionSource,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        
        // Extract transition target (second specialization)
        if let Some(target_spec) = trans.target() {
            if let Some(qn) = target_spec.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TransitionTarget,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        
        // For accept transitions: look for typing and qualified names directly
        // accept sig : Signal then running; has TYPING(Signal) and QUALIFIED_NAME(running) as children
        use crate::parser::SyntaxKind;
        for child in trans.syntax().children() {
            if let Some(typing) = parser::Typing::cast(child.clone()) {
                if let Some(target) = typing.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
            // Look for QUALIFIED_NAME after THEN_KW as transition target
            if child.kind() == SyntaxKind::QUALIFIED_NAME {
                if let Some(qn) = parser::QualifiedName::cast(child.clone()) {
                    // Check if this is after 'then' (a target), not part of accept
                    // We can't easily check position here, so add as potential target
                    // Skip if it's already been added as a typing target
                    let target_str = qn.to_string();
                    let already_exists = relationships.iter().any(|r| {
                        matches!(&r.target, RelTarget::Simple(t) if t == &target_str)
                    });
                    if !already_exists {
                        relationships.push(NormalizedRelationship {
                            kind: NormalizedRelKind::TransitionTarget,
                            target: RelTarget::Simple(target_str),
                            range: Some(qn.syntax().text_range()),
                        });
                    }
                }
            }
        }
        
        // Extract name if present (e.g., transition myTransition first source then target)
        let name = trans.name().and_then(|n| n.text());
        let short_name = trans.name().and_then(|n| n.short_name()).and_then(|sn| sn.text());
        let name_range = trans.name().map(|n| n.syntax().text_range());
        let short_name_range = trans.name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());
        
        // Extract accept payload as a child symbol
        let mut children = Vec::new();
        if let Some(accept_name) = trans.accept_payload_name() {
            let payload_text = accept_name.text();
            let payload_short = accept_name.short_name().and_then(|sn| sn.text());
            let payload_range = Some(accept_name.syntax().text_range());
            let payload_short_range = accept_name
                .short_name()
                .map(|sn| sn.syntax().text_range());
            
            // Get accept typing if present (e.g., `accept sig : Signal`)
            let mut payload_rels = Vec::new();
            if let Some(typing) = trans.accept_typing() {
                if let Some(target) = typing.target() {
                    payload_rels.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
            
            children.push(NormalizedElement::Usage(NormalizedUsage {
                name: payload_text,
                short_name: payload_short,
                kind: NormalizedUsageKind::Accept,
                range: payload_range,
                name_range: payload_range,
                short_name_range: payload_short_range,
                doc: None,
                relationships: payload_rels,
                children: Vec::new(),
            }));
        }
        
        Self {
            name,
            short_name,
            kind: NormalizedUsageKind::Transition,
            range: Some(trans.syntax().text_range()),
            name_range,
            short_name_range,
            doc: None,
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a KerML Connector
    fn from_connector(conn: &parser::Connector) -> Self {
        let mut relationships = Vec::new();
        
        // Extract connector ends if present
        if let Some(conn_part) = conn.connector_part() {
            let ends: Vec<_> = conn_part.ends().collect();
            if let Some(first) = ends.first() {
                if let Some(qn) = first.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            for end in ends.iter().skip(1) {
                if let Some(qn) = end.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract children from body
        let children = conn
            .body()
            .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
            .unwrap_or_default();
        
        Self {
            name: conn.name().and_then(|n| n.text()),
            short_name: conn.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
            kind: NormalizedUsageKind::Connection, // KerML connector
            range: Some(conn.syntax().text_range()),
            name_range: conn.name().map(|n| n.syntax().text_range()),
            short_name_range: conn.name().and_then(|n| n.short_name()).map(|sn| sn.syntax().text_range()),
            doc: None,
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a SysML ConnectUsage
    fn from_connect_usage(conn: &parser::ConnectUsage) -> Self {
        let mut relationships = Vec::new();
        
        // Extract connector ends if present
        if let Some(conn_part) = conn.connector_part() {
            if let Some(source) = conn_part.source() {
                if let Some(qn) = source.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectSource,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            if let Some(target) = conn_part.target() {
                if let Some(qn) = target.target() {
                    let target_str = qn.to_string();
                    let rel_target = make_chain_or_simple(&target_str, &qn);
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::ConnectTarget,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract name from NAME child if present
        let name = conn.syntax()
            .children()
            .find_map(parser::Name::cast)
            .and_then(|n| n.text());
        let name_range = conn.syntax()
            .children()
            .find_map(parser::Name::cast)
            .map(|n| n.syntax().text_range());
        
        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Connection,
            range: Some(conn.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships,
            children: Vec::new(), // ConnectUsage typically has no body children
        }
    }
    
    fn from_send_action(send: &parser::SendActionUsage) -> Self {
        // Extract children from the send action's body
        let children: Vec<NormalizedElement> = send.syntax()
            .children()
            .find_map(parser::NamespaceBody::cast)
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();
        
        // Extract name if present
        let name = send.syntax()
            .children()
            .find_map(parser::Name::cast)
            .and_then(|n| n.text());
        let name_range = send.syntax()
            .children()
            .find_map(parser::Name::cast)
            .map(|n| n.syntax().text_range());
            
        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(send.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships: Vec::new(),
            children,
        }
    }
    
    fn from_accept_action(accept: &parser::AcceptActionUsage) -> Self {
        let mut relationships = Vec::new();
        
        // Extract typing (: Type) for the accepted signal
        if let Some(typing) = accept.syntax().children().find_map(parser::Typing::cast) {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }
        
        // Extract name if present
        let name = accept.syntax()
            .children()
            .find_map(parser::Name::cast)
            .and_then(|n| n.text());
        let name_range = accept.syntax()
            .children()
            .find_map(parser::Name::cast)
            .map(|n| n.syntax().text_range());
            
        // Extract children from the accept action's body (if any)
        let children: Vec<NormalizedElement> = accept.syntax()
            .children()
            .find_map(parser::NamespaceBody::cast)
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();
            
        Self {
            name,
            short_name: None,
            kind: NormalizedUsageKind::Action,
            range: Some(accept.syntax().text_range()),
            name_range,
            short_name_range: None,
            doc: None,
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a StateSubaction (entry/do/exit)
    fn from_state_subaction(subaction: &parser::StateSubaction) -> Self {
        let mut relationships = Vec::new();
        
        // Extract typing if present (e.g., entry action myAction : ActionType)
        if let Some(typing) = subaction.syntax().children().find_map(parser::Typing::cast) {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }
        
        // Extract name if present
        let name = subaction.name().and_then(|n| n.text());
        let short_name = subaction.name().and_then(|n| n.short_name()).and_then(|sn| sn.text());
        let name_range = subaction.name().map(|n| n.syntax().text_range());
        let short_name_range = subaction.name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());
            
        // Extract children from the body (if any)
        let children: Vec<NormalizedElement> = subaction.body()
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();
            
        Self {
            name,
            short_name,
            kind: NormalizedUsageKind::Action,
            range: Some(subaction.syntax().text_range()),
            name_range,
            short_name_range,
            doc: None,
            relationships,
            children,
        }
    }
    
    /// Create a NormalizedUsage from a ControlNode (fork/join/merge/decide)
    fn from_control_node(node: &parser::ControlNode) -> Self {
        // Determine kind based on control node type
        let kind = match node.kind() {
            Some(parser::SyntaxKind::FORK_KW) => NormalizedUsageKind::Fork,
            Some(parser::SyntaxKind::JOIN_KW) => NormalizedUsageKind::Join,
            Some(parser::SyntaxKind::MERGE_KW) => NormalizedUsageKind::Merge,
            Some(parser::SyntaxKind::DECIDE_KW) => NormalizedUsageKind::Decide,
            _ => NormalizedUsageKind::Other,
        };
        
        // Extract name if present
        let name = node.name().and_then(|n| n.text());
        let short_name = node.name().and_then(|n| n.short_name()).and_then(|sn| sn.text());
        let name_range = node.name().map(|n| n.syntax().text_range());
        let short_name_range = node.name()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range());
            
        // Extract children from the body (if any)
        let children: Vec<NormalizedElement> = node.body()
            .map(|body| {
                body.members()
                    .map(|m| NormalizedElement::from_rowan(&m))
                    .collect()
            })
            .unwrap_or_default();
            
        Self {
            name,
            short_name,
            kind,
            range: Some(node.syntax().text_range()),
            name_range,
            short_name_range,
            doc: parser::extract_doc_comment(node.syntax()),
            relationships: Vec::new(),
            children,
        }
    }
}

impl NormalizedImport {
    fn from_rowan(import: &RowanImport) -> Self {
        let target = import.target();
        let path_range = target.as_ref().map(|t| t.syntax().text_range());
        let path = target
            .map(|t| {
                let mut path = t.to_string();
                if import.is_wildcard() {
                    path.push_str("::*");
                }
                if import.is_recursive() {
                    // Change ::* to ::** if recursive
                    if path.ends_with("::*") {
                        path.push('*');
                    } else {
                        path.push_str("::**");
                    }
                }
                path
            })
            .unwrap_or_default();

        // Extract filter metadata from bracket syntax [@Filter]
        // Multiple filters like [@A][@B] are all inside one FILTER_PACKAGE
        let filters = import
            .filter()
            .map(|fp| fp.targets().into_iter().map(|qn| qn.to_string()).collect())
            .unwrap_or_default();

        Self {
            path,
            path_range,
            range: Some(import.syntax().text_range()),
            is_public: import.is_public(),
            filters,
        }
    }
}

impl NormalizedAlias {
    fn from_rowan(alias: &parser::Alias) -> Self {
        Self {
            name: alias.name().and_then(|n| n.text()),
            short_name: alias.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
            target: alias.target().map(|t| t.to_string()).unwrap_or_default(),
            target_range: alias.target().map(|t| t.syntax().text_range()),
            range: Some(alias.syntax().text_range()),
        }
    }
}

impl NormalizedDependency {
    fn from_rowan(dep: &parser::Dependency) -> Self {
        let mut sources = Vec::new();
        let mut targets = Vec::new();
        
        // Extract source references (before "to")
        for source in dep.sources() {
            let target_str = source.to_string();
            let rel_target = make_chain_or_simple(&target_str, &source);
            sources.push(NormalizedRelationship {
                kind: NormalizedRelKind::DependencySource,
                target: rel_target,
                range: Some(source.syntax().text_range()),
            });
        }
        
        // Extract target reference (after "to")
        if let Some(target) = dep.target() {
            let target_str = target.to_string();
            let rel_target = make_chain_or_simple(&target_str, &target);
            targets.push(NormalizedRelationship {
                kind: NormalizedRelKind::DependencyTarget,
                target: rel_target,
                range: Some(target.syntax().text_range()),
            });
        }
        
        Self {
            name: None, // Dependencies typically don't have names
            short_name: None,
            sources,
            targets,
            range: Some(dep.syntax().text_range()),
        }
    }
}

// ============================================================================
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
