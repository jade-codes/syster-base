//! Symbol extraction from AST — pure functions that return symbols.
//!
//! This module provides functions to extract symbols from a parsed AST.
//! Extraction works directly with the typed AST wrapper types from
//! `crate::parser` (e.g., `Definition`, `Usage`, `Package`), producing
//! `HirSymbol` values without any intermediate representation.

use std::sync::Arc;

use uuid::Uuid;

use crate::base::FileId;
use crate::parser::{
    AstNode, DefinitionKind, Direction, Expression, Multiplicity,
    QualifiedName, SpecializationKind, Usage, UsageKind,
    ValueExpression,
};
use rowan::TextRange;

// Internal normalized types — private to the HIR module
use super::normalize::{
    NormalizedAlias, NormalizedComment, NormalizedDefKind, NormalizedDefinition,
    NormalizedDependency, NormalizedElement, NormalizedImport,
    NormalizedPackage, NormalizedRelKind, NormalizedRelationship, NormalizedUsage,
    NormalizedUsageKind,
};

// ============================================================================
// RELATIONSHIP HELPER TYPES (formerly in syntax/normalized)
// These types are scaffolding for incrementally migrating extraction functions
// to work directly with AST types instead of NormalizedXxx types.
// ============================================================================

/// A feature chain like `engine.power.value`
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct FeatureChain {
    pub parts: Vec<FeatureChainPart>,
    pub range: Option<TextRange>,
}

/// A single part of a feature chain
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct FeatureChainPart {
    pub name: String,
    pub range: Option<TextRange>,
}

#[allow(dead_code)]
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

/// A relationship target — either a simple name or a feature chain.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum RelTarget {
    /// A simple reference like `Vehicle`
    Simple(String),
    /// A feature chain like `engine.power.value`
    Chain(FeatureChain),
}

#[allow(dead_code)]
impl RelTarget {
    /// Get the target name (for simple refs) or the full dotted path (for chains)
    pub fn as_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            RelTarget::Simple(s) => std::borrow::Cow::Borrowed(s),
            RelTarget::Chain(chain) => std::borrow::Cow::Owned(chain.as_dotted_string()),
        }
    }
}

/// Kinds of relationships (internal representation for extraction).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelKind {
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

/// A relationship extracted from an AST node during symbol extraction.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ExtractedRel {
    pub kind: RelKind,
    pub target: RelTarget,
    pub range: Option<TextRange>,
}

/// Internal usage kind (mirrors the 27 normalized usage kinds + special variants).
/// Used during extraction to determine SymbolKind without an intermediate enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternalUsageKind {
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
    End,
    Fork,
    Join,
    Merge,
    Decide,
    View,
    Viewpoint,
    Rendering,
    Feature,
    Other,
}

/// Generate a new unique element ID for XMI interchange.
pub fn new_element_id() -> Arc<str> {
    Uuid::new_v4().to_string().into()
}

// ============================================================================
// BRIDGE FUNCTIONS: NormalizedRelKind ↔ RelKind (temporary, removed when normalized is deleted)
// ============================================================================

fn normalized_to_rel_kind(nk: NormalizedRelKind) -> RelKind {
    match nk {
        NormalizedRelKind::Specializes => RelKind::Specializes,
        NormalizedRelKind::Redefines => RelKind::Redefines,
        NormalizedRelKind::Subsets => RelKind::Subsets,
        NormalizedRelKind::TypedBy => RelKind::TypedBy,
        NormalizedRelKind::References => RelKind::References,
        NormalizedRelKind::Conjugates => RelKind::Conjugates,
        NormalizedRelKind::FeatureChain => RelKind::FeatureChain,
        NormalizedRelKind::Expression => RelKind::Expression,
        NormalizedRelKind::TransitionSource => RelKind::TransitionSource,
        NormalizedRelKind::TransitionTarget => RelKind::TransitionTarget,
        NormalizedRelKind::SuccessionSource => RelKind::SuccessionSource,
        NormalizedRelKind::SuccessionTarget => RelKind::SuccessionTarget,
        NormalizedRelKind::AcceptedMessage => RelKind::AcceptedMessage,
        NormalizedRelKind::AcceptVia => RelKind::AcceptVia,
        NormalizedRelKind::SentMessage => RelKind::SentMessage,
        NormalizedRelKind::SendVia => RelKind::SendVia,
        NormalizedRelKind::SendTo => RelKind::SendTo,
        NormalizedRelKind::MessageSource => RelKind::MessageSource,
        NormalizedRelKind::MessageTarget => RelKind::MessageTarget,
        NormalizedRelKind::Satisfies => RelKind::Satisfies,
        NormalizedRelKind::Verifies => RelKind::Verifies,
        NormalizedRelKind::Asserts => RelKind::Asserts,
        NormalizedRelKind::Assumes => RelKind::Assumes,
        NormalizedRelKind::Requires => RelKind::Requires,
        NormalizedRelKind::AllocateSource => RelKind::AllocateSource,
        NormalizedRelKind::AllocateTo => RelKind::AllocateTo,
        NormalizedRelKind::BindSource => RelKind::BindSource,
        NormalizedRelKind::BindTarget => RelKind::BindTarget,
        NormalizedRelKind::ConnectSource => RelKind::ConnectSource,
        NormalizedRelKind::ConnectTarget => RelKind::ConnectTarget,
        NormalizedRelKind::FlowItem => RelKind::FlowItem,
        NormalizedRelKind::FlowSource => RelKind::FlowSource,
        NormalizedRelKind::FlowTarget => RelKind::FlowTarget,
        NormalizedRelKind::InterfaceEnd => RelKind::InterfaceEnd,
        NormalizedRelKind::Performs => RelKind::Performs,
        NormalizedRelKind::Exhibits => RelKind::Exhibits,
        NormalizedRelKind::Includes => RelKind::Includes,
        NormalizedRelKind::About => RelKind::About,
        NormalizedRelKind::Meta => RelKind::Meta,
        NormalizedRelKind::Exposes => RelKind::Exposes,
        NormalizedRelKind::Renders => RelKind::Renders,
        NormalizedRelKind::Filters => RelKind::Filters,
        NormalizedRelKind::DependencySource => RelKind::DependencySource,
        NormalizedRelKind::DependencyTarget => RelKind::DependencyTarget,
        NormalizedRelKind::Crosses => RelKind::Crosses,
    }
}

fn normalized_to_definition_kind(nk: NormalizedDefKind) -> DefinitionKind {
    match nk {
        NormalizedDefKind::Part => DefinitionKind::Part,
        NormalizedDefKind::Item => DefinitionKind::Item,
        NormalizedDefKind::Action => DefinitionKind::Action,
        NormalizedDefKind::Port => DefinitionKind::Port,
        NormalizedDefKind::Attribute => DefinitionKind::Attribute,
        NormalizedDefKind::Connection => DefinitionKind::Connection,
        NormalizedDefKind::Interface => DefinitionKind::Interface,
        NormalizedDefKind::Allocation => DefinitionKind::Allocation,
        NormalizedDefKind::Requirement => DefinitionKind::Requirement,
        NormalizedDefKind::Constraint => DefinitionKind::Constraint,
        NormalizedDefKind::State => DefinitionKind::State,
        NormalizedDefKind::Calculation => DefinitionKind::Calc,
        NormalizedDefKind::UseCase => DefinitionKind::UseCase,
        NormalizedDefKind::AnalysisCase => DefinitionKind::Analysis,
        NormalizedDefKind::Concern => DefinitionKind::Concern,
        NormalizedDefKind::View => DefinitionKind::View,
        NormalizedDefKind::Viewpoint => DefinitionKind::Viewpoint,
        NormalizedDefKind::Rendering => DefinitionKind::Rendering,
        NormalizedDefKind::Enumeration => DefinitionKind::Enum,
        NormalizedDefKind::DataType => DefinitionKind::Datatype,
        NormalizedDefKind::Class => DefinitionKind::Class,
        NormalizedDefKind::Structure => DefinitionKind::Struct,
        NormalizedDefKind::Behavior => DefinitionKind::Behavior,
        NormalizedDefKind::Function => DefinitionKind::Function,
        NormalizedDefKind::Association => DefinitionKind::Assoc,
        NormalizedDefKind::Metaclass => DefinitionKind::Metaclass,
        NormalizedDefKind::Interaction => DefinitionKind::Interaction,
        NormalizedDefKind::Other => DefinitionKind::Type, // fallback
    }
}

fn normalized_to_internal_usage_kind(nk: NormalizedUsageKind) -> InternalUsageKind {
    match nk {
        NormalizedUsageKind::Part => InternalUsageKind::Part,
        NormalizedUsageKind::Item => InternalUsageKind::Item,
        NormalizedUsageKind::Action => InternalUsageKind::Action,
        NormalizedUsageKind::Port => InternalUsageKind::Port,
        NormalizedUsageKind::Attribute => InternalUsageKind::Attribute,
        NormalizedUsageKind::Connection => InternalUsageKind::Connection,
        NormalizedUsageKind::Interface => InternalUsageKind::Interface,
        NormalizedUsageKind::Allocation => InternalUsageKind::Allocation,
        NormalizedUsageKind::Requirement => InternalUsageKind::Requirement,
        NormalizedUsageKind::Constraint => InternalUsageKind::Constraint,
        NormalizedUsageKind::State => InternalUsageKind::State,
        NormalizedUsageKind::Calculation => InternalUsageKind::Calculation,
        NormalizedUsageKind::Reference => InternalUsageKind::Reference,
        NormalizedUsageKind::Occurrence => InternalUsageKind::Occurrence,
        NormalizedUsageKind::Flow => InternalUsageKind::Flow,
        NormalizedUsageKind::Transition => InternalUsageKind::Transition,
        NormalizedUsageKind::Accept => InternalUsageKind::Accept,
        NormalizedUsageKind::End => InternalUsageKind::End,
        NormalizedUsageKind::Fork => InternalUsageKind::Fork,
        NormalizedUsageKind::Join => InternalUsageKind::Join,
        NormalizedUsageKind::Merge => InternalUsageKind::Merge,
        NormalizedUsageKind::Decide => InternalUsageKind::Decide,
        NormalizedUsageKind::View => InternalUsageKind::View,
        NormalizedUsageKind::Viewpoint => InternalUsageKind::Viewpoint,
        NormalizedUsageKind::Rendering => InternalUsageKind::Rendering,
        NormalizedUsageKind::Feature => InternalUsageKind::Feature,
        NormalizedUsageKind::Other => InternalUsageKind::Other,
    }
}

/// The kind of reference - determines resolution strategy.
///
/// Type references (TypedBy, Specializes) resolve via scope walking.
/// Feature references (Redefines, Subsets, References) resolve via inheritance hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefKind {
    /// `: Type` - type annotation, resolves via scope
    TypedBy,
    /// `:> Type` for types - specialization, resolves via scope
    Specializes,
    /// `:>> feature` - redefinition, resolves via inheritance
    Redefines,
    /// `:> feature` for features - subsetting, resolves via inheritance
    Subsets,
    /// `::> feature` - references/featured-by, resolves via inheritance
    References,
    /// Reference in an expression - context dependent
    Expression,
    /// Other relationship types (performs, satisfies, etc.)
    Other,
}

impl RefKind {
    /// Returns true if this is a type reference that should resolve via scope walking.
    pub fn is_type_reference(&self) -> bool {
        matches!(self, RefKind::TypedBy | RefKind::Specializes)
    }

