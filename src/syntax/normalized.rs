//! Normalized syntax types for unified symbol extraction.
//!
//! This module provides a language-agnostic view of SysML and KerML syntax,
//! allowing the HIR layer to work with a single set of types instead of
//! duplicating logic for each language variant.
//!
//! The normalized types capture the essential structure needed for symbol
//! extraction while abstracting away language-specific details.

use crate::syntax::Span;

use crate::syntax::sysml::ast::parsers::ExtractedRef;
use crate::syntax::sysml::ast::types::FeatureChain;

/// A normalized relationship target - either a simple name or a feature chain.
#[derive(Debug, Clone)]
pub enum RelTarget<'a> {
    /// A simple reference like `Vehicle`
    Simple(&'a str),
    /// A feature chain like `engine.power.value`
    Chain(FeatureChain),
}

impl<'a> RelTarget<'a> {
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

    /// Create a RelTarget from an ExtractedRef (owned version for chains)
    pub fn from_extracted(extracted: &ExtractedRef) -> RelTarget<'static> {
        match extracted {
            ExtractedRef::Simple { name, .. } => {
                // We need to leak the string to get 'static lifetime
                // This is acceptable for symbols that live for the duration of the index
                RelTarget::Simple(Box::leak(name.clone().into_boxed_str()))
            }
            ExtractedRef::Chain(chain) => {
                // Chain parts are used directly in the clone below
                RelTarget::Chain(chain.clone())
            }
        }
    }
}

/// A normalized element that can appear in either SysML or KerML files.
#[derive(Debug, Clone)]
pub enum NormalizedElement<'a> {
    Package(NormalizedPackage<'a>),
    Definition(NormalizedDefinition<'a>),
    Usage(NormalizedUsage<'a>),
    Import(NormalizedImport<'a>),
    Alias(NormalizedAlias<'a>),
    Comment(NormalizedComment<'a>),
    Dependency(NormalizedDependency<'a>),
}

/// A normalized package with its children.
#[derive(Debug, Clone)]
pub struct NormalizedPackage<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub span: Option<Span>,
    pub children: Vec<NormalizedElement<'a>>,
}

/// A normalized definition (SysML definition or KerML classifier).
#[derive(Debug, Clone)]
pub struct NormalizedDefinition<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub kind: NormalizedDefKind,
    pub span: Option<Span>,
    /// Span of the short name (for hover support on short names)
    pub short_name_span: Option<Span>,
    pub doc: Option<&'a str>,
    pub relationships: Vec<NormalizedRelationship<'a>>,
    pub children: Vec<NormalizedElement<'a>>,
}

/// A normalized usage (SysML usage or KerML feature).
#[derive(Debug, Clone)]
pub struct NormalizedUsage<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub kind: NormalizedUsageKind,
    pub span: Option<Span>,
    /// Span of the short name (for hover support on short names)
    pub short_name_span: Option<Span>,
    pub doc: Option<&'a str>,
    pub relationships: Vec<NormalizedRelationship<'a>>,
    pub children: Vec<NormalizedElement<'a>>,
}

/// A normalized import statement.
#[derive(Debug, Clone)]
pub struct NormalizedImport<'a> {
    pub path: &'a str,
    pub span: Option<Span>,
    pub is_public: bool,
}

/// A normalized alias.
#[derive(Debug, Clone)]
pub struct NormalizedAlias<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub target: &'a str,
    pub target_span: Option<Span>,
    pub span: Option<Span>,
}

/// A normalized comment.
#[derive(Debug, Clone)]
pub struct NormalizedComment<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub content: &'a str,
    /// References in the `about` clause
    pub about: Vec<NormalizedRelationship<'a>>,
    pub span: Option<Span>,
}

/// A normalized dependency (relationships like refinement, derivation, etc.).
#[derive(Debug, Clone)]
pub struct NormalizedDependency<'a> {
    pub name: Option<&'a str>,
    pub short_name: Option<&'a str>,
    pub sources: Vec<NormalizedRelationship<'a>>,
    pub targets: Vec<NormalizedRelationship<'a>>,
    pub span: Option<Span>,
}

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

