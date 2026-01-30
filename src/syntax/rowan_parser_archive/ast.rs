//! Typed AST wrappers over the untyped rowan CST.
//!
//! This module provides strongly-typed accessors for SysML syntax nodes.
//! Each struct wraps a SyntaxNode and provides methods to access children.

use super::syntax_kind::SyntaxKind;
use super::{SyntaxNode, SyntaxToken};

/// Trait for AST nodes that wrap a SyntaxNode
pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

/// Trait for AST tokens that wrap a SyntaxToken
pub trait AstToken: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(token: SyntaxToken) -> Option<Self>;
    fn syntax(&self) -> &SyntaxToken;
    fn text(&self) -> &str {
        self.syntax().text()
    }
}

// ============================================================================
// Helper macros
// ============================================================================

macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(node: SyntaxNode) -> Option<Self> {
                if Self::can_cast(node.kind()) {
                    Some(Self(node))
                } else {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

// ============================================================================
// Root
// ============================================================================

ast_node!(SourceFile, SOURCE_FILE);

impl SourceFile {
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.0.children().filter_map(NamespaceMember::cast)
    }
}

// ============================================================================
// Namespace Members
// ============================================================================

/// Any member of a namespace (package, definition, usage, import, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NamespaceMember {
    Package(Package),
    LibraryPackage(LibraryPackage),
    Import(Import),
    Alias(Alias),
    Dependency(Dependency),
    Definition(Definition),
    Usage(Usage),
    Filter(ElementFilter),
    Metadata(MetadataUsage),
    Comment(Comment),
}

impl AstNode for NamespaceMember {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::PACKAGE
                | SyntaxKind::LIBRARY_PACKAGE
                | SyntaxKind::IMPORT
                | SyntaxKind::ALIAS_MEMBER
                | SyntaxKind::DEPENDENCY
                | SyntaxKind::DEFINITION
                | SyntaxKind::USAGE
                | SyntaxKind::ELEMENT_FILTER_MEMBER
                | SyntaxKind::METADATA_USAGE
                | SyntaxKind::COMMENT_ELEMENT
        )
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::PACKAGE => Some(Self::Package(Package(node))),
            SyntaxKind::LIBRARY_PACKAGE => Some(Self::LibraryPackage(LibraryPackage(node))),
            SyntaxKind::IMPORT => Some(Self::Import(Import(node))),
            SyntaxKind::ALIAS_MEMBER => Some(Self::Alias(Alias(node))),
            SyntaxKind::DEPENDENCY => Some(Self::Dependency(Dependency(node))),
            SyntaxKind::DEFINITION => Some(Self::Definition(Definition(node))),
            SyntaxKind::USAGE => Some(Self::Usage(Usage(node))),
            SyntaxKind::ELEMENT_FILTER_MEMBER => Some(Self::Filter(ElementFilter(node))),
            SyntaxKind::METADATA_USAGE => Some(Self::Metadata(MetadataUsage(node))),
            SyntaxKind::COMMENT_ELEMENT => Some(Self::Comment(Comment(node))),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Package(n) => n.syntax(),
            Self::LibraryPackage(n) => n.syntax(),
            Self::Import(n) => n.syntax(),
            Self::Alias(n) => n.syntax(),
            Self::Dependency(n) => n.syntax(),
            Self::Definition(n) => n.syntax(),
            Self::Usage(n) => n.syntax(),
            Self::Filter(n) => n.syntax(),
            Self::Metadata(n) => n.syntax(),
            Self::Comment(n) => n.syntax(),
        }
    }
}

// ============================================================================
// Package
// ============================================================================

ast_node!(Package, PACKAGE);

impl Package {
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }

    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.body()
            .into_iter()
            .flat_map(|body| body.members().collect::<Vec<_>>())
    }
}

ast_node!(LibraryPackage, LIBRARY_PACKAGE);

impl LibraryPackage {
    pub fn is_standard(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::STANDARD_KW)
    }

    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }

    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.body()
            .into_iter()
            .flat_map(|body| body.members().collect::<Vec<_>>())
    }
}

ast_node!(NamespaceBody, NAMESPACE_BODY);

impl NamespaceBody {
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.0.children().filter_map(NamespaceMember::cast)
    }
    
    /// Get send/accept action usages (searches descendants, not just children)
    pub fn send_accept_actions(&self) -> impl Iterator<Item = Usage> + '_ {
        self.0.descendants()
            .filter(|n| {
                n.kind() == SyntaxKind::SEND_ACTION_USAGE || n.kind() == SyntaxKind::ACCEPT_ACTION_USAGE
            })
            .filter_map(Usage::cast)
    }
}