    /// Returns true if this is a feature reference that resolves via inheritance.
    pub fn is_feature_reference(&self) -> bool {
        matches!(
            self,
            RefKind::Redefines | RefKind::Subsets | RefKind::References
        )
    }

    /// Convert from RelKind.
    pub(crate) fn from_rel_kind(kind: RelKind) -> Self {
        match kind {
            RelKind::TypedBy => RefKind::TypedBy,
            RelKind::Specializes => RefKind::Specializes,
            RelKind::Redefines => RefKind::Redefines,
            RelKind::Subsets => RefKind::Subsets,
            RelKind::References => RefKind::References,
            RelKind::Expression => RefKind::Expression,
            _ => RefKind::Other,
        }
    }

    /// Get a display label for this reference kind.
    pub fn display(&self) -> &'static str {
        match self {
            RefKind::TypedBy => "typed by",
            RefKind::Specializes => "specializes",
            RefKind::Redefines => "redefines",
            RefKind::Subsets => "subsets",
            RefKind::References => "references",
            RefKind::Expression => "expression",
            RefKind::Other => "other",
        }
    }
}

// ============================================================================
// RELATIONSHIPS
// ============================================================================

/// The kind of relationship between symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationshipKind {
    /// `:>` - specialization (for definitions)
    Specializes,
    /// `:` - typing (for usages)
    TypedBy,
    /// `:>>` - redefinition
    Redefines,
    /// `subsets` - subsetting
    Subsets,
    /// `::>` - references/featured-by
    References,
    // Domain-specific relationships
    /// `satisfy` - requirement satisfaction
    Satisfies,
    /// `perform` - action performance
    Performs,
    /// `exhibit` - state exhibition
    Exhibits,
    /// `include` - use case inclusion
    Includes,
    /// `assert` - constraint assertion
    Asserts,
    /// `verify` - verification
    Verifies,
}

impl RelationshipKind {
    /// Convert from RelKind.
    pub(crate) fn from_rel_kind(kind: RelKind) -> Option<Self> {
        match kind {
            RelKind::Specializes => Some(RelationshipKind::Specializes),
            RelKind::TypedBy => Some(RelationshipKind::TypedBy),
            RelKind::Redefines => Some(RelationshipKind::Redefines),
            RelKind::Subsets => Some(RelationshipKind::Subsets),
            RelKind::References => Some(RelationshipKind::References),
            RelKind::Satisfies => Some(RelationshipKind::Satisfies),
            RelKind::Performs => Some(RelationshipKind::Performs),
            RelKind::Exhibits => Some(RelationshipKind::Exhibits),
            RelKind::Includes => Some(RelationshipKind::Includes),
            RelKind::Asserts => Some(RelationshipKind::Asserts),
            RelKind::Verifies => Some(RelationshipKind::Verifies),
            // Expression, About, Meta, Crosses are not shown as relationships
            _ => None,
        }
    }

    /// Get a display label for this relationship kind.
    pub fn display(&self) -> &'static str {
        match self {
            RelationshipKind::Specializes => "Specializes",
            RelationshipKind::TypedBy => "Typed by",
            RelationshipKind::Redefines => "Redefines",
            RelationshipKind::Subsets => "Subsets",
            RelationshipKind::References => "References",
            RelationshipKind::Satisfies => "Satisfies",
            RelationshipKind::Performs => "Performs",
            RelationshipKind::Exhibits => "Exhibits",
            RelationshipKind::Includes => "Includes",
            RelationshipKind::Asserts => "Asserts",
            RelationshipKind::Verifies => "Verifies",
        }
    }
}

/// A relationship from this symbol to another.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HirRelationship {
    /// The kind of relationship
    pub kind: RelationshipKind,
    /// The target name as written in source
    pub target: Arc<str>,
    /// The resolved qualified name (if resolved)
    pub resolved_target: Option<Arc<str>>,
    /// Start line of the target reference (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_col: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_col: u32,
}

impl HirRelationship {
    /// Create a new relationship.
    pub fn new(kind: RelationshipKind, target: impl Into<Arc<str>>) -> Self {
        Self {
            kind,
            target: target.into(),
            resolved_target: None,
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        }
    }