/// A normalized relationship (specialization, typing, subsetting, etc.).
#[derive(Debug, Clone)]
pub struct NormalizedRelationship<'a> {
    pub kind: NormalizedRelKind,
    pub target: RelTarget<'a>,
    pub span: Option<Span>,
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
// Adapters from SysML AST
// ============================================================================

impl<'a> NormalizedElement<'a> {
    /// Create a normalized element from a SysML element.
    pub fn from_sysml(element: &'a crate::syntax::sysml::ast::enums::Element) -> Self {
        use crate::syntax::sysml::ast::enums::Element as SysMLElement;
        match element {
            SysMLElement::Package(pkg) => {
                NormalizedElement::Package(NormalizedPackage::from_sysml(pkg))
            }
            SysMLElement::Definition(def) => {
                NormalizedElement::Definition(NormalizedDefinition::from_sysml(def))
            }
            SysMLElement::Usage(usage) => {
                NormalizedElement::Usage(NormalizedUsage::from_sysml(usage))
            }
            SysMLElement::Import(import) => {
                NormalizedElement::Import(NormalizedImport::from_sysml(import))
            }
            SysMLElement::Alias(alias) => {
                NormalizedElement::Alias(NormalizedAlias::from_sysml(alias))
            }
            SysMLElement::Comment(comment) => {
                NormalizedElement::Comment(NormalizedComment::from_sysml(comment))
            }
            SysMLElement::Dependency(dep) => {
                NormalizedElement::Dependency(NormalizedDependency::from_sysml(dep))
            }
            SysMLElement::Filter(_) => {
                // Skip filters - they don't produce symbols
                NormalizedElement::Comment(NormalizedComment {
                    name: None,
                    short_name: None,
                    content: "",
                    about: Vec::new(),
                    span: None,
                })
            }
        }
    }

    /// Create a normalized element from a KerML element.
    pub fn from_kerml(element: &'a crate::syntax::kerml::ast::enums::Element) -> Self {
        use crate::syntax::kerml::ast::enums::Element as KerMLElement;
        match element {
            KerMLElement::Package(pkg) => {
                NormalizedElement::Package(NormalizedPackage::from_kerml(pkg))
            }
            KerMLElement::Classifier(classifier) => {
                NormalizedElement::Definition(NormalizedDefinition::from_kerml(classifier))
            }
            KerMLElement::Feature(feature) => {
                NormalizedElement::Usage(NormalizedUsage::from_kerml(feature))
            }
            KerMLElement::Import(import) => {
                NormalizedElement::Import(NormalizedImport::from_kerml(import))
            }
            KerMLElement::Comment(_) | KerMLElement::Annotation(_) => {
                // Skip these - they don't produce symbols currently
                NormalizedElement::Comment(NormalizedComment {
                    name: None,
                    short_name: None,
                    content: "",
                    about: Vec::new(),
                    span: None,
                })
            }
        }
    }
}

impl<'a> NormalizedPackage<'a> {
    fn from_sysml(pkg: &'a crate::syntax::sysml::ast::types::Package) -> Self {
        Self {
            name: pkg.name.as_deref(),
            short_name: pkg.short_name.as_deref(),
            span: pkg.span,
            children: pkg
                .elements
                .iter()
                .map(NormalizedElement::from_sysml)
                .collect(),
        }
    }

    fn from_kerml(pkg: &'a crate::syntax::kerml::ast::types::Package) -> Self {
        Self {
            name: pkg.name.as_deref(),
            short_name: pkg.short_name.as_deref(),
            span: pkg.span,
            children: pkg
                .elements
                .iter()
                .map(NormalizedElement::from_kerml)
                .collect(),
        }
    }
}

