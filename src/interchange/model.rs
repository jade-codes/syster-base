//! Standalone model representation for interchange.
//!
//! This module provides a `Model` type that represents a SysML/KerML model
//! independently of the Salsa database. This enables:
//!
//! - Loading models from XMI/KPAR without text parsing
//! - Exporting models to various formats
//! - Transferring models between tools
//!
//! ## Design
//!
//! The `Model` stores elements by ID, with relationships as separate edges.
//! This matches the OMG metamodel structure and enables efficient serialization.
//!
//! ```text
//! Model
//! ├── elements: IndexMap<ElementId, Element>  (preserves insertion order)
//! ├── relationships: Vec<Relationship>
//! └── metadata: ModelMetadata
//! ```

use indexmap::IndexMap;
use std::sync::Arc;

// ============================================================================
// IDs
// ============================================================================

/// Unique identifier for a model element.
///
/// This corresponds to `xmi:id` in XMI and `@id` in JSON-LD.
/// UUIDs are preferred for global uniqueness.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ElementId(pub Arc<str>);

impl ElementId {
    /// Create a new element ID.
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }

    /// Generate a new UUID-based ID.
    pub fn generate() -> Self {
        // Simple UUID v4 generation (would use uuid crate in real impl)
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self(format!("{:032x}", nanos).into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ElementId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ElementId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// ELEMENT KINDS
// ============================================================================

/// The metatype of a model element.
///
/// Maps to SysML v2 / KerML metaclasses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElementKind {
    // Namespaces and Packages
    Namespace,
    Package,
    LibraryPackage,

    // KerML Classifiers
    Class,
    DataType,
    Structure,
    Association,
    AssociationStructure,
    Interaction,
    Behavior,
    Function,
    Predicate,

    // SysML Definitions
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
    EnumerationDefinition,
    MetadataDefinition,

    // SysML Usages
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
    SuccessionFlowConnectionUsage,

    // KerML Features
    Feature,
    Step,
    Expression,
    BooleanExpression,
    Invariant,
    Connector,
    BindingConnector,
    Succession,
    Flow,

    // Multiplicity and Literals
    MultiplicityRange,
    LiteralInteger,
    LiteralReal,
    LiteralInfinity,
    LiteralBoolean,
    LiteralString,
    NullExpression,

    // Expressions
    FeatureReferenceExpression,
    OperatorExpression,
    InvocationExpression,
    FeatureChainExpression,
    ConstructorExpression,

    // Relationships (first-class)
    Membership,
    OwningMembership,
    FeatureMembership,
    ReturnParameterMembership,
    ParameterMembership,
    EndFeatureMembership,
    ResultExpressionMembership,
    Import,
    NamespaceImport,
    MembershipImport,
    Specialization,
    FeatureTyping,
    Subsetting,
    ReferenceSubsetting,
    CrossSubsetting,
    Redefinition,
    Conjugation,
    FeatureValue,
    FeatureChaining,
    FeatureInverting,
    Intersecting,
    Disjoining,
    Unioning,

    // Dependency and requirement relationships
    Dependency,
    Satisfaction,
    Verification,

    // Comments and documentation
    Comment,
    Documentation,
    TextualRepresentation,

    // Annotations
    MetadataUsage,
    AnnotatingElement,
    Annotation,

    // Classifiers
    Classifier,
    Metaclass,

    // Generic
    Other,
}

impl ElementKind {
    /// Returns true if this is a definition (type-like).
    pub fn is_definition(&self) -> bool {
        matches!(
            self,
            Self::Package
                | Self::LibraryPackage
                | Self::Class
                | Self::DataType
                | Self::Structure
                | Self::Association
                | Self::AssociationStructure
                | Self::Interaction
                | Self::Behavior
                | Self::Function
                | Self::Predicate
                | Self::PartDefinition
                | Self::ItemDefinition
                | Self::ActionDefinition
                | Self::PortDefinition
                | Self::AttributeDefinition
                | Self::ConnectionDefinition
                | Self::InterfaceDefinition
                | Self::AllocationDefinition
                | Self::RequirementDefinition
                | Self::ConstraintDefinition
                | Self::StateDefinition
                | Self::CalculationDefinition
                | Self::UseCaseDefinition
                | Self::AnalysisCaseDefinition
                | Self::ConcernDefinition
                | Self::ViewDefinition
                | Self::ViewpointDefinition
                | Self::RenderingDefinition
                | Self::EnumerationDefinition
                | Self::MetadataDefinition
        )
    }

    /// Returns true if this is a usage (instance-like).
    pub fn is_usage(&self) -> bool {
        matches!(
            self,
            Self::PartUsage
                | Self::ItemUsage
                | Self::ActionUsage
                | Self::PortUsage
                | Self::AttributeUsage
                | Self::ConnectionUsage
                | Self::InterfaceUsage
                | Self::AllocationUsage
                | Self::RequirementUsage
                | Self::ConstraintUsage
                | Self::StateUsage
                | Self::TransitionUsage
                | Self::CalculationUsage
                | Self::ReferenceUsage
                | Self::OccurrenceUsage
                | Self::FlowConnectionUsage
                | Self::SuccessionFlowConnectionUsage
                | Self::Feature
                | Self::Step
                | Self::Expression
                | Self::BooleanExpression
                | Self::Invariant
        )
    }

    /// Returns true if this is a relationship.
    pub fn is_relationship(&self) -> bool {
        matches!(
            self,
            Self::Membership
                | Self::OwningMembership
                | Self::FeatureMembership
                | Self::ReturnParameterMembership
                | Self::ParameterMembership
                | Self::EndFeatureMembership
                | Self::ResultExpressionMembership
                | Self::Import
                | Self::NamespaceImport
                | Self::MembershipImport
                | Self::Specialization
                | Self::FeatureTyping
                | Self::Subsetting
                | Self::ReferenceSubsetting
                | Self::CrossSubsetting
                | Self::Redefinition
                | Self::Conjugation
                | Self::FeatureValue
                | Self::FeatureChaining
                | Self::FeatureInverting
                | Self::Intersecting
                | Self::Disjoining
                | Self::Unioning
                | Self::Dependency
                | Self::Satisfaction
                | Self::Verification
                | Self::Annotation
        )
    }

    /// Returns true if this is a membership relationship — an ownership
    /// intermediary in the KerML metamodel.
    ///
    /// Per KerML, every ownership edge passes through a Membership (or subtype).
    /// Membership elements are transparent containers: traversal helpers like
    /// `Model::owned_members()` "look through" them to reach the actual content.
    pub fn is_membership(&self) -> bool {
        matches!(
            self,
            Self::OwningMembership
                | Self::FeatureMembership
                | Self::ReturnParameterMembership
                | Self::ParameterMembership
                | Self::EndFeatureMembership
                | Self::ResultExpressionMembership
        )
    }

    /// Returns true if this element kind is a Feature (or Feature subtype)
    /// in the KerML metamodel.
    ///
    /// Feature children of Types are owned through `FeatureMembership`;
    /// other children use `OwningMembership`.
    pub fn is_feature_kind(&self) -> bool {
        self.is_usage()
            || matches!(
                self,
                Self::Connector
                    | Self::BindingConnector
                    | Self::Succession
                    | Self::Flow
                    | Self::MultiplicityRange
                    | Self::LiteralInteger
                    | Self::LiteralReal
                    | Self::LiteralInfinity
                    | Self::LiteralBoolean
                    | Self::LiteralString
                    | Self::NullExpression
                    | Self::FeatureReferenceExpression
                    | Self::OperatorExpression
                    | Self::InvocationExpression
                    | Self::FeatureChainExpression
                    | Self::ConstructorExpression
            )
    }

    /// Pick the appropriate membership kind for wrapping this child element.
    ///
    /// Returns `FeatureMembership` if the child is a Feature subtype,
    /// otherwise `OwningMembership`.
    pub fn membership_kind_for(child_kind: ElementKind) -> ElementKind {
        if child_kind.is_feature_kind() {
            Self::FeatureMembership
        } else {
            Self::OwningMembership
        }
    }

    /// Returns true if this element kind is rendered inline in decompiled
    /// SysML (as part of the declaration line) rather than as a body member.
    /// Used to decide whether a usage/feature needs `{ }` braces.
    pub fn is_inline_rendered(&self) -> bool {
        matches!(
            self,
            Self::FeatureValue
                | Self::FeatureTyping
                | Self::Specialization
                | Self::Redefinition
                | Self::Subsetting
                | Self::ReferenceSubsetting
                | Self::CrossSubsetting
                | Self::FeatureChaining
                | Self::Conjugation
        )
    }

    /// Returns true if this is a SysML (not KerML) element kind.
    /// SysML elements use `declaredName` instead of `name`.
    pub fn is_sysml(&self) -> bool {
        matches!(
            self,
            Self::Namespace
                | Self::Package
                | Self::LibraryPackage
                | Self::PartDefinition
                | Self::ItemDefinition
                | Self::ActionDefinition
                | Self::PortDefinition
                | Self::AttributeDefinition
                | Self::ConnectionDefinition
                | Self::InterfaceDefinition
                | Self::AllocationDefinition
                | Self::RequirementDefinition
                | Self::ConstraintDefinition
                | Self::UseCaseDefinition
                | Self::ConcernDefinition
                | Self::ViewDefinition
                | Self::ViewpointDefinition
                | Self::RenderingDefinition
                | Self::StateDefinition
                | Self::TransitionUsage
                | Self::CalculationDefinition
                | Self::AnalysisCaseDefinition
                | Self::EnumerationDefinition
                | Self::MetadataDefinition
                | Self::PartUsage
                | Self::ItemUsage
                | Self::ActionUsage
                | Self::PortUsage
                | Self::AttributeUsage
                | Self::ConnectionUsage
                | Self::InterfaceUsage
                | Self::AllocationUsage
                | Self::RequirementUsage
                | Self::ConstraintUsage
                | Self::StateUsage
                | Self::CalculationUsage
                | Self::ReferenceUsage
                | Self::OccurrenceUsage
                | Self::FlowConnectionUsage
                | Self::SuccessionFlowConnectionUsage
                | Self::MetadataUsage
        )
    }

    /// Get the XMI type name for this kind.
    pub fn xmi_type(&self) -> &'static str {
        match self {
            Self::Namespace => "sysml:Namespace",
            Self::Package => "sysml:Package",
            Self::LibraryPackage => "sysml:LibraryPackage",
            Self::Class => "kerml:Class",
            Self::DataType => "kerml:DataType",
            Self::Structure => "kerml:Structure",
            Self::Association => "kerml:Association",
            Self::AssociationStructure => "kerml:AssociationStructure",
            Self::Interaction => "kerml:Interaction",
            Self::Behavior => "kerml:Behavior",
            Self::Function => "kerml:Function",
            Self::Predicate => "kerml:Predicate",
            Self::PartDefinition => "sysml:PartDefinition",
            Self::ItemDefinition => "sysml:ItemDefinition",
            Self::ActionDefinition => "sysml:ActionDefinition",
            Self::PortDefinition => "sysml:PortDefinition",
            Self::AttributeDefinition => "sysml:AttributeDefinition",
            Self::ConnectionDefinition => "sysml:ConnectionDefinition",
            Self::InterfaceDefinition => "sysml:InterfaceDefinition",
            Self::AllocationDefinition => "sysml:AllocationDefinition",
            Self::RequirementDefinition => "sysml:RequirementDefinition",
            Self::ConstraintDefinition => "sysml:ConstraintDefinition",
            Self::StateDefinition => "sysml:StateDefinition",
            Self::CalculationDefinition => "sysml:CalculationDefinition",
            Self::UseCaseDefinition => "sysml:UseCaseDefinition",
            Self::AnalysisCaseDefinition => "sysml:AnalysisCaseDefinition",
            Self::ConcernDefinition => "sysml:ConcernDefinition",
            Self::ViewDefinition => "sysml:ViewDefinition",
            Self::ViewpointDefinition => "sysml:ViewpointDefinition",
            Self::RenderingDefinition => "sysml:RenderingDefinition",
            Self::EnumerationDefinition => "sysml:EnumerationDefinition",
            Self::MetadataDefinition => "sysml:MetadataDefinition",
            Self::PartUsage => "sysml:PartUsage",
            Self::ItemUsage => "sysml:ItemUsage",
            Self::ActionUsage => "sysml:ActionUsage",
            Self::PortUsage => "sysml:PortUsage",
            Self::AttributeUsage => "sysml:AttributeUsage",
            Self::ConnectionUsage => "sysml:ConnectionUsage",
            Self::InterfaceUsage => "sysml:InterfaceUsage",
            Self::AllocationUsage => "sysml:AllocationUsage",
            Self::RequirementUsage => "sysml:RequirementUsage",
            Self::ConstraintUsage => "sysml:ConstraintUsage",
            Self::StateUsage => "sysml:StateUsage",
            Self::TransitionUsage => "sysml:TransitionUsage",
            Self::CalculationUsage => "sysml:CalculationUsage",
            Self::ReferenceUsage => "sysml:ReferenceUsage",
            Self::OccurrenceUsage => "sysml:OccurrenceUsage",
            Self::FlowConnectionUsage => "sysml:FlowConnectionUsage",
            Self::SuccessionFlowConnectionUsage => "sysml:SuccessionFlowConnectionUsage",
            Self::Feature => "kerml:Feature",
            Self::Step => "kerml:Step",
            Self::Expression => "kerml:Expression",
            Self::BooleanExpression => "kerml:BooleanExpression",
            Self::Invariant => "kerml:Invariant",
            Self::Connector => "kerml:Connector",
            Self::BindingConnector => "kerml:BindingConnector",
            Self::Succession => "kerml:Succession",
            Self::Flow => "kerml:Flow",
            Self::MultiplicityRange => "kerml:MultiplicityRange",
            Self::LiteralInteger => "kerml:LiteralInteger",
            Self::LiteralReal => "kerml:LiteralRational",
            Self::LiteralInfinity => "kerml:LiteralInfinity",
            Self::LiteralBoolean => "kerml:LiteralBoolean",
            Self::LiteralString => "kerml:LiteralString",
            Self::NullExpression => "kerml:NullExpression",
            Self::FeatureReferenceExpression => "kerml:FeatureReferenceExpression",
            Self::OperatorExpression => "kerml:OperatorExpression",
            Self::InvocationExpression => "kerml:InvocationExpression",
            Self::FeatureChainExpression => "kerml:FeatureChainExpression",
            Self::ConstructorExpression => "kerml:ConstructorExpression",
            Self::Membership => "kerml:Membership",
            Self::OwningMembership => "kerml:OwningMembership",
            Self::FeatureMembership => "kerml:FeatureMembership",
            Self::ReturnParameterMembership => "kerml:ReturnParameterMembership",
            Self::ParameterMembership => "kerml:ParameterMembership",
            Self::EndFeatureMembership => "kerml:EndFeatureMembership",
            Self::ResultExpressionMembership => "kerml:ResultExpressionMembership",
            Self::Import => "kerml:Import",
            Self::NamespaceImport => "kerml:NamespaceImport",
            Self::MembershipImport => "kerml:MembershipImport",
            Self::Specialization => "kerml:Specialization",
            Self::FeatureTyping => "kerml:FeatureTyping",
            Self::Subsetting => "kerml:Subsetting",
            Self::ReferenceSubsetting => "kerml:ReferenceSubsetting",
            Self::CrossSubsetting => "kerml:CrossSubsetting",
            Self::Redefinition => "kerml:Redefinition",
            Self::Conjugation => "kerml:Conjugation",
            Self::FeatureValue => "kerml:FeatureValue",
            Self::FeatureChaining => "kerml:FeatureChaining",
            Self::FeatureInverting => "kerml:FeatureInverting",
            Self::Intersecting => "kerml:Intersecting",
            Self::Disjoining => "kerml:Disjoining",
            Self::Unioning => "kerml:Unioning",
            Self::Dependency => "kerml:Dependency",
            Self::Satisfaction => "sysml:SatisfyRequirementUsage",
            Self::Verification => "sysml:RequirementVerificationMembership",
            Self::Comment => "kerml:Comment",
            Self::Documentation => "kerml:Documentation",
            Self::TextualRepresentation => "kerml:TextualRepresentation",
            Self::MetadataUsage => "sysml:MetadataUsage",
            Self::AnnotatingElement => "kerml:AnnotatingElement",
            Self::Annotation => "kerml:Annotation",
            Self::Classifier => "kerml:Classifier",
            Self::Metaclass => "kerml:Metaclass",
            Self::Other => "kerml:Element",
        }
    }

    /// Get the xsi:type value for this kind (official XMI format).
    pub fn xsi_type(&self) -> &'static str {
        match self {
            Self::Package => "sysml:Package",
            Self::LibraryPackage => "sysml:LibraryPackage",
            Self::Class => "sysml:Class",
            Self::DataType => "sysml:DataType",
            Self::Structure => "sysml:Structure",
            Self::Association => "sysml:Association",
            Self::AssociationStructure => "sysml:AssociationStructure",
            Self::Interaction => "sysml:Interaction",
            Self::Behavior => "sysml:Behavior",
            Self::Function => "sysml:Function",
            Self::Predicate => "sysml:Predicate",
            Self::PartDefinition => "sysml:PartDefinition",
            Self::ItemDefinition => "sysml:ItemDefinition",
            Self::ActionDefinition => "sysml:ActionDefinition",
            Self::PortDefinition => "sysml:PortDefinition",
            Self::AttributeDefinition => "sysml:AttributeDefinition",
            Self::ConnectionDefinition => "sysml:ConnectionDefinition",
            Self::InterfaceDefinition => "sysml:InterfaceDefinition",
            Self::AllocationDefinition => "sysml:AllocationDefinition",
            Self::RequirementDefinition => "sysml:RequirementDefinition",
            Self::ConstraintDefinition => "sysml:ConstraintDefinition",
            Self::StateDefinition => "sysml:StateDefinition",
            Self::CalculationDefinition => "sysml:CalculationDefinition",
            Self::UseCaseDefinition => "sysml:UseCaseDefinition",
            Self::AnalysisCaseDefinition => "sysml:AnalysisCaseDefinition",
            Self::ConcernDefinition => "sysml:ConcernDefinition",
            Self::ViewDefinition => "sysml:ViewDefinition",
            Self::ViewpointDefinition => "sysml:ViewpointDefinition",
            Self::RenderingDefinition => "sysml:RenderingDefinition",
            Self::EnumerationDefinition => "sysml:EnumerationDefinition",
            Self::MetadataDefinition => "sysml:MetadataDefinition",
            Self::PartUsage => "sysml:PartUsage",
            Self::ItemUsage => "sysml:ItemUsage",
            Self::ActionUsage => "sysml:ActionUsage",
            Self::PortUsage => "sysml:PortUsage",
            Self::AttributeUsage => "sysml:AttributeUsage",
            Self::ConnectionUsage => "sysml:ConnectionUsage",
            Self::InterfaceUsage => "sysml:InterfaceUsage",
            Self::AllocationUsage => "sysml:AllocationUsage",
            Self::RequirementUsage => "sysml:RequirementUsage",
            Self::ConstraintUsage => "sysml:ConstraintUsage",
            Self::StateUsage => "sysml:StateUsage",
            Self::TransitionUsage => "sysml:TransitionUsage",
            Self::CalculationUsage => "sysml:CalculationUsage",
            Self::ReferenceUsage => "sysml:ReferenceUsage",
            Self::OccurrenceUsage => "sysml:OccurrenceUsage",
            Self::FlowConnectionUsage => "sysml:FlowConnectionUsage",
            Self::SuccessionFlowConnectionUsage => "sysml:SuccessionFlowConnectionUsage",
            Self::Feature => "sysml:Feature",
            Self::Step => "sysml:Step",
            Self::Expression => "sysml:Expression",
            Self::BooleanExpression => "sysml:BooleanExpression",
            Self::Invariant => "sysml:Invariant",
            Self::Connector => "sysml:Connector",
            Self::BindingConnector => "sysml:BindingConnector",
            Self::Succession => "sysml:Succession",
            Self::Flow => "sysml:Flow",
            Self::MultiplicityRange => "sysml:MultiplicityRange",
            Self::LiteralInteger => "sysml:LiteralInteger",
            Self::LiteralReal => "sysml:LiteralRational",
            Self::LiteralInfinity => "sysml:LiteralInfinity",
            Self::LiteralBoolean => "sysml:LiteralBoolean",
            Self::LiteralString => "sysml:LiteralString",
            Self::NullExpression => "sysml:NullExpression",
            Self::FeatureReferenceExpression => "sysml:FeatureReferenceExpression",
            Self::OperatorExpression => "sysml:OperatorExpression",
            Self::InvocationExpression => "sysml:InvocationExpression",
            Self::FeatureChainExpression => "sysml:FeatureChainExpression",
            Self::ConstructorExpression => "sysml:ConstructorExpression",
            Self::Membership => "sysml:Membership",
            Self::OwningMembership => "sysml:OwningMembership",
            Self::FeatureMembership => "sysml:FeatureMembership",
            Self::ReturnParameterMembership => "sysml:ReturnParameterMembership",
            Self::ParameterMembership => "sysml:ParameterMembership",
            Self::EndFeatureMembership => "sysml:EndFeatureMembership",
            Self::ResultExpressionMembership => "sysml:ResultExpressionMembership",
            Self::Import => "sysml:Import",
            Self::NamespaceImport => "sysml:NamespaceImport",
            Self::MembershipImport => "sysml:MembershipImport",
            Self::Specialization => "sysml:Subclassification",
            Self::FeatureTyping => "sysml:FeatureTyping",
            Self::Subsetting => "sysml:Subsetting",
            Self::ReferenceSubsetting => "sysml:ReferenceSubsetting",
            Self::CrossSubsetting => "sysml:CrossSubsetting",
            Self::Redefinition => "sysml:Redefinition",
            Self::Conjugation => "sysml:Conjugation",
            Self::FeatureValue => "sysml:FeatureValue",
            Self::FeatureChaining => "sysml:FeatureChaining",
            Self::FeatureInverting => "sysml:FeatureInverting",
            Self::Intersecting => "sysml:Intersecting",
            Self::Disjoining => "sysml:Disjoining",
            Self::Unioning => "sysml:Unioning",
            Self::Dependency => "sysml:Dependency",
            Self::Satisfaction => "sysml:SatisfyRequirementUsage",
            Self::Verification => "sysml:RequirementVerificationMembership",
            Self::Comment => "sysml:Comment",
            Self::Documentation => "sysml:Documentation",
            Self::TextualRepresentation => "sysml:TextualRepresentation",
            Self::MetadataUsage => "sysml:MetadataUsage",
            Self::AnnotatingElement => "sysml:AnnotatingElement",
            Self::Annotation => "sysml:Annotation",
            Self::Classifier => "sysml:Classifier",
            Self::Metaclass => "sysml:Metaclass",
            Self::Other => "sysml:Element",
            Self::Namespace => "sysml:Namespace",
        }
    }

    /// Parse from XMI type name.
    pub fn from_xmi_type(xmi_type: &str) -> Self {
        // Strip namespace prefix if present
        let type_name = xmi_type.rsplit(':').next().unwrap_or(xmi_type);

        match type_name {
            "Namespace" => Self::Namespace,
            "Package" => Self::Package,
            "LibraryPackage" => Self::LibraryPackage,
            "Class" => Self::Class,
            "DataType" => Self::DataType,
            "Structure" => Self::Structure,
            "Association" => Self::Association,
            "AssociationStructure" => Self::AssociationStructure,
            "Interaction" => Self::Interaction,
            "Behavior" => Self::Behavior,
            "Function" => Self::Function,
            "Predicate" => Self::Predicate,
            "PartDefinition" => Self::PartDefinition,
            "ItemDefinition" => Self::ItemDefinition,
            "ActionDefinition" => Self::ActionDefinition,
            "PortDefinition" => Self::PortDefinition,
            "AttributeDefinition" => Self::AttributeDefinition,
            "ConnectionDefinition" => Self::ConnectionDefinition,
            "InterfaceDefinition" => Self::InterfaceDefinition,
            "AllocationDefinition" => Self::AllocationDefinition,
            "RequirementDefinition" => Self::RequirementDefinition,
            "ConstraintDefinition" => Self::ConstraintDefinition,
            "StateDefinition" => Self::StateDefinition,
            "CalculationDefinition" => Self::CalculationDefinition,
            "UseCaseDefinition" => Self::UseCaseDefinition,
            "AnalysisCaseDefinition" => Self::AnalysisCaseDefinition,
            "ConcernDefinition" => Self::ConcernDefinition,
            "ViewDefinition" => Self::ViewDefinition,
            "ViewpointDefinition" => Self::ViewpointDefinition,
            "RenderingDefinition" => Self::RenderingDefinition,
            "EnumerationDefinition" => Self::EnumerationDefinition,
            "MetadataDefinition" => Self::MetadataDefinition,
            "PartUsage" => Self::PartUsage,
            "ItemUsage" => Self::ItemUsage,
            "ActionUsage" => Self::ActionUsage,
            "PortUsage" => Self::PortUsage,
            "AttributeUsage" => Self::AttributeUsage,
            "ConnectionUsage" => Self::ConnectionUsage,
            "InterfaceUsage" => Self::InterfaceUsage,
            "AllocationUsage" => Self::AllocationUsage,
            "RequirementUsage" => Self::RequirementUsage,
            "ConstraintUsage" => Self::ConstraintUsage,
            "StateUsage" => Self::StateUsage,
            "TransitionUsage" => Self::TransitionUsage,
            "CalculationUsage" => Self::CalculationUsage,
            "ReferenceUsage" => Self::ReferenceUsage,
            "OccurrenceUsage" => Self::OccurrenceUsage,
            "FlowConnectionUsage" => Self::FlowConnectionUsage,
            "SuccessionFlowConnectionUsage" => Self::SuccessionFlowConnectionUsage,
            "Feature" => Self::Feature,
            "Step" => Self::Step,
            "Expression" => Self::Expression,
            "BooleanExpression" => Self::BooleanExpression,
            "Invariant" => Self::Invariant,
            "Connector" => Self::Connector,
            "BindingConnector" => Self::BindingConnector,
            "Succession" => Self::Succession,
            "Flow" => Self::Flow,
            "MultiplicityRange" => Self::MultiplicityRange,
            "LiteralInteger" => Self::LiteralInteger,
            "LiteralRational" | "LiteralReal" => Self::LiteralReal,
            "LiteralInfinity" => Self::LiteralInfinity,
            "LiteralBoolean" => Self::LiteralBoolean,
            "LiteralString" => Self::LiteralString,
            "NullExpression" => Self::NullExpression,
            "FeatureReferenceExpression" => Self::FeatureReferenceExpression,
            "OperatorExpression" => Self::OperatorExpression,
            "InvocationExpression" => Self::InvocationExpression,
            "FeatureChainExpression" => Self::FeatureChainExpression,
            "ConstructorExpression" => Self::ConstructorExpression,
            "Membership" => Self::Membership,
            "OwningMembership" => Self::OwningMembership,
            "FeatureMembership" => Self::FeatureMembership,
            "ReturnParameterMembership" => Self::ReturnParameterMembership,
            "ParameterMembership" => Self::ParameterMembership,
            "EndFeatureMembership" => Self::EndFeatureMembership,
            "ResultExpressionMembership" => Self::ResultExpressionMembership,
            "Import" => Self::Import,
            "NamespaceImport" => Self::NamespaceImport,
            "MembershipImport" => Self::MembershipImport,
            "Specialization" | "Subclassification" => Self::Specialization,
            "FeatureTyping" => Self::FeatureTyping,
            "Subsetting" => Self::Subsetting,
            "ReferenceSubsetting" => Self::ReferenceSubsetting,
            "CrossSubsetting" => Self::CrossSubsetting,
            "Redefinition" => Self::Redefinition,
            "Conjugation" => Self::Conjugation,
            "FeatureValue" => Self::FeatureValue,
            "FeatureChaining" => Self::FeatureChaining,
            "FeatureInverting" => Self::FeatureInverting,
            "Intersecting" => Self::Intersecting,
            "Disjoining" => Self::Disjoining,
            "Unioning" => Self::Unioning,
            "Dependency" => Self::Dependency,
            "SatisfyRequirementUsage" => Self::Satisfaction,
            "RequirementVerificationMembership" => Self::Verification,
            "Comment" => Self::Comment,
            "Documentation" => Self::Documentation,
            "TextualRepresentation" => Self::TextualRepresentation,
            "MetadataUsage" => Self::MetadataUsage,
            "AnnotatingElement" => Self::AnnotatingElement,
            "Annotation" => Self::Annotation,
            "Classifier" => Self::Classifier,
            "Metaclass" => Self::Metaclass,
            _ => Self::Other,
        }
    }

    /// Get the JSON-LD @type value.
    pub fn jsonld_type(&self) -> &'static str {
        // JSON-LD uses the same type names without namespace prefix
        self.xmi_type().rsplit(':').next().unwrap_or("Element")
    }
}

