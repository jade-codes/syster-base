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

    /// Find all descendant nodes of a specific AST type
    fn descendants<T: AstNode>(&self) -> impl Iterator<Item = T> {
        self.syntax().descendants().filter_map(T::cast)
    }

    /// Extract doc comment preceding this node.
    /// Looks for block comments (`/* ... */`) or consecutive line comments (`// ...`)
    /// immediately preceding the node (separated only by whitespace).
    fn doc_comment(&self) -> Option<String> {
        extract_doc_comment(self.syntax())
    }
}

/// Extract doc comment from preceding trivia or COMMENT_ELEMENT of a syntax node.
///
/// SysML supports two forms of documentation:
/// 1. `doc /* content */` - formal SysML documentation (COMMENT_ELEMENT nodes)
/// 2. Regular comments `/* ... */` or `// ...` - informal doc comments (trivia)
pub fn extract_doc_comment(node: &SyntaxNode) -> Option<String> {
    let mut comments = Vec::new();
    let mut current = node.prev_sibling_or_token();

    while let Some(node_or_token) = current {
        match node_or_token {
            rowan::NodeOrToken::Token(ref t) => {
                match t.kind() {
                    SyntaxKind::WHITESPACE => {
                        // Allow whitespace, continue looking
                        current = t.prev_sibling_or_token();
                    }
                    SyntaxKind::BLOCK_COMMENT => {
                        // Block comment - use as doc
                        let text = t.text();
                        // Strip /* and */ and trim
                        let content = text
                            .strip_prefix("/*")
                            .and_then(|s| s.strip_suffix("*/"))
                            .map(clean_doc_comment)
                            .unwrap_or_default();
                        if !content.is_empty() {
                            comments.push(content);
                        }
                        // Block comment found, stop looking
                        break;
                    }
                    SyntaxKind::LINE_COMMENT => {
                        // Line comment - collect consecutive ones
                        let text = t.text();
                        // Strip // and trim
                        let content = text.strip_prefix("//").unwrap_or(text).trim();
                        if !content.is_empty() {
                            comments.push(content.to_string());
                        }
                        current = t.prev_sibling_or_token();
                    }
                    _ => break, // Any other token stops the search
                }
            }
            rowan::NodeOrToken::Node(ref n) => {
                // Check for COMMENT_ELEMENT (SysML `doc /* ... */` syntax)
                if n.kind() == SyntaxKind::COMMENT_ELEMENT {
                    // Extract the content from the COMMENT_ELEMENT
                    // The structure is: doc /* content */
                    // We need to find the BLOCK_COMMENT token inside
                    for child in n.children_with_tokens() {
                        if let rowan::NodeOrToken::Token(t) = child {
                            if t.kind() == SyntaxKind::BLOCK_COMMENT {
                                let text = t.text();
                                let content = text
                                    .strip_prefix("/*")
                                    .and_then(|s| s.strip_suffix("*/"))
                                    .map(clean_doc_comment)
                                    .unwrap_or_default();
                                if !content.is_empty() {
                                    comments.push(content);
                                }
                                break;
                            }
                        }
                    }
                    // Found doc element, stop looking
                    break;
                } else {
                    // Another node that's not a comment stops the search
                    break;
                }
            }
        }
    }

    if comments.is_empty() {
        return None;
    }

    // Reverse because we collected bottom-up
    comments.reverse();
    Some(comments.join("\n"))
}