impl<'a> NormalizedDefinition<'a> {
    fn from_sysml(def: &'a crate::syntax::sysml::ast::types::Definition) -> Self {
        use crate::syntax::sysml::ast::enums::DefinitionKind;
        use crate::syntax::sysml::ast::enums::DefinitionMember;

        let kind = match def.kind {
            DefinitionKind::Part => NormalizedDefKind::Part,
            DefinitionKind::Item => NormalizedDefKind::Item,
            DefinitionKind::Action => NormalizedDefKind::Action,
            DefinitionKind::Port => NormalizedDefKind::Port,
            DefinitionKind::Attribute => NormalizedDefKind::Attribute,
            DefinitionKind::Connection => NormalizedDefKind::Connection,
            DefinitionKind::Interface => NormalizedDefKind::Interface,
            DefinitionKind::Allocation => NormalizedDefKind::Allocation,
            DefinitionKind::Requirement => NormalizedDefKind::Requirement,
            DefinitionKind::Constraint => NormalizedDefKind::Constraint,
            DefinitionKind::State => NormalizedDefKind::State,
            DefinitionKind::Calculation => NormalizedDefKind::Calculation,
            DefinitionKind::UseCase | DefinitionKind::Case => NormalizedDefKind::UseCase,
            DefinitionKind::AnalysisCase | DefinitionKind::VerificationCase => {
                NormalizedDefKind::AnalysisCase
            }
            DefinitionKind::Concern => NormalizedDefKind::Concern,
            DefinitionKind::View => NormalizedDefKind::View,
            DefinitionKind::Viewpoint => NormalizedDefKind::Viewpoint,
            DefinitionKind::Rendering => NormalizedDefKind::Rendering,
            DefinitionKind::Enumeration => NormalizedDefKind::Enumeration,
            _ => NormalizedDefKind::Other,
        };

        // Extract relationships
        let mut relationships = Vec::new();
        for spec in &def.relationships.specializes {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Specializes,
                target: RelTarget::from_extracted(&spec.extracted),
                span: spec.span(),
            });
        }
        for redef in &def.relationships.redefines {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Redefines,
                target: RelTarget::from_extracted(&redef.extracted),
                span: redef.span(),
            });
        }
        for subset in &def.relationships.subsets {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Subsets,
                target: RelTarget::from_extracted(&subset.extracted),
                span: subset.span(),
            });
        }
        if let Some(ref typed) = def.relationships.typed_by {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::TypedBy,
                target: RelTarget::Simple(Box::leak(typed.clone().into_boxed_str())),
                span: def.relationships.typed_by_span,
            });
        }
        for refs in &def.relationships.references {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::References,
                target: RelTarget::from_extracted(&refs.extracted),
                span: refs.span(),
            });
        }

        // Convert body members to normalized elements
        let mut children = Vec::new();
        let mut doc = None;

        for member in &def.body {
            match member {
                DefinitionMember::Usage(usage) => {
                    children.push(NormalizedElement::Usage(NormalizedUsage::from_sysml(usage)));
                }
                DefinitionMember::Import(import) => {
                    children.push(NormalizedElement::Import(NormalizedImport::from_sysml(
                        import,
                    )));
                }
                DefinitionMember::Comment(comment) => {
                    // Check for doc comment
                    let content = comment.content.trim();
                    if content.starts_with("doc") {
                        if let Some(start) = content.find("/*") {
                            if let Some(end) = content.rfind("*/") {
                                doc = Some(&content[start + 2..end]);
                            }
                        }
                    }
                    children.push(NormalizedElement::Comment(NormalizedComment::from_sysml(
                        comment,
                    )));
                }
            }
        }

        Self {
            name: def.name.as_deref(),
            short_name: def.short_name.as_deref(),
            kind,
            span: def.span,
            short_name_span: def.short_name_span,
            doc,
            relationships,
            children,
        }
    }

    fn from_kerml(classifier: &'a crate::syntax::kerml::ast::types::Classifier) -> Self {
        use crate::syntax::kerml::ast::enums::ClassifierKind;
        use crate::syntax::kerml::ast::enums::ClassifierMember;

        let kind = match classifier.kind {
            ClassifierKind::DataType => NormalizedDefKind::DataType,
            ClassifierKind::Class => NormalizedDefKind::Class,
            ClassifierKind::Structure => NormalizedDefKind::Structure,
            ClassifierKind::Behavior => NormalizedDefKind::Behavior,
            ClassifierKind::Function => NormalizedDefKind::Function,
            ClassifierKind::Association => NormalizedDefKind::Association,
            ClassifierKind::AssociationStructure => NormalizedDefKind::Association,
            ClassifierKind::Metaclass => NormalizedDefKind::Metaclass,
            ClassifierKind::Interaction => NormalizedDefKind::Interaction,
            // Type and Classifier are treated as Class (closest equivalent)
            ClassifierKind::Type | ClassifierKind::Classifier => NormalizedDefKind::Class,
        };

        // Extract relationships from body
        let mut relationships = Vec::new();
        let mut children = Vec::new();

        for member in &classifier.body {
            match member {
                ClassifierMember::Specialization(spec) => {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Specializes,
                        target: RelTarget::Simple(&spec.general),
                        span: spec.span,
                    });
                }
                ClassifierMember::Feature(feature) => {
                    children.push(NormalizedElement::Usage(NormalizedUsage::from_kerml(
                        feature,
                    )));
                }
                ClassifierMember::Import(import) => {
                    children.push(NormalizedElement::Import(NormalizedImport::from_kerml(
                        import,
                    )));
                }
                ClassifierMember::Comment(_) => {} // Skip comments for now
            }
        }

        Self {
            name: classifier.name.as_deref(),
            short_name: None, // KerML classifiers don't have short names in AST yet
            kind,
            span: classifier.span,
            short_name_span: None, // KerML classifiers don't have short names
            doc: None,
            relationships,
            children,
        }
    }
}