    /// Create a new relationship with span information.
    pub fn with_span(
        kind: RelationshipKind,
        target: impl Into<Arc<str>>,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Self {
        Self {
            kind,
            target: target.into(),
            resolved_target: None,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// A type reference with its source location.
///
/// This tracks where a type name appears in the source code,
/// enabling go-to-definition from type annotations.
///
/// Feature chains like `takePicture.focus` are detected at resolution time
/// by checking if TypeRefs are adjacent (separated by a dot). This avoids
/// storing chain metadata in the HIR layer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeRef {
    /// The target type name as written in source (e.g., "Car", "focus")
    pub target: Arc<str>,
    /// The fully resolved qualified name (e.g., "Vehicle::Car", "TakePicture::focus")
    /// This is computed during the semantic resolution pass.
    pub resolved_target: Option<Arc<str>>,
    /// The kind of reference - determines resolution strategy.
    pub kind: RefKind,
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_col: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_col: u32,
}

impl TypeRef {
    /// Create a new type reference.
    pub fn new(
        target: impl Into<Arc<str>>,
        kind: RefKind,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Self {
        Self {
            target: target.into(),
            resolved_target: None,
            kind,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Check if a position is within this type reference.
    pub fn contains(&self, line: u32, col: u32) -> bool {
        let after_start =
            line > self.start_line || (line == self.start_line && col >= self.start_col);
        let before_end = line < self.end_line || (line == self.end_line && col <= self.end_col);
        after_start && before_end
    }

    /// Check if another TypeRef immediately follows this one (separated by a dot).
    /// Used to detect feature chains like `takePicture.focus` at resolution time.
    pub fn immediately_precedes(&self, other: &TypeRef) -> bool {
        // Must be on the same line
        if self.end_line != other.start_line {
            return false;
        }
        // The other ref must start exactly 1 character after this one ends (the dot)
        self.end_col + 1 == other.start_col
    }

    /// Get the best target to use for resolution - resolved if available, else raw.
    pub fn effective_target(&self) -> &Arc<str> {
        self.resolved_target.as_ref().unwrap_or(&self.target)
    }
}

/// A type reference that can be either a simple reference or a chain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeRefKind {
    /// A simple reference like `Vehicle`
    Simple(TypeRef),
    /// A feature chain like `engine.power.value`
    Chain(TypeRefChain),
}

impl TypeRefKind {
    /// Get all individual TypeRefs for iteration
    pub fn as_refs(&self) -> Vec<&TypeRef> {
        match self {
            TypeRefKind::Simple(r) => vec![r],
            TypeRefKind::Chain(c) => c.parts.iter().collect(),
        }
    }

    /// Check if this is a chain
    pub fn is_chain(&self) -> bool {
        matches!(self, TypeRefKind::Chain(_))
    }

    /// Get the first part's target name
    pub fn first_target(&self) -> &Arc<str> {
        match self {
            TypeRefKind::Simple(r) => &r.target,
            TypeRefKind::Chain(c) => &c.parts[0].target,
        }
    }

    /// Check if a position is within this type reference
    pub fn contains(&self, line: u32, col: u32) -> bool {
        match self {
            TypeRefKind::Simple(r) => r.contains(line, col),
            TypeRefKind::Chain(c) => c.parts.iter().any(|r| r.contains(line, col)),
        }
    }

    /// Find which part contains the position (for chains)
    pub fn part_at(&self, line: u32, col: u32) -> Option<(usize, &TypeRef)> {
        match self {
            TypeRefKind::Simple(r) if r.contains(line, col) => Some((0, r)),
            TypeRefKind::Chain(c) => c
                .parts
                .iter()
                .enumerate()
                .find(|(_, r)| r.contains(line, col)),
            _ => None,
        }
    }
}

/// A chain of type references like `engine.power.value`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeRefChain {
    /// The parts of the chain, each with its own span
    pub parts: Vec<TypeRef>,
}

impl TypeRefChain {
    /// Get the full dotted path
    pub fn as_dotted_string(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.target.as_ref())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// A symbol extracted from the AST.
///
/// This is a simplified symbol type for the new HIR layer.
/// It captures the essential information needed for IDE features.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HirSymbol {
    /// The simple name of the symbol
    pub name: Arc<str>,
    /// The short name alias (e.g., "m" for "metre"), if any
    pub short_name: Option<Arc<str>>,
    /// The fully qualified name
    pub qualified_name: Arc<str>,
    /// Unique element ID for XMI interchange.
    /// Generated at parse time for all symbols, preserved on import/export.
    pub element_id: Arc<str>,
    /// What kind of symbol this is
    pub kind: SymbolKind,
    /// The file containing this symbol
    pub file: FileId,
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_col: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_col: u32,
    /// Short name span (for hover support on short names)
    pub short_name_start_line: Option<u32>,
    pub short_name_start_col: Option<u32>,
    pub short_name_end_line: Option<u32>,
    pub short_name_end_col: Option<u32>,
    /// Documentation comment, if any
    pub doc: Option<Arc<str>>,
    /// Types this symbol specializes/subsets (kept for backwards compat)
    pub supertypes: Vec<Arc<str>>,
    /// All relationships from this symbol (specializes, typed by, satisfies, etc.)
    pub relationships: Vec<HirRelationship>,
    /// Type references with their source locations (for goto-definition on type annotations)
    pub type_refs: Vec<TypeRefKind>,
    /// Whether this symbol is public (for imports: re-exported to child scopes)
    pub is_public: bool,
    /// View-specific data (for ViewDefinition, ViewUsage, etc.)
    pub view_data: Option<crate::hir::views::ViewData>,
    /// Metadata types applied to this symbol (e.g., ["Safety", "Approved"])
    /// Used for filter import evaluation (SysML v2 §7.5.4)
    pub metadata_annotations: Vec<Arc<str>>,
    /// Whether this symbol is abstract (for definitions and usages)
    pub is_abstract: bool,
    /// Whether this symbol is a variation (for definitions and usages)
    pub is_variation: bool,
    /// Whether this symbol is readonly (for usages only)
    pub is_readonly: bool,
    /// Whether this symbol is derived (for usages only)
    pub is_derived: bool,
    /// Whether this symbol is parallel (for state usages)
    pub is_parallel: bool,
    /// Whether this symbol is individual (singleton occurrence)
    pub is_individual: bool,
    /// Whether this symbol is an end feature (connector endpoint)
    pub is_end: bool,
    /// Whether this symbol has a default value
    pub is_default: bool,
    /// Whether this symbol's values are ordered
    pub is_ordered: bool,
    /// Whether this symbol's values are nonunique (can have duplicates)
    pub is_nonunique: bool,
    /// Whether this symbol is a portion (slice of occurrence)
    pub is_portion: bool,
    /// Direction (in, out, inout) for ports and parameters
    pub direction: Option<Direction>,
    /// Multiplicity bounds [lower..upper]
    pub multiplicity: Option<Multiplicity>,
    /// Value expression assigned to this feature (e.g., `= 42`, `= "hello"`)
    pub value: Option<ValueExpression>,
}

/// The kind of a symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Package,
    // Definitions
    PartDefinition,
    ItemDefinition,
    ActionDefinition,
    PortDefinition,
    AttributeDefinition,
    ConnectionDefinition,
    InterfaceDefinition,
    AllocationDefinition,
    RequirementDefinition,
    ConstraintDefinition,
    StateDefinition,
    CalculationDefinition,
    UseCaseDefinition,
    AnalysisCaseDefinition,
    ConcernDefinition,
    ViewDefinition,
    ViewpointDefinition,
    RenderingDefinition,
    ViewUsage,
    ViewpointUsage,
    RenderingUsage,
    EnumerationDefinition,
    MetadataDefinition,
    Interaction,
    // KerML Definitions
    DataType,
    Class,
    Structure,
    Behavior,
    Function,
    Association,
    // Usages
    PartUsage,
    ItemUsage,
    ActionUsage,
    PortUsage,
    AttributeUsage,
    ConnectionUsage,
    InterfaceUsage,
    AllocationUsage,
    RequirementUsage,
    ConstraintUsage,
    StateUsage,
    TransitionUsage,
    CalculationUsage,
    ReferenceUsage,
    OccurrenceUsage,
    FlowConnectionUsage,
    // Relationships
    ExposeRelationship,
    // Other
    Import,
    Alias,
    Comment,
    Dependency,
    // Generic fallback
    Other,
}

impl SymbolKind {
    /// Create from a DefinitionKind (AST-level kind).
    pub(crate) fn from_definition_kind(kind: Option<DefinitionKind>) -> Self {
        match kind {
            Some(DefinitionKind::Part) => Self::PartDefinition,
            Some(DefinitionKind::Item) => Self::ItemDefinition,
            Some(DefinitionKind::Action) => Self::ActionDefinition,
            Some(DefinitionKind::Port) => Self::PortDefinition,
            Some(DefinitionKind::Attribute) => Self::AttributeDefinition,
            Some(DefinitionKind::Connection) => Self::ConnectionDefinition,
            Some(DefinitionKind::Interface) => Self::InterfaceDefinition,
            Some(DefinitionKind::Allocation) => Self::AllocationDefinition,
            Some(DefinitionKind::Requirement) => Self::RequirementDefinition,
            Some(DefinitionKind::Constraint) => Self::ConstraintDefinition,
            Some(DefinitionKind::State) => Self::StateDefinition,
            Some(DefinitionKind::Calc) => Self::CalculationDefinition,
            Some(DefinitionKind::Case) | Some(DefinitionKind::UseCase) => {
                Self::UseCaseDefinition
            }
            Some(DefinitionKind::Analysis) | Some(DefinitionKind::Verification) => {
                Self::AnalysisCaseDefinition
            }
            Some(DefinitionKind::Concern) => Self::ConcernDefinition,
            Some(DefinitionKind::View) => Self::ViewDefinition,
            Some(DefinitionKind::Viewpoint) => Self::ViewpointDefinition,
            Some(DefinitionKind::Rendering) => Self::RenderingDefinition,
            Some(DefinitionKind::Enum) => Self::EnumerationDefinition,
            Some(DefinitionKind::Flow) => Self::Other,
            Some(DefinitionKind::Metadata) => Self::Other,
            Some(DefinitionKind::Occurrence) => Self::Other,
            // KerML mappings
            Some(DefinitionKind::Class) => Self::PartDefinition,
            Some(DefinitionKind::Struct) => Self::PartDefinition,
            Some(DefinitionKind::Datatype) => Self::AttributeDefinition,
            Some(DefinitionKind::Assoc) => Self::ConnectionDefinition,
            Some(DefinitionKind::Behavior) => Self::ActionDefinition,
            Some(DefinitionKind::Function) => Self::CalculationDefinition,
            Some(DefinitionKind::Predicate) => Self::ConstraintDefinition,
            Some(DefinitionKind::Interaction) => Self::ActionDefinition,
            Some(DefinitionKind::Classifier) => Self::PartDefinition,
            Some(DefinitionKind::Type) => Self::Other,
            Some(DefinitionKind::Metaclass) => Self::MetadataDefinition,
            None => Self::Other,
        }
    }

    /// Create from an InternalUsageKind.
    pub(crate) fn from_usage_kind(kind: InternalUsageKind) -> Self {
        match kind {
            InternalUsageKind::Part => Self::PartUsage,
            InternalUsageKind::Item => Self::ItemUsage,
            InternalUsageKind::Action => Self::ActionUsage,
            InternalUsageKind::Port => Self::PortUsage,
            InternalUsageKind::Attribute => Self::AttributeUsage,
            InternalUsageKind::Connection => Self::ConnectionUsage,
            InternalUsageKind::Interface => Self::InterfaceUsage,
            InternalUsageKind::Allocation => Self::AllocationUsage,
            InternalUsageKind::Requirement => Self::RequirementUsage,
            InternalUsageKind::Constraint => Self::ConstraintUsage,
            InternalUsageKind::State => Self::StateUsage,
            InternalUsageKind::Calculation => Self::CalculationUsage,
            InternalUsageKind::Reference => Self::ReferenceUsage,
            InternalUsageKind::Occurrence => Self::OccurrenceUsage,
            InternalUsageKind::Flow => Self::FlowConnectionUsage,
            InternalUsageKind::Transition => Self::TransitionUsage,
            InternalUsageKind::Accept => Self::ActionUsage,
            InternalUsageKind::End => Self::PortUsage,
            InternalUsageKind::Fork => Self::ActionUsage,
            InternalUsageKind::Join => Self::ActionUsage,
            InternalUsageKind::Merge => Self::ActionUsage,
            InternalUsageKind::Decide => Self::ActionUsage,
            InternalUsageKind::View => Self::ViewUsage,
            InternalUsageKind::Viewpoint => Self::ViewpointUsage,
            InternalUsageKind::Rendering => Self::RenderingUsage,
            InternalUsageKind::Feature => Self::AttributeUsage,
            InternalUsageKind::Other => Self::Other,
        }
    }

    /// Get a display string for this kind (capitalized for UI display).
    pub fn display(&self) -> &'static str {
        match self {
            Self::Package => "Package",
            Self::PartDefinition => "Part def",
            Self::ItemDefinition => "Item def",
            Self::ActionDefinition => "Action def",
            Self::PortDefinition => "Port def",
            Self::AttributeDefinition => "Attribute def",
            Self::ConnectionDefinition => "Connection def",
            Self::InterfaceDefinition => "Interface def",
            Self::AllocationDefinition => "Allocation def",
            Self::RequirementDefinition => "Requirement def",
            Self::ConstraintDefinition => "Constraint def",
            Self::StateDefinition => "State def",
            Self::CalculationDefinition => "Calc def",
            Self::UseCaseDefinition => "Use case def",
            Self::AnalysisCaseDefinition => "Analysis case def",
            Self::ConcernDefinition => "Concern def",
            Self::ViewDefinition => "View def",
            Self::ViewpointDefinition => "Viewpoint def",
            Self::RenderingDefinition => "Rendering def",
            Self::ViewUsage => "View",
            Self::ViewpointUsage => "Viewpoint",
            Self::RenderingUsage => "Rendering",
            Self::EnumerationDefinition => "Enum def",
            Self::MetadataDefinition => "Metaclass def",
            Self::Interaction => "Interaction def",
            // KerML definitions
            Self::DataType => "Datatype",
            Self::Class => "Class",
            Self::Structure => "Struct",
            Self::Behavior => "Behavior",
            Self::Function => "Function",
            Self::Association => "Assoc",
            Self::PartUsage => "Part",
            Self::ItemUsage => "Item",
            Self::ActionUsage => "Action",
            Self::PortUsage => "Port",
            Self::AttributeUsage => "Attribute",
            Self::ConnectionUsage => "Connection",
            Self::InterfaceUsage => "Interface",
            Self::AllocationUsage => "Allocation",
            Self::RequirementUsage => "Requirement",
            Self::ConstraintUsage => "Constraint",
            Self::StateUsage => "State",
            Self::TransitionUsage => "Transition",
            Self::CalculationUsage => "Calc",
            Self::ReferenceUsage => "Ref",
            Self::OccurrenceUsage => "Occurrence",
            Self::FlowConnectionUsage => "Flow",
            Self::ExposeRelationship => "Expose",
            Self::Import => "Import",
            Self::Alias => "Alias",
            Self::Comment => "Comment",
            Self::Dependency => "Dependency",
            Self::Other => "Element",
        }
    }

    /// Convert a normalized definition kind to a SymbolKind (bridge — will be removed).
    pub fn from_normalized_def_kind(kind: NormalizedDefKind) -> Self {
        Self::from_definition_kind(Some(normalized_to_definition_kind(kind)))
    }

    /// Convert a normalized usage kind to a SymbolKind (bridge — will be removed).
    pub fn from_normalized_usage_kind(kind: NormalizedUsageKind) -> Self {
        Self::from_usage_kind(normalized_to_internal_usage_kind(kind))
    }
}

// ============================================================================
// EXTRACTION CONTEXT
// ============================================================================

struct ExtractionContext {
    file: FileId,
    prefix: String,
    /// Counter for generating unique anonymous scope names
    anon_counter: u32,
    /// Stack of scope segments for proper push/pop
    scope_stack: Vec<String>,
    /// Line index for converting byte offsets to line/column
    line_index: crate::base::LineIndex,
}

impl ExtractionContext {
    fn qualified_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.prefix, name)
        }
    }

    /// Get the current scope name (the prefix without trailing ::)
    fn current_scope_name(&self) -> String {
        self.prefix.clone()
    }

    fn push_scope(&mut self, name: &str) {
        self.scope_stack.push(name.to_string());
        if self.prefix.is_empty() {
            self.prefix = name.to_string();
        } else {
            self.prefix = format!("{}::{}", self.prefix, name);
        }
    }

    fn pop_scope(&mut self) {
        if let Some(popped) = self.scope_stack.pop() {
            // Remove the last segment (which may contain ::) plus the joining ::
            let suffix_len = if self.scope_stack.is_empty() {
                popped.len()
            } else {
                popped.len() + 2 // +2 for the "::" separator
            };
            self.prefix
                .truncate(self.prefix.len().saturating_sub(suffix_len));
        }
    }

    /// Generate a unique anonymous scope name
    fn next_anon_scope(&mut self, rel_prefix: &str, target: &str, line: u32) -> String {
        self.anon_counter += 1;
        format!("<{}{}#{}@L{}>", rel_prefix, target, self.anon_counter, line)
    }

    /// Convert a TextRange to SpanInfo using the line index
    fn range_to_info(&self, range: Option<rowan::TextRange>) -> SpanInfo {
        match range {
            Some(r) => {
                let start = self.line_index.line_col(r.start());
                let end = self.line_index.line_col(r.end());
                SpanInfo {
                    start_line: start.line,
                    start_col: start.col,
                    end_line: end.line,
                    end_col: end.col,
                }
            }
            None => SpanInfo::default(),
        }
    }

    /// Convert a TextRange to optional line/col values (for short_name fields)
    fn range_to_optional(
        &self,
        range: Option<rowan::TextRange>,
    ) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
        match range {
            Some(r) => {
                let start = self.line_index.line_col(r.start());
                let end = self.line_index.line_col(r.end());
                (
                    Some(start.line),
                    Some(start.col),
                    Some(end.line),
                    Some(end.col),
                )
            }
            None => (None, None, None, None),
        }
    }
}

// ============================================================================
// AST → INTERNAL TYPE HELPERS
// ============================================================================