// ============================================================================
// Import
// ============================================================================

ast_node!(Import, IMPORT);

impl Import {
    /// Check if this is an 'import all'
    pub fn is_all(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ALL_KW)
    }

    /// Get the qualified name being imported
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Check if this is a wildcard import (::*)
    pub fn is_wildcard(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::STAR)
    }

    /// Check if this is a recursive import (::**)
    pub fn is_recursive(&self) -> bool {
        // Check for STAR_STAR token (lexed as single token)
        // or two consecutive STAR tokens
        let has_star_star = self
            .0
            .descendants_with_tokens()
            .filter_map(|e| match e { rowan::NodeOrToken::Token(t) => Some(t), _ => None })
            .any(|t| t.kind() == SyntaxKind::STAR_STAR);
        
        if has_star_star {
            return true;
        }
        
        // Fallback: count individual stars
        let stars: Vec<_> = self
            .0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::STAR)
            .collect();
        stars.len() >= 2
    }

    /// Get the filter package if present
    pub fn filter(&self) -> Option<FilterPackage> {
        self.0.children().find_map(FilterPackage::cast)
    }
}

ast_node!(FilterPackage, FILTER_PACKAGE);

impl FilterPackage {
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

// ============================================================================
// Alias
// ============================================================================

ast_node!(Alias, ALIAS_MEMBER);

impl Alias {
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

// ============================================================================
// Dependency
// ============================================================================

ast_node!(Dependency, DEPENDENCY);

// ============================================================================
// Filter
// ============================================================================

ast_node!(ElementFilter, ELEMENT_FILTER_MEMBER);

impl ElementFilter {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }
}

// ============================================================================
// Comment
// ============================================================================

ast_node!(Comment, COMMENT_ELEMENT);

impl Comment {
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }
    
    pub fn about_target(&self) -> Option<QualifiedName> {
        // Find the qualified name after the 'about' keyword
        let mut found_about = false;
        for child in self.0.children_with_tokens() {
            if let Some(token) = child.as_token() {
                if token.kind() == SyntaxKind::ABOUT_KW {
                    found_about = true;
                }
            }
            if found_about {
                if let Some(node) = child.as_node() {
                    if let Some(qname) = QualifiedName::cast(node.clone()) {
                        return Some(qname);
                    }
                }
            }
        }
        None
    }
}

// ============================================================================
// Metadata
// ============================================================================

ast_node!(MetadataUsage, METADATA_USAGE);

impl MetadataUsage {
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
    
    /// Check if this metadata usage has a references operator (::>)
    pub fn has_references(&self) -> bool {
        self.0.children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::COLON_COLON_GT)
    }
    
    /// Get all qualified names within this metadata body
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.descendants().filter_map(QualifiedName::cast)
    }
}

// ============================================================================
// Definition
// ============================================================================

ast_node!(Definition, DEFINITION);

