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
    Import as RowanImport, NamespaceMember, Package as RowanPackage, SourceFile,
    SpecializationKind, Usage as RowanUsage,
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
    Specializes,
    Redefines,
    Subsets,
    TypedBy,
    References,
    Expression, // For expression references like `time / distance`
    About,      // For comment `about` references
    FeatureChain, // For dotted feature chains like `from source.endpoint to target.endpoint`
    // Domain-specific relationships
    Performs,
    Satisfies,
    Exhibits,
    Includes,
    Asserts,
    Verifies,
    Meta,
    Crosses,
}



// ============================================================================
// Adapters from Rowan AST
// ============================================================================

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
            None => NormalizedDefKind::Other,
        };

        // Extract relationships from specializations
        let relationships: Vec<NormalizedRelationship> = def
            .specializations()
            .filter_map(|spec| {
                let rel_kind = match spec.kind() {
                    Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                    Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                    Some(SpecializationKind::References) => NormalizedRelKind::References,
                    Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                    Some(SpecializationKind::FeatureChain) => NormalizedRelKind::Specializes,
                    None => return None,
                };
                let target = spec.target()?.to_string();
                Some(NormalizedRelationship {
                    kind: rel_kind,
                    target: RelTarget::Simple(target),
                    range: Some(spec.syntax().text_range()),
                })
            })
            .collect();

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
            doc: None, // TODO: Extract doc comments
            relationships,
            children,
        }
    }
}

impl NormalizedUsage {
    fn from_rowan(usage: &RowanUsage) -> Self {
        // Determine usage kind based on context (usage doesn't have explicit kind in rowan yet)
        // Default to Part for now - the actual kind comes from the keyword tokens
        let kind = NormalizedUsageKind::Part;

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
            let rel_kind = match spec.kind() {
                Some(SpecializationKind::Specializes) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::Subsets) => NormalizedRelKind::Subsets,
                Some(SpecializationKind::Redefines) => NormalizedRelKind::Redefines,
                Some(SpecializationKind::References) => NormalizedRelKind::References,
                Some(SpecializationKind::Conjugates) => NormalizedRelKind::Specializes,
                Some(SpecializationKind::FeatureChain) => NormalizedRelKind::FeatureChain,
                None => continue,
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