/// Helper to create a chain or simple RelTarget from a dotted qualified name.
#[allow(dead_code)]
fn make_chain_or_simple(target_str: &str, qn: &QualifiedName) -> RelTarget {
    if target_str.contains('.') {
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

/// Extract feature chain expression references from an Expression AST node.
#[allow(dead_code)]
fn extract_expression_chains(
    expr: &Expression,
    relationships: &mut Vec<ExtractedRel>,
) {
    for chain in expr.feature_chains() {
        if chain.parts.len() == 1 {
            let (name, range) = &chain.parts[0];
            relationships.push(ExtractedRel {
                kind: RelKind::Expression,
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
            relationships.push(ExtractedRel {
                kind: RelKind::Expression,
                target: RelTarget::Chain(FeatureChain {
                    parts,
                    range: Some(chain.full_range),
                }),
                range: Some(chain.full_range),
            });
        }
    }
}

/// Map a SpecializationKind to a RelKind, with a default for comma-continuation.
#[allow(dead_code)]
fn spec_kind_to_rel_kind(kind: Option<SpecializationKind>, default: RelKind) -> RelKind {
    match kind {
        Some(SpecializationKind::Specializes) => RelKind::Specializes,
        Some(SpecializationKind::Subsets) => RelKind::Subsets,
        Some(SpecializationKind::Redefines) => RelKind::Redefines,
        Some(SpecializationKind::References) => RelKind::References,
        Some(SpecializationKind::Conjugates) => RelKind::Specializes,
        Some(SpecializationKind::FeatureChain) => RelKind::FeatureChain,
        None => default, // Comma-continuation
    }
}

/// Determine the internal usage kind for a Usage AST node.
#[allow(dead_code)]
fn determine_usage_kind(usage: &Usage) -> InternalUsageKind {
    // Check for nested transition first, then perform action
    if usage.transition_usage().is_some() {
        InternalUsageKind::Transition
    } else if usage.perform_action_usage().is_some() {
        InternalUsageKind::Action
    } else {
        match usage.usage_kind() {
            Some(UsageKind::Part) => InternalUsageKind::Part,
            Some(UsageKind::Attribute) => InternalUsageKind::Attribute,
            Some(UsageKind::Port) => InternalUsageKind::Port,
            Some(UsageKind::Item) => InternalUsageKind::Item,
            Some(UsageKind::Action) => InternalUsageKind::Action,
            Some(UsageKind::State) => InternalUsageKind::State,
            Some(UsageKind::Constraint) => InternalUsageKind::Constraint,
            Some(UsageKind::Requirement) => InternalUsageKind::Requirement,
            Some(UsageKind::Calc) => InternalUsageKind::Calculation,
            Some(UsageKind::Connection) => InternalUsageKind::Connection,
            Some(UsageKind::Interface) => InternalUsageKind::Interface,
            Some(UsageKind::Allocation) => InternalUsageKind::Allocation,
            Some(UsageKind::Flow) => InternalUsageKind::Flow,
            Some(UsageKind::Occurrence) => InternalUsageKind::Occurrence,
            Some(UsageKind::Ref) => InternalUsageKind::Reference,
            Some(UsageKind::Feature) => InternalUsageKind::Attribute,
            Some(UsageKind::Step) => InternalUsageKind::Action,
            Some(UsageKind::Expr) => InternalUsageKind::Calculation,
            Some(UsageKind::Connector) => InternalUsageKind::Connection,
            Some(UsageKind::Case) => InternalUsageKind::Other,
            None => InternalUsageKind::Part, // Default to Part for usages without keyword
        }
    }
}

/// Map DefinitionKind to implicit supertype name.
#[allow(dead_code)]
fn implicit_supertype_for_definition_kind(kind: Option<DefinitionKind>) -> Option<&'static str> {
    match kind {
        Some(DefinitionKind::Part) | Some(DefinitionKind::Class) | Some(DefinitionKind::Struct)
        | Some(DefinitionKind::Classifier) => Some("Parts::Part"),
        Some(DefinitionKind::Item) => Some("Items::Item"),
        Some(DefinitionKind::Action) | Some(DefinitionKind::Behavior)
        | Some(DefinitionKind::Interaction) => Some("Actions::Action"),
        Some(DefinitionKind::State) => Some("States::StateAction"),
        Some(DefinitionKind::Constraint) | Some(DefinitionKind::Predicate) => {
            Some("Constraints::ConstraintCheck")
        }
        Some(DefinitionKind::Requirement) => Some("Requirements::RequirementCheck"),
        Some(DefinitionKind::Calc) | Some(DefinitionKind::Function) => {
            Some("Calculations::Calculation")
        }
        Some(DefinitionKind::Port) => Some("Ports::Port"),
        Some(DefinitionKind::Connection) | Some(DefinitionKind::Assoc) => {
            Some("Connections::BinaryConnection")
        }
        Some(DefinitionKind::Interface) => Some("Interfaces::Interface"),
        Some(DefinitionKind::Allocation) => Some("Allocations::Allocation"),
        Some(DefinitionKind::UseCase) | Some(DefinitionKind::Case) => Some("UseCases::UseCase"),
        Some(DefinitionKind::Analysis) | Some(DefinitionKind::Verification) => {
            Some("AnalysisCases::AnalysisCase")
        }
        Some(DefinitionKind::Attribute) | Some(DefinitionKind::Datatype) => {
            Some("Attributes::AttributeValue")
        }
        _ => None,
    }
}

/// Map InternalUsageKind to implicit supertype name.
#[allow(dead_code)]
fn implicit_supertype_for_internal_usage_kind(kind: InternalUsageKind) -> Option<&'static str> {
    match kind {
        InternalUsageKind::Part => Some("Parts::Part"),
        InternalUsageKind::Item => Some("Items::Item"),
        InternalUsageKind::Action => Some("Actions::Action"),
        InternalUsageKind::State => Some("States::StateAction"),
        InternalUsageKind::Flow => Some("Flows::Message"),
        InternalUsageKind::Connection => Some("Connections::Connection"),
        InternalUsageKind::Interface => Some("Interfaces::Interface"),
        InternalUsageKind::Allocation => Some("Allocations::Allocation"),
        InternalUsageKind::Requirement => Some("Requirements::RequirementCheck"),
        InternalUsageKind::Constraint => Some("Constraints::ConstraintCheck"),
        InternalUsageKind::Calculation => Some("Calculations::Calculation"),
        InternalUsageKind::Port => Some("Ports::Port"),
        InternalUsageKind::Attribute => Some("Attributes::AttributeValue"),
        _ => None,
    }
}

/// Extract type references from ExtractedRel relationships.
#[allow(dead_code)]
fn extract_type_refs(
    relationships: &[ExtractedRel],
    line_index: &crate::base::LineIndex,
) -> Vec<TypeRefKind> {
    let mut type_refs = Vec::new();

    for rel in relationships.iter() {
        let ref_kind = RefKind::from_rel_kind(rel.kind);

        match &rel.target {
            RelTarget::Chain(chain) => {
                let num_parts = chain.parts.len();
                let parts: Vec<TypeRef> = chain
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(idx, part)| {
                        let (start_line, start_col, end_line, end_col) = if let Some(r) = part.range
                        {
                            let start = line_index.line_col(r.start());
                            let end = line_index.line_col(r.end());
                            (start.line, start.col, end.line, end.col)
                        } else if idx == num_parts - 1 {
                            if let Some(r) = rel.range {
                                let start = line_index.line_col(r.start());
                                let end = line_index.line_col(r.end());
                                (start.line, start.col, end.line, end.col)
                            } else {
                                (0, 0, 0, 0)
                            }
                        } else {
                            (0, 0, 0, 0)
                        };
                        TypeRef {
                            target: Arc::from(part.name.as_str()),
                            resolved_target: None,
                            kind: ref_kind,
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        }
                    })
                    .collect();

                if !parts.is_empty() {
                    type_refs.push(TypeRefKind::Chain(TypeRefChain { parts }));
                }
            }
            RelTarget::Simple(target) => {
                if let Some(r) = rel.range {
                    let start = line_index.line_col(r.start());
                    let end = line_index.line_col(r.end());
                    type_refs.push(TypeRefKind::Simple(TypeRef {
                        target: Arc::from(target.as_str()),
                        resolved_target: None,
                        kind: ref_kind,
                        start_line: start.line,
                        start_col: start.col,
                        end_line: end.line,
                        end_col: end.col,
                    }));

                    // Also add prefix segments as references
                    let parts: Vec<&str> = target.split("::").collect();
                    if parts.len() > 1 {
                        let mut prefix = String::new();
                        for (i, part) in parts.iter().enumerate() {
                            if i == parts.len() - 1 {
                                break;
                            }
                            if !prefix.is_empty() {
                                prefix.push_str("::");
                            }
                            prefix.push_str(part);

                            type_refs.push(TypeRefKind::Simple(TypeRef {
                                target: Arc::from(prefix.as_str()),
                                resolved_target: None,
                                kind: ref_kind,
                                start_line: start.line,
                                start_col: start.col,
                                end_line: end.line,
                                end_col: end.col,
                            }));
                        }
                    }
                }
            }
        }
    }

    type_refs
}

/// Extract HirRelationship values from ExtractedRel relationships.
#[allow(dead_code)]
fn extract_hir_relationships(
    relationships: &[ExtractedRel],
    line_index: &crate::base::LineIndex,
) -> Vec<HirRelationship> {
    relationships
        .iter()
        .filter_map(|rel| {
            RelationshipKind::from_rel_kind(rel.kind).map(|kind| {
                let (start_line, start_col, end_line, end_col) = rel
                    .range
                    .map(|r| {
                        let start = line_index.line_col(r.start());
                        let end = line_index.line_col(r.end());
                        (start.line, start.col, end.line, end.col)
                    })
                    .unwrap_or((0, 0, 0, 0));
                HirRelationship::with_span(
                    kind,
                    rel.target.as_str().as_ref(),
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                )
            })
        })
        .collect()
}

/// Extract metadata annotations from ExtractedRel relationships and NamespaceMember children.
#[allow(dead_code)]
fn extract_metadata_from_rels(
    relationships: &[ExtractedRel],
    children: &[NormalizedElement],
) -> Vec<Arc<str>> {
    let mut annotations = Vec::new();

    for rel in relationships.iter() {
        if matches!(rel.kind, RelKind::Meta) {
            let target = rel.target.as_str();
            let simple_name = target.rsplit("::").next().unwrap_or(&target);
            annotations.push(Arc::from(simple_name));
        }
    }

    for child in children.iter() {
        if let NormalizedElement::Usage(usage) = child {
            if usage.name.is_none() {
                for rel in &usage.relationships {
                    if matches!(rel.kind, NormalizedRelKind::TypedBy) {
                        let target = rel.target.as_str();
                        let simple_name = target.rsplit("::").next().unwrap_or(&target);
                        annotations.push(Arc::from(simple_name));
                    }
                }
            }
        }
    }

    annotations
}

// ============================================================================
// UNIFIED EXTRACTION (using normalized types)
// ============================================================================

/// Result of symbol extraction, including both symbols and scope filters.
#[derive(Debug, Default)]
pub struct ExtractionResult {
    /// Extracted symbols.
    pub symbols: Vec<HirSymbol>,
    /// Filters for each scope (scope qualified name -> metadata names).
    /// Elements imported into a scope must have ALL listed metadata to be visible.
    /// These come from `filter @Metadata;` statements.
    pub scope_filters: Vec<(Arc<str>, Vec<String>)>,
    /// Filters for specific imports (import qualified name -> metadata names).
    /// These come from bracket syntax: `import X::*[@Filter]`
    pub import_filters: Vec<(Arc<str>, Vec<String>)>,
}

/// Extract all symbols from any syntax file using the normalized adapter layer.
///
/// This is the preferred extraction function as it handles both SysML and KerML
/// through a unified code path.
pub fn extract_symbols_unified(file: FileId, syntax: &crate::syntax::SyntaxFile) -> Vec<HirSymbol> {
    extract_with_filters(file, syntax).symbols
}

/// Extract symbols and filters from any syntax file.
///
/// Returns both symbols and scope filter information for import filtering.
pub fn extract_with_filters(file: FileId, syntax: &crate::syntax::SyntaxFile) -> ExtractionResult {
    let mut result = ExtractionResult::default();
    let line_index = syntax.line_index();
    let mut context = ExtractionContext {
        file,
        prefix: String::new(),
        anon_counter: 0,
        scope_stack: Vec::new(),
        line_index,
    };

    // Get the rowan SourceFile and iterate over its members
    if let Some(source_file) = syntax.source_file() {
        for member in source_file.members() {
            let normalized = NormalizedElement::from_rowan(&member);
            extract_from_normalized(&mut result, &mut context, &normalized);
        }
    }

    result
}

/// Extract from a normalized element.
fn extract_from_normalized(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    element: &NormalizedElement,
) {
    match element {
        NormalizedElement::Package(pkg) => extract_from_normalized_package(result, ctx, pkg),
        NormalizedElement::Definition(def) => {
            extract_from_normalized_definition(&mut result.symbols, ctx, def)
        }
        NormalizedElement::Usage(usage) => {
            extract_from_normalized_usage(&mut result.symbols, ctx, usage)
        }
        NormalizedElement::Import(import) => {
            // Extract import symbol
            extract_from_normalized_import(&mut result.symbols, ctx, import);
            // Store bracket filters if present
            if !import.filters.is_empty() {
                let import_qname = ctx.qualified_name(&format!("import:{}", import.path));
                result
                    .import_filters
                    .push((Arc::from(import_qname.as_str()), import.filters.clone()));
            }
        }
        NormalizedElement::Alias(alias) => {
            extract_from_normalized_alias(&mut result.symbols, ctx, alias)
        }
        NormalizedElement::Comment(comment) => {
            extract_from_normalized_comment(&mut result.symbols, ctx, comment)
        }
        NormalizedElement::Dependency(dep) => {
            extract_from_normalized_dependency(&mut result.symbols, ctx, dep)
        }
        NormalizedElement::Filter(filter) => {
            // Store filter for current scope (for import filtering)
            let scope = ctx.current_scope_name();
            if !filter.metadata_refs.is_empty() {
                result.scope_filters.push((
                    Arc::from(scope.as_str()),
                    filter.metadata_refs.iter().map(|s| s.to_string()).collect(),
                ));
            }

            // Create type_refs for all qualified names in the filter expression
            // so hover/go-to-def works on filter terms like `Safety::isMandatory`
            if !filter.all_refs.is_empty() {
                let type_refs: Vec<TypeRefKind> = filter
                    .all_refs
                    .iter()
                    .map(|(name, range)| {
                        let start = ctx.line_index.line_col(range.start());
                        let end = ctx.line_index.line_col(range.end());
                        TypeRefKind::Simple(TypeRef {
                            target: Arc::from(name.as_str()),
                            resolved_target: None,
                            kind: RefKind::Other, // Filter refs are expression-like
                            start_line: start.line,
                            start_col: start.col,
                            end_line: end.line,
                            end_col: end.col,
                        })
                    })
                    .collect();

                // Create an anonymous symbol to hold the type_refs
                let span = ctx.range_to_info(filter.range);
                let filter_qname = ctx.qualified_name(&format!("<filter@L{}>", span.start_line));
                result.symbols.push(HirSymbol {
                    name: Arc::from("<filter>"),
                    short_name: None,
                    qualified_name: Arc::from(filter_qname.as_str()),
                    element_id: new_element_id(),
                    kind: SymbolKind::Other,
                    file: ctx.file,
                    start_line: span.start_line,
                    start_col: span.start_col,
                    end_line: span.end_line,
                    end_col: span.end_col,
                    short_name_start_line: None,
                    short_name_start_col: None,
                    short_name_end_line: None,
                    short_name_end_col: None,
                    doc: None,
                    supertypes: Vec::new(),
                    relationships: Vec::new(),
                    type_refs,
                    is_public: false,
                    view_data: None,
                    metadata_annotations: Vec::new(),
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
                });
            }
        }
        NormalizedElement::Expose(_expose) => {
            // Expose relationships are handled during view extraction, not symbol extraction
        }
    }
}

/// Extract from a normalized element into a symbol list (no filter support).
/// Used for nested extraction within definitions/usages.
fn extract_from_normalized_into_symbols(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    element: &NormalizedElement,
) {
    match element {
        NormalizedElement::Package(pkg) => {
            // For nested packages, create a temporary result
            let mut result = ExtractionResult::default();
            extract_from_normalized_package(&mut result, ctx, pkg);
            symbols.extend(result.symbols);
        }
        NormalizedElement::Definition(def) => extract_from_normalized_definition(symbols, ctx, def),
        NormalizedElement::Usage(usage) => extract_from_normalized_usage(symbols, ctx, usage),
        NormalizedElement::Import(import) => extract_from_normalized_import(symbols, ctx, import),
        NormalizedElement::Alias(alias) => extract_from_normalized_alias(symbols, ctx, alias),
        NormalizedElement::Comment(comment) => {
            extract_from_normalized_comment(symbols, ctx, comment)
        }
        NormalizedElement::Dependency(dep) => extract_from_normalized_dependency(symbols, ctx, dep),
        NormalizedElement::Filter(_filter) => {
            // Filters don't produce symbols, they're metadata for views
            // They will be extracted when processing the parent view definition
        }
        NormalizedElement::Expose(_expose) => {
            // Expose relationships don't produce symbols, they define view visibility
            // They will be extracted when processing the parent view definition/usage
        }
    }
}

fn extract_from_normalized_package(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    pkg: &NormalizedPackage,
) {
    let name = match &pkg.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    // Use name_range for precise position, fall back to full range
    let span = ctx.range_to_info(pkg.name_range.or(pkg.range));

    // Extract doc comment
    let doc = pkg.doc.as_ref().map(|s| Arc::from(s.trim()));

    result.symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: pkg.short_name.as_ref().map(|s| Arc::from(s.as_str())),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Package,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc,
        supertypes: Vec::new(),
        relationships: Vec::new(),
        type_refs: Vec::new(),
        is_public: false,
        view_data: None,
        metadata_annotations: Vec::new(),
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
    });

    ctx.push_scope(&name);
    for child in &pkg.children {
        extract_from_normalized(result, ctx, child);
    }
    ctx.pop_scope();
}