impl Definition {
    pub fn is_abstract(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ABSTRACT_KW)
    }

    pub fn is_variation(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::VARIATION_KW)
    }
    
    pub fn is_public(&self) -> bool {
        // Check if preceded by PUBLIC_KW (visibility is parsed before the node)
        self.0
            .siblings_with_tokens(rowan::Direction::Prev)
            .filter_map(|e| e.into_token())
            .take(5)
            .any(|t| t.kind() == SyntaxKind::PUBLIC_KW)
    }

    pub fn definition_kind(&self) -> Option<DefinitionKind> {
        for token in self.0.children_with_tokens().filter_map(|e| e.into_token()) {
            match token.kind() {
                SyntaxKind::PART_KW => return Some(DefinitionKind::Part),
                SyntaxKind::ATTRIBUTE_KW => return Some(DefinitionKind::Attribute),
                SyntaxKind::PORT_KW => return Some(DefinitionKind::Port),
                SyntaxKind::ITEM_KW => return Some(DefinitionKind::Item),
                SyntaxKind::ACTION_KW => return Some(DefinitionKind::Action),
                SyntaxKind::STATE_KW => return Some(DefinitionKind::State),
                SyntaxKind::CONSTRAINT_KW => return Some(DefinitionKind::Constraint),
                SyntaxKind::REQUIREMENT_KW => return Some(DefinitionKind::Requirement),
                SyntaxKind::CASE_KW => return Some(DefinitionKind::Case),
                SyntaxKind::CALC_KW => return Some(DefinitionKind::Calc),
                SyntaxKind::CONNECTION_KW => return Some(DefinitionKind::Connection),
                SyntaxKind::INTERFACE_KW => return Some(DefinitionKind::Interface),
                SyntaxKind::ALLOCATION_KW => return Some(DefinitionKind::Allocation),
                SyntaxKind::FLOW_KW => return Some(DefinitionKind::Flow),
                SyntaxKind::VIEW_KW => return Some(DefinitionKind::View),
                SyntaxKind::VIEWPOINT_KW => return Some(DefinitionKind::Viewpoint),
                SyntaxKind::RENDERING_KW => return Some(DefinitionKind::Rendering),
                SyntaxKind::METADATA_KW => return Some(DefinitionKind::Metadata),
                SyntaxKind::OCCURRENCE_KW => return Some(DefinitionKind::Occurrence),
                SyntaxKind::ENUM_KW => return Some(DefinitionKind::Enum),
                SyntaxKind::ANALYSIS_KW => return Some(DefinitionKind::Analysis),
                SyntaxKind::VERIFICATION_KW => return Some(DefinitionKind::Verification),
                SyntaxKind::USE_KW => return Some(DefinitionKind::UseCase),
                SyntaxKind::CONCERN_KW => return Some(DefinitionKind::Concern),
                SyntaxKind::INDIVIDUAL_KW => return Some(DefinitionKind::Individual),
                // KerML-specific types
                SyntaxKind::CLASSIFIER_KW => return Some(DefinitionKind::Classifier),
                SyntaxKind::CLASS_KW => return Some(DefinitionKind::Class),
                SyntaxKind::STRUCT_KW => return Some(DefinitionKind::Struct),
                SyntaxKind::DATATYPE_KW => return Some(DefinitionKind::DataType),
                SyntaxKind::BEHAVIOR_KW => return Some(DefinitionKind::Behavior),
                SyntaxKind::FUNCTION_KW => return Some(DefinitionKind::Function),
                SyntaxKind::PREDICATE_KW => return Some(DefinitionKind::Predicate),
                SyntaxKind::METACLASS_KW => return Some(DefinitionKind::Metaclass),
                SyntaxKind::INTERACTION_KW => return Some(DefinitionKind::Interaction),
                _ => {}
            }
        }
        None
    }

    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }

    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
    
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.body()
            .into_iter()
            .flat_map(|body| body.members().collect::<Vec<_>>())
    }
    
    /// Get all prefix metadata annotations (like #annotation)
    pub fn prefix_metadata(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0.descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::PREFIX_METADATA)
    }
    
    /// Get all metadata usage nodes (like @Meta { ... })
    pub fn metadata_usages(&self) -> impl Iterator<Item = MetadataUsage> + '_ {
        self.0.descendants().filter_map(MetadataUsage::cast)
    }
    
    /// Get all expressions within this definition
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        self.0.descendants().filter_map(Expression::cast)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    Part,
    Attribute,
    Port,
    Item,
    Action,
    State,
    Constraint,
    Requirement,
    Case,
    Calc,
    Connection,
    Interface,
    Allocation,
    Flow,
    View,
    Viewpoint,
    Rendering,
    Metadata,
    Occurrence,
    Enum,
    Analysis,
    Verification,
    UseCase,
    Concern,
    Individual,
    // KerML-specific types
    Classifier,
    Class,
    Struct,
    DataType,
    Behavior,
    Function,
    Predicate,
    Metaclass,
    Interaction,
}

// ============================================================================
// Usage
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Usage(SyntaxNode);