impl<'a> NormalizedUsage<'a> {
    fn from_sysml(usage: &'a crate::syntax::sysml::ast::types::Usage) -> Self {
        use crate::syntax::sysml::ast::enums::UsageKind;
        use crate::syntax::sysml::ast::enums::UsageMember;

        let kind = match usage.kind {
            UsageKind::Part => NormalizedUsageKind::Part,
            UsageKind::Item => NormalizedUsageKind::Item,
            UsageKind::Action
            | UsageKind::PerformAction
            | UsageKind::SendAction
            | UsageKind::AcceptAction => NormalizedUsageKind::Action,
            UsageKind::Port => NormalizedUsageKind::Port,
            UsageKind::Attribute => NormalizedUsageKind::Attribute,
            UsageKind::Connection => NormalizedUsageKind::Connection,
            UsageKind::Interface => NormalizedUsageKind::Interface,
            UsageKind::Allocation => NormalizedUsageKind::Allocation,
            UsageKind::Requirement | UsageKind::SatisfyRequirement => {
                NormalizedUsageKind::Requirement
            }
            UsageKind::Constraint => NormalizedUsageKind::Constraint,
            UsageKind::State | UsageKind::ExhibitState | UsageKind::Transition => {
                NormalizedUsageKind::State
            }
            UsageKind::Calculation => NormalizedUsageKind::Calculation,
            UsageKind::Reference => NormalizedUsageKind::Reference,
            UsageKind::Occurrence
            | UsageKind::Individual
            | UsageKind::Snapshot
            | UsageKind::Timeslice => NormalizedUsageKind::Occurrence,
            UsageKind::Flow | UsageKind::Message => NormalizedUsageKind::Flow,
            _ => NormalizedUsageKind::Other,
        };

        // Extract relationships
        let mut relationships = Vec::new();
        for spec in &usage.relationships.specializes {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Specializes,
                target: RelTarget::from_extracted(&spec.extracted),
                span: spec.span(),
            });
        }
        for redef in &usage.relationships.redefines {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Redefines,
                target: RelTarget::from_extracted(&redef.extracted),
                span: redef.span(),
            });
        }
        for subset in &usage.relationships.subsets {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Subsets,
                target: RelTarget::from_extracted(&subset.extracted),
                span: subset.span(),
            });
        }
        if let Some(ref typed) = usage.relationships.typed_by {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::TypedBy,
                target: RelTarget::Simple(Box::leak(typed.clone().into_boxed_str())),
                span: usage.relationships.typed_by_span,
            });
        }
        for refs in &usage.relationships.references {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::References,
                target: RelTarget::from_extracted(&refs.extracted),
                span: refs.span(),
            });
        }
        // Domain-specific relationships
        for perf in &usage.relationships.performs {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Performs,
                target: RelTarget::from_extracted(&perf.extracted),
                span: perf.span(),
            });
        }
        for sat in &usage.relationships.satisfies {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Satisfies,
                target: RelTarget::from_extracted(&sat.extracted),
                span: sat.span(),
            });
        }
        for exh in &usage.relationships.exhibits {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Exhibits,
                target: RelTarget::from_extracted(&exh.extracted),
                span: exh.span(),
            });
        }
        for inc in &usage.relationships.includes {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Includes,
                target: RelTarget::from_extracted(&inc.extracted),
                span: inc.span(),
            });
        }
        for cross in &usage.relationships.crosses {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Crosses,
                target: RelTarget::from_extracted(&cross.extracted),
                span: cross.span(),
            });
        }

        // Meta type references (e.g., from "new Type()" instantiation expressions)
        for meta_ref in &usage.relationships.meta {
            relationships.push(NormalizedRelationship {
                kind: NormalizedRelKind::Meta,
                target: RelTarget::from_extracted(&meta_ref.extracted),
                span: meta_ref.span(),
            });
        }

        // Expression references - now we can handle chains properly!
        for expr_ref in &usage.expression_refs {
            if expr_ref.is_chain() {
                if let ExtractedRef::Chain(chain) = expr_ref {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Expression,
                        target: RelTarget::Chain(chain.clone()),
                        span: chain.span,
                    });
                }
            } else {
                // For simple refs, we need to handle the owned string
                // by storing it in the Chain variant with a single part
                let chain = FeatureChain {
                    parts: vec![super::sysml::ast::types::FeatureChainPart {
                        name: expr_ref.name(),
                        span: expr_ref.span(),
                    }],
                    span: expr_ref.span(),
                };
                relationships.push(NormalizedRelationship {
                    kind: NormalizedRelKind::Expression,
                    target: RelTarget::Chain(chain),
                    span: expr_ref.span(),
                });
            }
        }

        // Convert body to children
        let mut children = Vec::new();
        let mut doc = None;

        for member in &usage.body {
            match member {
                UsageMember::Usage(nested) => {
                    children.push(NormalizedElement::Usage(NormalizedUsage::from_sysml(
                        nested,
                    )));
                }
                UsageMember::Comment(comment) => {
                    // Check for doc comment
                    let content = comment.content.trim();
                    if content.starts_with("doc") {
                        if let Some(start) = content.find("/*") {
                            if let Some(end) = content.rfind("*/") {
                                doc = Some(&content[start + 2..end]);
                            }
                        }
                    }
                    children.push(NormalizedElement::Comment(
                        NormalizedComment::from_sysml_comment(comment),
                    ));
                }
            }
        }

        Self {
            name: usage.name.as_deref(),
            short_name: usage.short_name.as_deref(),
            kind,
            span: usage.span,
            short_name_span: usage.short_name_span,
            doc,
            relationships,
            children,
        }
    }

    fn from_kerml(feature: &'a crate::syntax::kerml::ast::types::Feature) -> Self {
        use crate::syntax::kerml::ast::enums::FeatureMember;

        // Extract relationships from body
        let mut relationships = Vec::new();
        for member in &feature.body {
            match member {
                FeatureMember::Typing(typing) => {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::TypedBy,
                        target: RelTarget::Simple(&typing.typed),
                        span: typing.span,
                    });
                }
                FeatureMember::Subsetting(subset) => {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Subsets,
                        target: RelTarget::Simple(&subset.subset),
                        span: subset.span,
                    });
                }
                FeatureMember::Redefinition(redef) => {
                    relationships.push(NormalizedRelationship {
                        kind: NormalizedRelKind::Redefines,
                        target: RelTarget::Simple(&redef.redefined),
                        span: redef.span,
                    });
                }
                FeatureMember::Comment(_) => {}
            }
        }

        Self {
            name: feature.name.as_deref(),
            short_name: None, // KerML features don't have short names in AST yet
            kind: NormalizedUsageKind::Feature,
            span: feature.span,
            short_name_span: None, // KerML features don't have short names
            doc: None,
            relationships,
            children: Vec::new(),
        }
    }
}