/// Clean up doc comment content by removing leading asterisks and normalizing whitespace.
fn clean_doc_comment(s: &str) -> String {
    s.lines()
        .map(|line| {
            let trimmed = line.trim();
            // Remove leading * from doc comment lines
            if let Some(rest) = trimmed.strip_prefix('*') {
                rest.trim_start().to_string()
            } else {
                trimmed.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
    /// Standalone bind statement (e.g., `bind p1 = p2;`)
    Bind(BindingConnector),
    /// Standalone succession (e.g., `first a then b;`)
    Succession(Succession),
    /// Transition usage (e.g., `accept sig : Signal then running;`)
    Transition(TransitionUsage),
    /// KerML connector (e.g., `connector link;`)
    Connector(Connector),
    /// Connect usage (e.g., `connect p ::> a to b;`)
    ConnectUsage(ConnectUsage),
    /// Send action usage (e.g., `send x via port`)
    SendAction(SendActionUsage),
    /// Accept action usage (e.g., `accept e : Signal via port`)
    AcceptAction(AcceptActionUsage),
    /// State subaction (e.g., `entry action initial;`)
    StateSubaction(StateSubaction),
    /// Control node (fork, join, merge, decide)
    ControlNode(ControlNode),
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
                | SyntaxKind::SUBJECT_USAGE
                | SyntaxKind::ACTOR_USAGE
                | SyntaxKind::STAKEHOLDER_USAGE
                | SyntaxKind::OBJECTIVE_USAGE
                | SyntaxKind::ELEMENT_FILTER_MEMBER
                | SyntaxKind::METADATA_USAGE
                | SyntaxKind::COMMENT_ELEMENT
                | SyntaxKind::BINDING_CONNECTOR
                | SyntaxKind::SUCCESSION
                | SyntaxKind::TRANSITION_USAGE
                | SyntaxKind::CONNECTOR
                | SyntaxKind::CONNECT_USAGE
                | SyntaxKind::SEND_ACTION_USAGE
                | SyntaxKind::ACCEPT_ACTION_USAGE
                | SyntaxKind::STATE_SUBACTION
                | SyntaxKind::CONTROL_NODE
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
            SyntaxKind::USAGE
            | SyntaxKind::SUBJECT_USAGE
            | SyntaxKind::ACTOR_USAGE
            | SyntaxKind::STAKEHOLDER_USAGE
            | SyntaxKind::OBJECTIVE_USAGE => Some(Self::Usage(Usage(node))),
            SyntaxKind::ELEMENT_FILTER_MEMBER => Some(Self::Filter(ElementFilter(node))),
            SyntaxKind::METADATA_USAGE => Some(Self::Metadata(MetadataUsage(node))),
            SyntaxKind::COMMENT_ELEMENT => Some(Self::Comment(Comment(node))),
            SyntaxKind::BINDING_CONNECTOR => Some(Self::Bind(BindingConnector(node))),
            SyntaxKind::SUCCESSION => Some(Self::Succession(Succession(node))),
            SyntaxKind::TRANSITION_USAGE => Some(Self::Transition(TransitionUsage(node))),
            SyntaxKind::CONNECTOR => Some(Self::Connector(Connector(node))),
            SyntaxKind::CONNECT_USAGE => Some(Self::ConnectUsage(ConnectUsage(node))),
            SyntaxKind::SEND_ACTION_USAGE => Some(Self::SendAction(SendActionUsage(node))),
            SyntaxKind::ACCEPT_ACTION_USAGE => Some(Self::AcceptAction(AcceptActionUsage(node))),
            SyntaxKind::STATE_SUBACTION => Some(Self::StateSubaction(StateSubaction(node))),
            SyntaxKind::CONTROL_NODE => Some(Self::ControlNode(ControlNode(node))),
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
            Self::Bind(n) => n.syntax(),
            Self::Succession(n) => n.syntax(),
            Self::Transition(n) => n.syntax(),
            Self::Connector(n) => n.syntax(),
            Self::ConnectUsage(n) => n.syntax(),
            Self::SendAction(n) => n.syntax(),
            Self::AcceptAction(n) => n.syntax(),
            Self::StateSubaction(n) => n.syntax(),
            Self::ControlNode(n) => n.syntax(),
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
        self.0.children().flat_map(|child| {
            // STATE_SUBACTION is a container for entry/do/exit actions
            // We need to look inside for nested members (ACCEPT_ACTION_USAGE, SEND_ACTION_USAGE)
            // as well as try casting the child itself
            if child.kind() == SyntaxKind::STATE_SUBACTION {
                // First try direct children of STATE_SUBACTION
                let nested: Vec<NamespaceMember> =
                    child.children().filter_map(NamespaceMember::cast).collect();
                if nested.is_empty() {
                    // If no nested namespace members, wrap the STATE_SUBACTION itself as a StateSubaction
                    StateSubaction::cast(child)
                        .map(NamespaceMember::StateSubaction)
                        .into_iter()
                        .collect::<Vec<_>>()
                } else {
                    nested
                }
            } else {
                NamespaceMember::cast(child).into_iter().collect()
            }
        })
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
            .filter_map(|e| match e {
                rowan::NodeOrToken::Token(t) => Some(t),
                _ => None,
            })
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

    /// Check if this is a public import
    pub fn is_public(&self) -> bool {
        // PUBLIC_KW may be a sibling (before the IMPORT node) rather than a child
        // Check both inside and before
        let has_inside = self
            .0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::PUBLIC_KW);

        if has_inside {
            return true;
        }

        // Check previous sibling
        if let Some(prev) = self.0.prev_sibling_or_token() {
            // Skip whitespace
            let mut current = Some(prev);
            while let Some(node_or_token) = current {
                match node_or_token {
                    rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::PUBLIC_KW => {
                        return true;
                    }
                    rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::WHITESPACE => {
                        current = t.prev_sibling_or_token();
                    }
                    _ => break,
                }
            }
        }

        false
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

    /// Get all filter targets (for multiple filters like [@A][@B])
    pub fn targets(&self) -> Vec<QualifiedName> {
        self.0.children().filter_map(QualifiedName::cast).collect()
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

impl Dependency {
    /// Get all qualified names (sources and target)
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.children().filter_map(QualifiedName::cast)
    }

    /// Get the source qualified name(s) - everything before "to"
    /// For `dependency a, b to c` returns [a, b]
    pub fn sources(&self) -> Vec<QualifiedName> {
        let mut sources = Vec::new();
        let mut found_to = false;

        for elem in self.0.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                if token.kind() == SyntaxKind::TO_KW {
                    found_to = true;
                }
            } else if let Some(node) = elem.as_node() {
                if !found_to {
                    if let Some(qn) = QualifiedName::cast(node.clone()) {
                        sources.push(qn);
                    }
                }
            }
        }
        sources
    }

    /// Get the target qualified name - after "to"
    /// For `dependency a to c` returns c
    pub fn target(&self) -> Option<QualifiedName> {
        let mut found_to = false;

        for elem in self.0.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                if token.kind() == SyntaxKind::TO_KW {
                    found_to = true;
                }
            } else if let Some(node) = elem.as_node() {
                if found_to {
                    if let Some(qn) = QualifiedName::cast(node.clone()) {
                        return Some(qn);
                    }
                }
            }
        }
        None
    }

    /// Get prefix metadata references from preceding siblings.
    /// e.g., `#refinement dependency a to b;` -> returns [PrefixMetadata for "refinement"]
    pub fn prefix_metadata(&self) -> Vec<PrefixMetadata> {
        let mut result = Vec::new();
        let mut current = self.0.prev_sibling();
        while let Some(sibling) = current {
            if sibling.kind() == SyntaxKind::PREFIX_METADATA {
                if let Some(pm) = PrefixMetadata::cast(sibling.clone()) {
                    result.push(pm);
                }
                current = sibling.prev_sibling();
            } else {
                break;
            }
        }
        result.reverse();
        result
    }
}

