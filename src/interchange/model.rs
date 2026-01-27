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
    // Packages
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

    // Relationships (first-class)
    Membership,
    OwningMembership,
    FeatureMembership,
    Import,
    NamespaceImport,
    MembershipImport,
    Specialization,
    FeatureTyping,
    Subsetting,
    Redefinition,
    Conjugation,

    // Comments and documentation
    Comment,
    Documentation,
    TextualRepresentation,

    // Annotations
    MetadataUsage,
    AnnotatingElement,

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
                | Self::Import
                | Self::NamespaceImport
                | Self::MembershipImport
                | Self::Specialization
                | Self::FeatureTyping
                | Self::Subsetting
                | Self::Redefinition
                | Self::Conjugation
        )
    }

    /// Get the XMI type name for this kind.
    pub fn xmi_type(&self) -> &'static str {
        match self {
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
            Self::Membership => "kerml:Membership",
            Self::OwningMembership => "kerml:OwningMembership",
            Self::FeatureMembership => "kerml:FeatureMembership",
            Self::Import => "kerml:Import",
            Self::NamespaceImport => "kerml:NamespaceImport",
            Self::MembershipImport => "kerml:MembershipImport",
            Self::Specialization => "kerml:Specialization",
            Self::FeatureTyping => "kerml:FeatureTyping",
            Self::Subsetting => "kerml:Subsetting",
            Self::Redefinition => "kerml:Redefinition",
            Self::Conjugation => "kerml:Conjugation",
            Self::Comment => "kerml:Comment",
            Self::Documentation => "kerml:Documentation",
            Self::TextualRepresentation => "kerml:TextualRepresentation",
            Self::MetadataUsage => "sysml:MetadataUsage",
            Self::AnnotatingElement => "kerml:AnnotatingElement",
            Self::Other => "kerml:Element",
        }
    }

    /// Parse from XMI type name.
    pub fn from_xmi_type(xmi_type: &str) -> Self {
        // Strip namespace prefix if present
        let type_name = xmi_type
            .split(':')
            .last()
            .unwrap_or(xmi_type);

        match type_name {
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
            "Membership" => Self::Membership,
            "OwningMembership" => Self::OwningMembership,
            "FeatureMembership" => Self::FeatureMembership,
            "Import" => Self::Import,
            "NamespaceImport" => Self::NamespaceImport,
            "MembershipImport" => Self::MembershipImport,
            "Specialization" => Self::Specialization,
            "FeatureTyping" => Self::FeatureTyping,
            "Subsetting" => Self::Subsetting,
            "Redefinition" => Self::Redefinition,
            "Conjugation" => Self::Conjugation,
            "Comment" => Self::Comment,
            "Documentation" => Self::Documentation,
            "TextualRepresentation" => Self::TextualRepresentation,
            "MetadataUsage" => Self::MetadataUsage,
            "AnnotatingElement" => Self::AnnotatingElement,
            _ => Self::Other,
        }
    }

    /// Get the JSON-LD @type value.
    pub fn jsonld_type(&self) -> &'static str {
        // JSON-LD uses the same type names without namespace prefix
        self.xmi_type().split(':').last().unwrap_or("Element")
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
    /// Visibility (public, private, protected).
    pub visibility: Visibility,
    /// Additional properties as key-value pairs (IndexMap preserves order).
    pub properties: IndexMap<Arc<str>, PropertyValue>,
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
            visibility: Visibility::Public,
            properties: IndexMap::new(),
        }
    }

    /// Set the name.
    pub fn with_name(mut self, name: impl Into<Arc<str>>) -> Self {
        self.name = Some(name.into());
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
// RELATIONSHIP
// ============================================================================

/// A relationship between two elements.
///
/// In the KerML/SysML metamodel, relationships are first-class elements.
/// This struct captures the essential information for interchange.
#[derive(Clone, Debug)]
pub struct Relationship {
    /// Unique identifier for the relationship itself.
    pub id: ElementId,
    /// The kind of relationship.
    pub kind: RelationshipKind,
    /// The source element ID.
    pub source: ElementId,
    /// The target element ID.
    pub target: ElementId,
    /// The owning element (usually the source).
    pub owner: Option<ElementId>,
}

impl Relationship {
    /// Create a new relationship.
    pub fn new(
        id: impl Into<ElementId>,
        kind: RelationshipKind,
        source: impl Into<ElementId>,
        target: impl Into<ElementId>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            source: source.into(),
            target: target.into(),
            owner: None,
        }
    }
}