/// Get the implicit supertype for a definition kind based on SysML kernel library.
/// In SysML, all definitions implicitly specialize their kernel metaclass:
/// - `part def X` implicitly specializes `Parts::Part`
/// - `item def X` implicitly specializes `Items::Item`
/// - `action def X` implicitly specializes `Actions::Action`
/// - etc.
fn implicit_supertype_for_def_kind(kind: NormalizedDefKind) -> Option<&'static str> {
    match kind {
        NormalizedDefKind::Part => Some("Parts::Part"),
        NormalizedDefKind::Item => Some("Items::Item"),
        NormalizedDefKind::Action => Some("Actions::Action"),
        NormalizedDefKind::State => Some("States::StateAction"),
        NormalizedDefKind::Constraint => Some("Constraints::ConstraintCheck"),
        NormalizedDefKind::Requirement => Some("Requirements::RequirementCheck"),
        NormalizedDefKind::Calculation => Some("Calculations::Calculation"),
        NormalizedDefKind::Port => Some("Ports::Port"),
        // Use BinaryConnection for connection def since most connections are binary
        // and need access to source/target features from BinaryLinkObject
        NormalizedDefKind::Connection => Some("Connections::BinaryConnection"),
        NormalizedDefKind::Interface => Some("Interfaces::Interface"),
        NormalizedDefKind::Allocation => Some("Allocations::Allocation"),
        NormalizedDefKind::UseCase => Some("UseCases::UseCase"),
        NormalizedDefKind::AnalysisCase => Some("AnalysisCases::AnalysisCase"),
        NormalizedDefKind::Attribute => Some("Attributes::AttributeValue"),
        _ => None,
    }
}

/// Get the implicit supertype for a usage kind based on SysML kernel library.
/// In SysML, usages implicitly specialize their kernel metaclass base type:
/// - `part x` implicitly specializes `Parts::Part`
/// - `item x` implicitly specializes `Items::Item`
/// - `message x` implicitly specializes `Flows::Message`
/// - `flow x` implicitly specializes `Flows::Flow`
/// - etc.
fn implicit_supertype_for_usage_kind(kind: NormalizedUsageKind) -> Option<&'static str> {
    match kind {
        NormalizedUsageKind::Part => Some("Parts::Part"),
        NormalizedUsageKind::Item => Some("Items::Item"),
        NormalizedUsageKind::Action => Some("Actions::Action"),
        NormalizedUsageKind::State => Some("States::StateAction"),
        NormalizedUsageKind::Flow => Some("Flows::Message"),
        NormalizedUsageKind::Connection => Some("Connections::Connection"),
        NormalizedUsageKind::Interface => Some("Interfaces::Interface"),
        NormalizedUsageKind::Allocation => Some("Allocations::Allocation"),
        NormalizedUsageKind::Requirement => Some("Requirements::RequirementCheck"),
        NormalizedUsageKind::Constraint => Some("Constraints::ConstraintCheck"),
        NormalizedUsageKind::Calculation => Some("Calculations::Calculation"),
        NormalizedUsageKind::Port => Some("Ports::Port"),
        NormalizedUsageKind::Attribute => Some("Attributes::AttributeValue"),
        _ => None,
    }
}

/// Extract relationships from normalized relationships.
fn extract_relationships_from_normalized(
    relationships: &[NormalizedRelationship],
    line_index: &crate::base::LineIndex,
) -> Vec<HirRelationship> {
    relationships
        .iter()
        .filter_map(|rel| {
            RelationshipKind::from_rel_kind(normalized_to_rel_kind(rel.kind)).map(|kind| {
                let (start_line, start_col, end_line, end_col) = rel
                    .range
                    .map(|r| {
                        let start = line_index.line_col(r.start());
                        let end = line_index.line_col(r.end());
                        (start.line, start.col, end.line, end.col)
                    })
                    .unwrap_or((0, 0, 0, 0));
                HirRelationship::with_span(
                    kind,
                    rel.target.as_str().as_ref(),
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                )
            })
        })
        .collect()
}

/// Extract metadata annotations from normalized relationships and children.
/// Returns the simple names of metadata types applied to this element.
///
/// Metadata can be applied in two ways:
/// 1. Via `relationships.meta` (e.g., from expression contexts)
/// 2. Via nested metadata usage children (e.g., `part x { @Safety; }`)
///
/// For nested metadata usages, we look at children that have no original name
/// (anonymous usages) and are typed by metadata definitions.
fn extract_metadata_annotations(
    relationships: &[NormalizedRelationship],
    children: &[NormalizedElement],
) -> Vec<Arc<str>> {
    let mut annotations = Vec::new();

    // Extract from relationships (Meta kind)
    for rel in relationships.iter() {
        if matches!(rel.kind, NormalizedRelKind::Meta) {
            let target = rel.target.as_str();
            let simple_name = target.rsplit("::").next().unwrap_or(&target);
            annotations.push(Arc::from(simple_name));
        }
    }

    // Extract from children that are metadata usages
    // In SysML, `@Safety` inside a body becomes a child usage typed by Safety
    // These children are originally anonymous (name=None in the source) but get
    // generated anon names during extraction. We identify them by:
    // 1. Having no original name (name.is_none()) - they're originally anonymous
    // 2. Having a TypedBy relationship to what looks like a metadata type
    for child in children.iter() {
        if let NormalizedElement::Usage(usage) = child {
            // Metadata usages are originally anonymous in the source
            // (the name gets assigned later during symbol extraction)
            if usage.name.is_none() {
                for rel in &usage.relationships {
                    if matches!(rel.kind, NormalizedRelKind::TypedBy) {
                        let target = rel.target.as_str();
                        let simple_name = target.rsplit("::").next().unwrap_or(&target);
                        annotations.push(Arc::from(simple_name));
                    }
                }
            }
        }
    }

    annotations
}

