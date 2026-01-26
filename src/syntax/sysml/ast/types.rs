use super::enums::{DefinitionKind, DefinitionMember, Element, UsageKind, UsageMember};
use super::parsers::ExtractedRef;
use crate::syntax::Span;

// ============================================================================
// FEATURE CHAIN TYPES (for proper chain resolution)
// ============================================================================

/// A part of a feature chain with its own span.
/// E.g., in `providePower.distributeTorque`, each identifier is a part.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureChainPart {
    pub name: String,
    pub span: Option<Span>,
}

/// A feature chain is a sequence of identifiers separated by dots.
/// E.g., `providePower.distributeTorque` or `camera.takePicture.focus`
///
/// Unlike the old approach of detecting chains from adjacent spans,
/// this struct explicitly captures the chain structure from the parser.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureChain {
    /// The parts of the chain, in order
    pub parts: Vec<FeatureChainPart>,
    /// The span of the entire chain
    pub span: Option<Span>,
}

impl FeatureChain {
    /// Create a new feature chain from parts
    pub fn new(parts: Vec<FeatureChainPart>, span: Option<Span>) -> Self {
        Self { parts, span }
    }

    /// Check if this is actually a chain (more than one part)
    pub fn is_chain(&self) -> bool {
        self.parts.len() > 1
    }

    /// Get the first part of the chain
    pub fn first(&self) -> Option<&FeatureChainPart> {
        self.parts.first()
    }

    /// Get the last part of the chain  
    pub fn last(&self) -> Option<&FeatureChainPart> {
        self.parts.last()
    }

    /// Get all parts as a dot-separated string (for backwards compat)
    pub fn as_dotted_string(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

// ============================================================================
// LEGACY CHAIN CONTEXT (deprecated, use FeatureChain instead)
// ============================================================================

/// Chain context for feature chain references (e.g., takePicture.focus)
/// This tracks which part of a chain this reference is, enabling proper resolution.
///
/// DEPRECATED: Use FeatureChain instead. This is kept for backwards compatibility
/// during migration.
pub type ChainContext = Option<(Vec<String>, usize)>;

// ============================================================================
// RELATIONSHIP TYPES
// ============================================================================

// Relationship types now store ExtractedRef directly to preserve chain spans
#[derive(Debug, Clone, PartialEq)]
pub struct SpecializationRel {
    pub extracted: ExtractedRef,
}

impl SpecializationRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    /// Legacy accessor for target name
    pub fn target(&self) -> String {
        self.extracted.name()
    }

    /// Legacy accessor for span
    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedefinitionRel {
    pub extracted: ExtractedRef,
}

impl RedefinitionRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubsettingRel {
    pub extracted: ExtractedRef,
}

impl SubsettingRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceRel {
    pub extracted: ExtractedRef,
}

impl ReferenceRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrossRel {
    pub extracted: ExtractedRef,
}

impl CrossRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SatisfyRel {
    pub extracted: ExtractedRef,
}

impl SatisfyRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerformRel {
    pub extracted: ExtractedRef,
}

impl PerformRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExhibitRel {
    pub extracted: ExtractedRef,
}

impl ExhibitRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeRel {
    pub extracted: ExtractedRef,
}

impl IncludeRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssertRel {
    pub extracted: ExtractedRef,
}

impl AssertRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyRel {
    pub extracted: ExtractedRef,
}

impl VerifyRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaRel {
    pub extracted: ExtractedRef,
}

impl MetaRel {
    pub fn new(extracted: ExtractedRef) -> Self {
        Self { extracted }
    }

    pub fn target(&self) -> String {
        self.extracted.name()
    }

    pub fn span(&self) -> Option<Span> {
        self.extracted.span()
    }
}

/// Represents an element filter member (e.g., `filter @Safety;`)
/// Used in packages and views to filter elements by metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    /// References in the filter expression (e.g., `@Safety`, `SysML::PartUsage`)
    pub meta_refs: Vec<MetaRel>,
    /// Feature references in the filter expression (e.g., `Safety::isMandatory`)
    pub expression_refs: Vec<crate::syntax::sysml::ast::parsers::ExtractedRef>,
    /// Span of the filter statement
    pub span: Option<Span>,
}

/// Represents a parsed SysML file with support for multiple package declarations
#[derive(Debug, Clone, PartialEq)]
pub struct SysMLFile {
    /// Primary namespace (first package) - maintained for backward compatibility
    pub namespace: Option<NamespaceDeclaration>,
    /// All namespace declarations in the file (Issue #10)
    pub namespaces: Vec<NamespaceDeclaration>,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDeclaration {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub name: Option<String>,
    /// Short name (e.g., "m" from `<m> 'metre'`)
    pub short_name: Option<String>,
    pub elements: Vec<Element>,
    /// Span of the package name identifier
    pub span: Option<Span>,
}

/// Represents relationship information that can be attached to definitions and usages
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Relationships {
    /// Specializations (:> or "specializes")
    pub specializes: Vec<SpecializationRel>,
    /// Redefinitions (:>> or "redefines")
    pub redefines: Vec<RedefinitionRel>,
    /// Subsetting (:> or "subsets")
    pub subsets: Vec<SubsettingRel>,
    /// Feature typing (: or "typed by")
    pub typed_by: Option<String>,
    /// Span of the type reference (if typed_by is set)
    pub typed_by_span: Option<crate::syntax::Span>,
    /// References (::> or "references")
    pub references: Vec<ReferenceRel>,
    /// Crosses (=> or "crosses")  
    pub crosses: Vec<CrossRel>,

    // Domain-specific SysML relationships
    /// Satisfy (satisfy) - satisfaction of requirements
    pub satisfies: Vec<SatisfyRel>,
    /// Perform (perform) - performance relationships
    pub performs: Vec<PerformRel>,
    /// Exhibit (exhibit) - exhibition of states
    pub exhibits: Vec<ExhibitRel>,
    /// Include (include) - use case inclusion
    pub includes: Vec<IncludeRel>,
    /// Assert (assert) - constraint assertion
    pub asserts: Vec<AssertRel>,
    /// Verify (verify) - requirement verification
    pub verifies: Vec<VerifyRel>,
    /// Meta type references (meta Qualified::Name)
    pub meta: Vec<MetaRel>,
}

impl Relationships {
    /// Create an empty relationships struct (for tests)
    pub fn none() -> Self {
        Self::default()
    }

