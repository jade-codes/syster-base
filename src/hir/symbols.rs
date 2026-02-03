//! Symbol extraction from AST — pure functions that return symbols.
//!
//! This module provides functions to extract symbols from a parsed AST.
//! Unlike the old visitor-based approach, these are pure functions that
//! return data rather than mutating state.
//!
//! The extraction uses the normalized syntax layer (`crate::syntax::normalized`)
//! to provide a unified extraction path for both SysML and KerML files.

use std::sync::Arc;

use uuid::Uuid;

use crate::base::FileId;
use crate::syntax::normalized::{
    NormalizedAlias, NormalizedComment, NormalizedDefKind, NormalizedDefinition,
    NormalizedDependency, NormalizedElement, NormalizedImport, NormalizedPackage,
    NormalizedRelKind, NormalizedRelationship, NormalizedUsage, NormalizedUsageKind,
};

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

    /// Convert from NormalizedRelKind.
    pub fn from_normalized(kind: NormalizedRelKind) -> Self {
        match kind {
            NormalizedRelKind::TypedBy => RefKind::TypedBy,
            NormalizedRelKind::Specializes => RefKind::Specializes,
            NormalizedRelKind::Redefines => RefKind::Redefines,
            NormalizedRelKind::Subsets => RefKind::Subsets,
            NormalizedRelKind::References => RefKind::References,
            NormalizedRelKind::Expression => RefKind::Expression,
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
    /// Convert from NormalizedRelKind.
    pub fn from_normalized(kind: NormalizedRelKind) -> Option<Self> {
        match kind {
            NormalizedRelKind::Specializes => Some(RelationshipKind::Specializes),
            NormalizedRelKind::TypedBy => Some(RelationshipKind::TypedBy),
            NormalizedRelKind::Redefines => Some(RelationshipKind::Redefines),
            NormalizedRelKind::Subsets => Some(RelationshipKind::Subsets),
            NormalizedRelKind::References => Some(RelationshipKind::References),
            NormalizedRelKind::Satisfies => Some(RelationshipKind::Satisfies),
            NormalizedRelKind::Performs => Some(RelationshipKind::Performs),
            NormalizedRelKind::Exhibits => Some(RelationshipKind::Exhibits),
            NormalizedRelKind::Includes => Some(RelationshipKind::Includes),
            NormalizedRelKind::Asserts => Some(RelationshipKind::Asserts),
            NormalizedRelKind::Verifies => Some(RelationshipKind::Verifies),
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
}

/// The kind of a symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Package,
    // Definitions
    PartDef,
    ItemDef,
    ActionDef,
    PortDef,
    AttributeDef,
    ConnectionDef,
    InterfaceDef,
    AllocationDef,
    RequirementDef,
    ConstraintDef,
    StateDef,
    CalculationDef,
    UseCaseDef,
    AnalysisCaseDef,
    ConcernDef,
    ViewDef,
    ViewpointDef,
    RenderingDef,
    ViewUsage,
    ViewpointUsage,
    RenderingUsage,
    EnumerationDef,
    MetaclassDef,
    InteractionDef,
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
    CalculationUsage,
    ReferenceUsage,
    OccurrenceUsage,
    FlowUsage,
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
    /// Create from a NormalizedDefKind.
    pub fn from_definition_kind(kind: &NormalizedDefKind) -> Self {
        match kind {
            NormalizedDefKind::Part => Self::PartDef,
            NormalizedDefKind::Item => Self::ItemDef,
            NormalizedDefKind::Action => Self::ActionDef,
            NormalizedDefKind::Port => Self::PortDef,
            NormalizedDefKind::Attribute => Self::AttributeDef,
            NormalizedDefKind::Connection => Self::ConnectionDef,
            NormalizedDefKind::Interface => Self::InterfaceDef,
            NormalizedDefKind::Allocation => Self::AllocationDef,
            NormalizedDefKind::Requirement => Self::RequirementDef,
            NormalizedDefKind::Constraint => Self::ConstraintDef,
            NormalizedDefKind::State => Self::StateDef,
            NormalizedDefKind::Calculation => Self::CalculationDef,
            NormalizedDefKind::UseCase => Self::UseCaseDef,
            NormalizedDefKind::AnalysisCase => Self::AnalysisCaseDef,
            NormalizedDefKind::Concern => Self::ConcernDef,
            NormalizedDefKind::View => Self::ViewDef,
            NormalizedDefKind::Viewpoint => Self::ViewpointDef,
            NormalizedDefKind::Rendering => Self::RenderingDef,
            NormalizedDefKind::Enumeration => Self::EnumerationDef,
            _ => Self::Other,
        }
    }

    /// Create from a NormalizedUsageKind.
    pub fn from_usage_kind(kind: &NormalizedUsageKind) -> Self {
        match kind {
            NormalizedUsageKind::Part => Self::PartUsage,
            NormalizedUsageKind::Item => Self::ItemUsage,
            NormalizedUsageKind::Action => Self::ActionUsage,
            NormalizedUsageKind::Port => Self::PortUsage,
            NormalizedUsageKind::Attribute => Self::AttributeUsage,
            NormalizedUsageKind::Connection => Self::ConnectionUsage,
            NormalizedUsageKind::Interface => Self::InterfaceUsage,
            NormalizedUsageKind::Allocation => Self::AllocationUsage,
            NormalizedUsageKind::Requirement => Self::RequirementUsage,
            NormalizedUsageKind::Constraint => Self::ConstraintUsage,
            NormalizedUsageKind::State => Self::StateUsage,
            NormalizedUsageKind::Calculation => Self::CalculationUsage,
            NormalizedUsageKind::Reference => Self::ReferenceUsage,
            NormalizedUsageKind::Occurrence => Self::OccurrenceUsage,
            NormalizedUsageKind::Flow => Self::FlowUsage,
            NormalizedUsageKind::Transition => Self::Other, // Transitions map to Other
            NormalizedUsageKind::Accept => Self::ActionUsage, // Accept payloads are action usages
            NormalizedUsageKind::End => Self::PortUsage,    // Connection endpoints are like ports
            NormalizedUsageKind::Fork => Self::ActionUsage, // Fork nodes are action usages
            NormalizedUsageKind::Join => Self::ActionUsage, // Join nodes are action usages
            NormalizedUsageKind::Merge => Self::ActionUsage, // Merge nodes are action usages
            NormalizedUsageKind::Decide => Self::ActionUsage, // Decide nodes are action usages
            NormalizedUsageKind::Feature => Self::PartUsage, // KerML features map to part usage
            NormalizedUsageKind::View => Self::ViewUsage,
            NormalizedUsageKind::Viewpoint => Self::ViewpointUsage,
            NormalizedUsageKind::Rendering => Self::RenderingUsage,
            NormalizedUsageKind::Other => Self::Other,
        }
    }

    /// Get a display string for this kind (capitalized for UI display).
    pub fn display(&self) -> &'static str {
        match self {
            Self::Package => "Package",
            Self::PartDef => "Part def",
            Self::ItemDef => "Item def",
            Self::ActionDef => "Action def",
            Self::PortDef => "Port def",
            Self::AttributeDef => "Attribute def",
            Self::ConnectionDef => "Connection def",
            Self::InterfaceDef => "Interface def",
            Self::AllocationDef => "Allocation def",
            Self::RequirementDef => "Requirement def",
            Self::ConstraintDef => "Constraint def",
            Self::StateDef => "State def",
            Self::CalculationDef => "Calc def",
            Self::UseCaseDef => "Use case def",
            Self::AnalysisCaseDef => "Analysis case def",
            Self::ConcernDef => "Concern def",
            Self::ViewDef => "View def",
            Self::ViewpointDef => "Viewpoint def",
            Self::RenderingDef => "Rendering def",
            Self::ViewUsage => "View",
            Self::ViewpointUsage => "Viewpoint",
            Self::RenderingUsage => "Rendering",
            Self::EnumerationDef => "Enum def",
            Self::MetaclassDef => "Metaclass def",
            Self::InteractionDef => "Interaction def",
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
            Self::CalculationUsage => "Calc",
            Self::ReferenceUsage => "Ref",
            Self::OccurrenceUsage => "Occurrence",
            Self::FlowUsage => "Flow",
            Self::ExposeRelationship => "Expose",
            Self::Import => "Import",
            Self::Alias => "Alias",
            Self::Comment => "Comment",
            Self::Dependency => "Dependency",
            Self::Other => "Element",
        }
    }

    /// Convert a normalized definition kind to a SymbolKind.
    pub fn from_normalized_def_kind(kind: NormalizedDefKind) -> Self {
        match kind {
            NormalizedDefKind::Part => Self::PartDef,
            NormalizedDefKind::Item => Self::ItemDef,
            NormalizedDefKind::Action => Self::ActionDef,
            NormalizedDefKind::Port => Self::PortDef,
            NormalizedDefKind::Attribute => Self::AttributeDef,
            NormalizedDefKind::Connection => Self::ConnectionDef,
            NormalizedDefKind::Interface => Self::InterfaceDef,
            NormalizedDefKind::Allocation => Self::AllocationDef,
            NormalizedDefKind::Requirement => Self::RequirementDef,
            NormalizedDefKind::Constraint => Self::ConstraintDef,
            NormalizedDefKind::State => Self::StateDef,
            NormalizedDefKind::Calculation => Self::CalculationDef,
            NormalizedDefKind::UseCase => Self::UseCaseDef,
            NormalizedDefKind::AnalysisCase => Self::AnalysisCaseDef,
            NormalizedDefKind::Concern => Self::ConcernDef,
            NormalizedDefKind::View => Self::ViewDef,
            NormalizedDefKind::Viewpoint => Self::ViewpointDef,
            NormalizedDefKind::Rendering => Self::RenderingDef,
            NormalizedDefKind::Enumeration => Self::EnumerationDef,
            // KerML classifier types map to closest SysML equivalents
            NormalizedDefKind::DataType => Self::AttributeDef,
            NormalizedDefKind::Class | NormalizedDefKind::Structure => Self::PartDef,
            NormalizedDefKind::Behavior => Self::ActionDef,
            NormalizedDefKind::Function => Self::CalculationDef,
            NormalizedDefKind::Association => Self::ConnectionDef,
            NormalizedDefKind::Metaclass => Self::MetaclassDef,
            NormalizedDefKind::Interaction => Self::InteractionDef,
            NormalizedDefKind::Other => Self::Other,
        }
    }

    /// Convert a normalized usage kind to a SymbolKind.
    pub fn from_normalized_usage_kind(kind: NormalizedUsageKind) -> Self {
        match kind {
            NormalizedUsageKind::Part => Self::PartUsage,
            NormalizedUsageKind::Item => Self::ItemUsage,
            NormalizedUsageKind::Action => Self::ActionUsage,
            NormalizedUsageKind::Port => Self::PortUsage,
            NormalizedUsageKind::Attribute => Self::AttributeUsage,
            NormalizedUsageKind::Connection => Self::ConnectionUsage,
            NormalizedUsageKind::Interface => Self::InterfaceUsage,
            NormalizedUsageKind::Allocation => Self::AllocationUsage,
            NormalizedUsageKind::Requirement => Self::RequirementUsage,
            NormalizedUsageKind::Constraint => Self::ConstraintUsage,
            NormalizedUsageKind::State => Self::StateUsage,
            NormalizedUsageKind::Calculation => Self::CalculationUsage,
            NormalizedUsageKind::Reference => Self::ReferenceUsage,
            NormalizedUsageKind::Occurrence => Self::OccurrenceUsage,
            NormalizedUsageKind::Flow => Self::FlowUsage,
            NormalizedUsageKind::Transition => Self::Other, // Transitions map to Other
            NormalizedUsageKind::Accept => Self::ActionUsage, // Accept payloads are action usages
            NormalizedUsageKind::End => Self::PortUsage,    // Connection endpoints are like ports
            NormalizedUsageKind::Fork => Self::ActionUsage, // Fork nodes are action usages
            NormalizedUsageKind::Join => Self::ActionUsage, // Join nodes are action usages
            NormalizedUsageKind::Merge => Self::ActionUsage, // Merge nodes are action usages
            NormalizedUsageKind::Decide => Self::ActionUsage, // Decide nodes are action usages
            NormalizedUsageKind::View => Self::ViewUsage,
            NormalizedUsageKind::Viewpoint => Self::ViewpointUsage,
            NormalizedUsageKind::Rendering => Self::RenderingUsage,
            // KerML features are treated as attribute usages
            NormalizedUsageKind::Feature => Self::AttributeUsage,
            NormalizedUsageKind::Other => Self::Other,
        }
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
        NormalizedDefKind::Connection => Some("Connections::Connection"),
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
/// - `message x` implicitly specializes `Flows::Message`
/// - `flow x` implicitly specializes `Flows::Flow`
/// - etc.
fn implicit_supertype_for_usage_kind(kind: NormalizedUsageKind) -> Option<&'static str> {
    match kind {
        NormalizedUsageKind::Flow => Some("Flows::Message"),
        NormalizedUsageKind::Connection => Some("Connections::Connection"),
        NormalizedUsageKind::Interface => Some("Interfaces::Interface"),
        NormalizedUsageKind::Allocation => Some("Allocations::Allocation"),
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
            RelationshipKind::from_normalized(rel.kind).map(|kind| {
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
            if !type_refs.is_empty() {
                if let Some(parent) = symbols
                    .iter_mut()
                    .rev()
                    .find(|s| s.qualified_name.as_ref() == ctx.prefix)
                {
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
    let mut supertypes: Vec<Arc<str>> = usage
        .relationships
        .iter()
        .filter(|r| {
            matches!(
                r.kind,
                NormalizedRelKind::TypedBy
                    | NormalizedRelKind::Subsets
                    | NormalizedRelKind::Specializes
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

    // Add implicit supertypes from SysML kernel library if not already specialized
    // This models the implicit inheritance: message → Message, flow → Flow, etc.
    if let Some(implicit) = implicit_supertype_for_usage_kind(usage.kind) {
        // Only add if no explicit specialization of this type
        if !supertypes
            .iter()
            .any(|s| s.contains("Message") || s.contains("Flow"))
        {
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
    use crate::syntax::normalized::RelTarget;

    let mut type_refs = Vec::new();

    for rel in relationships.iter() {
        let ref_kind = RefKind::from_normalized(rel.kind);

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
        assert_eq!(SymbolKind::PartDef.display(), "Part def");
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
