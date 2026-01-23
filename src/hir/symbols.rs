//! Symbol extraction from AST — pure functions that return symbols.
//!
//! This module provides functions to extract symbols from a parsed AST.
//! Unlike the old visitor-based approach, these are pure functions that
//! return data rather than mutating state.
//!
//! The extraction uses the normalized syntax layer (`crate::syntax::normalized`)
//! to provide a unified extraction path for both SysML and KerML files.

use std::sync::Arc;

use crate::base::FileId;
use crate::syntax::sysml::ast::enums::{DefinitionKind, UsageKind};
use crate::syntax::normalized::{
    NormalizedElement, NormalizedPackage, NormalizedDefinition, NormalizedUsage,
    NormalizedImport, NormalizedAlias, NormalizedComment, NormalizedDependency,
    NormalizedDefKind, NormalizedUsageKind, NormalizedRelationship, NormalizedRelKind,
};

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
        matches!(self, RefKind::Redefines | RefKind::Subsets | RefKind::References)
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
    pub fn new(target: impl Into<Arc<str>>, kind: RefKind, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
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
        let after_start = line > self.start_line 
            || (line == self.start_line && col >= self.start_col);
        let before_end = line < self.end_line 
            || (line == self.end_line && col <= self.end_col);
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
            TypeRefKind::Chain(c) => {
                c.parts.iter().enumerate().find(|(_, r)| r.contains(line, col))
            }
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
        self.parts.iter().map(|p| p.target.as_ref()).collect::<Vec<_>>().join(".")
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
    /// Types this symbol specializes/subsets
    pub supertypes: Vec<Arc<str>>,
    /// Type references with their source locations (for goto-definition on type annotations)
    pub type_refs: Vec<TypeRefKind>,
    /// Whether this symbol is public (for imports: re-exported to child scopes)
    pub is_public: bool,
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
    EnumerationDef,
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
    // Other
    Import,
    Alias,
    Comment,
    Dependency,
    // Generic fallback
    Other,
}

impl SymbolKind {
    /// Create from a DefinitionKind.
    pub fn from_definition_kind(kind: &DefinitionKind) -> Self {
        match kind {
            DefinitionKind::Part => Self::PartDef,
            DefinitionKind::Item => Self::ItemDef,
            DefinitionKind::Action => Self::ActionDef,
            DefinitionKind::Port => Self::PortDef,
            DefinitionKind::Attribute => Self::AttributeDef,
            DefinitionKind::Connection => Self::ConnectionDef,
            DefinitionKind::Interface => Self::InterfaceDef,
            DefinitionKind::Allocation => Self::AllocationDef,
            DefinitionKind::Requirement => Self::RequirementDef,
            DefinitionKind::Constraint => Self::ConstraintDef,
            DefinitionKind::State => Self::StateDef,
            DefinitionKind::Calculation => Self::CalculationDef,
            DefinitionKind::UseCase | DefinitionKind::Case => Self::UseCaseDef,
            DefinitionKind::AnalysisCase | DefinitionKind::VerificationCase => Self::AnalysisCaseDef,
            DefinitionKind::Concern => Self::ConcernDef,
            DefinitionKind::View => Self::ViewDef,
            DefinitionKind::Viewpoint => Self::ViewpointDef,
            DefinitionKind::Rendering => Self::RenderingDef,
            DefinitionKind::Enumeration => Self::EnumerationDef,
            _ => Self::Other,
        }
    }

    /// Create from a UsageKind.
    pub fn from_usage_kind(kind: &UsageKind) -> Self {
        match kind {
            UsageKind::Part => Self::PartUsage,
            UsageKind::Item => Self::ItemUsage,
            UsageKind::Action | UsageKind::PerformAction | UsageKind::SendAction | UsageKind::AcceptAction => Self::ActionUsage,
            UsageKind::Port => Self::PortUsage,
            UsageKind::Attribute => Self::AttributeUsage,
            UsageKind::Connection => Self::ConnectionUsage,
            UsageKind::Interface => Self::InterfaceUsage,
            UsageKind::Allocation => Self::AllocationUsage,
            UsageKind::Requirement | UsageKind::SatisfyRequirement => Self::RequirementUsage,
            UsageKind::Constraint => Self::ConstraintUsage,
            UsageKind::State | UsageKind::ExhibitState | UsageKind::Transition => Self::StateUsage,
            UsageKind::Calculation => Self::CalculationUsage,
            UsageKind::Reference => Self::ReferenceUsage,
            UsageKind::Occurrence | UsageKind::Individual | UsageKind::Snapshot | UsageKind::Timeslice => Self::OccurrenceUsage,
            UsageKind::Flow | UsageKind::Message => Self::FlowUsage,
            _ => Self::Other,
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
            Self::EnumerationDef => "Enum def",
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
            NormalizedDefKind::Metaclass => Self::Other,
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
}

impl ExtractionContext {
    fn qualified_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.prefix, name)
        }
    }

    fn push_scope(&mut self, name: &str) {
        if self.prefix.is_empty() {
            self.prefix = name.to_string();
        } else {
            self.prefix = format!("{}::{}", self.prefix, name);
        }
    }

    fn pop_scope(&mut self) {
        if let Some(pos) = self.prefix.rfind("::") {
            self.prefix.truncate(pos);
        } else {
            self.prefix.clear();
        }
    }
    
    /// Generate a unique anonymous scope name
    fn next_anon_scope(&mut self, rel_prefix: &str, target: &str, line: u32) -> String {
        self.anon_counter += 1;
        format!("<{}{}#{}@L{}>", rel_prefix, target, self.anon_counter, line)
    }
}