    /// Get the span for a relationship to a specific target.
    /// Searches all relationship types for a matching target name.
    pub fn get_span_for_target(&self, target: &str) -> Option<Span> {
        // Check typing first (most common for usages)
        if self.typed_by.as_deref() == Some(target) {
            return self.typed_by_span;
        }

        // Check all one-to-many relationship vectors
        for rel in &self.specializes {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.redefines {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.subsets {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.references {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.crosses {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.satisfies {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.performs {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.exhibits {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.includes {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.asserts {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.verifies {
            if rel.target() == target {
                return rel.span();
            }
        }
        for rel in &self.meta {
            if rel.target() == target {
                return rel.span();
            }
        }

        None
    }

    /// Get all relationship targets with their spans.
    /// Returns tuples of (relationship_kind, target_name, span).
    pub fn all_targets_with_spans(&self) -> Vec<(&'static str, String, Option<Span>)> {
        let mut result = Vec::new();

        if let Some(ref target) = self.typed_by {
            result.push(("typing", target.clone(), self.typed_by_span));
        }

        for rel in &self.specializes {
            result.push(("specialization", rel.target(), rel.span()));
        }
        for rel in &self.redefines {
            result.push(("redefinition", rel.target(), rel.span()));
        }
        for rel in &self.subsets {
            result.push(("subsetting", rel.target(), rel.span()));
        }
        for rel in &self.references {
            result.push(("reference", rel.target(), rel.span()));
        }
        for rel in &self.crosses {
            result.push(("cross", rel.target(), rel.span()));
        }
        for rel in &self.satisfies {
            result.push(("satisfy", rel.target(), rel.span()));
        }
        for rel in &self.performs {
            result.push(("perform", rel.target(), rel.span()));
        }
        for rel in &self.exhibits {
            result.push(("exhibit", rel.target(), rel.span()));
        }
        for rel in &self.includes {
            result.push(("include", rel.target(), rel.span()));
        }
        for rel in &self.asserts {
            result.push(("assert", rel.target(), rel.span()));
        }
        for rel in &self.verifies {
            result.push(("verify", rel.target(), rel.span()));
        }
        for rel in &self.meta {
            result.push(("meta", rel.target(), rel.span()));
        }

        result
    }

    /// Get all relationship targets with their spans and chain context.
    /// Returns tuples of (relationship_kind, target_name, span, chain_context).
    ///
    /// Chain context is `Some((parts, index))` when the target is part of a feature chain
    /// like `takePicture.focus`. For example, `focus` would have chain_context
    /// `Some((["takePicture", "focus"], 1))`.
    pub fn all_targets_with_chain_context(
        &self,
    ) -> Vec<(&'static str, String, Option<Span>, ChainContext)> {
        let mut result = Vec::new();

        if let Some(ref target) = self.typed_by {
            // typed_by doesn't have chain_context stored separately, so None
            result.push(("typing", target.clone(), self.typed_by_span, None));
        }

        for rel in &self.specializes {
            result.push((
                "specialization",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.redefines {
            result.push((
                "redefinition",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.subsets {
            result.push((
                "subsetting",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.references {
            result.push((
                "reference",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.crosses {
            result.push((
                "cross",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.satisfies {
            result.push((
                "satisfy",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.performs {
            result.push((
                "perform",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.exhibits {
            result.push((
                "exhibit",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.includes {
            result.push((
                "include",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.asserts {
            result.push((
                "assert",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.verifies {
            result.push((
                "verify",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }
        for rel in &self.meta {
            result.push((
                "meta",
                rel.target(),
                rel.span(),
                rel.extracted.chain_context(),
            ));
        }

        result
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Definition {
    pub kind: DefinitionKind,
    pub name: Option<String>,
    /// Short name (e.g., "kg" from `<kg> kilogram`)
    pub short_name: Option<String>,
    /// Span of the short name identifier (inside the `<` and `>`)
    pub short_name_span: Option<Span>,
    pub relationships: Relationships,
    pub body: Vec<DefinitionMember>,
    /// Span of the definition name identifier
    pub span: Option<Span>,
    // Property modifiers
    #[doc(hidden)]
    pub is_abstract: bool,
    #[doc(hidden)]
    pub is_variation: bool,
}

impl Definition {
    pub fn new(
        kind: DefinitionKind,
        name: Option<String>,
        relationships: Relationships,
        body: Vec<DefinitionMember>,
    ) -> Self {
        Self {
            kind,
            name,
            short_name: None,
            short_name_span: None,
            relationships,
            body,
            span: None,
            is_abstract: false,
            is_variation: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Usage {
    pub kind: UsageKind,
    pub name: Option<String>,
    /// Short name (e.g., "kg" from `<kg> kilogram`)
    pub short_name: Option<String>,
    /// Span of the short name identifier (inside the `<` and `>`)
    pub short_name_span: Option<Span>,
    pub relationships: Relationships,
    pub body: Vec<UsageMember>,
    /// Span of the usage name identifier
    pub span: Option<Span>,
    /// References found in value expressions (e.g., `= 2*elapseTime.num`)
    /// These are identifiers and feature chains used in expressions.
    pub expression_refs: Vec<super::parsers::ExtractedRef>,
    // Property modifiers
    #[doc(hidden)]
    pub is_derived: bool,
    #[doc(hidden)]
    pub is_const: bool,
}

impl Usage {
    /// Create a new Usage with default property flags
    pub fn new(
        kind: UsageKind,
        name: Option<String>,
        relationships: Relationships,
        body: Vec<UsageMember>,
    ) -> Self {
        Self {
            kind,
            name,
            short_name: None,
            short_name_span: None,
            relationships,
            body,
            span: None,
            expression_refs: Vec::new(),
            is_derived: false,
            is_const: false,
        }
    }

    /// For domain-specific usages (satisfy, perform, exhibit, include), get the primary target.
    /// Returns the target and its span from whichever syntactic position it appears in.
    /// Priority: typed_by > first subset > name
    pub fn domain_target(&self) -> Option<(&str, Option<Span>)> {
        match self.kind {
            UsageKind::SatisfyRequirement
            | UsageKind::PerformAction
            | UsageKind::ExhibitState { is_parallel: _ }
            | UsageKind::IncludeUseCase => {
                if let Some(ref typed_by) = self.relationships.typed_by {
                    Some((typed_by.as_str(), self.relationships.typed_by_span))
                } else if let Some(first_subset) = self.relationships.subsets.first() {
                    let target = first_subset.target();
                    // Leak the string to get static lifetime
                    let target_str: &'static str = Box::leak(target.into_boxed_str());
                    Some((target_str, first_subset.span()))
                } else if let Some(ref name) = self.name {
                    Some((name.as_str(), self.span))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Represents a reference to an element in `about` clause
#[derive(Debug, Clone, PartialEq)]
pub struct AboutReference {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Comment {
    /// Optional name for named comments
    pub name: Option<String>,
    /// Optional name span for named comments
    pub name_span: Option<Span>,
    /// The content of the comment (the entire raw string)
    pub content: String,
    /// References in the `about` clause
    pub about: Vec<AboutReference>,
    pub span: Option<Span>,
}

impl Comment {
    /// Create a new Comment with only content and span
    pub fn new(content: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            name: None,
            name_span: None,
            content: content.into(),
            about: Vec::new(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,
    pub path_span: Option<Span>,
    pub is_recursive: bool,
    pub is_public: bool,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Documentation {
    pub comment: Comment,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alias {
    pub name: Option<String>,
    pub target: String,
    pub target_span: Option<Span>,
    pub span: Option<Span>,
}

/// Represents a dependency relationship: `#refinement dependency X to Y::Z`
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    pub name: Option<String>,
    pub name_span: Option<Span>,
    /// The source elements (before "to")
    pub sources: Vec<DependencyRef>,
    /// The target elements (after "to")
    pub targets: Vec<DependencyRef>,
    pub span: Option<Span>,
}

/// A reference in a dependency (source or target)
#[derive(Debug, Clone, PartialEq)]
pub struct DependencyRef {
    pub path: String,
    pub span: Option<Span>,
}
