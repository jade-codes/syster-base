//! Public and internal type definitions for symbol extraction.
//!
//! Contains all HIR symbol types (`HirSymbol`, `SymbolKind`, `TypeRef`, etc.)
//! and internal extraction types (`RelKind`, `ExtractedRel`, `InternalUsageKind`).

use std::sync::Arc;

use uuid::Uuid;

use crate::base::FileId;
use crate::parser::{DefinitionKind, Direction, Multiplicity, ValueExpression};
use rowan::TextRange;

// ============================================================================
// RELATIONSHIP HELPER TYPES
// ============================================================================

/// A feature chain like `engine.power.value`
#[derive(Debug, Clone)]
pub(crate) struct FeatureChain {
    pub parts: Vec<FeatureChainPart>,
    #[allow(dead_code)]
    pub range: Option<TextRange>,
}

/// A single part of a feature chain
#[derive(Debug, Clone)]
pub(crate) struct FeatureChainPart {
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

/// A relationship target — either a simple name or a feature chain.
#[derive(Debug, Clone)]
pub(crate) enum RelTarget {
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
}

/// Kinds of relationships (internal representation for extraction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
#[derive(Debug, Clone)]
pub(crate) struct ExtractedRel {
    pub kind: RelKind,
    pub target: RelTarget,
    pub range: Option<TextRange>,
}

/// Internal usage kind for the 27 SysML/KerML usage categories.
/// Used during extraction to determine SymbolKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
            Some(DefinitionKind::Case) | Some(DefinitionKind::UseCase) => Self::UseCaseDefinition,
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
}

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

/// Span information extracted from an AST node.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SpanInfo {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}