// ============================================================================
// ELEMENT
// ============================================================================

/// A model element with its properties.
#[derive(Clone, Debug)]
pub struct Element {
    /// Unique identifier.
    pub id: ElementId,
    /// The metatype.
    pub kind: ElementKind,
    /// The declared name (may be None for anonymous elements).
    pub name: Option<Arc<str>>,
    /// Short name alias.
    pub short_name: Option<Arc<str>>,
    /// Qualified name (computed from ownership hierarchy).
    pub qualified_name: Option<Arc<str>>,
    /// The owning element's ID (None for root elements).
    pub owner: Option<ElementId>,
    /// IDs of directly owned elements.
    pub owned_elements: Vec<ElementId>,
    /// Documentation text.
    pub documentation: Option<Arc<str>>,
    /// Whether this element is abstract.
    pub is_abstract: bool,
    /// Whether this is a variation (SysML).
    pub is_variation: bool,
    /// Whether this feature is derived.
    pub is_derived: bool,
    /// Whether this feature is read-only.
    pub is_readonly: bool,
    /// Whether this state is parallel (SysML).
    pub is_parallel: bool,
    /// Whether this is an individual (singleton occurrence).
    pub is_individual: bool,
    /// Whether this is an end feature (connector endpoint).
    pub is_end: bool,
    /// Whether this has a default value.
    pub is_default: bool,
    /// Whether values are ordered.
    pub is_ordered: bool,
    /// Whether values are nonunique (can have duplicates).
    pub is_nonunique: bool,
    /// Whether this is a portion (slice of occurrence).
    pub is_portion: bool,
    /// Visibility (public, private, protected).
    pub visibility: Visibility,
    /// Additional properties as key-value pairs (IndexMap preserves order).
    pub properties: IndexMap<Arc<str>, PropertyValue>,
    /// Relationship-specific data (source/target).
    /// Present when this element represents a relationship edge.
    pub relationship: Option<RelationshipData>,
}