// ============================================================================
// Filter
// ============================================================================

ast_node!(ElementFilter, ELEMENT_FILTER_MEMBER);

impl ElementFilter {
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Extract metadata references from the filter expression.
    /// For `filter @Safety;` returns ["Safety"]
    pub fn metadata_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        if let Some(expr) = self.expression() {
            // Walk the expression looking for @ followed by QUALIFIED_NAME
            let mut at_seen = false;
            for child in expr.syntax().children_with_tokens() {
                match child {
                    rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::AT => {
                        at_seen = true;
                    }
                    rowan::NodeOrToken::Node(n)
                        if at_seen && n.kind() == SyntaxKind::QUALIFIED_NAME =>
                    {
                        if let Some(qn) = QualifiedName::cast(n) {
                            refs.push(qn.to_string());
                        }
                        at_seen = false;
                    }
                    _ => {}
                }
            }
        }
        refs
    }

    /// Extract ALL qualified name references from the filter expression with their ranges.
    /// This includes both @-prefixed metadata refs and feature refs like `Safety::isMandatory`.
    /// Returns (name, range) pairs for IDE features (hover, go-to-def).
    pub fn all_qualified_refs(&self) -> Vec<(String, rowan::TextRange)> {
        let mut refs = Vec::new();
        if let Some(expr) = self.expression() {
            // Use descendants() to walk the entire tree, not just direct children
            for node in expr.syntax().descendants() {
                if node.kind() == SyntaxKind::QUALIFIED_NAME {
                    if let Some(qn) = QualifiedName::cast(node.clone()) {
                        refs.push((qn.to_string(), node.text_range()));
                    }
                }
            }
        }
        refs
    }
}

// ============================================================================
// Comment
// ============================================================================

ast_node!(Comment, COMMENT_ELEMENT);

impl Comment {
    /// Get the comment name if present
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the about target(s) - references after the 'about' keyword
    pub fn about_targets(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.children().filter_map(QualifiedName::cast)
    }

    /// Check if this comment has an about clause
    pub fn has_about(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ABOUT_KW)
    }
}

// ============================================================================
// Metadata
// ============================================================================

ast_node!(MetadataUsage, METADATA_USAGE);

impl MetadataUsage {
    /// Get the metadata type target (e.g., `Rationale` in `@Rationale about ...`)
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Get the about target(s) - references after the 'about' keyword
    /// e.g., `@Rationale about vehicle::engine` returns [vehicle::engine]
    pub fn about_targets(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        // Skip the first QualifiedName (which is the metadata type)
        // All subsequent QualifiedNames are about targets
        self.0.children().filter_map(QualifiedName::cast).skip(1)
    }

    /// Check if this metadata has an about clause
    pub fn has_about(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ABOUT_KW)
    }

    /// Get the body of the metadata (for nested metadata definitions)
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
}

// ============================================================================
// Prefix Metadata (#name)
// ============================================================================

ast_node!(PrefixMetadata, PREFIX_METADATA);

impl PrefixMetadata {
    /// Get the metadata type name (the identifier after #)
    /// e.g., `mop` in `#mop attribute mass : Real;`
    pub fn name(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
    }

    /// Get the text range of the identifier (for hover/goto)
    pub fn name_range(&self) -> Option<rowan::TextRange> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text_range())
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
                // KerML definition keywords
                SyntaxKind::CLASS_KW => return Some(DefinitionKind::Class),
                SyntaxKind::STRUCT_KW => return Some(DefinitionKind::Struct),
                SyntaxKind::ASSOC_KW => return Some(DefinitionKind::Assoc),
                SyntaxKind::BEHAVIOR_KW => return Some(DefinitionKind::Behavior),
                SyntaxKind::FUNCTION_KW => return Some(DefinitionKind::Function),
                SyntaxKind::PREDICATE_KW => return Some(DefinitionKind::Predicate),
                SyntaxKind::INTERACTION_KW => return Some(DefinitionKind::Interaction),
                SyntaxKind::DATATYPE_KW => return Some(DefinitionKind::Datatype),
                SyntaxKind::CLASSIFIER_KW => return Some(DefinitionKind::Classifier),
                SyntaxKind::TYPE_KW => return Some(DefinitionKind::Type),
                SyntaxKind::METACLASS_KW => return Some(DefinitionKind::Metaclass),
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
    // KerML kinds
    Class,
    Struct,
    Assoc,
    Behavior,
    Function,
    Predicate,
    Interaction,
    Datatype,
    Classifier,
    Type,
    Metaclass,
}

// ============================================================================
// Usage
// ============================================================================

/// Usage node - covers USAGE and requirement-specific usage kinds
/// (SUBJECT_USAGE, ACTOR_USAGE, STAKEHOLDER_USAGE, OBJECTIVE_USAGE)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Usage(SyntaxNode);