impl AstNode for Usage {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::USAGE | SyntaxKind::SEND_ACTION_USAGE | SyntaxKind::ACCEPT_ACTION_USAGE)
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        if Self::can_cast(node.kind()) {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Usage {
    pub fn is_ref(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::REF_KW)
    }

    pub fn is_readonly(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::READONLY_KW)
    }

    pub fn is_derived(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::DERIVED_KW)
    }
    
    pub fn is_public(&self) -> bool {
        // Check if preceded by PUBLIC_KW (visibility is parsed before the node)
        self.0
            .siblings_with_tokens(rowan::Direction::Prev)
            .filter_map(|e| e.into_token())
            .take(5)
            .any(|t| t.kind() == SyntaxKind::PUBLIC_KW)
    }

    pub fn is_variation(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::VARIATION_KW)
    }

    pub fn usage_kind(&self) -> Option<UsageKind> {
        for token in self.0.children_with_tokens().filter_map(|e| e.into_token()) {
            match token.kind() {
                SyntaxKind::PART_KW => return Some(UsageKind::Part),
                SyntaxKind::ATTRIBUTE_KW => return Some(UsageKind::Attribute),
                SyntaxKind::PORT_KW => return Some(UsageKind::Port),
                SyntaxKind::ITEM_KW => return Some(UsageKind::Item),
                SyntaxKind::ACTION_KW => return Some(UsageKind::Action),
                SyntaxKind::STATE_KW => return Some(UsageKind::State),
                SyntaxKind::CONSTRAINT_KW => return Some(UsageKind::Constraint),
                SyntaxKind::REQUIREMENT_KW => return Some(UsageKind::Requirement),
                SyntaxKind::CASE_KW => return Some(UsageKind::Case),
                SyntaxKind::CALC_KW => return Some(UsageKind::Calc),
                SyntaxKind::CONNECTION_KW => return Some(UsageKind::Connection),
                SyntaxKind::INTERFACE_KW => return Some(UsageKind::Interface),
                SyntaxKind::ALLOCATION_KW => return Some(UsageKind::Allocation),
                SyntaxKind::FLOW_KW => return Some(UsageKind::Flow),
                SyntaxKind::VIEW_KW => return Some(UsageKind::View),
                SyntaxKind::VIEWPOINT_KW => return Some(UsageKind::Viewpoint),
                SyntaxKind::RENDERING_KW => return Some(UsageKind::Rendering),
                SyntaxKind::OCCURRENCE_KW => return Some(UsageKind::Occurrence),
                SyntaxKind::ENUM_KW => return Some(UsageKind::Enum),
                SyntaxKind::CONCERN_KW => return Some(UsageKind::Concern),
                SyntaxKind::REF_KW => return Some(UsageKind::Ref),
                SyntaxKind::INDIVIDUAL_KW => return Some(UsageKind::Individual),
                // MESSAGE_KW and EVENT_KW not in lexer yet
                _ => {}
            }
        }
        None
    }

    pub fn direction(&self) -> Option<Direction> {
        for token in self.0.children_with_tokens().filter_map(|e| e.into_token()) {
            match token.kind() {
                SyntaxKind::IN_KW => return Some(Direction::In),
                SyntaxKind::OUT_KW => return Some(Direction::Out),
                SyntaxKind::INOUT_KW => return Some(Direction::InOut),
                _ => {}
            }
        }
        None
    }

    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }

    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
    
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.body()
            .into_iter()
            .flat_map(|body| body.members().collect::<Vec<_>>())
    }
    
    /// Get all nested usages, including send/accept actions that may not be in members enum
    pub fn nested_usages(&self) -> impl Iterator<Item = Usage> + '_ {
        self.0.descendants().filter_map(Usage::cast)
    }
    
    /// Get all qualified names within this usage (e.g., for typing, expressions)
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.descendants().filter_map(QualifiedName::cast)
    }
    
    /// Get all prefix metadata annotations (like #annotation)
    pub fn prefix_metadata(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0.descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::PREFIX_METADATA)
    }
    
    /// Get all metadata usage nodes (like @Meta { ... })
    pub fn metadata_usages(&self) -> impl Iterator<Item = MetadataUsage> + '_ {
        self.0.descendants().filter_map(MetadataUsage::cast)
    }
    
    /// Get all expressions within this usage
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        self.0.descendants().filter_map(Expression::cast)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    Part,
    Attribute,
    Port,
    Item,
    Action,
    State,
    Constraint,
    Requirement,
    Case,
    Calc,
    Connection,
    Interface,
    Allocation,
    Flow,
    View,
    Viewpoint,
    Rendering,
    Occurrence,
    Enum,
    Concern,
    Ref,
    Individual,
    Message,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
    InOut,
}

// ============================================================================
// Names
// ============================================================================

ast_node!(Name, NAME);

impl Name {
    pub fn short_name(&self) -> Option<ShortName> {
        self.0.children().find_map(ShortName::cast)
    }

    pub fn text(&self) -> Option<String> {
        // Get the identifier token
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
    }
    
    /// Get the name as Arc<str> for efficient storage
    pub fn text_arc(&self) -> Option<std::sync::Arc<str>> {
        self.text().map(|s| std::sync::Arc::from(s.as_str()))
    }

    /// Get the text range of the identifier token
    pub fn ident_range(&self) -> Option<text_size::TextRange> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text_range())
    }
    
    /// Get short name text if present
    pub fn short_text(&self) -> Option<String> {
        self.short_name().and_then(|s| s.text())
    }
    
    /// Get short name text range if present
    pub fn short_ident_range(&self) -> Option<text_size::TextRange> {
        self.short_name().and_then(|s| s.ident_range())
    }
}