// ============================================================================
// UNIFIED EXTRACTION (using normalized types)
// ============================================================================

/// Extract all symbols from any syntax file using the normalized adapter layer.
///
/// This is the preferred extraction function as it handles both SysML and KerML
/// through a unified code path.
pub fn extract_symbols_unified(file: FileId, syntax: &crate::syntax::SyntaxFile) -> Vec<HirSymbol> {
    use crate::syntax::SyntaxFile;
    
    let mut symbols = Vec::new();
    let mut context = ExtractionContext {
        file,
        prefix: String::new(),
        anon_counter: 0,
    };

    match syntax {
        SyntaxFile::SysML(sysml) => {
            // Set namespace prefix if present
            if let Some(ns) = &sysml.namespace {
                context.prefix = ns.name.clone();
            }
            for element in &sysml.elements {
                let normalized = NormalizedElement::from_sysml(element);
                extract_from_normalized(&mut symbols, &mut context, &normalized);
            }
        }
        SyntaxFile::KerML(kerml) => {
            // Set namespace prefix if present
            if let Some(ns) = &kerml.namespace {
                context.prefix = ns.name.clone();
            }
            for element in &kerml.elements {
                let normalized = NormalizedElement::from_kerml(element);
                extract_from_normalized(&mut symbols, &mut context, &normalized);
            }
        }
    }

    symbols
}

/// Extract from a normalized element.
fn extract_from_normalized(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    element: &NormalizedElement,
) {
    match element {
        NormalizedElement::Package(pkg) => extract_from_normalized_package(symbols, ctx, pkg),
        NormalizedElement::Definition(def) => extract_from_normalized_definition(symbols, ctx, def),
        NormalizedElement::Usage(usage) => extract_from_normalized_usage(symbols, ctx, usage),
        NormalizedElement::Import(import) => extract_from_normalized_import(symbols, ctx, import),
        NormalizedElement::Alias(alias) => extract_from_normalized_alias(symbols, ctx, alias),
        NormalizedElement::Comment(comment) => extract_from_normalized_comment(symbols, ctx, comment),
        NormalizedElement::Dependency(dep) => extract_from_normalized_dependency(symbols, ctx, dep),
    }
}

fn extract_from_normalized_package(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    pkg: &NormalizedPackage,
) {
    let name = match pkg.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    let span = span_to_info(pkg.span);

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: pkg.short_name.map(|s| Arc::from(s)),
        qualified_name: Arc::from(qualified_name.as_str()),
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
        doc: None,
        supertypes: Vec::new(),
        type_refs: Vec::new(),
        is_public: false,
    });

    ctx.push_scope(&name);
    for child in &pkg.children {
        extract_from_normalized(symbols, ctx, child);
    }
    ctx.pop_scope();
}

/// Get the implicit supertype for a definition kind based on SysML kernel library.
/// In SysML, all definitions implicitly specialize their kernel metaclass:
/// - `part def X` implicitly specializes `Parts::Part`
/// - `item def X` implicitly specializes `Items::Item`
/// - `action def X` implicitly specializes `Actions::Action`
/// etc.
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
/// etc.
fn implicit_supertype_for_usage_kind(kind: NormalizedUsageKind) -> Option<&'static str> {
    match kind {
        NormalizedUsageKind::Flow => Some("Flows::Message"),
        NormalizedUsageKind::Connection => Some("Connections::Connection"),
        NormalizedUsageKind::Interface => Some("Interfaces::Interface"),
        NormalizedUsageKind::Allocation => Some("Allocations::Allocation"),
        _ => None,
    }
}