impl AstNode for Usage {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::USAGE
                | SyntaxKind::SUBJECT_USAGE
                | SyntaxKind::ACTOR_USAGE
                | SyntaxKind::STAKEHOLDER_USAGE
                | SyntaxKind::OBJECTIVE_USAGE
        )
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

    /// Get prefix metadata references.
    /// e.g., `#mop attribute mass : Real;` -> returns [PrefixMetadata for "mop"]
    /// 
    /// PREFIX_METADATA nodes can be in two locations depending on the usage type:
    /// 1. For most usages (part, attribute, etc.): preceding siblings in the namespace body
    /// 2. For end features: children of the USAGE node (after END_KW)
    pub fn prefix_metadata(&self) -> Vec<PrefixMetadata> {
        let mut result = Vec::new();
        
        // First check preceding siblings (most common case)
        let mut current = self.0.prev_sibling();
        while let Some(sibling) = current {
            if sibling.kind() == SyntaxKind::PREFIX_METADATA {
                if let Some(pm) = PrefixMetadata::cast(sibling.clone()) {
                    result.push(pm);
                }
                current = sibling.prev_sibling();
            } else {
                // Stop when we hit a non-PREFIX_METADATA node
                break;
            }
        }
        // Reverse to get them in source order
        result.reverse();
        
        // Also check children (for end features where PREFIX_METADATA is inside USAGE)
        for child in self.0.children() {
            if child.kind() == SyntaxKind::PREFIX_METADATA {
                if let Some(pm) = PrefixMetadata::cast(child) {
                    result.push(pm);
                }
            }
        }
        
        result
    }

    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    pub fn typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }

    /// Get the "of Type" qualified name for messages/items
    /// e.g., `message sendCmd of SensedSpeed` -> returns "SensedSpeed" QualifiedName
    /// This handles the `of` clause which is different from regular typing `:`.
    pub fn of_type(&self) -> Option<QualifiedName> {
        // Look for `of` keyword followed by a qualified name
        let mut found_of = false;
        for elem in self.0.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                if token.kind() == SyntaxKind::OF_KW {
                    found_of = true;
                }
            } else if let Some(node) = elem.as_node() {
                if found_of && node.kind() == SyntaxKind::QUALIFIED_NAME {
                    return QualifiedName::cast(node.clone());
                }
            }
        }
        None
    }

    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }

    /// Get the value expression (after `=`) if present
    pub fn value_expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Get the from-to clause for message/flow usages
    pub fn from_to_clause(&self) -> Option<FromToClause> {
        self.0.children().find_map(FromToClause::cast)
    }

    /// Get the nested transition usage (for transition statements)
    pub fn transition_usage(&self) -> Option<TransitionUsage> {
        self.0.children().find_map(TransitionUsage::cast)
    }

    /// Get the nested succession usage (for first/then statements)  
    pub fn succession(&self) -> Option<Succession> {
        self.0.children().find_map(Succession::cast)
    }

    /// Get the nested perform action usage (for perform statements)
    pub fn perform_action_usage(&self) -> Option<PerformActionUsage> {
        self.0.children().find_map(PerformActionUsage::cast)
    }

    /// Get the nested accept action usage (for accept statements)
    pub fn accept_action_usage(&self) -> Option<AcceptActionUsage> {
        self.0.children().find_map(AcceptActionUsage::cast)
    }

    /// Get the nested send action usage (for send statements)
    pub fn send_action_usage(&self) -> Option<SendActionUsage> {
        self.0.children().find_map(SendActionUsage::cast)
    }

    /// Get the nested requirement verification (for satisfy/verify statements)
    pub fn requirement_verification(&self) -> Option<RequirementVerification> {
        self.0.children().find_map(RequirementVerification::cast)
    }

    /// Get the nested connect usage (for connect statements)
    pub fn connect_usage(&self) -> Option<ConnectUsage> {
        self.0.children().find_map(ConnectUsage::cast)
    }

    /// Get the connector part directly (for connection usages with inline connect)
    /// e.g., `connection multicausation connect ( cause1 ::> causer1 )`
    pub fn connector_part(&self) -> Option<ConnectorPart> {
        self.0.children().find_map(ConnectorPart::cast)
    }

    /// Get the nested binding connector (for bind statements)
    pub fn binding_connector(&self) -> Option<BindingConnector> {
        self.0.children().find_map(BindingConnector::cast)
    }

    /// Get the constraint body (for constraint usages)
    pub fn constraint_body(&self) -> Option<ConstraintBody> {
        self.0.children().find_map(ConstraintBody::cast)
    }

    /// Check if this usage has exhibit keyword
    pub fn is_exhibit(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::EXHIBIT_KW)
    }

    /// Check if this usage has include keyword
    pub fn is_include(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::INCLUDE_KW)
    }

    /// Check if this usage has allocate keyword
    pub fn is_allocate(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ALLOCATE_KW)
    }

    /// Check if this usage has flow keyword
    pub fn is_flow(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::FLOW_KW)
    }

    /// Check if this usage has assert keyword
    pub fn is_assert(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ASSERT_KW)
    }

    /// Check if this usage has assume keyword
    pub fn is_assume(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ASSUME_KW)
    }

    /// Check if this usage has require keyword  
    pub fn is_require(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::REQUIRE_KW)
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
                SyntaxKind::FLOW_KW | SyntaxKind::MESSAGE_KW => return Some(UsageKind::Flow),
                SyntaxKind::OCCURRENCE_KW => return Some(UsageKind::Occurrence),
                SyntaxKind::REF_KW => return Some(UsageKind::Ref),
                // KerML usage keywords
                SyntaxKind::FEATURE_KW => return Some(UsageKind::Feature),
                SyntaxKind::STEP_KW => return Some(UsageKind::Step),
                SyntaxKind::EXPR_KW => return Some(UsageKind::Expr),
                SyntaxKind::CONNECTOR_KW => return Some(UsageKind::Connector),
                _ => {}
            }
        }
        None
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
    Occurrence,
    Ref,
    // KerML
    Feature,
    Step,
    Expr,
    Connector,
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
        // Get the identifier token (including contextual keywords used as names)
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::IDENT
                        | SyntaxKind::START_KW
                        | SyntaxKind::END_KW
                        | SyntaxKind::DONE_KW
                )
            })
            .map(|t| t.text().to_string())
    }
}