fn extract_from_normalized_definition(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    def: &NormalizedDefinition,
) {
    let name = match &def.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    let kind = SymbolKind::from_normalized_def_kind(def.kind);
    // Use name_range for precise position, fall back to full range
    let span = ctx.range_to_info(def.name_range.or(def.range));
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) =
        ctx.range_to_optional(def.short_name_range);

    // Extract explicit supertypes from relationships
    let mut supertypes: Vec<Arc<str>> = def
        .relationships
        .iter()
        .filter(|r| matches!(r.kind, NormalizedRelKind::Specializes))
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();

    // Add implicit supertypes from SysML kernel library if no explicit specialization
    // This models the implicit inheritance: part def → Part, item def → Item, etc.
    if supertypes.is_empty() {
        if let Some(implicit) = implicit_supertype_for_def_kind(def.kind) {
            supertypes.push(Arc::from(implicit));
        }
    }

    // Extract type references from relationships
    let type_refs = extract_type_refs_from_normalized(&def.relationships, &ctx.line_index);

    // Extract all relationships for hover display
    let relationships = extract_relationships_from_normalized(&def.relationships, &ctx.line_index);

    // Extract metadata annotations for filter imports
    let metadata_annotations = extract_metadata_annotations(&def.relationships, &def.children);

    // Extract doc comment
    let doc = def.doc.as_ref().map(|s| Arc::from(s.trim()));

    // Extract view-specific data if this is a view/viewpoint/rendering
    let view_data = extract_view_data_from_definition(def, def.kind);

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: def.short_name.as_ref().map(|s| Arc::from(s.as_str())),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: sn_start_line,
        short_name_start_col: sn_start_col,
        short_name_end_line: sn_end_line,
        short_name_end_col: sn_end_col,
        doc,
        supertypes,
        relationships,
        type_refs,
        is_public: false,
        view_data,
        metadata_annotations,
        is_abstract: def.is_abstract,
        is_variation: def.is_variation,
        is_readonly: false,
        is_derived: false,
        is_parallel: false,
        is_individual: def.is_individual,
        is_end: false,
        is_default: false,
        is_ordered: false,
        is_nonunique: false,
        is_portion: false,
        direction: None,
        multiplicity: None,
        value: None,
    });

    // Recurse into children
    ctx.push_scope(&name);
    for child in &def.children {
        extract_from_normalized_into_symbols(symbols, ctx, child);
    }
    ctx.pop_scope();
}

fn extract_from_normalized_usage(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    usage: &NormalizedUsage,
) {
    // Extract type references even for anonymous usages
    let type_refs = extract_type_refs_from_normalized(&usage.relationships, &ctx.line_index);

    // Extract all relationships for hover display
    let relationships =
        extract_relationships_from_normalized(&usage.relationships, &ctx.line_index);

    // Extract metadata annotations for filter imports
    let metadata_annotations = extract_metadata_annotations(&usage.relationships, &usage.children);

    // For anonymous usages, attach refs to the parent but still recurse into children
    let name = match &usage.name {
        Some(n) => strip_quotes(n),
        None => {
            // Attach ONLY type refs (not feature refs) to parent for anonymous usages
            // Feature refs (Redefines, Subsets, Specializes) need inheritance context
            // that the parent doesn't have
            // Also skip packages - they shouldn't have type_refs from their anonymous children
            if !type_refs.is_empty() {
                if let Some(parent) = symbols
                    .iter_mut()
                    .rev()
                    .find(|s| s.qualified_name.as_ref() == ctx.prefix)
                {
                    // Skip packages - they shouldn't inherit type_refs from anonymous children
                    if parent.kind != SymbolKind::Package {
                        // Only extend TypedBy refs - feature refs would cause false "undefined reference" errors
                        let typing_refs: Vec<_> = type_refs
                            .iter()
                            .filter(
                                |tr| matches!(tr, TypeRefKind::Simple(r) if r.kind == RefKind::TypedBy),
                            )
                            .cloned()
                            .collect();
                        parent.type_refs.extend(typing_refs);
                    }
                }
            }

            // Generate unique anonymous scope name for children
            // Try to use relationship target for meaningful names, otherwise use generic anon
            let line = usage
                .range
                .map(|r| ctx.line_index.line_col(r.start()).line)
                .unwrap_or(0);

            let anon_scope = usage
                .relationships
                .iter()
                .find(|r| !matches!(r.kind, NormalizedRelKind::Expression))
                .map(|r| {
                    let prefix = match r.kind {
                        NormalizedRelKind::Subsets => ":>",
                        NormalizedRelKind::TypedBy => ":",
                        NormalizedRelKind::Specializes => ":>:",
                        NormalizedRelKind::Redefines => ":>>",
                        NormalizedRelKind::About => "about:",
                        NormalizedRelKind::Performs => "perform:",
                        NormalizedRelKind::Satisfies => "satisfy:",
                        NormalizedRelKind::Exhibits => "exhibit:",
                        NormalizedRelKind::Includes => "include:",
                        NormalizedRelKind::Asserts => "assert:",
                        NormalizedRelKind::Verifies => "verify:",
                        NormalizedRelKind::References => "ref:",
                        NormalizedRelKind::Meta => "meta:",
                        NormalizedRelKind::Crosses => "crosses:",
                        NormalizedRelKind::Expression => "~",
                        NormalizedRelKind::FeatureChain => "chain:",
                        NormalizedRelKind::Conjugates => "~:",
                        // State/Transition
                        NormalizedRelKind::TransitionSource => "from:",
                        NormalizedRelKind::TransitionTarget => "then:",
                        NormalizedRelKind::SuccessionSource => "first:",
                        NormalizedRelKind::SuccessionTarget => "then:",
                        // Message
                        NormalizedRelKind::AcceptedMessage => "accept:",
                        NormalizedRelKind::AcceptVia => "via:",
                        NormalizedRelKind::SentMessage => "send:",
                        NormalizedRelKind::SendVia => "via:",
                        NormalizedRelKind::SendTo => "to:",
                        NormalizedRelKind::MessageSource => "from:",
                        NormalizedRelKind::MessageTarget => "to:",
                        // Requirement/Constraint
                        NormalizedRelKind::Assumes => "assume:",
                        NormalizedRelKind::Requires => "require:",
                        // Allocation/Connection
                        NormalizedRelKind::AllocateSource => "allocate:",
                        NormalizedRelKind::AllocateTo => "to:",
                        NormalizedRelKind::BindSource => "bind:",
                        NormalizedRelKind::BindTarget => "=:",
                        NormalizedRelKind::ConnectSource => "connect:",
                        NormalizedRelKind::ConnectTarget => "to:",
                        NormalizedRelKind::FlowItem => "flow:",
                        NormalizedRelKind::FlowSource => "from:",
                        NormalizedRelKind::FlowTarget => "to:",
                        NormalizedRelKind::InterfaceEnd => "end:",
                        // View
                        NormalizedRelKind::Exposes => "expose:",
                        NormalizedRelKind::Renders => "render:",
                        NormalizedRelKind::Filters => "filter:",
                        // Dependency
                        NormalizedRelKind::DependencySource => "dep:",
                        NormalizedRelKind::DependencyTarget => "to:",
                    };
                    ctx.next_anon_scope(prefix, &r.target.as_str(), line)
                })
                // Fallback: always create a unique scope for anonymous usages with children
                .unwrap_or_else(|| ctx.next_anon_scope("anon", "", line));

            // Create a symbol for the anonymous usage so it can be looked up during resolution
            // This is needed for satisfy/perform/exhibit blocks where children need to resolve
            // redefines in the context of the satisfied/performed/exhibited element
            let qualified_name = ctx.qualified_name(&anon_scope);
            let kind = SymbolKind::from_normalized_usage_kind(usage.kind);
            // For anonymous usages, use the first non-expression relationship's range as the span
            // This ensures hover works on the redefines/subsets target, not the keyword (e.g., "port")
            let anon_span_range = usage
                .relationships
                .iter()
                .find(|r| !matches!(r.kind, NormalizedRelKind::Expression))
                .and_then(|r| r.range)
                .or(usage.range);
            let span = ctx.range_to_info(anon_span_range);

            // Build supertypes for anonymous symbol:
            // 1. From relationships (Redefines, Subsets, TypedBy, Specializes, Satisfies, Verifies)
            // 2. From parent's TypedBy (so we can resolve inherited members)
            // Note: Satisfies/Verifies blocks should inherit from the satisfied/verified requirement
            //       so that nested members can resolve redefines targets in the satisfied element
            let mut anon_supertypes: Vec<Arc<str>> = usage
                .relationships
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        NormalizedRelKind::TypedBy
                            | NormalizedRelKind::Subsets
                            | NormalizedRelKind::Specializes
                            | NormalizedRelKind::Redefines
                            | NormalizedRelKind::Satisfies
                            | NormalizedRelKind::Verifies
                    )
                })
                .map(|r| Arc::from(r.target.as_str().as_ref()))
                .collect();

            // Add parent's TypedBy types so inherited members are visible
            // This handles cases like: `spatialCF: Type { :>> mRefs }` where mRefs is from Type
            // But skip this for:
            // 1. Anonymous scopes that are pure expression containers (inv, etc.)
            // 2. Connection-like usages (bind, connect, flow, interface) - they're structural, not type hierarchies
            let is_expression_scope = usage.name.is_none()
                && usage
                    .relationships
                    .iter()
                    .all(|r| matches!(r.kind, NormalizedRelKind::Expression));

            // Connection kinds shouldn't inherit parent supertypes - they define connections, not type inheritance
            let is_connection_kind = matches!(
                usage.kind,
                NormalizedUsageKind::Connection
                    | NormalizedUsageKind::Flow
                    | NormalizedUsageKind::Interface
                    | NormalizedUsageKind::Allocation
            );

            if !is_expression_scope && !is_connection_kind {
                if let Some(parent) = symbols
                    .iter()
                    .rev()
                    .find(|s| s.qualified_name.as_ref() == ctx.prefix)
                {
                    for supertype in &parent.supertypes {
                        if !anon_supertypes.contains(supertype) {
                            anon_supertypes.push(supertype.clone());
                        }
                    }
                }
            }

            let anon_symbol = HirSymbol {
                file: ctx.file,
                name: Arc::from(anon_scope.as_str()),
                short_name: None,
                qualified_name: Arc::from(qualified_name.as_str()),
                element_id: new_element_id(),
                kind,
                start_line: span.start_line,
                start_col: span.start_col,
                end_line: span.end_line,
                end_col: span.end_col,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: anon_supertypes,
                relationships: relationships.clone(),
                type_refs,
                doc: None,
                is_public: false,
                view_data: None,
                metadata_annotations: metadata_annotations.clone(),
                is_abstract: usage.is_abstract,
                is_variation: usage.is_variation,
                is_readonly: usage.is_readonly,
                is_derived: usage.is_derived,
                is_parallel: usage.is_parallel,
                is_individual: usage.is_individual,
                is_end: usage.is_end,
                is_default: usage.is_default,
                is_ordered: usage.is_ordered,
                is_nonunique: usage.is_nonunique,
                is_portion: usage.is_portion,
                direction: usage.direction,
                multiplicity: usage.multiplicity,
                value: None,
            };
            symbols.push(anon_symbol);

            // Push scope for children of anonymous usages
            ctx.push_scope(&anon_scope);

            // Recurse into children for anonymous usages
            for child in &usage.children {
                extract_from_normalized_into_symbols(symbols, ctx, child);
            }

            ctx.pop_scope();
            return;
        }
    };

    let qualified_name = ctx.qualified_name(&name);
    let kind = SymbolKind::from_normalized_usage_kind(usage.kind);
    // Use name_range for precise position, fall back to full range
    let span = ctx.range_to_info(usage.name_range.or(usage.range));
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) =
        ctx.range_to_optional(usage.short_name_range);

    // Extract typing and subsetting as supertypes
    // For member resolution, we need to look in:
    // - TypedBy: explicit type annotation (`: Type`)
    // - Subsets: subset relationship (`:>` on usages)
    // - Specializes: specialization (`:>` can also be specializes in some contexts)
    // - Redefines: redefinition (`:>>` on usages) - needed for inherited member lookup
    // - Performs/Exhibits/Includes/etc.: domain-specific relationships that establish type context
    //   e.g., `perform takePicture :> TakePicture` - the performed action defines the type hierarchy
    let mut supertypes: Vec<Arc<str>> = usage
        .relationships
        .iter()
        .filter(|r| {
            matches!(
                r.kind,
                NormalizedRelKind::TypedBy
                    | NormalizedRelKind::Subsets
                    | NormalizedRelKind::Specializes
                    | NormalizedRelKind::Redefines
                    | NormalizedRelKind::Performs
                    | NormalizedRelKind::Exhibits
                    | NormalizedRelKind::Includes
                    | NormalizedRelKind::Satisfies
                    | NormalizedRelKind::Asserts
                    | NormalizedRelKind::Verifies
            )
        })
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();

    // Detect implicit redefinition: if parent has a type, and that type has a member
    // with the same name as this usage, then this usage implicitly redefines that member.
    // e.g., `action transport : TransportScenario { action trigger { ... } }`
    // Here `transport::trigger` implicitly redefines `TransportScenario::trigger`
    if supertypes.is_empty() && !ctx.prefix.is_empty() {
        // Find the parent symbol
        if let Some(parent) = symbols
            .iter()
            .rev()
            .find(|s| s.qualified_name.as_ref() == ctx.prefix)
        {
            // Check if parent has a type
            if let Some(parent_type) = parent.supertypes.first() {
                // The parent_type might be unqualified (e.g., "TransportScenario")
                // We need to find the fully qualified version
                let parent_type_qualified = symbols
                    .iter()
                    .find(|s| {
                        s.name.as_ref() == parent_type.as_ref()
                            || s.qualified_name.as_ref() == parent_type.as_ref()
                    })
                    .map(|s| s.qualified_name.clone());

                if let Some(type_qname) = parent_type_qualified {
                    // Look for a member in the parent's type with the same name
                    let potential_redef = format!("{}::{}", type_qname, name);
                    if symbols
                        .iter()
                        .any(|s| s.qualified_name.as_ref() == potential_redef)
                    {
                        supertypes.push(Arc::from(potential_redef));
                    }
                }
            }
        }
    }

    // Add implicit supertypes from SysML kernel library if no explicit supertypes present
    // This models the implicit inheritance: part → Parts::Part, item → Items::Item, etc.
    // Only add if no explicit type/specialization (otherwise we rely on that)
    if supertypes.is_empty() {
        if let Some(implicit) = implicit_supertype_for_usage_kind(usage.kind) {
            supertypes.push(Arc::from(implicit));
        }
    }

    // Extract doc comment
    let doc = usage.doc.as_ref().map(|s| Arc::from(s.trim()));

    // Extract view-specific data if this is a view/viewpoint/rendering
    let typed_by = supertypes.first();
    let view_data = extract_view_data_from_usage(usage, usage.kind, typed_by);

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: usage.short_name.as_ref().map(|s| Arc::from(s.as_str())),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: sn_start_line,
        short_name_start_col: sn_start_col,
        short_name_end_line: sn_end_line,
        short_name_end_col: sn_end_col,
        doc,
        supertypes,
        relationships,
        type_refs,
        is_public: false,
        view_data,
        metadata_annotations,
        is_abstract: usage.is_abstract,
        is_variation: usage.is_variation,
        is_readonly: usage.is_readonly,
        is_derived: usage.is_derived,
        is_parallel: usage.is_parallel,
        is_individual: usage.is_individual,
        is_end: usage.is_end,
        is_default: usage.is_default,
        is_ordered: usage.is_ordered,
        is_nonunique: usage.is_nonunique,
        is_portion: usage.is_portion,
        direction: usage.direction,
        multiplicity: usage.multiplicity,
        value: usage.value.clone(),
    });

    // Recurse into children
    ctx.push_scope(&name);
    for child in &usage.children {
        extract_from_normalized_into_symbols(symbols, ctx, child);
    }
    ctx.pop_scope();
}