impl<'a> NormalizedImport<'a> {
    fn from_sysml(import: &'a crate::syntax::sysml::ast::types::Import) -> Self {
        Self {
            path: &import.path,
            span: import.span,
            is_public: import.is_public,
        }
    }

    fn from_kerml(import: &'a crate::syntax::kerml::ast::types::Import) -> Self {
        Self {
            path: &import.path,
            span: import.span,
            is_public: import.is_public,
        }
    }
}

impl<'a> NormalizedAlias<'a> {
    fn from_sysml(alias: &'a crate::syntax::sysml::ast::types::Alias) -> Self {
        Self {
            name: alias.name.as_deref(),
            short_name: None, // TODO: Add short_name to Alias AST type
            target: &alias.target,
            target_span: alias.target_span,
            span: alias.span,
        }
    }
}

impl<'a> NormalizedComment<'a> {
    fn from_sysml(comment: &'a crate::syntax::sysml::ast::types::Comment) -> Self {
        let about = comment
            .about
            .iter()
            .map(|a| NormalizedRelationship {
                kind: NormalizedRelKind::About,
                target: RelTarget::Simple(&a.name),
                span: a.span,
            })
            .collect();

        Self {
            name: comment.name.as_deref(),
            short_name: None, // TODO: Add short_name to Comment AST type
            content: &comment.content,
            about,
            span: comment.span,
        }
    }