ast_node!(ShortName, SHORT_NAME);

impl ShortName {
    pub fn text(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::IDENT
                        | SyntaxKind::START_KW
                        | SyntaxKind::END_KW
                        | SyntaxKind::DONE_KW
                )
            })
            .map(|t| t.text().to_string())
    }
}

ast_node!(QualifiedName, QUALIFIED_NAME);

impl QualifiedName {
    /// Get all name segments
    /// Includes IDENT tokens and contextual keywords that can be used as identifiers
    pub fn segments(&self) -> Vec<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| {
                // Include IDENT and contextual keywords that can be used as names
                matches!(
                    t.kind(),
                    SyntaxKind::IDENT
                        | SyntaxKind::START_KW
                        | SyntaxKind::END_KW
                        | SyntaxKind::DONE_KW
                )
            })
            .map(|t| {
                let text = t.text();
                // Strip surrounding quotes from unrestricted names like 'My Name'
                if text.starts_with('\'') && text.ends_with('\'') && text.len() > 1 {
                    text[1..text.len() - 1].to_string()
                } else {
                    text.to_string()
                }
            })
            .collect()
    }

    /// Get all name segments with their text ranges
    /// Includes IDENT tokens and contextual keywords that can be used as identifiers
    pub fn segments_with_ranges(&self) -> Vec<(String, rowan::TextRange)> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| {
                // Include IDENT and contextual keywords that can be used as names
                matches!(
                    t.kind(),
                    SyntaxKind::IDENT
                        | SyntaxKind::START_KW
                        | SyntaxKind::END_KW
                        | SyntaxKind::DONE_KW
                )
            })
            .map(|t| {
                let text = t.text();
                // Strip surrounding quotes from unrestricted names like 'My Name'
                let stripped = if text.starts_with('\'') && text.ends_with('\'') && text.len() > 1 {
                    text[1..text.len() - 1].to_string()
                } else {
                    text.to_string()
                };
                (stripped, t.text_range())
            })
            .collect()
    }

    /// Get the full qualified name as a string
    /// Uses '::' for namespace paths, '.' for feature chains
    fn to_string_inner(&self) -> String {
        // Check if this is a feature chain (uses '.' separator) or namespace path (uses '::')
        let has_dot = self
            .0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::DOT);

        let separator = if has_dot { "." } else { "::" };
        self.segments().join(separator)
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_inner())
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
                SyntaxKind::COLON_GT_GT => return Some(SpecializationKind::Redefines),
                SyntaxKind::COLON_COLON_GT => return Some(SpecializationKind::FeatureChain),
                SyntaxKind::SPECIALIZES_KW => return Some(SpecializationKind::Specializes),
                SyntaxKind::SUBSETS_KW => return Some(SpecializationKind::Subsets),
                SyntaxKind::REDEFINES_KW => return Some(SpecializationKind::Redefines),
                SyntaxKind::REFERENCES_KW => return Some(SpecializationKind::References),
                SyntaxKind::TILDE => return Some(SpecializationKind::Conjugates),
                SyntaxKind::FROM_KW => return Some(SpecializationKind::FeatureChain),
                SyntaxKind::TO_KW => return Some(SpecializationKind::FeatureChain),
                _ => {}
            }
        }
        None
    }

    /// Check if this is a shorthand redefines (`:>>`) vs keyword (`redefines`)
    /// Returns true for `:>> name`, false for `redefines name`
    pub fn is_shorthand_redefines(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::COLON_GT_GT)
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
// From-To Clause (for message/flow usages)
// ============================================================================

ast_node!(FromToClause, FROM_TO_CLAUSE);

impl FromToClause {
    /// Get the source reference (e.g., `driver.turnVehicleOn`)
    pub fn source(&self) -> Option<FromToSource> {
        self.0.children().find_map(FromToSource::cast)
    }

    /// Get the target reference (e.g., `vehicle.trigger1`)
    pub fn target(&self) -> Option<FromToTarget> {
        self.0.children().find_map(FromToTarget::cast)
    }
}

ast_node!(FromToSource, FROM_TO_SOURCE);

impl FromToSource {
    /// Get the qualified name (which may be a feature chain like `a.b.c`)
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

ast_node!(FromToTarget, FROM_TO_TARGET);

impl FromToTarget {
    /// Get the qualified name (which may be a feature chain like `a.b.c`)
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }
}

// ============================================================================
// Transition Usage
// ============================================================================

ast_node!(TransitionUsage, TRANSITION_USAGE);

impl TransitionUsage {
    /// Get the transition name (if named)
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get all specializations (source and target states)
    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    /// Get the source state (first specialization, before 'then')
    pub fn source(&self) -> Option<Specialization> {
        self.specializations().next()
    }

    /// Get the target state (second specialization, after 'then')
    pub fn target(&self) -> Option<Specialization> {
        self.specializations().nth(1)
    }