ast_node!(ShortName, SHORT_NAME);

impl ShortName {
    pub fn text(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
    }
    
    /// Get the text range of the identifier token  
    pub fn ident_range(&self) -> Option<text_size::TextRange> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text_range())
    }
}

ast_node!(QualifiedName, QUALIFIED_NAME);

impl QualifiedName {
    /// Get all name segments
    pub fn segments(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
            .collect()
    }

    /// Get the full qualified name as a string (joined with ::)
    pub fn text(&self) -> String {
        self.segments().join("::")
    }
    
    /// Get identifier tokens with their text ranges for precise location tracking
    pub fn ident_tokens(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
    }
}

// ============================================================================
// Typing and Specialization
// ============================================================================

ast_node!(Typing, TYPING);

impl Typing {
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

ast_node!(Specialization, SPECIALIZATION);

impl Specialization {
    pub fn kind(&self) -> Option<SpecializationKind> {
        for token in self.0.children_with_tokens().filter_map(|e| e.into_token()) {
            match token.kind() {
                SyntaxKind::COLON_GT => return Some(SpecializationKind::Specializes),
                SyntaxKind::COLON_GT_GT => return Some(SpecializationKind::Redefines), // :>> is redefines in SysML
                SyntaxKind::COLON_COLON_GT => return Some(SpecializationKind::FeatureChain),
                SyntaxKind::SPECIALIZES_KW => return Some(SpecializationKind::Specializes),
                SyntaxKind::SUBSETS_KW => return Some(SpecializationKind::Subsets),
                SyntaxKind::REDEFINES_KW => return Some(SpecializationKind::Redefines),
                SyntaxKind::REFERENCES_KW => return Some(SpecializationKind::References),
                _ => {}
            }
        }
        None
    }

    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializationKind {
    Specializes,
    Subsets,
    Redefines,
    References,
    Conjugates,
    FeatureChain,
}

// ============================================================================
// Expression
// ============================================================================

ast_node!(Expression, EXPRESSION);

impl Expression {
    /// Get all qualified names within this expression
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.descendants().filter_map(QualifiedName::cast)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::rowan_parser::parse;

    #[test]
    fn test_ast_package() {
        let parsed = parse("package Test;");
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        assert_eq!(members.len(), 1);

        if let NamespaceMember::Package(pkg) = &members[0] {
            let name = pkg.name().unwrap();
            assert_eq!(name.text(), Some("Test".to_string()));
        } else {
            panic!("expected Package");
        }
    }

    #[test]
    fn test_ast_import() {
        let parsed = parse("import ISQ::*;");
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        assert_eq!(members.len(), 1);

        if let NamespaceMember::Import(imp) = &members[0] {
            assert!(!imp.is_all());
            assert!(imp.is_wildcard());
            assert!(!imp.is_recursive());
            let target = imp.target().unwrap();
            assert_eq!(target.segments(), vec!["ISQ"]);
        } else {
            panic!("expected Import");
        }
    }

    #[test]
    fn test_ast_import_recursive() {
        let parsed = parse("import all Library::**;");
        assert!(parsed.ok(), "errors: {:?}", parsed.errors);
        
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        if let NamespaceMember::Import(imp) = &members[0] {
            assert!(imp.is_all());
            assert!(imp.is_recursive());
        } else {
            panic!("expected Import");
        }
    }

    #[test]
    fn test_ast_definition() {
        let parsed = parse("abstract part def Vehicle :> Base;");
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        if let NamespaceMember::Definition(def) = &members[0] {
            assert!(def.is_abstract());
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Part));
            let name = def.name().unwrap();
            assert_eq!(name.text(), Some("Vehicle".to_string()));

            let specializations: Vec<_> = def.specializations().collect();
            assert_eq!(specializations.len(), 1);
            assert_eq!(
                specializations[0].kind(),
                Some(SpecializationKind::Specializes)
            );
        } else {
            panic!("expected Definition");
        }
    }

    #[test]
    fn test_ast_usage() {
        let parsed = parse("ref part engine : Engine;");
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        if let NamespaceMember::Usage(usage) = &members[0] {
            assert!(usage.is_ref());
            let name = usage.name().unwrap();
            assert_eq!(name.text(), Some("engine".to_string()));

            let typing = usage.typing().unwrap();
            let target = typing.target().unwrap();
            assert_eq!(target.segments(), vec!["Engine"]);
        } else {
            panic!("expected Usage");
        }
    }
}