    fn from_sysml_comment(comment: &'a crate::syntax::sysml::ast::types::Comment) -> Self {
        Self::from_sysml(comment)
    }
}

impl<'a> NormalizedDependency<'a> {
    fn from_sysml(dep: &'a crate::syntax::sysml::ast::types::Dependency) -> Self {
        let mut sources = Vec::new();
        let mut targets = Vec::new();

        // Convert source refs to relationships
        for src in &dep.sources {
            sources.push(NormalizedRelationship {
                kind: NormalizedRelKind::Expression,
                target: RelTarget::Simple(&src.path),
                span: src.span,
            });
        }

        // Convert target refs to relationships
        for tgt in &dep.targets {
            targets.push(NormalizedRelationship {
                kind: NormalizedRelKind::Expression,
                target: RelTarget::Simple(&tgt.path),
                span: tgt.span,
            });
        }

        Self {
            name: dep.name.as_deref(),
            short_name: None, // TODO: Add short_name to Dependency AST type
            sources,
            targets,
            span: dep.span,
        }
    }
}

// ============================================================================
// Iteration helpers for normalized files
// ============================================================================

/// An iterator over normalized elements from a SysML file.
pub struct SysMLNormalizedIter<'a> {
    elements: std::slice::Iter<'a, crate::syntax::sysml::ast::enums::Element>,
}

impl<'a> SysMLNormalizedIter<'a> {
    pub fn new(file: &'a crate::syntax::sysml::ast::types::SysMLFile) -> Self {
        Self {
            elements: file.elements.iter(),
        }
    }
}

impl<'a> Iterator for SysMLNormalizedIter<'a> {
    type Item = NormalizedElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.elements.next().map(NormalizedElement::from_sysml)
    }
}

/// An iterator over normalized elements from a KerML file.
pub struct KerMLNormalizedIter<'a> {
    elements: std::slice::Iter<'a, crate::syntax::kerml::ast::enums::Element>,
}

impl<'a> KerMLNormalizedIter<'a> {
    pub fn new(file: &'a crate::syntax::kerml::ast::types::KerMLFile) -> Self {
        Self {
            elements: file.elements.iter(),
        }
    }
}

impl<'a> Iterator for KerMLNormalizedIter<'a> {
    type Item = NormalizedElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.elements.next().map(NormalizedElement::from_kerml)
    }
}