fn extract_from_normalized_definition(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    def: &NormalizedDefinition,
) {
    let name = match def.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    let kind = SymbolKind::from_normalized_def_kind(def.kind);
    let span = span_to_info(def.span);
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) = span_to_optional(def.short_name_span);

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
    let type_refs = extract_type_refs_from_normalized(&def.relationships);

    // Extract doc comment
    let doc = def.doc.map(|s| Arc::from(s.trim()));

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: def.short_name.map(|s| Arc::from(s)),
        qualified_name: Arc::from(qualified_name.as_str()),
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
        type_refs,
        is_public: false,
    });

    // Recurse into children
    ctx.push_scope(&name);
    for child in &def.children {
        extract_from_normalized(symbols, ctx, child);
    }
    ctx.pop_scope();
}

fn extract_from_normalized_usage(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    usage: &NormalizedUsage,
) {
    // Extract type references even for anonymous usages
    let type_refs = extract_type_refs_from_normalized(&usage.relationships);
    
    // For anonymous usages, attach refs to the parent but still recurse into children
    let name = match usage.name {
        Some(n) => strip_quotes(n),
        None => {
            if !type_refs.is_empty() {
                if let Some(parent) = symbols.last_mut() {
                    parent.type_refs.extend(type_refs);
                }
            }
            
            // Generate unique anonymous scope name for children
            // Try to use relationship target for meaningful names, otherwise use generic anon
            let line = usage.span.map(|s| s.start.line as u32).unwrap_or(0);
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
                    };
                    ctx.next_anon_scope(prefix, &r.target.as_str(), line)
                })
                // Fallback: always create a unique scope for anonymous usages with children
                .unwrap_or_else(|| ctx.next_anon_scope("anon", "", line));
            
            // Always push scope for children of anonymous usages
            ctx.push_scope(&anon_scope);
            
            // Recurse into children for anonymous usages
            for child in &usage.children {
                extract_from_normalized(symbols, ctx, child);
            }
            
            ctx.pop_scope();
            return;
        }
    };

    let qualified_name = ctx.qualified_name(&name);
    let kind = SymbolKind::from_normalized_usage_kind(usage.kind);
    let span = span_to_info(usage.span);
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) = span_to_optional(usage.short_name_span);

    // Extract typing and subsetting as supertypes
    let mut supertypes: Vec<Arc<str>> = usage
        .relationships
        .iter()
        .filter(|r| matches!(r.kind, NormalizedRelKind::TypedBy | NormalizedRelKind::Subsets))
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();
    
    // Add implicit supertypes from SysML kernel library if not already specialized
    // This models the implicit inheritance: message → Message, flow → Flow, etc.
    if let Some(implicit) = implicit_supertype_for_usage_kind(usage.kind) {
        // Only add if no explicit specialization of this type
        if !supertypes.iter().any(|s| s.contains("Message") || s.contains("Flow")) {
            supertypes.push(Arc::from(implicit));
        }
    }

    // Extract doc comment
    let doc = usage.doc.map(|s| Arc::from(s.trim()));

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: usage.short_name.map(|s| Arc::from(s)),
        qualified_name: Arc::from(qualified_name.as_str()),
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
        type_refs,
        is_public: false,
    });

    // Recurse into children
    ctx.push_scope(&name);
    for child in &usage.children {
        extract_from_normalized(symbols, ctx, child);
    }
    ctx.pop_scope();
}

fn extract_from_normalized_import(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    import: &NormalizedImport,
) {
    let path = import.path;
    let qualified_name = ctx.qualified_name(&format!("import:{}", path));
    let span = span_to_info(import.span);

    symbols.push(HirSymbol {
        name: Arc::from(path),
        short_name: None, // Imports don't have short names
        qualified_name: Arc::from(qualified_name.as_str()),
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
        type_refs: Vec::new(),
        is_public: import.is_public,
    });
}

fn extract_from_normalized_alias(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    alias: &NormalizedAlias,
) {
    let name = match alias.name {
        Some(n) => strip_quotes(n),
        None => return,
    };

    let qualified_name = ctx.qualified_name(&name);
    let span = span_to_info(alias.span);
    
    // Create type_ref for the alias target so hover works on it
    let type_refs = if let Some(s) = alias.target_span {
        vec![TypeRefKind::Simple(TypeRef {
            target: Arc::from(alias.target),
            resolved_target: None,
            kind: RefKind::Other, // Alias targets are special
            start_line: s.start.line as u32,
            start_col: s.start.column as u32,
            end_line: s.end.line as u32,
            end_col: s.end.column as u32,
        })]
    } else {
        Vec::new()
    };

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: alias.short_name.map(|s| Arc::from(s)),
        qualified_name: Arc::from(qualified_name.as_str()),
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
        supertypes: vec![Arc::from(alias.target)],
        type_refs,
        is_public: false,
    });
}