    /// Get the accept payload name (the second NAME if present, after ACCEPT_KW)
    /// e.g., in `accept ignitionCmd:IgnitionCmd`, returns the Name for `ignitionCmd`
    pub fn accept_payload_name(&self) -> Option<Name> {
        use crate::parser::SyntaxKind;
        let mut found_accept = false;
        for child in self.0.children_with_tokens() {
            match &child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::ACCEPT_KW => {
                    found_accept = true;
                }
                rowan::NodeOrToken::Node(n) if found_accept => {
                    if let Some(name) = Name::cast(n.clone()) {
                        return Some(name);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get typing for the accept payload (e.g., `:IgnitionCmd`)
    pub fn accept_typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }

    /// Get the 'via' target for the accept trigger
    /// e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`
    pub fn accept_via(&self) -> Option<QualifiedName> {
        use crate::parser::SyntaxKind;
        let mut seen_via = false;
        for child in self.0.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::VIA_KW => {
                    seen_via = true;
                }
                rowan::NodeOrToken::Node(n) if seen_via => {
                    if let Some(qn) = QualifiedName::cast(n) {
                        return Some(qn);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

// ============================================================================
// Perform Action Usage
// ============================================================================

ast_node!(PerformActionUsage, PERFORM_ACTION_USAGE);

impl PerformActionUsage {
    /// Get the perform action name (if named, e.g., `perform action startVehicle`)
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the typing (e.g., `: GetOutOfVehicle` in `perform action x : GetOutOfVehicle`)
    pub fn typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }

    /// Get all specializations (includes the performed action and redefines)
    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    /// Get the performed action (first specialization, the action being performed)
    pub fn performed(&self) -> Option<Specialization> {
        self.specializations().next()
    }

    /// Get the body of the perform action
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
}

// ============================================================================
// Accept Action Usage
// ============================================================================

ast_node!(AcceptActionUsage, ACCEPT_ACTION_USAGE);

impl AcceptActionUsage {
    /// Get the accept action name (if named, e.g., the payload name)
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the trigger expression (what's being accepted)
    pub fn trigger(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Get the qualified name if accepting a named signal
    pub fn accepted(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Get the 'via' target port (the QualifiedName after VIA_KW)
    /// e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`
    pub fn via(&self) -> Option<QualifiedName> {
        let mut seen_via = false;
        for child in self.0.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::VIA_KW => {
                    seen_via = true;
                }
                rowan::NodeOrToken::Node(n) if seen_via => {
                    if let Some(qn) = QualifiedName::cast(n) {
                        return Some(qn);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

// ============================================================================
// Send Action Usage
// ============================================================================

ast_node!(SendActionUsage, SEND_ACTION_USAGE);

impl SendActionUsage {
    /// Get the payload expression (what's being sent)
    pub fn payload(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Get qualified names (for via/to targets)
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.children().filter_map(QualifiedName::cast)
    }
}

// ============================================================================
// State Subaction (entry/do/exit)
// ============================================================================

ast_node!(StateSubaction, STATE_SUBACTION);

impl StateSubaction {
    /// Get the state subaction kind (entry, do, or exit)
    pub fn kind(&self) -> Option<SyntaxKind> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::ENTRY_KW | SyntaxKind::DO_KW | SyntaxKind::EXIT_KW
                )
            })
            .map(|t| t.kind())
    }

    /// Get the name of the action (if named)
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the body if present
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }

    /// Check if this is an 'entry' subaction
    pub fn is_entry(&self) -> bool {
        self.kind() == Some(SyntaxKind::ENTRY_KW)
    }

    /// Check if this is a 'do' subaction
    pub fn is_do(&self) -> bool {
        self.kind() == Some(SyntaxKind::DO_KW)
    }

    /// Check if this is an 'exit' subaction
    pub fn is_exit(&self) -> bool {
        self.kind() == Some(SyntaxKind::EXIT_KW)
    }
}

// ============================================================================
// Control Node (fork, join, merge, decide)
// ============================================================================

ast_node!(ControlNode, CONTROL_NODE);

impl ControlNode {
    /// Get the control node kind (fork, join, merge, or decide)
    pub fn kind(&self) -> Option<SyntaxKind> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    SyntaxKind::FORK_KW
                        | SyntaxKind::JOIN_KW
                        | SyntaxKind::MERGE_KW
                        | SyntaxKind::DECIDE_KW
                )
            })
            .map(|t| t.kind())
    }

    /// Get the name of the control node (if named)
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the body if present
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }

    /// Check if this is a 'fork' node
    pub fn is_fork(&self) -> bool {
        self.kind() == Some(SyntaxKind::FORK_KW)
    }

    /// Check if this is a 'join' node
    pub fn is_join(&self) -> bool {
        self.kind() == Some(SyntaxKind::JOIN_KW)
    }

    /// Check if this is a 'merge' node
    pub fn is_merge(&self) -> bool {
        self.kind() == Some(SyntaxKind::MERGE_KW)
    }

    /// Check if this is a 'decide' node
    pub fn is_decide(&self) -> bool {
        self.kind() == Some(SyntaxKind::DECIDE_KW)
    }
}

// ============================================================================
// Requirement Verification (satisfy/verify)
// ============================================================================

ast_node!(RequirementVerification, REQUIREMENT_VERIFICATION);

impl RequirementVerification {
    /// Check if this is a 'satisfy' (vs 'verify')
    pub fn is_satisfy(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::SATISFY_KW)
    }

    /// Check if this is a 'verify' (vs 'satisfy')
    pub fn is_verify(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::VERIFY_KW)
    }

    /// Check if negated ('not satisfy')
    pub fn is_negated(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::NOT_KW)
    }