impl Element {
    /// Create a new element with the given ID and kind.
    pub fn new(id: impl Into<ElementId>, kind: ElementKind) -> Self {
        Self {
            id: id.into(),
            kind,
            name: None,
            short_name: None,
            qualified_name: None,
            owner: None,
            owned_elements: Vec::new(),
            documentation: None,
            is_abstract: false,
            is_variation: false,
            is_derived: false,
            is_readonly: false,
            is_parallel: false,
            is_individual: false,
            is_end: false,
            is_default: false,
            is_ordered: false,
            is_nonunique: false,
            is_portion: false,
            visibility: Visibility::Public,
            properties: IndexMap::new(),
            relationship: None,
        }
    }

    /// Create a new relationship element with the given source and target.
    ///
    /// This is a convenience constructor for elements where
    /// `kind.is_relationship()` is true.
    pub fn new_relationship(
        id: impl Into<ElementId>,
        kind: ElementKind,
        source: impl Into<ElementId>,
        target: impl Into<ElementId>,
    ) -> Self {
        Self {
            relationship: Some(RelationshipData::new(source, target)),
            ..Self::new(id, kind)
        }
    }

    /// Returns `true` if this element carries relationship data.
    pub fn is_relationship_element(&self) -> bool {
        self.relationship.is_some()
    }