fn extract_from_normalized_import(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    import: &NormalizedImport,
) {
    let path = &import.path;
    let qualified_name = ctx.qualified_name(&format!("import:{}", path));

    // Use path_range for the symbol span since name is the path.
    // Fall back to full range if path_range is not available.
    let span = import
        .path_range
        .map(|r| ctx.range_to_info(Some(r)))
        .unwrap_or_else(|| ctx.range_to_info(import.range));

    // Create type_ref for the import target so hover/go-to-def works on it
    // Strip ::* or ::** suffix to get the actual package name
    let target_path = path
        .strip_suffix("::**")
        .or_else(|| path.strip_suffix("::*"))
        .unwrap_or(path);

    let type_refs = if let Some(r) = import.path_range {
        let start = ctx.line_index.line_col(r.start());
        let end = ctx.line_index.line_col(r.end());
        vec![TypeRefKind::Simple(TypeRef {
            target: Arc::from(target_path),
            resolved_target: None,
            kind: RefKind::Other, // Import targets are special
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col,
        })]
    } else {
        Vec::new()
    };

    symbols.push(HirSymbol {
        name: Arc::from(path.as_str()),
        short_name: None, // Imports don't have short names
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Import,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: None,
        supertypes: Vec::new(),
        relationships: Vec::new(),
        type_refs,
        is_public: import.is_public,
        view_data: None,
        metadata_annotations: Vec::new(), // Imports don't have metadata
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
    });
}

fn extract_from_normalized_alias(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    alias: &NormalizedAlias,
) {
    let name = match &alias.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    // Use name_range for precise position, fall back to full range
    let span = ctx.range_to_info(alias.name_range.or(alias.range));

    // Create type_ref for the alias target so hover works on it
    let type_refs = if let Some(r) = alias.target_range {
        let start = ctx.line_index.line_col(r.start());
        let end = ctx.line_index.line_col(r.end());
        vec![TypeRefKind::Simple(TypeRef {
            target: Arc::from(alias.target.as_str()),
            resolved_target: None,
            kind: RefKind::Other, // Alias targets are special
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col,
        })]
    } else {
        Vec::new()
    };

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: alias.short_name.as_ref().map(|s| Arc::from(s.as_str())),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Alias,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None, // Aliases don't have tracked short_name_span
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: None,
        supertypes: vec![Arc::from(alias.target.as_str())],
        relationships: Vec::new(),
        type_refs,
        is_public: false,
        view_data: None,
        metadata_annotations: Vec::new(), // Aliases don't have metadata
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
    });
}

fn extract_from_normalized_comment(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    comment: &NormalizedComment,
) {
    // Extract type_refs from about references
    let type_refs = extract_type_refs_from_normalized(&comment.about, &ctx.line_index);

    let (name, is_anonymous) = match &comment.name {
        Some(n) => (strip_quotes(n), false),
        None => {
            // Anonymous comment - if it has about refs, we need to track them
            // Create an internal symbol to hold the type_refs for hover/goto
            if type_refs.is_empty() {
                return; // Nothing to track
            }
            // Use a synthetic name based on the range
            let anon_name = if let Some(r) = comment.range {
                let pos = ctx.line_index.line_col(r.start());
                format!("<anonymous_comment_{}_{}>", pos.line, pos.col)
            } else {
                "<anonymous_comment>".to_string()
            };
            (anon_name, true)
        }
    };

    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(comment.range);

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: comment.short_name.as_ref().map(|s| Arc::from(s.as_str())),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Comment,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None, // Comments don't have tracked short_name_span
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: if is_anonymous {
            None
        } else {
            Some(Arc::from(comment.content.as_str()))
        },
        supertypes: Vec::new(),
        relationships: Vec::new(),
        type_refs,
        is_public: false,
        view_data: None,
        metadata_annotations: Vec::new(), // Comments don't have metadata
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
    });
}

/// Extract type references from normalized relationships.
///
/// Chains are now preserved explicitly from the normalized layer.
/// Each TypeRef now includes its RefKind so callers can distinguish
/// type references from feature references.
fn extract_type_refs_from_normalized(
    relationships: &[NormalizedRelationship],
    line_index: &crate::base::LineIndex,
) -> Vec<TypeRefKind> {
    use super::normalize::RelTarget;

    let mut type_refs = Vec::new();

    for rel in relationships.iter() {
        let ref_kind = RefKind::from_rel_kind(normalized_to_rel_kind(rel.kind));

        match &rel.target {
            RelTarget::Chain(chain) => {
                // Emit as a TypeRefChain with individual parts
                let num_parts = chain.parts.len();
                let parts: Vec<TypeRef> = chain
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(idx, part)| {
                        let (start_line, start_col, end_line, end_col) = if let Some(r) = part.range
                        {
                            let start = line_index.line_col(r.start());
                            let end = line_index.line_col(r.end());
                            (start.line, start.col, end.line, end.col)
                        } else if idx == num_parts - 1 {
                            // For the last part, fallback to relationship range if available
                            if let Some(r) = rel.range {
                                let start = line_index.line_col(r.start());
                                let end = line_index.line_col(r.end());
                                (start.line, start.col, end.line, end.col)
                            } else {
                                (0, 0, 0, 0)
                            }
                        } else {
                            // Non-last parts without ranges are synthetic (not hoverable)
                            (0, 0, 0, 0)
                        };
                        TypeRef {
                            target: Arc::from(part.name.as_str()),
                            resolved_target: None,
                            kind: ref_kind,
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        }
                    })
                    .collect();

                if !parts.is_empty() {
                    type_refs.push(TypeRefKind::Chain(TypeRefChain { parts }));
                }
            }
            RelTarget::Simple(target) => {
                if let Some(r) = rel.range {
                    let start = line_index.line_col(r.start());
                    let end = line_index.line_col(r.end());
                    type_refs.push(TypeRefKind::Simple(TypeRef {
                        target: Arc::from(target.as_str()),
                        resolved_target: None,
                        kind: ref_kind,
                        start_line: start.line,
                        start_col: start.col,
                        end_line: end.line,
                        end_col: end.col,
                    }));

                    // Also add prefix segments as references (e.g., Vehicle::speed -> Vehicle)
                    let parts: Vec<&str> = target.split("::").collect();
                    if parts.len() > 1 {
                        let mut prefix = String::new();
                        for (i, part) in parts.iter().enumerate() {
                            if i == parts.len() - 1 {
                                break;
                            }
                            if !prefix.is_empty() {
                                prefix.push_str("::");
                            }
                            prefix.push_str(part);

                            type_refs.push(TypeRefKind::Simple(TypeRef {
                                target: Arc::from(prefix.as_str()),
                                resolved_target: None,
                                kind: ref_kind,
                                start_line: start.line,
                                start_col: start.col,
                                end_line: end.line,
                                end_col: end.col,
                            }));
                        }
                    }
                }
            }
        }
    }

    type_refs
}

