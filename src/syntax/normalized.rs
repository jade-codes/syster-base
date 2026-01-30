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
    
    // Other
    Crosses,
}



// ============================================================================
// Adapters from Rowan AST
// ============================================================================

/// Helper to create a feature chain or simple target from a qualified name
fn make_chain_or_simple(target_str: &str, qn: &crate::parser::QualifiedName) -> RelTarget {
    if target_str.contains('.') {
        let parts: Vec<FeatureChainPart> = target_str
            .split('.')
            .map(|s| FeatureChainPart {
                name: s.to_string(),
                range: None,
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
            NamespaceMember::Dependency(_dep) => {
                // TODO: Implement dependency conversion
                NormalizedElement::Dependency(NormalizedDependency {
                    name: None,
                    short_name: None,
                    sources: Vec::new(),
                    targets: Vec::new(),
                    range: None,
                })
            }
            NamespaceMember::Filter(filter) => {
                NormalizedElement::Filter(NormalizedFilter {
                    metadata_refs: Vec::new(), // TODO: Extract filter refs
                    range: Some(filter.syntax().text_range()),
                })
            }
            NamespaceMember::Metadata(_meta) => {
                // Metadata usages are skipped for now
                NormalizedElement::Comment(NormalizedComment {
                    name: None,
                    short_name: None,
                    content: String::new(),
                    about: Vec::new(),
                    range: None,
                })
            }
            NamespaceMember::Comment(_comment) => {
                NormalizedElement::Comment(NormalizedComment {
                    name: None,
                    short_name: None,
                    content: String::new(), // TODO: Extract comment content
                    about: Vec::new(),
                    range: None,
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
                let target = spec.target()?.to_string();
                Some(NormalizedRelationship {
                    kind: rel_kind,
                    target: RelTarget::Simple(target),
                    range: Some(spec.syntax().text_range()),
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
        eprintln!("[TRACE normalized::from_def] def={:?}, has body={}", def.name().and_then(|n| n.text()), def.body().is_some());
        if let Some(body) = def.body() {
            eprintln!("[TRACE normalized::from_def]   body syntax children:");
            for child in body.syntax().children() {
                eprintln!("[TRACE normalized::from_def]     child kind={:?}", child.kind());
            }
        }
        let children: Vec<NormalizedElement> = def
            .body()
            .map(|b| {
                let members: Vec<_> = b.members().collect();
                eprintln!("[TRACE normalized::from_def]   body has {} members", members.len());
                for (i, m) in members.iter().enumerate() {
                    eprintln!("[TRACE normalized::from_def]   member[{}]: {:?}", i, m);
                }
                members.into_iter().map(|m| NormalizedElement::from_rowan(&m)).collect()
            })
            .unwrap_or_default();
        eprintln!("[TRACE normalized::from_def]   extracted {} children", children.len());

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
            doc: None, // TODO: Extract doc comments
            relationships,
            children,
        }
    }
}

impl NormalizedUsage {
    fn from_rowan(usage: &RowanUsage) -> Self {
        // Determine usage kind based on keyword tokens
        let kind = match usage.usage_kind() {
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
        };

        // Extract typing as a relationship
        let mut relationships: Vec<NormalizedRelationship> = Vec::new();

        if let Some(typing) = usage.typing() {
            if let Some(target) = typing.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(typing.syntax().text_range()),
                });
            }
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
                        range: Some(target.syntax().text_range()),
                    })
                } else {
                    RelTarget::Simple(target_str)
                };
                relationships.push(NormalizedRelationship {
                    kind: rel_kind,
                    target: rel_target,
                    range: Some(spec.syntax().text_range()),
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
                            range: Some(qn.syntax().text_range()),
                        })
                    } else {
                        RelTarget::Simple(target_str)
                    };
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FeatureChain,
                        target: rel_target,
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            
            // Extract target chain
            if let Some(target) = from_to.target() {
                if let Some(qn) = target.target() {
                    let target_str = qn.to_string();
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
                            range: Some(qn.syntax().text_range()),
                        })
                    } else {
                        RelTarget::Simple(target_str)
                    };
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::FeatureChain,
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
        
        // Extract perform action (e.g., `perform engineStart`)
        if let Some(perform) = usage.perform_action_usage() {
            if let Some(spec) = perform.performed() {
                if let Some(qn) = spec.target() {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Performs,
                        target: RelTarget::Simple(qn.to_string()),
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
                        range: Some(typing.syntax().text_range()),
                    });
                }
            }
        }
        
        // Extract connect endpoints (e.g., `connect engine.output to wheel.input`)
        if let Some(connect) = usage.connect_usage() {
            if let Some(part) = connect.connector_part() {
                if let Some(source) = part.source() {
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
                if let Some(target) = part.target() {
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
        // Allocations use qualified names directly
        if usage.is_allocate() {
            let qnames: Vec<_> = usage.syntax().children()
                .filter_map(|n| crate::parser::QualifiedName::cast(n))
                .collect();
            if qnames.len() >= 1 {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateSource,
                    target: RelTarget::Simple(qnames[0].to_string()),
                    range: Some(qnames[0].syntax().text_range()),
                });
            }
            if qnames.len() >= 2 {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::AllocateTo,
                    target: RelTarget::Simple(qnames[1].to_string()),
                    range: Some(qnames[1].syntax().text_range()),
                });
            }
        }

        // Extract children from body
        let children: Vec<NormalizedElement> = usage
            .body()
            .map(|b| b.members().map(|m| NormalizedElement::from_rowan(&m)).collect())
            .unwrap_or_default();

        Self {
            name: usage.name().and_then(|n| n.text()),
            short_name: usage.name().and_then(|n| n.short_name()).and_then(|sn| sn.text()),
            kind,
            range: Some(usage.syntax().text_range()),
            name_range: usage.name().map(|n| n.syntax().text_range()),
            short_name_range: usage
                .name()
                .and_then(|n| n.short_name())
                .map(|sn| sn.syntax().text_range()),
            doc: None, // TODO: Extract doc comments
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
        
        let items: Vec<_> = succ.items().collect();
        if !items.is_empty() {
            if let Some(qn) = items[0].target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionSource,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        for item in items.iter().skip(1) {
            if let Some(qn) = item.target() {
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::SuccessionTarget,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
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
            children: Vec::new(),
        }
    }
}

impl NormalizedImport {
    fn from_rowan(import: &RowanImport) -> Self {
        let path = import
            .target()
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

        Self {
            path,
            range: Some(import.syntax().text_range()),
            is_public: false, // TODO: Detect public imports
            filters: Vec::new(), // TODO: Extract filter metadata
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