    /// Get the relationship data, if this element is a relationship.
    pub fn as_relationship(&self) -> Option<&RelationshipData> {
        self.relationship.as_ref()
    }

    /// Get a mutable reference to the relationship data.
    pub fn as_relationship_mut(&mut self) -> Option<&mut RelationshipData> {
        self.relationship.as_mut()
    }

    /// Get the first source element ID (convenience for relationship elements).
    pub fn source(&self) -> Option<&ElementId> {
        self.relationship.as_ref()?.source()
    }

    /// Get the first target element ID (convenience for relationship elements).
    pub fn target(&self) -> Option<&ElementId> {
        self.relationship.as_ref()?.target()
    }

    /// Set relationship data on this element.
    pub fn with_relationship(
        mut self,
        source: impl Into<ElementId>,
        target: impl Into<ElementId>,
    ) -> Self {
        self.relationship = Some(RelationshipData::new(source, target));
        self
    }

    /// Set the name.
    pub fn with_name(mut self, name: impl Into<Arc<str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the qualified name.
    pub fn with_qualified_name(mut self, qualified_name: impl Into<Arc<str>>) -> Self {
        self.qualified_name = Some(qualified_name.into());
        self
    }

    /// Set the short name.
    pub fn with_short_name(mut self, short_name: impl Into<Arc<str>>) -> Self {
        self.short_name = Some(short_name.into());
        self
    }

    /// Set the owner.
    pub fn with_owner(mut self, owner: impl Into<ElementId>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Set the owner from an `Option<ElementId>`.
    pub fn with_owner_opt(mut self, owner: Option<ElementId>) -> Self {
        self.owner = owner;
        self
    }

    /// Add an owned element ID.
    pub fn with_owned(mut self, owned: impl Into<ElementId>) -> Self {
        self.owned_elements.push(owned.into());
        self
    }

    /// Set a property value.
    pub fn with_property(mut self, key: impl Into<Arc<str>>, value: PropertyValue) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Set isAbstract (syncs to property for roundtrip fidelity).
    pub fn set_abstract(&mut self, value: bool) {
        self.is_abstract = value;
        self.properties
            .insert(Arc::from("isAbstract"), PropertyValue::Boolean(value));
    }

    /// Set isVariation (syncs to property for roundtrip fidelity).
    pub fn set_variation(&mut self, value: bool) {
        self.is_variation = value;
        self.properties
            .insert(Arc::from("isVariation"), PropertyValue::Boolean(value));
    }

    /// Set isDerived (syncs to property for roundtrip fidelity).
    pub fn set_derived(&mut self, value: bool) {
        self.is_derived = value;
        self.properties
            .insert(Arc::from("isDerived"), PropertyValue::Boolean(value));
    }

    /// Set isReadOnly (syncs to property for roundtrip fidelity).
    pub fn set_readonly(&mut self, value: bool) {
        self.is_readonly = value;
        self.properties
            .insert(Arc::from("isReadOnly"), PropertyValue::Boolean(value));
    }

    /// Set isParallel (syncs to property for roundtrip fidelity).
    pub fn set_parallel(&mut self, value: bool) {
        self.is_parallel = value;
        self.properties
            .insert(Arc::from("isParallel"), PropertyValue::Boolean(value));
    }

    /// Set isIndividual (syncs to property for roundtrip fidelity).
    pub fn set_individual(&mut self, value: bool) {
        self.is_individual = value;
        self.properties
            .insert(Arc::from("isIndividual"), PropertyValue::Boolean(value));
    }

    /// Set isEnd (syncs to property for roundtrip fidelity).
    pub fn set_end(&mut self, value: bool) {
        self.is_end = value;
        self.properties
            .insert(Arc::from("isEnd"), PropertyValue::Boolean(value));
    }

    /// Set isDefault (syncs to property for roundtrip fidelity).
    pub fn set_default(&mut self, value: bool) {
        self.is_default = value;
        self.properties
            .insert(Arc::from("isDefault"), PropertyValue::Boolean(value));
    }

    /// Set isOrdered (syncs to property for roundtrip fidelity).
    pub fn set_ordered(&mut self, value: bool) {
        self.is_ordered = value;
        self.properties
            .insert(Arc::from("isOrdered"), PropertyValue::Boolean(value));
    }

    /// Set isNonunique (syncs to property for roundtrip fidelity).
    pub fn set_nonunique(&mut self, value: bool) {
        self.is_nonunique = value;
        self.properties
            .insert(Arc::from("isNonunique"), PropertyValue::Boolean(value));
    }

    /// Set isPortion (syncs to property for roundtrip fidelity).
    pub fn set_portion(&mut self, value: bool) {
        self.is_portion = value;
        self.properties
            .insert(Arc::from("isPortion"), PropertyValue::Boolean(value));
    }
}

/// Visibility of an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
}

/// A property value that can be stored on an element.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// String value.
    String(Arc<str>),
    /// Integer value.
    Integer(i64),
    /// Floating-point value.
    Real(f64),
    /// Boolean value.
    Boolean(bool),
    /// Reference to another element by ID.
    Reference(ElementId),
    /// List of values.
    List(Vec<PropertyValue>),
}