    /// Check if asserted ('assert satisfy')
    pub fn is_asserted(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::ASSERT_KW)
    }

    /// Get the requirement being satisfied/verified (the first QualifiedName)
    pub fn requirement(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Get the 'by' target (the QualifiedName after BY_KW)
    /// e.g., `vehicle_b` in `satisfy R by vehicle_b`
    pub fn by_target(&self) -> Option<QualifiedName> {
        let mut seen_by = false;
        for child in self.0.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::BY_KW => {
                    seen_by = true;
                }
                rowan::NodeOrToken::Node(n) if seen_by => {
                    if let Some(qn) = QualifiedName::cast(n) {
                        return Some(qn);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the typing if present
    pub fn typing(&self) -> Option<Typing> {
        self.0.children().find_map(Typing::cast)
    }
}

// ============================================================================
// KerML Connector (standalone connector, not SysML Connection)
// ============================================================================

ast_node!(Connector, CONNECTOR);

impl Connector {
    /// Get the name
    pub fn name(&self) -> Option<Name> {
        self.0.children().find_map(Name::cast)
    }

    /// Get the connector part (contains ends)
    pub fn connector_part(&self) -> Option<ConnectorPart> {
        self.0.children().find_map(ConnectorPart::cast)
    }

    /// Get the namespace body
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
}

// ============================================================================
// Connect Usage
// ============================================================================

ast_node!(ConnectUsage, CONNECT_USAGE);

impl ConnectUsage {
    /// Get the connector part (contains ends)
    pub fn connector_part(&self) -> Option<ConnectorPart> {
        self.0.children().find_map(ConnectorPart::cast)
    }
}

ast_node!(ConnectorPart, CONNECTOR_PART);

impl ConnectorPart {
    /// Get all connector ends
    pub fn ends(&self) -> impl Iterator<Item = ConnectorEnd> + '_ {
        self.0.children().filter_map(ConnectorEnd::cast)
    }

    /// Get source end (first)
    pub fn source(&self) -> Option<ConnectorEnd> {
        self.ends().next()
    }

    /// Get target end (second)
    pub fn target(&self) -> Option<ConnectorEnd> {
        self.ends().nth(1)
    }
}

ast_node!(ConnectorEnd, CONNECTOR_END);

impl ConnectorEnd {
    /// Get the qualified name target reference.
    /// For patterns like `p ::> comp.lugNutPort`, returns `comp.lugNutPort`.
    /// For simple patterns like `comp.lugNutPort`, returns `comp.lugNutPort`.
    pub fn target(&self) -> Option<QualifiedName> {
        // First check if we have a CONNECTOR_END_REFERENCE child
        if let Some(ref_node) = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::CONNECTOR_END_REFERENCE)
        {
            // Within CONNECTOR_END_REFERENCE, find qualified names
            let qns: Vec<_> = ref_node
                .children()
                .filter_map(QualifiedName::cast)
                .collect();
            // If there's a ::> or references keyword, return the second QN (the target)
            // Otherwise return the first/only QN
            let has_references = ref_node.children_with_tokens().any(|n| {
                n.kind() == SyntaxKind::COLON_COLON_GT || n.kind() == SyntaxKind::REFERENCES_KW
            });
            if has_references && qns.len() > 1 {
                return Some(qns[1].clone());
            } else {
                return qns.into_iter().next();
            }
        }
        // Direct child lookup as fallback
        self.0.children().find_map(QualifiedName::cast)
    }
}

// ============================================================================
// Binding Connector
// ============================================================================

ast_node!(BindingConnector, BINDING_CONNECTOR);

impl BindingConnector {
    /// Get all qualified names (source and target)
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.children().filter_map(QualifiedName::cast)
    }

    /// Get source (first qualified name)
    pub fn source(&self) -> Option<QualifiedName> {
        self.qualified_names().next()
    }

    /// Get target (second qualified name)
    pub fn target(&self) -> Option<QualifiedName> {
        self.qualified_names().nth(1)
    }
}

// ============================================================================
// Succession
// ============================================================================

ast_node!(Succession, SUCCESSION);

impl Succession {
    /// Get all items in the succession
    pub fn items(&self) -> impl Iterator<Item = SuccessionItem> + '_ {
        self.0.children().filter_map(SuccessionItem::cast)
    }

    /// Get the first item (source)
    pub fn source(&self) -> Option<SuccessionItem> {
        self.items().next()
    }

    /// Get the second item (target)
    pub fn target(&self) -> Option<SuccessionItem> {
        self.items().nth(1)
    }

    /// Get inline usages directly inside the succession (not wrapped in SUCCESSION_ITEM)
    /// These come from `then action a { ... }` style successions
    pub fn inline_usages(&self) -> impl Iterator<Item = Usage> + '_ {
        self.0.children().filter_map(Usage::cast)
    }
}

ast_node!(SuccessionItem, SUCCESSION_ITEM);

impl SuccessionItem {
    /// Get the qualified name reference (for simple succession like `then start`)
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    /// Get the inline usage (for succession with inline definition like `then action a { ... }`)
    pub fn usage(&self) -> Option<Usage> {
        self.0.children().find_map(Usage::cast)
    }
}

// ============================================================================
// Constraint Body
// ============================================================================

ast_node!(ConstraintBody, CONSTRAINT_BODY);

impl ConstraintBody {
    /// Get the expression inside the constraint body
    pub fn expression(&self) -> Option<Expression> {
        self.0.children().find_map(Expression::cast)
    }

    /// Get the namespace members inside the constraint body (for satisfy/verify blocks)
    /// e.g., `satisfy R { requirement x :>> y { ... } }` - the nested requirement is a member
    pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
        self.0.children().filter_map(NamespaceMember::cast)
    }
}

// ============================================================================
// Expression
// ============================================================================

ast_node!(Expression, EXPRESSION);

/// A feature chain like `fuelTank.mass` with individual part ranges
#[derive(Debug, Clone)]
pub struct FeatureChainRef {
    /// The parts of the chain (e.g., ["fuelTank", "mass"])
    pub parts: Vec<(String, rowan::TextRange)>,
    /// The full range of the chain
    pub full_range: rowan::TextRange,
}

