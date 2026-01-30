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
}

ast_node!(NamespaceBody, NAMESPACE_BODY);

impl NamespaceBody {
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.0.children().filter_map(NamespaceMember::cast)
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

// ============================================================================
// Metadata
// ============================================================================

ast_node!(MetadataUsage, METADATA_USAGE);

impl MetadataUsage {
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
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
                _ => {}
            }
        }
        None
    }

    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
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
}

// ============================================================================
// Usage
// ============================================================================

ast_node!(Usage, USAGE);

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

    /// Get the full qualified name as a string
    /// Uses '::' for namespace paths, '.' for feature chains
    pub fn to_string(&self) -> String {
        // Check if this is a feature chain (uses '.' separator) or namespace path (uses '::')
        let has_dot = self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::DOT);
        
        let separator = if has_dot { "." } else { "::" };
        self.segments().join(separator)
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
                SyntaxKind::COLON_GT_GT => return Some(SpecializationKind::Conjugates),
                SyntaxKind::COLON_COLON_GT => return Some(SpecializationKind::FeatureChain),
                SyntaxKind::SPECIALIZES_KW => return Some(SpecializationKind::Specializes),
                SyntaxKind::SUBSETS_KW => return Some(SpecializationKind::Subsets),
                SyntaxKind::REDEFINES_KW => return Some(SpecializationKind::Redefines),
                SyntaxKind::REFERENCES_KW => return Some(SpecializationKind::References),
                SyntaxKind::FROM_KW => return Some(SpecializationKind::FeatureChain),
                SyntaxKind::TO_KW => return Some(SpecializationKind::FeatureChain),
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

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

    #[test]
    fn test_message_usage_name() {
        // Test that message usages extract names correctly
        // Message usages need to be inside a package/part body
        let parsed = parse("part p { message of ignitionCmd : IgnitionCmd; }");
        let root = SourceFile::cast(parsed.syntax()).unwrap();

        let members: Vec<_> = root.members().collect();
        
        // Get the usage inside the part
        if let NamespaceMember::Usage(part_usage) = &members[0] {
            if let Some(body) = part_usage.body() {
                let inner_members: Vec<_> = body.members().collect();
                if let NamespaceMember::Usage(usage) = &inner_members[0] {
                    let name = usage.name();
                    assert!(name.is_some(), "message usage should have a name");
                    assert_eq!(name.unwrap().text(), Some("ignitionCmd".to_string()));
                    return;
                }
            }
        }
        panic!("expected Usage for part p with message inside");
    }
}