impl From<&str> for PropertyValue {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}

impl From<String> for PropertyValue {
    fn from(s: String) -> Self {
        Self::String(s.into())
    }
}

impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}

impl From<f64> for PropertyValue {
    fn from(v: f64) -> Self {
        Self::Real(v)
    }
}

impl From<bool> for PropertyValue {
    fn from(v: bool) -> Self {
        Self::Boolean(v)
    }
}

impl From<ElementId> for PropertyValue {
    fn from(id: ElementId) -> Self {
        Self::Reference(id)
    }
}

// ============================================================================
// RELATIONSHIP DATA (on Element)
// ============================================================================

/// Relationship-specific data stored on an `Element`.
///
/// When an element's `ElementKind::is_relationship()` is true, this carries
/// the source/target references that make it a relationship edge.
/// This is the first step toward the KerML metamodel where every
/// Relationship IS an Element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipData {
    /// Source element(s) of the relationship.
    pub source: Vec<ElementId>,
    /// Target element(s) of the relationship.
    pub target: Vec<ElementId>,
}

impl RelationshipData {
    /// Create relationship data with a single source and target.
    pub fn new(source: impl Into<ElementId>, target: impl Into<ElementId>) -> Self {
        Self {
            source: vec![source.into()],
            target: vec![target.into()],
        }
    }