impl Expression {
    /// Extract all identifier references from this expression
    /// Returns pairs of (identifier_name, text_range)
    pub fn references(&self) -> Vec<(String, rowan::TextRange)> {
        let mut refs = Vec::new();
        self.collect_references(&self.0, &mut refs);
        refs
    }

    /// Extract feature chains from this expression.
    /// A feature chain is a sequence of identifiers separated by `.` (e.g., `fuelTank.mass`).
    /// Returns each chain with its parts and their individual ranges.
    pub fn feature_chains(&self) -> Vec<FeatureChainRef> {
        let mut chains = Vec::new();
        self.collect_feature_chains(&self.0, &mut chains);
        chains
    }

    /// Extract named constructor arguments from `new Type(argName = value)` patterns.
    /// Returns tuples of (type_name, arg_name, arg_name_range).
    /// The arg_name should resolve as Type.argName (a feature of the constructed type).
    pub fn named_constructor_args(&self) -> Vec<(String, String, rowan::TextRange)> {
        let mut results = Vec::new();
        self.collect_named_constructor_args(&self.0, &mut results);
        results
    }

    fn collect_named_constructor_args(
        &self,
        node: &SyntaxNode,
        results: &mut Vec<(String, String, rowan::TextRange)>,
    ) {
        // Look for pattern: NEW_KW followed by QUALIFIED_NAME then ARGUMENT_LIST
        let children: Vec<_> = node.children_with_tokens().collect();
        
        let mut i = 0;
        while i < children.len() {
            // Check for NEW_KW token
            if let Some(token) = children[i].as_token() {
                if token.kind() == SyntaxKind::NEW_KW {
                    // Find the type name (QUALIFIED_NAME after new)
                    let mut type_name = None;
                    for j in (i + 1)..children.len() {
                        if let Some(qn_node) = children[j].as_node() {
                            if qn_node.kind() == SyntaxKind::QUALIFIED_NAME {
                                type_name = Some(qn_node.text().to_string());
                                break;
                            }
                        }
                    }
                    
                    // Find ARGUMENT_LIST and extract named arguments
                    if let Some(type_name) = type_name {
                        for j in (i + 1)..children.len() {
                            if let Some(arg_list_node) = children[j].as_node() {
                                if arg_list_node.kind() == SyntaxKind::ARGUMENT_LIST {
                                    self.extract_named_args_from_list(
                                        arg_list_node,
                                        &type_name,
                                        results,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            i += 1;
        }

        // Recurse into child nodes
        for child in node.children() {
            self.collect_named_constructor_args(&child, results);
        }
    }

    fn extract_named_args_from_list(
        &self,
        arg_list: &SyntaxNode,
        type_name: &str,
        results: &mut Vec<(String, String, rowan::TextRange)>,
    ) {
        // Nested ARGUMENT_LIST nodes contain the actual arguments
        for child in arg_list.children() {
            if child.kind() == SyntaxKind::ARGUMENT_LIST {
                // Check for pattern: IDENT EQ ...
                let tokens: Vec<_> = child.children_with_tokens().collect();
                
                // Look for IDENT followed by EQ (named argument pattern)
                for (idx, elem) in tokens.iter().enumerate() {
                    if let Some(token) = elem.as_token() {
                        if token.kind() == SyntaxKind::IDENT {
                            // Check if next non-whitespace is EQ
                            for next_elem in tokens.iter().skip(idx + 1) {
                                if let Some(next_token) = next_elem.as_token() {
                                    if next_token.kind() == SyntaxKind::WHITESPACE {
                                        continue;
                                    }
                                    if next_token.kind() == SyntaxKind::EQ {
                                        // Found named argument!
                                        results.push((
                                            type_name.to_string(),
                                            token.text().to_string(),
                                            token.text_range(),
                                        ));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
                
                // Recurse into nested argument lists
                self.extract_named_args_from_list(&child, type_name, results);
            }
        }
    }

    fn collect_feature_chains(&self, node: &SyntaxNode, chains: &mut Vec<FeatureChainRef>) {
        // Check if this node is a QUALIFIED_NAME (which represents a feature chain)
        if node.kind() == SyntaxKind::QUALIFIED_NAME {
            let mut parts = Vec::new();
            for child in node.children_with_tokens() {
                if let rowan::NodeOrToken::Token(token) = child {
                    if token.kind() == SyntaxKind::IDENT {
                        parts.push((token.text().to_string(), token.text_range()));
                    }
                }
            }
            if !parts.is_empty() {
                let full_range = node.text_range();
                chains.push(FeatureChainRef { parts, full_range });
            }
            return; // Don't recurse into QUALIFIED_NAME, we've handled it
        }

        // Recurse into child nodes
        for child in node.children() {
            self.collect_feature_chains(&child, chains);
        }
    }

    fn collect_references(&self, node: &SyntaxNode, refs: &mut Vec<(String, rowan::TextRange)>) {
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(token) => {
                    if token.kind() == SyntaxKind::IDENT {
                        refs.push((token.text().to_string(), token.text_range()));
                    }
                }
                rowan::NodeOrToken::Node(child_node) => {
                    // Recurse into child nodes
                    self.collect_references(&child_node, refs);
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_sysml;

    #[test]
    fn test_ast_package() {
        let parsed = parse_sysml("package Test;");
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
        let parsed = parse_sysml("import ISQ::*;");
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
        let parsed = parse_sysml("import all Library::**;");
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
        let parsed = parse_sysml("abstract part def Vehicle :> Base;");
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
        let parsed = parse_sysml("ref part engine : Engine;");
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
        let parsed = parse_sysml("part p { message of ignitionCmd : IgnitionCmd; }");
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