/// Extract symbols from a normalized dependency.
fn extract_from_normalized_dependency(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    dep: &NormalizedDependency,
) {
    // Collect type refs from sources, targets, and relationships (including prefix metadata)
    let mut type_refs = extract_type_refs_from_normalized(&dep.sources, &ctx.line_index);
    type_refs.extend(extract_type_refs_from_normalized(
        &dep.targets,
        &ctx.line_index,
    ));
    type_refs.extend(extract_type_refs_from_normalized(
        &dep.relationships,
        &ctx.line_index,
    ));

    // If dependency has a name, create a symbol for it
    if let Some(name) = &dep.name {
        let qualified_name = ctx.qualified_name(name);
        let span = ctx.range_to_info(dep.range);

        symbols.push(HirSymbol {
            name: Arc::from(name.as_str()),
            short_name: dep.short_name.as_ref().map(|s| Arc::from(s.as_str())),
            qualified_name: Arc::from(qualified_name.as_str()),
            element_id: new_element_id(),
            kind: SymbolKind::Dependency,
            file: ctx.file,
            start_line: span.start_line,
            start_col: span.start_col,
            end_line: span.end_line,
            end_col: span.end_col,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            type_refs,
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
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
        });
    } else if !type_refs.is_empty() {
        // Anonymous dependency - attach type refs to parent or create anonymous symbol
        // For now, create an anonymous symbol so refs are tracked
        let span = ctx.range_to_info(dep.range);

        symbols.push(HirSymbol {
            name: Arc::from("<anonymous-dependency>"),
            short_name: None,
            qualified_name: Arc::from(format!("{}::<anonymous-dependency>", ctx.prefix)),
            element_id: new_element_id(),
            kind: SymbolKind::Dependency,
            file: ctx.file,
            start_line: span.start_line,
            start_col: span.start_col,
            end_line: span.end_line,
            end_col: span.end_col,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            type_refs,
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
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
        });
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Span information extracted from an AST node.
#[derive(Clone, Copy, Debug, Default)]
struct SpanInfo {
    start_line: u32,
    start_col: u32,
    end_line: u32,
    end_col: u32,
}

/// Strip single quotes from a string.
fn strip_quotes(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_display() {
        assert_eq!(SymbolKind::PartDefinition.display(), "Part def");
        assert_eq!(SymbolKind::PartUsage.display(), "Part");
    }

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("'"), "'");
    }

    #[test]
    fn test_extraction_context() {
        let mut ctx = ExtractionContext {
            file: FileId::new(0),
            prefix: String::new(),
            anon_counter: 0,
            scope_stack: Vec::new(),
            line_index: crate::base::LineIndex::new(""),
        };

        assert_eq!(ctx.qualified_name("Foo"), "Foo");

        ctx.push_scope("Outer");
        assert_eq!(ctx.qualified_name("Inner"), "Outer::Inner");

        ctx.push_scope("Deep");
        assert_eq!(ctx.qualified_name("Leaf"), "Outer::Deep::Leaf");

        ctx.pop_scope();
        assert_eq!(ctx.qualified_name("Sibling"), "Outer::Sibling");

        ctx.pop_scope();
        assert_eq!(ctx.qualified_name("Root"), "Root");
    }

    #[test]
    fn test_direction_and_multiplicity_extraction() {
        use crate::base::FileId;
        use crate::syntax::parser::parse_content;

        let source = r#"part def Vehicle {
            in port fuelIn : FuelType[1];
            out port exhaust : GasType[0..*];
            inout port control : ControlType[1..5];
            part wheels[4];
        }"#;

        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();
        let symbols = super::extract_symbols_unified(FileId::new(0), &syntax);

        // Find the ports and verify direction
        let fuel_in = symbols
            .iter()
            .find(|s| s.name.as_ref() == "fuelIn")
            .unwrap();
        assert_eq!(fuel_in.direction, Some(Direction::In));
        assert_eq!(
            fuel_in.multiplicity,
            Some(Multiplicity {
                lower: Some(1),
                upper: Some(1)
            })
        );

        let exhaust = symbols
            .iter()
            .find(|s| s.name.as_ref() == "exhaust")
            .unwrap();
        assert_eq!(exhaust.direction, Some(Direction::Out));
        assert_eq!(
            exhaust.multiplicity,
            Some(Multiplicity {
                lower: Some(0),
                upper: None
            })
        );

        let control = symbols
            .iter()
            .find(|s| s.name.as_ref() == "control")
            .unwrap();
        assert_eq!(control.direction, Some(Direction::InOut));
        assert_eq!(
            control.multiplicity,
            Some(Multiplicity {
                lower: Some(1),
                upper: Some(5)
            })
        );

        let wheels = symbols
            .iter()
            .find(|s| s.name.as_ref() == "wheels")
            .unwrap();
        assert_eq!(wheels.direction, None);
        assert_eq!(
            wheels.multiplicity,
            Some(Multiplicity {
                lower: Some(4),
                upper: Some(4)
            })
        );
    }
}

#[cfg(test)]
mod test_package_span {
    use super::*;
    use crate::base::FileId;
    use crate::syntax::parser::parse_content;

    #[test]
    fn test_hir_simple_package_span() {
        let source = "package SimpleVehicleModel { }";
        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();

        let symbols = extract_symbols_unified(FileId(1), &syntax);

        let pkg_sym = symbols
            .iter()
            .find(|s| s.name.as_ref() == "SimpleVehicleModel")
            .unwrap();

        println!(
            "Package symbol: name='{}' start=({},{}), end=({},{})",
            pkg_sym.name, pkg_sym.start_line, pkg_sym.start_col, pkg_sym.end_line, pkg_sym.end_col
        );

        // The name "SimpleVehicleModel" starts at column 8 (after "package ")
        assert_eq!(pkg_sym.start_col, 8, "start_col should be 8");
        assert_eq!(pkg_sym.end_col, 26, "end_col should be 26");
    }

    #[test]
    fn test_hir_nested_package_span() {
        // Match the structure of VehicleIndividuals.sysml
        let source = r#"package VehicleIndividuals {
	package IndividualDefinitions {
	}
}"#;
        let syntax = parse_content(source, std::path::Path::new("test.sysml")).unwrap();

        let symbols = extract_symbols_unified(FileId(1), &syntax);

        for sym in &symbols {
            println!(
                "Symbol: name='{}' kind={:?} start=({},{}), end=({},{})",
                sym.name, sym.kind, sym.start_line, sym.start_col, sym.end_line, sym.end_col
            );
        }

        // Top-level package: "VehicleIndividuals" starts at column 8
        let outer = symbols
            .iter()
            .find(|s| s.name.as_ref() == "VehicleIndividuals")
            .unwrap();
        assert_eq!(outer.start_col, 8, "outer start_col should be 8");
        assert_eq!(
            outer.end_col, 26,
            "outer end_col should be 26 (8 + 18 = 26)"
        );

        // Nested package: "IndividualDefinitions" on line 2, with a tab prefix
        // "package IndividualDefinitions" - tab is 1 char, "package " is 8 chars = 9
        let nested = symbols
            .iter()
            .find(|s| s.name.as_ref() == "IndividualDefinitions")
            .unwrap();
        println!(
            "Nested package: start_col={}, end_col={}",
            nested.start_col, nested.end_col
        );
        // After tab (1) and "package " (8) = 9
        assert_eq!(nested.start_col, 9, "nested start_col should be 9");
        // "IndividualDefinitions" is 21 chars, so 9 + 21 = 30
        assert_eq!(nested.end_col, 30, "nested end_col should be 30");
    }
}

/// Extract view-specific data from a normalized definition if it's a view/viewpoint/rendering.
fn extract_view_data_from_definition(
    def: &NormalizedDefinition,
    kind: NormalizedDefKind,
) -> Option<crate::hir::views::ViewData> {
    use crate::hir::views::{
        ExposeRelationship, FilterCondition, ViewData, ViewDefinition, WildcardKind,
    };

    // Check if this is a view-related definition
    match kind {
        NormalizedDefKind::View => {
            let mut view_def = ViewDefinition::new();

            // Extract expose relationships and filters from children
            for child in &def.children {
                match child {
                    NormalizedElement::Expose(expose) => {
                        let wildcard = if expose.is_recursive {
                            WildcardKind::Recursive
                        } else if expose.import_path.ends_with("::*") {
                            WildcardKind::Direct
                        } else {
                            WildcardKind::None
                        };

                        let expose_rel = ExposeRelationship::new(
                            Arc::from(expose.import_path.as_str()),
                            wildcard,
                        );
                        view_def.add_expose(expose_rel);
                    }
                    NormalizedElement::Filter(filter) => {
                        // Extract metadata filters
                        for meta_ref in &filter.metadata_refs {
                            let filter_cond =
                                FilterCondition::metadata(Arc::from(meta_ref.as_str()));
                            view_def.add_filter(filter_cond);
                        }
                    }
                    _ => {} // Other children are normal nested elements
                }
            }

            Some(ViewData::ViewDefinition(view_def))
        }
        NormalizedDefKind::Viewpoint => {
            // TODO: Extract stakeholders and concerns from children
            // Viewpoints contain requirement usages that specify stakeholders
            Some(ViewData::ViewpointDefinition(
                crate::hir::views::ViewpointDefinition {
                    stakeholders: Vec::new(),
                    concerns: Vec::new(),
                    span: None, // TODO: Convert TextRange to Span
                },
            ))
        }
        NormalizedDefKind::Rendering => {
            // TODO: Extract layout algorithm from metadata or properties
            Some(ViewData::RenderingDefinition(
                crate::hir::views::RenderingDefinition {
                    layout: None,
                    span: None, // TODO: Convert TextRange to Span
                },
            ))
        }
        _ => None,
    }
}

/// Extract view-specific data from a normalized usage if it's a view/viewpoint/rendering.
fn extract_view_data_from_usage(
    _usage: &NormalizedUsage,
    _kind: NormalizedUsageKind,
    _typed_by: Option<&Arc<str>>,
) -> Option<crate::hir::views::ViewData> {
    use crate::hir::views::{ViewData, ViewUsage};

    // Check if this is a view-related usage
    match _kind {
        NormalizedUsageKind::View => {
            // TODO Phase 2: Extract expose/filter/render from usage body
            // View usages can have:
            // 1. Expose relationships to specify which elements to show
            // 2. Filter conditions to refine what's shown
            // 3. Rendering specifications to override the view definition
            //
            // Currently creates empty ViewUsage as placeholder
            Some(ViewData::ViewUsage(ViewUsage::new(_typed_by.cloned())))
        }
        NormalizedUsageKind::Viewpoint => Some(ViewData::ViewpointUsage(
            crate::hir::views::ViewpointUsage {
                viewpoint_def: _typed_by.cloned(),
                span: None, // TODO: Convert TextRange to Span
            },
        )),
        NormalizedUsageKind::Rendering => Some(ViewData::RenderingUsage(
            crate::hir::views::RenderingUsage {
                rendering_def: _typed_by.cloned(),
                span: None, // TODO: Convert TextRange to Span
            },
        )),
        _ => None,
    }
}