    /// Create relationship data with multiple sources and targets.
    pub fn new_multi(sources: Vec<ElementId>, targets: Vec<ElementId>) -> Self {
        Self {
            source: sources,
            target: targets,
        }
    }

    /// Get the first source element ID, if any.
    pub fn source(&self) -> Option<&ElementId> {
        self.source.first()
    }

    /// Get the first target element ID, if any.
    pub fn target(&self) -> Option<&ElementId> {
        self.target.first()
    }
}

// ============================================================================
// RELATIONSHIP (legacy — will be removed in Phase 5)
// ============================================================================

/// A relationship between two elements.
///
/// **Deprecated**: This separate struct will be removed once all code
/// migrates to using `Element` with `RelationshipData`.  See
// ============================================================================
// MODEL
// ============================================================================

/// A complete SysML/KerML model.
///
/// This is a standalone representation that can be:
/// - Loaded from XMI, KPAR, or JSON-LD
/// - Exported to various formats
/// - Integrated into a `RootDatabase` for IDE features
#[derive(Clone, Debug, Default)]
pub struct Model {
    /// All elements by ID (IndexMap preserves insertion order for deterministic serialization).
    pub elements: IndexMap<ElementId, Element>,
    /// Root element IDs (top-level packages).
    pub roots: Vec<ElementId>,
    /// Metadata about the model.
    pub metadata: ModelMetadata,
}

impl Model {
    /// Create a new empty model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an element to the model.
    pub fn add_element(&mut self, element: Element) -> &ElementId {
        let id = element.id.clone();
        if element.owner.is_none() {
            self.roots.push(id.clone());
        }
        self.elements.insert(id.clone(), element);
        // Return reference to the ID in the map
        &self.elements.get(&id).unwrap().id
    }

    /// Add a relationship as an `Element` with `RelationshipData`.
    ///
    /// The element is stored in `self.elements` only.
    pub fn add_rel(
        &mut self,
        id: impl Into<ElementId>,
        kind: ElementKind,
        source: impl Into<ElementId>,
        target: impl Into<ElementId>,
        owner: Option<ElementId>,
    ) -> ElementId {
        let id = id.into();
        let element = Element::new_relationship(id.clone(), kind, source.into(), target.into())
            .with_owner_opt(owner);
        self.elements.insert(id.clone(), element);
        id
    }

    /// Get an element by ID.
    pub fn get(&self, id: &ElementId) -> Option<&Element> {
        self.elements.get(id)
    }

    /// Get a mutable element by ID.
    pub fn get_mut(&mut self, id: &ElementId) -> Option<&mut Element> {
        self.elements.get_mut(id)
    }

    /// Iterate over all elements.
    pub fn iter_elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    /// Iterate over root elements.
    pub fn iter_roots(&self) -> impl Iterator<Item = &Element> {
        self.roots.iter().filter_map(|id| self.elements.get(id))
    }