/// The kind of relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationshipKind {
    /// Specialization (general → specific).
    Specialization,
    /// Feature typing (feature → type).
    FeatureTyping,
    /// Subsetting (feature → subsetted feature).
    Subsetting,
    /// Redefinition (feature → redefined feature).
    Redefinition,
    /// Conjugation (conjugated port → original).
    Conjugation,
    /// Membership (namespace → member).
    Membership,
    /// Owning membership (owner → owned).
    OwningMembership,
    /// Feature membership (type → feature).
    FeatureMembership,
    /// Namespace import.
    NamespaceImport,
    /// Membership import.
    MembershipImport,
    /// Dependency.
    Dependency,
    /// Requirement satisfaction.
    Satisfaction,
    /// Requirement verification.
    Verification,
    /// Allocation.
    Allocation,
    /// Connection.
    Connection,
    /// Flow connection.
    FlowConnection,
    /// Succession.
    Succession,
}

impl RelationshipKind {
    /// Get the XMI type name.
    pub fn xmi_type(&self) -> &'static str {
        match self {
            Self::Specialization => "kerml:Specialization",
            Self::FeatureTyping => "kerml:FeatureTyping",
            Self::Subsetting => "kerml:Subsetting",
            Self::Redefinition => "kerml:Redefinition",
            Self::Conjugation => "kerml:Conjugation",
            Self::Membership => "kerml:Membership",
            Self::OwningMembership => "kerml:OwningMembership",
            Self::FeatureMembership => "kerml:FeatureMembership",
            Self::NamespaceImport => "kerml:NamespaceImport",
            Self::MembershipImport => "kerml:MembershipImport",
            Self::Dependency => "kerml:Dependency",
            Self::Satisfaction => "sysml:SatisfyRequirementUsage",
            Self::Verification => "sysml:RequirementVerificationMembership",
            Self::Allocation => "sysml:AllocationUsage",
            Self::Connection => "sysml:ConnectionUsage",
            Self::FlowConnection => "sysml:FlowConnectionUsage",
            Self::Succession => "sysml:SuccessionAsUsage",
        }
    }
}

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
    /// All relationships.
    pub relationships: Vec<Relationship>,
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

    /// Add a relationship to the model.
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationships.push(relationship);
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

    /// Get relationships where the given element is the source.
    pub fn relationships_from<'a>(&'a self, source: &'a ElementId) -> impl Iterator<Item = &'a Relationship> {
        self.relationships.iter().filter(move |r| &r.source == source)
    }

    /// Get relationships where the given element is the target.
    pub fn relationships_to<'a>(&'a self, target: &'a ElementId) -> impl Iterator<Item = &'a Relationship> {
        self.relationships.iter().filter(move |r| &r.target == target)
    }

    /// Get the number of elements.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Get the number of relationships.
    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
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
        assert_eq!(model.get(&ElementId::new("pkg1")).unwrap().name.as_deref(), Some("Root"));
    }

    #[test]
    fn test_model_relationships() {
        let mut model = Model::new();

        model.add_element(Element::new("def1", ElementKind::PartDefinition).with_name("Base"));
        model.add_element(Element::new("def2", ElementKind::PartDefinition).with_name("Derived"));

        model.add_relationship(Relationship::new(
            "rel1",
            RelationshipKind::Specialization,
            "def2",
            "def1",
        ));

        assert_eq!(model.relationship_count(), 1);
        let source_id = ElementId::new("def2");
        let rels: Vec<_> = model.relationships_from(&source_id).collect();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].target.as_str(), "def1");
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
}