fn extract_from_normalized_comment(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    comment: &NormalizedComment,
) {
    // Extract type_refs from about references
    let type_refs = extract_type_refs_from_normalized(&comment.about);
    
    let (name, is_anonymous) = match comment.name {
        Some(n) => (strip_quotes(n), false),
        None => {
            // Anonymous comment - if it has about refs, we need to track them
            // Create an internal symbol to hold the type_refs for hover/goto
            if type_refs.is_empty() {
                return; // Nothing to track
            }
            // Use a synthetic name based on the span
            let anon_name = if let Some(span) = comment.span {
                format!("<anonymous_comment_{}_{}>", span.start.line, span.start.column)
            } else {
                "<anonymous_comment>".to_string()
            };
            (anon_name, true)
        }
    };

    let qualified_name = ctx.qualified_name(&name);
    let span = span_to_info(comment.span);

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: comment.short_name.map(|s| Arc::from(s)),
        qualified_name: Arc::from(qualified_name.as_str()),
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
        doc: if is_anonymous { None } else { Some(Arc::from(comment.content)) },
        supertypes: Vec::new(),
        type_refs,
        is_public: false,
    });
}

/// Extract type references from normalized relationships.
/// 
/// Chains are now preserved explicitly from the normalized layer.
/// Each TypeRef now includes its RefKind so callers can distinguish
/// type references from feature references.
fn extract_type_refs_from_normalized(relationships: &[NormalizedRelationship]) -> Vec<TypeRefKind> {
    use crate::syntax::normalized::RelTarget;
    
    let mut type_refs = Vec::new();
    
    for (_rel_idx, rel) in relationships.iter().enumerate() {
        let ref_kind = RefKind::from_normalized(rel.kind);
        
        match &rel.target {
            RelTarget::Chain(chain) => {
                // Emit as a TypeRefChain with individual parts
                let parts: Vec<TypeRef> = chain.parts.iter().enumerate().map(|(_i, part)| {
                    let (start_line, start_col, end_line, end_col) = if let Some(s) = &part.span {
                        (s.start.line as u32, s.start.column as u32, s.end.line as u32, s.end.column as u32)
                    } else if let Some(s) = rel.span {
                        // Fallback to relationship span if part span is missing
                        (s.start.line as u32, s.start.column as u32, s.end.line as u32, s.end.column as u32)
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
                }).collect();
                
                if !parts.is_empty() {
                    type_refs.push(TypeRefKind::Chain(TypeRefChain { parts }));
                }
            }
            RelTarget::Simple(target) => {
                if let Some(s) = rel.span {
                    type_refs.push(TypeRefKind::Simple(TypeRef {
                        target: Arc::from(*target),
                        resolved_target: None,
                        kind: ref_kind,
                        start_line: s.start.line as u32,
                        start_col: s.start.column as u32,
                        end_line: s.end.line as u32,
                        end_col: s.end.column as u32,
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
                                start_line: s.start.line as u32,
                                start_col: s.start.column as u32,
                                end_line: s.end.line as u32,
                                end_col: s.end.column as u32,
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
    // Collect type refs from both sources and targets
    let mut type_refs = extract_type_refs_from_normalized(&dep.sources);
    type_refs.extend(extract_type_refs_from_normalized(&dep.targets));
    
    // If dependency has a name, create a symbol for it
    if let Some(name) = dep.name {
        let qualified_name = ctx.qualified_name(name);
        let span = span_to_info(dep.span);
        
        symbols.push(HirSymbol {
            name: Arc::from(name),
            short_name: dep.short_name.map(|s| Arc::from(s)),
            qualified_name: Arc::from(qualified_name.as_str()),
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
            type_refs,
            is_public: false,
        });
    } else if !type_refs.is_empty() {
        // Anonymous dependency - attach type refs to parent or create anonymous symbol
        // For now, create an anonymous symbol so refs are tracked
        let span = span_to_info(dep.span);
        
        symbols.push(HirSymbol {
            name: Arc::from("<anonymous-dependency>"),
            short_name: None,
            qualified_name: Arc::from(format!("{}::<anonymous-dependency>", ctx.prefix)),
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
            type_refs,
            is_public: false,
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

/// Convert an optional Span to SpanInfo.
fn span_to_info(span: Option<crate::core::Span>) -> SpanInfo {
    match span {
        Some(s) => SpanInfo {
            start_line: s.start.line as u32,
            start_col: s.start.column as u32,
            end_line: s.end.line as u32,
            end_col: s.end.column as u32,
        },
        None => SpanInfo::default(),
    }
}

/// Convert an optional span to the 4 Option<u32> fields for short_name_span.
fn span_to_optional(span: Option<crate::core::Span>) -> (Option<u32>, Option<u32>, Option<u32>, Option<u32>) {
    match span {
        Some(s) => (
            Some(s.start.line as u32),
            Some(s.start.column as u32),
            Some(s.end.line as u32),
            Some(s.end.column as u32),
        ),
        None => (None, None, None, None),
    }
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