    // ── Relationship queries ────────────────────────────────────────

    /// Get relationship **elements** where the given element is a source.
    ///
    /// This queries `self.elements` (not `self.relationships`), returning
    /// `&Element` values that have `RelationshipData` with a matching source.
    pub fn rel_elements_from<'a>(
        &'a self,
        source: &'a ElementId,
    ) -> impl Iterator<Item = &'a Element> {
        self.elements.values().filter(move |e| {
            e.relationship
                .as_ref()
                .map_or(false, |rd| rd.source.contains(source))
        })
    }

    /// Get relationship **elements** where the given element is a target.
    pub fn rel_elements_to<'a>(
        &'a self,
        target: &'a ElementId,
    ) -> impl Iterator<Item = &'a Element> {
        self.elements.values().filter(move |e| {
            e.relationship
                .as_ref()
                .map_or(false, |rd| rd.target.contains(target))
        })
    }

    /// Get relationship elements of a specific `ElementKind` from a source.
    pub fn rel_elements_of_kind<'a>(
        &'a self,
        source: &'a ElementId,
        kind: ElementKind,
    ) -> impl Iterator<Item = &'a Element> {
        self.rel_elements_from(source)
            .filter(move |e| e.kind == kind)
    }

    /// Resolve the first target element of a relationship element.
    pub fn rel_target<'a>(&'a self, rel_element: &'a Element) -> Option<&'a Element> {
        rel_element.target().and_then(|tid| self.get(tid))
    }

    /// Get relationship elements owned by a given element, optionally filtered by kind.
    pub fn rel_elements_owned_by<'a>(
        &'a self,
        owner_id: &'a ElementId,
        kind: ElementKind,
    ) -> impl Iterator<Item = &'a Element> {
        self.elements.values().filter(move |e| {
            e.kind == kind && e.relationship.is_some() && e.owner.as_ref() == Some(owner_id)
        })
    }

    /// Get the number of elements.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Get the number of relationships (counts relationship elements).
    pub fn relationship_count(&self) -> usize {
        self.elements
            .values()
            .filter(|e| e.relationship.is_some())
            .count()
    }

    /// Iterate over all relationship elements.
    pub fn iter_relationship_elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.values().filter(|e| e.relationship.is_some())
    }

    // ── Membership traversal ────────────────────────────────────────

    /// Get the "content" children of an element, looking through Membership
    /// wrappers.
    ///
    /// In the KerML metamodel, non-relationship children are wrapped in
    /// `OwningMembership` / `FeatureMembership` intermediaries.  This method
    /// transparently "unwraps" them so callers see the actual definitions,
    /// usages, and other content elements.
    ///
    /// For models **without** membership wrappers (legacy or HIR-synthesized
    /// before Phase 6), this falls back to returning direct non-relationship
    /// children — so it works with both representation styles.
    pub fn owned_members(&self, id: &ElementId) -> Vec<&Element> {
        let element = match self.get(id) {
            Some(e) => e,
            None => return Vec::new(),
        };
        let mut result = Vec::new();
        for child_id in &element.owned_elements {
            if let Some(child) = self.get(child_id) {
                if child.kind.is_membership() {
                    // Look through the membership to its children
                    for grandchild_id in &child.owned_elements {
                        if let Some(gc) = self.get(grandchild_id) {
                            if !gc.kind.is_relationship() {
                                result.push(gc);
                            }
                        }
                    }
                } else if !child.kind.is_relationship() {
                    // Direct non-relationship child (legacy / unwrapped)
                    result.push(child);
                }
            }
        }
        result
    }

    /// Wrap all direct non-relationship, non-membership children of every
    /// element in `OwningMembership` or `FeatureMembership` intermediaries.
    ///
    /// After calling this, the ownership tree matches the KerML metamodel:
    /// ```text
    /// Package
    /// └── OwningMembership        (generated)
    ///     └── PartDefinition
    ///         ├── FeatureTyping    (relationship — direct child, no wrapper)
    ///         └── FeatureMembership  (generated)
    ///             └── AttributeUsage
    /// ```
    ///
    /// Idempotent: elements whose parent is already a membership are skipped.
    pub fn wrap_children_in_memberships(&mut self) {
        // Collect (child_id, child_kind, parent_id) for elements that need wrapping.
        let to_wrap: Vec<(ElementId, ElementKind, ElementId)> = self
            .elements
            .values()
            .filter(|e| {
                // Has an owner
                if let Some(ref owner_id) = e.owner {
                    // Not a relationship (relationships are direct ownedRelationship children)
                    if e.kind.is_relationship() {
                        return false;
                    }
                    // Not already a membership itself
                    if e.kind.is_membership() {
                        return false;
                    }
                    // Parent is not already a membership (i.e. not already wrapped)
                    // AND parent is not a relationship (relationships own their
                    // related elements directly, not through memberships —
                    // e.g. FeatureValue → LiteralString stays direct)
                    if let Some(owner) = self.elements.get(owner_id) {
                        return !owner.kind.is_membership() && !owner.kind.is_relationship();
                    }
                }
                false
            })
            .map(|e| (e.id.clone(), e.kind, e.owner.clone().unwrap()))
            .collect();

        for (child_id, child_kind, parent_id) in to_wrap {
            let m_kind = ElementKind::membership_kind_for(child_kind);
            let m_id = ElementId::new(format!("{}-m", child_id.as_str()));

            // Create membership element
            let mut membership = Element::new(m_id.clone(), m_kind);
            membership.owner = Some(parent_id.clone());
            membership.owned_elements.push(child_id.clone());

            // Re-parent child: child.owner = membership
            if let Some(child) = self.elements.get_mut(&child_id) {
                child.owner = Some(m_id.clone());
            }

            // In parent's owned_elements: replace child_id → m_id
            if let Some(parent) = self.elements.get_mut(&parent_id) {
                if let Some(pos) = parent.owned_elements.iter().position(|id| *id == child_id) {
                    parent.owned_elements[pos] = m_id.clone();
                }
            }

            // Insert the membership element
            self.elements.insert(m_id, membership);
        }
    }
}

/// Metadata about a model.
#[derive(Clone, Debug, Default)]
pub struct ModelMetadata {
    /// Name of the model/project.
    pub name: Option<String>,
    /// Version string.
    pub version: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// URI of the model.
    pub uri: Option<String>,
    /// SysML/KerML version this model conforms to.
    pub sysml_version: Option<String>,
    /// Tool that created this model.
    pub tool: Option<String>,
    /// Creation timestamp.
    pub created: Option<String>,
    /// Last modified timestamp.
    pub modified: Option<String>,
    /// Declared XML namespaces (for roundtrip fidelity).
    /// Maps prefix -> namespace URI (e.g., "sysml" -> "https://www.omg.org/spec/SysML/20250201").
    pub declared_namespaces: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_generation() {
        let id1 = ElementId::generate();
        let id2 = ElementId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_element_builder() {
        let element = Element::new("pkg1", ElementKind::Package)
            .with_name("MyPackage")
            .with_short_name("mp");

        assert_eq!(element.id.as_str(), "pkg1");
        assert_eq!(element.name.as_deref(), Some("MyPackage"));
        assert_eq!(element.short_name.as_deref(), Some("mp"));
        assert_eq!(element.kind, ElementKind::Package);
    }

    #[test]
    fn test_model_add_elements() {
        let mut model = Model::new();

        let pkg = Element::new("pkg1", ElementKind::Package).with_name("Root");
        model.add_element(pkg);

        let part = Element::new("part1", ElementKind::PartDefinition)
            .with_name("Vehicle")
            .with_owner("pkg1");
        model.add_element(part);

        assert_eq!(model.element_count(), 2);
        assert_eq!(model.roots.len(), 1);
        assert_eq!(
            model.get(&ElementId::new("pkg1")).unwrap().name.as_deref(),
            Some("Root")
        );
    }

    #[test]
    fn test_model_relationships() {
        let mut model = Model::new();

        model.add_element(Element::new("def1", ElementKind::PartDefinition).with_name("Base"));
        model.add_element(Element::new("def2", ElementKind::PartDefinition).with_name("Derived"));

        model.add_rel("rel1", ElementKind::Specialization, "def2", "def1", None);

        assert_eq!(model.relationship_count(), 1);
        let source_id = ElementId::new("def2");
        let rels: Vec<_> = model.rel_elements_from(&source_id).collect();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].target().unwrap().as_str(), "def1");
    }

    #[test]
    fn test_element_kind_xmi_roundtrip() {
        let kinds = [
            ElementKind::Package,
            ElementKind::PartDefinition,
            ElementKind::ActionUsage,
            ElementKind::Specialization,
        ];

        for kind in kinds {
            let xmi_type = kind.xmi_type();
            let parsed = ElementKind::from_xmi_type(xmi_type);
            assert_eq!(kind, parsed, "Failed roundtrip for {xmi_type}");
        }
    }

    // ── Phase 1: RelationshipData tests ────────────────────────────

    #[test]
    fn test_relationship_data_new() {
        let rd = RelationshipData::new("src1", "tgt1");
        assert_eq!(rd.source().unwrap().as_str(), "src1");
        assert_eq!(rd.target().unwrap().as_str(), "tgt1");
        assert_eq!(rd.source.len(), 1);
        assert_eq!(rd.target.len(), 1);
    }

    #[test]
    fn test_relationship_data_multi() {
        let rd = RelationshipData::new_multi(
            vec![ElementId::new("s1"), ElementId::new("s2")],
            vec![ElementId::new("t1")],
        );
        assert_eq!(rd.source.len(), 2);
        assert_eq!(rd.target.len(), 1);
        assert_eq!(rd.source().unwrap().as_str(), "s1");
    }

    #[test]
    fn test_element_new_has_no_relationship_data() {
        let e = Element::new("e1", ElementKind::Package);
        assert!(!e.is_relationship_element());
        assert!(e.as_relationship().is_none());
        assert!(e.source().is_none());
        assert!(e.target().is_none());
    }

    #[test]
    fn test_element_new_relationship() {
        let e = Element::new_relationship("rel1", ElementKind::Specialization, "derived", "base");
        assert!(e.is_relationship_element());
        assert_eq!(e.source().unwrap().as_str(), "derived");
        assert_eq!(e.target().unwrap().as_str(), "base");
        assert_eq!(e.kind, ElementKind::Specialization);
    }

    #[test]
    fn test_element_with_relationship_builder() {
        let e = Element::new("rel2", ElementKind::FeatureTyping)
            .with_name("typing1")
            .with_relationship("feature1", "type1");
        assert!(e.is_relationship_element());
        assert_eq!(e.name.as_deref(), Some("typing1"));
        assert_eq!(e.source().unwrap().as_str(), "feature1");
        assert_eq!(e.target().unwrap().as_str(), "type1");
    }

    #[test]
    fn test_element_as_relationship_mut() {
        let mut e = Element::new_relationship("rel3", ElementKind::Subsetting, "sub", "super");
        let rd = e.as_relationship_mut().unwrap();
        rd.target.push(ElementId::new("super2"));
        assert_eq!(e.as_relationship().unwrap().target.len(), 2);
    }

    // ── Phase 4: Element-based relationship creation tests ────────

    #[test]
    fn test_add_rel_creates_element() {
        let mut model = Model::new();
        model.add_element(Element::new("a", ElementKind::PartDefinition).with_name("A"));
        model.add_element(Element::new("b", ElementKind::PartDefinition).with_name("B"));

        model.add_rel("rel1", ElementKind::Specialization, "b", "a", None);

        // Element store has the relationship element
        assert_eq!(model.relationship_count(), 1);
        let rel_el = model.get(&ElementId::new("rel1")).unwrap();
        assert!(rel_el.is_relationship_element());
        assert_eq!(rel_el.kind, ElementKind::Specialization);
        assert_eq!(rel_el.source().unwrap().as_str(), "b");
        assert_eq!(rel_el.target().unwrap().as_str(), "a");
    }

    #[test]
    fn test_add_rel_preserves_owner() {
        let mut model = Model::new();
        model.add_element(Element::new("feat", ElementKind::Feature));
        model.add_element(Element::new("typ", ElementKind::Class));

        model.add_rel(
            "rel_t",
            ElementKind::FeatureTyping,
            "feat",
            "typ",
            Some(ElementId::new("feat")),
        );

        let el = model.get(&ElementId::new("rel_t")).unwrap();
        assert_eq!(el.owner.as_ref().unwrap().as_str(), "feat");
    }

    #[test]
    fn test_add_rel_does_not_duplicate() {
        let mut model = Model::new();

        // Pre-insert an element with the same ID
        let existing = Element::new("rel_x", ElementKind::FeatureTyping).with_name("pre-existing");
        model.add_element(existing);

        // add_rel overwrites (it's an insert into IndexMap)
        model.add_rel("rel_x", ElementKind::FeatureTyping, "s", "t", None);

        let el = model.get(&ElementId::new("rel_x")).unwrap();
        // After add_rel, the element is the relationship element (overwritten)
        assert!(el.is_relationship_element());
        assert_eq!(model.relationship_count(), 1);
    }

    #[test]
    fn test_relationship_element_kinds_are_relationships() {
        let kinds = [
            ElementKind::Specialization,
            ElementKind::FeatureTyping,
            ElementKind::Subsetting,
            ElementKind::Redefinition,
            ElementKind::Membership,
            ElementKind::OwningMembership,
            ElementKind::FeatureMembership,
            ElementKind::NamespaceImport,
            ElementKind::MembershipImport,
            ElementKind::FeatureChaining,
            ElementKind::Disjoining,
            ElementKind::Dependency,
            ElementKind::Satisfaction,
            ElementKind::Verification,
        ];

        for ek in kinds {
            assert!(
                ek.is_relationship(),
                "{:?} should be a relationship ElementKind",
                ek
            );
        }
    }
}
