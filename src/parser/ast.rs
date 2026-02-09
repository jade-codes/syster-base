//! Typed AST wrappers over the untyped rowan CST.
//!
//! This module provides strongly-typed accessors for SysML syntax nodes.
//! Each struct wraps a SyntaxNode and provides methods to access children.

use super::syntax_kind::SyntaxKind;
use super::{SyntaxNode, SyntaxToken};

// ============================================================================
// Helper utilities for reducing code duplication
// ============================================================================

/// Check if a token kind is an identifier or contextual keyword that can be used as a name.
#[inline]
fn is_name_token(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::IDENT
            | SyntaxKind::START_KW
            | SyntaxKind::END_KW
            | SyntaxKind::DONE_KW
            | SyntaxKind::THIS_KW
    )
}

/// Strip surrounding single quotes from unrestricted names like `'My Name'`.
#[inline]
fn strip_unrestricted_name(text: &str) -> String {
    if text.starts_with('\'') && text.ends_with('\'') && text.len() > 1 {
        text[1..text.len() - 1].to_string()
    } else {
        text.to_string()
    }
}

/// Check if a syntax node has a direct child token of the specified kind.
///
/// This is a common pattern used throughout the AST to check for modifier keywords
/// like `abstract`, `ref`, `readonly`, etc.
///
/// # Example
/// ```ignore
/// // Instead of:
/// self.0.children_with_tokens()
///     .filter_map(|e| e.into_token())
///     .any(|t| t.kind() == SyntaxKind::ABSTRACT_KW)
///
/// // Use:
/// has_token(&self.0, SyntaxKind::ABSTRACT_KW)
/// ```
#[inline]
fn has_token(node: &SyntaxNode, kind: SyntaxKind) -> bool {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t| t.kind() == kind)
}

/// Macro to generate boolean property methods that check for a specific token kind.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     has_token_method!(is_abstract, ABSTRACT_KW, "abstract part def P {}");
///     has_token_method!(is_ref, REF_KW, "ref part p;");
/// }
/// ```
macro_rules! has_token_method {
    ($name:ident, $kind:ident) => {
        #[doc = concat!("Check if this node has the `", stringify!($kind), "` token.")]
        pub fn $name(&self) -> bool {
            has_token(&self.0, SyntaxKind::$kind)
        }
    };
    ($name:ident, $kind:ident, $example:literal) => {
        #[doc = concat!("Check if this node has the `", stringify!($kind), "` token (e.g., `", $example, "`).")]
        pub fn $name(&self) -> bool {
            has_token(&self.0, SyntaxKind::$kind)
        }
    };
}

/// Macro to generate a method that finds the first child of a specific AST type.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     first_child_method!(name, Name);
///     first_child_method!(body, NamespaceBody);
/// }
/// ```
macro_rules! first_child_method {
    ($name:ident, $type:ident) => {
        #[doc = concat!("Get the first `", stringify!($type), "` child of this node.")]
        pub fn $name(&self) -> Option<$type> {
            self.0.children().find_map($type::cast)
        }
    };
}

/// Macro to generate a method that returns an iterator over children of a specific AST type.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     children_method!(specializations, Specialization);
///     children_method!(members, NamespaceMember);
/// }
/// ```
macro_rules! children_method {
    ($name:ident, $type:ident) => {
        #[doc = concat!("Get all `", stringify!($type), "` children of this node.")]
        pub fn $name(&self) -> impl Iterator<Item = $type> + '_ {
            self.0.children().filter_map($type::cast)
        }
    };
}

/// Macro to generate a method that gets the first child node of a type after a specific keyword.
///
/// This is a common pattern for things like `via port` or `by target` clauses.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     child_after_keyword_method!(via, QualifiedName, VIA_KW, "get the 'via' target port");
///     child_after_keyword_method!(by_target, QualifiedName, BY_KW, "get the 'by' target");
/// }
/// ```
macro_rules! child_after_keyword_method {
    ($name:ident, $type:ident, $keyword:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(&self) -> Option<$type> {
            let mut seen_keyword = false;
            for child in self.0.children_with_tokens() {
                match child {
                    rowan::NodeOrToken::Token(t) if t.kind() == SyntaxKind::$keyword => {
                        seen_keyword = true;
                    }
                    rowan::NodeOrToken::Node(n) if seen_keyword => {
                        if let Some(result) = $type::cast(n) {
                            return Some(result);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
    };
}

/// Macro to generate `members()` method that delegates to `body().members()`.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     body_members_method!();
/// }
/// ```
macro_rules! body_members_method {
    () => {
        /// Get members from the body.
        pub fn members(&self) -> impl Iterator<Item = NamespaceMember> + '_ {
            self.body()
                .into_iter()
                .flat_map(|body| body.members().collect::<Vec<_>>())
        }
    };
}

/// Macro to generate a method that finds the first matching token from a set of kinds.
///
/// Returns `Option<SyntaxKind>` of the matched token.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     find_token_kind_method!(kind, [ENTRY_KW, DO_KW, EXIT_KW], "Get the subaction kind.");
/// }
/// ```
macro_rules! find_token_kind_method {
    ($name:ident, [$($kind:ident),+ $(,)?], $doc:literal) => {
        #[doc = $doc]
        pub fn $name(&self) -> Option<SyntaxKind> {
            self.0
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| matches!(t.kind(), $(SyntaxKind::$kind)|+))
                .map(|t| t.kind())
        }
    };
}

/// Macro to generate source/target pair methods from an iterator method.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     children_method!(items, Item);
///     source_target_pair!(source, target, items, Item);
/// }
/// ```
macro_rules! source_target_pair {
    ($source:ident, $target:ident, $iter_method:ident, $type:ident) => {
        #[doc = concat!("Get the first `", stringify!($type), "` (source).")]
        pub fn $source(&self) -> Option<$type> {
            self.$iter_method().next()
        }

        #[doc = concat!("Get the second `", stringify!($type), "` (target).")]
        pub fn $target(&self) -> Option<$type> {
            self.$iter_method().nth(1)
        }
    };
}

/// Helper to collect prefix metadata from preceding siblings.
///
/// PREFIX_METADATA nodes precede definitions/usages in the source.
fn collect_prefix_metadata(node: &SyntaxNode) -> Vec<PrefixMetadata> {
    let mut result = Vec::new();
    let mut current = node.prev_sibling();
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
    /// For loop action usage (e.g., `for n : Integer in (1,2,3) { }`)
    ForLoop(ForLoopActionUsage),
    /// If action usage (e.g., `if x == 1 then A1;`)
    IfAction(IfActionUsage),
    /// While loop action usage (e.g., `while x > 0 { }`)
    WhileLoop(WhileLoopActionUsage),
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
                | SyntaxKind::FOR_LOOP_ACTION_USAGE
                | SyntaxKind::IF_ACTION_USAGE
                | SyntaxKind::WHILE_LOOP_ACTION_USAGE
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
            SyntaxKind::FOR_LOOP_ACTION_USAGE => Some(Self::ForLoop(ForLoopActionUsage(node))),
            SyntaxKind::IF_ACTION_USAGE => Some(Self::IfAction(IfActionUsage(node))),
            SyntaxKind::WHILE_LOOP_ACTION_USAGE => {
                Some(Self::WhileLoop(WhileLoopActionUsage(node)))
            }
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
            Self::ForLoop(n) => n.syntax(),
            Self::IfAction(n) => n.syntax(),
            Self::WhileLoop(n) => n.syntax(),
        }
    }
}

// ============================================================================
// Package
// ============================================================================

ast_node!(Package, PACKAGE);

impl Package {
    first_child_method!(name, Name);
    first_child_method!(body, NamespaceBody);
    body_members_method!();
}

ast_node!(LibraryPackage, LIBRARY_PACKAGE);

impl LibraryPackage {
    has_token_method!(is_standard, STANDARD_KW, "standard library package P {}");
    first_child_method!(name, Name);
    first_child_method!(body, NamespaceBody);
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
    has_token_method!(is_all, ALL_KW, "import all P::*");

    /// Get the qualified name being imported
    pub fn target(&self) -> Option<QualifiedName> {
        self.0.children().find_map(QualifiedName::cast)
    }

    has_token_method!(is_wildcard, STAR, "import P::*");

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
        if has_token(&self.0, SyntaxKind::PUBLIC_KW) {
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

    first_child_method!(filter, FilterPackage);
}

ast_node!(FilterPackage, FILTER_PACKAGE);

impl FilterPackage {
    first_child_method!(target, QualifiedName);

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
    first_child_method!(name, Name);
    first_child_method!(target, QualifiedName);
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
        collect_prefix_metadata(&self.0)
    }
}

// ============================================================================
// Filter
// ============================================================================

ast_node!(ElementFilter, ELEMENT_FILTER_MEMBER);

impl ElementFilter {
    first_child_method!(expression, Expression);

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
    first_child_method!(name, Name);
    children_method!(about_targets, QualifiedName);
    has_token_method!(has_about, ABOUT_KW, "doc /* text */ about x");
}

// ============================================================================
// Metadata
// ============================================================================

ast_node!(MetadataUsage, METADATA_USAGE);

impl MetadataUsage {
    first_child_method!(target, QualifiedName);

    /// Get the about target(s) - references after the 'about' keyword
    /// e.g., `@Rationale about vehicle::engine` returns [vehicle::engine]
    pub fn about_targets(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        // Skip the first QualifiedName (which is the metadata type)
        // All subsequent QualifiedNames are about targets
        self.0.children().filter_map(QualifiedName::cast).skip(1)
    }

    has_token_method!(has_about, ABOUT_KW, "@Rationale about x");
    first_child_method!(body, NamespaceBody);
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
    has_token_method!(is_abstract, ABSTRACT_KW, "abstract part def P {}");
    has_token_method!(is_variation, VARIATION_KW, "variation part def V {}");
    has_token_method!(is_individual, INDIVIDUAL_KW, "individual part def Earth;");

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

    first_child_method!(name, Name);
    children_method!(specializations, Specialization);
    first_child_method!(body, NamespaceBody);
    first_child_method!(constraint_body, ConstraintBody);

    body_members_method!();

    /// Get prefix metadata references from preceding siblings.
    /// e.g., `#service port def ServiceDiscovery` -> returns [PrefixMetadata for "service"]
    pub fn prefix_metadata(&self) -> Vec<PrefixMetadata> {
        collect_prefix_metadata(&self.0)
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
    has_token_method!(is_ref, REF_KW, "ref part p;");
    has_token_method!(is_readonly, READONLY_KW, "readonly attribute x;");
    has_token_method!(is_derived, DERIVED_KW, "derived attribute x;");
    has_token_method!(is_abstract, ABSTRACT_KW, "abstract part p;");
    has_token_method!(is_variation, VARIATION_KW, "variation part p;");
    has_token_method!(is_var, VAR_KW, "var attribute x;");
    has_token_method!(is_all, ALL_KW, "feature all instances : C[*]");
    has_token_method!(is_parallel, PARALLEL_KW, "parallel action a;");
    has_token_method!(is_individual, INDIVIDUAL_KW, "individual part earth : Earth;");
    has_token_method!(is_end, END_KW, "end part wheel : Wheel[4];");
    has_token_method!(is_default, DEFAULT_KW, "default attribute rgb : RGB;");
    has_token_method!(is_ordered, ORDERED_KW, "ordered part wheels : Wheel[4];");
    has_token_method!(is_nonunique, NONUNIQUE_KW, "nonunique attribute scores : Integer[*];");
    has_token_method!(is_portion, PORTION_KW, "portion part fuelLoad : Fuel;");

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

    /// Get multiplicity bounds [lower..upper] from the usage.
    /// Returns (lower, upper) where None means unbounded (*).
    /// E.g., `[1..5]` -> `(Some(1), Some(5))`, `[*]` -> `(None, None)`, `[0..*]` -> `(Some(0), None)`
    pub fn multiplicity(&self) -> Option<(Option<u64>, Option<u64>)> {
        // Find MULTIPLICITY node in children first (direct multiplicity like `wheels[4]`)
        if let Some(mult_node) = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::MULTIPLICITY)
        {
            return Self::parse_multiplicity_node(&mult_node);
        }

        // Check for multiplicity in TYPING or SPECIALIZATION children (like `fuelIn : FuelType[1]`)
        for child in self.0.children() {
            match child.kind() {
                SyntaxKind::TYPING | SyntaxKind::SPECIALIZATION => {
                    if let Some(mult_node) = child
                        .children()
                        .find(|n| n.kind() == SyntaxKind::MULTIPLICITY)
                    {
                        return Self::parse_multiplicity_node(&mult_node);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Parse a MULTIPLICITY node and extract bounds
    fn parse_multiplicity_node(mult_node: &SyntaxNode) -> Option<(Option<u64>, Option<u64>)> {
        let mut lower: Option<u64> = None;
        let mut upper: Option<u64> = None;
        let mut found_dot_dot = false;

        // Recursively search for INTEGER and STAR tokens in the multiplicity node
        fn find_bounds(
            node: &SyntaxNode,
            lower: &mut Option<u64>,
            upper: &mut Option<u64>,
            found_dot_dot: &mut bool,
        ) {
            for child in node.children_with_tokens() {
                match child.kind() {
                    SyntaxKind::INTEGER => {
                        if let Some(token) = child.into_token() {
                            let text = token.text();
                            if let Ok(val) = text.parse::<u64>() {
                                if *found_dot_dot {
                                    *upper = Some(val);
                                } else {
                                    *lower = Some(val);
                                }
                            }
                        }
                    }
                    SyntaxKind::STAR => {
                        // * means unbounded - leave as None
                        if *found_dot_dot {
                            *upper = None;
                        } else {
                            *lower = None;
                        }
                    }
                    SyntaxKind::DOT_DOT => {
                        *found_dot_dot = true;
                    }
                    _ => {
                        // Recurse into child nodes (e.g., LITERAL_EXPR)
                        if let Some(node) = child.into_node() {
                            find_bounds(&node, lower, upper, found_dot_dot);
                        }
                    }
                }
            }
        }

        find_bounds(mult_node, &mut lower, &mut upper, &mut found_dot_dot);

        // If no ".." found, lower is also the upper bound (e.g., [4] means [4..4])
        if !found_dot_dot && lower.is_some() {
            upper = lower;
        }

        // Only return Some if we found at least one bound or a star
        if lower.is_some() || upper.is_some() || found_dot_dot {
            Some((lower, upper))
        } else {
            // Try returning the found structure anyway - might be [*] or similar
            Some((lower, upper))
        }
    }

    /// Get prefix metadata references.
    /// e.g., `#mop attribute mass : Real;` -> returns [PrefixMetadata for "mop"]
    ///
    /// PREFIX_METADATA nodes can be in two locations depending on the usage type:
    /// 1. For most usages (part, attribute, etc.): preceding siblings in the namespace body
    /// 2. For end features: children of the USAGE node (after END_KW)
    pub fn prefix_metadata(&self) -> Vec<PrefixMetadata> {
        let mut result = collect_prefix_metadata(&self.0);

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

    first_child_method!(name, Name);

    /// Get all Name nodes within this usage.
    /// For `end self2 [1] feature sameThing: ...`, returns both `self2` and `sameThing`.
    /// This helps handle cases where the identification and feature name differ.
    pub fn names(&self) -> Vec<Name> {
        self.0.children().filter_map(Name::cast).collect()
    }

    first_child_method!(typing, Typing);

    child_after_keyword_method!(of_type, QualifiedName, OF_KW,
        "Get the 'of Type' qualified name for messages/items (e.g., `message sendCmd of SensedSpeed`).");

    children_method!(specializations, Specialization);
    first_child_method!(body, NamespaceBody);
    first_child_method!(value_expression, Expression);
    first_child_method!(from_to_clause, FromToClause);
    first_child_method!(transition_usage, TransitionUsage);
    first_child_method!(succession, Succession);
    first_child_method!(perform_action_usage, PerformActionUsage);
    first_child_method!(accept_action_usage, AcceptActionUsage);
    first_child_method!(send_action_usage, SendActionUsage);
    first_child_method!(requirement_verification, RequirementVerification);
    first_child_method!(connect_usage, ConnectUsage);
    first_child_method!(constraint_body, ConstraintBody);
    first_child_method!(connector_part, ConnectorPart);
    first_child_method!(binding_connector, BindingConnector);

    has_token_method!(is_exhibit, EXHIBIT_KW, "exhibit state s;");
    has_token_method!(is_include, INCLUDE_KW, "include use case u;");
    has_token_method!(is_allocate, ALLOCATE_KW, "allocate x to y;");
    has_token_method!(is_flow, FLOW_KW, "flow x to y;");

    /// Get direct flow endpoints for flows without `from` keyword.
    /// Pattern: `flow X.Y to A.B` returns (Some(X.Y), Some(A.B))
    /// This is different from `flow name from X to Y` which uses from_to_clause().
    pub fn direct_flow_endpoints(&self) -> (Option<QualifiedName>, Option<QualifiedName>) {
        // Only applicable to flow usages
        if !self.is_flow() {
            return (None, None);
        }

        // If there's a from_to_clause, this isn't a direct flow
        if self.from_to_clause().is_some() {
            return (None, None);
        }

        // Look for pattern: FLOW_KW ... QUALIFIED_NAME TO_KW QUALIFIED_NAME
        let mut found_flow = false;
        let mut found_to = false;
        let mut source: Option<QualifiedName> = None;
        let mut target: Option<QualifiedName> = None;

        for elem in self.0.children_with_tokens() {
            if let Some(token) = elem.as_token() {
                if token.kind() == SyntaxKind::FLOW_KW {
                    found_flow = true;
                } else if token.kind() == SyntaxKind::TO_KW && found_flow {
                    found_to = true;
                }
            } else if let Some(node) = elem.as_node() {
                if found_flow && node.kind() == SyntaxKind::QUALIFIED_NAME {
                    if !found_to && source.is_none() {
                        source = QualifiedName::cast(node.clone());
                    } else if found_to && target.is_none() {
                        target = QualifiedName::cast(node.clone());
                    }
                }
            }
        }

        (source, target)
    }

    has_token_method!(is_assert, ASSERT_KW, "assert constraint c;");
    has_token_method!(is_assume, ASSUME_KW, "assume constraint c;");
    has_token_method!(is_require, REQUIRE_KW, "require constraint c;");

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    first_child_method!(short_name, ShortName);

    pub fn text(&self) -> Option<String> {
        // Get the identifier token (including contextual keywords used as names)
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| is_name_token(t.kind()))
            .map(|t| t.text().to_string())
    }
}

ast_node!(ShortName, SHORT_NAME);

impl ShortName {
    pub fn text(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| is_name_token(t.kind()))
            .map(|t| strip_unrestricted_name(t.text()))
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
            .filter(|t| is_name_token(t.kind()))
            .map(|t| strip_unrestricted_name(t.text()))
            .collect()
    }

    /// Get all name segments with their text ranges
    /// Includes IDENT tokens and contextual keywords that can be used as identifiers
    pub fn segments_with_ranges(&self) -> Vec<(String, rowan::TextRange)> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| is_name_token(t.kind()))
            .map(|t| (strip_unrestricted_name(t.text()), t.text_range()))
            .collect()
    }

    /// Get the full qualified name as a string
    /// Uses '::' for namespace paths, '.' for feature chains
    fn to_string_inner(&self) -> String {
        // Check if this is a feature chain (uses '.' separator) or namespace path (uses '::')
        let has_dot = has_token(&self.0, SyntaxKind::DOT);

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
    first_child_method!(target, QualifiedName);
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
                SyntaxKind::CHAINS_KW => return Some(SpecializationKind::FeatureChain),
                _ => {}
            }
        }
        None
    }

    /// Check if this is a shorthand redefines (`:>>`) vs keyword (`redefines`)
    /// Returns true for `:>> name`, false for `redefines name`
    pub fn is_shorthand_redefines(&self) -> bool {
        has_token(&self.0, SyntaxKind::COLON_GT_GT)
    }

    first_child_method!(target, QualifiedName);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializationKind {
    Specializes,
    Subsets,
    Redefines,
    References,
    Conjugates,
    /// Feature chaining via `::>` shorthand or `chains` keyword.
    /// Per SysML v2 Spec §7.3.4.5: indicates a feature chain relationship.
    /// e.g., `feature x ::> a.b` or `feature self subsets things chains things.that`
    FeatureChain,
}

// ============================================================================
// From-To Clause (for message/flow usages)
// ============================================================================

ast_node!(FromToClause, FROM_TO_CLAUSE);

impl FromToClause {
    first_child_method!(source, FromToSource);
    first_child_method!(target, FromToTarget);
}

ast_node!(FromToSource, FROM_TO_SOURCE);

impl FromToSource {
    first_child_method!(target, QualifiedName);
}

ast_node!(FromToTarget, FROM_TO_TARGET);

impl FromToTarget {
    first_child_method!(target, QualifiedName);
}

// ============================================================================
// Transition Usage
// ============================================================================

ast_node!(TransitionUsage, TRANSITION_USAGE);

impl TransitionUsage {
    /// Get the transition name (if explicitly named before 'first' keyword)
    /// e.g., `transition T first S1 then S2` returns Some(T)
    /// but `transition first S1 accept s then S2` returns None (s is the accept payload, not the name)
    pub fn name(&self) -> Option<Name> {
        use crate::parser::SyntaxKind;
        // Only return a NAME that appears before FIRST_KW, ACCEPT_KW, or other transition body keywords
        for child in self.0.children_with_tokens() {
            match &child {
                rowan::NodeOrToken::Token(t) => {
                    // If we hit first/accept/then/do/if/via before finding a name, there's no name
                    match t.kind() {
                        SyntaxKind::FIRST_KW
                        | SyntaxKind::ACCEPT_KW
                        | SyntaxKind::THEN_KW
                        | SyntaxKind::DO_KW
                        | SyntaxKind::IF_KW
                        | SyntaxKind::VIA_KW => return None,
                        _ => {}
                    }
                }
                rowan::NodeOrToken::Node(n) => {
                    if let Some(name) = Name::cast(n.clone()) {
                        return Some(name);
                    }
                }
            }
        }
        None
    }

    /// Get all specializations (source and target states)
    pub fn specializations(&self) -> impl Iterator<Item = Specialization> + '_ {
        self.0.children().filter_map(Specialization::cast)
    }

    source_target_pair!(source, target, specializations, Specialization);

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

    first_child_method!(accept_typing, Typing);

    child_after_keyword_method!(accept_via, QualifiedName, VIA_KW,
        "Get the 'via' target for the accept trigger (e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`).");
}

// ============================================================================
// Perform Action Usage
// ============================================================================

ast_node!(PerformActionUsage, PERFORM_ACTION_USAGE);

impl PerformActionUsage {
    first_child_method!(name, Name);
    first_child_method!(typing, Typing);
    children_method!(specializations, Specialization);

    /// Get the performed action (first specialization, the action being performed)
    pub fn performed(&self) -> Option<Specialization> {
        self.specializations().next()
    }

    first_child_method!(body, NamespaceBody);
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

    first_child_method!(trigger, Expression);
    first_child_method!(accepted, QualifiedName);

    child_after_keyword_method!(via, QualifiedName, VIA_KW,
        "Get the 'via' target port (e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`).");
}

// ============================================================================
// Send Action Usage
// ============================================================================

ast_node!(SendActionUsage, SEND_ACTION_USAGE);

impl SendActionUsage {
    first_child_method!(payload, Expression);
    children_method!(qualified_names, QualifiedName);
}

// ============================================================================
// For Loop Action Usage
// ============================================================================

ast_node!(ForLoopActionUsage, FOR_LOOP_ACTION_USAGE);

impl ForLoopActionUsage {
    first_child_method!(variable_name, Name);
    first_child_method!(typing, Typing);
    first_child_method!(body, NamespaceBody);
    body_members_method!();
}

// ============================================================================
// If Action Usage
// ============================================================================

ast_node!(IfActionUsage, IF_ACTION_USAGE);

impl IfActionUsage {
    /// Get descendant expressions (condition and then/else targets)
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        self.0.descendants().filter_map(Expression::cast)
    }

    /// Get qualified names (then/else action references)
    pub fn qualified_names(&self) -> impl Iterator<Item = QualifiedName> + '_ {
        self.0.children().filter_map(QualifiedName::cast)
    }

    /// Get the body of the if action (if it has one)
    pub fn body(&self) -> Option<NamespaceBody> {
        self.0.children().find_map(NamespaceBody::cast)
    }
}

// ============================================================================
// While Loop Action Usage
// ============================================================================

ast_node!(WhileLoopActionUsage, WHILE_LOOP_ACTION_USAGE);

impl WhileLoopActionUsage {
    /// Get descendant expressions (condition)
    pub fn expressions(&self) -> impl Iterator<Item = Expression> + '_ {
        self.0.descendants().filter_map(Expression::cast)
    }

    first_child_method!(body, NamespaceBody);
    body_members_method!();
}

// ============================================================================
// State Subaction (entry/do/exit)
// ============================================================================

ast_node!(StateSubaction, STATE_SUBACTION);

impl StateSubaction {
    find_token_kind_method!(kind, [ENTRY_KW, DO_KW, EXIT_KW],
        "Get the state subaction kind (entry, do, or exit).");

    first_child_method!(name, Name);
    first_child_method!(body, NamespaceBody);

    has_token_method!(is_entry, ENTRY_KW, "entry action initial;");
    has_token_method!(is_do, DO_KW, "do action running;");
    has_token_method!(is_exit, EXIT_KW, "exit action cleanup;");
}

// ============================================================================
// Control Node (fork, join, merge, decide)
// ============================================================================

ast_node!(ControlNode, CONTROL_NODE);

impl ControlNode {
    find_token_kind_method!(kind, [FORK_KW, JOIN_KW, MERGE_KW, DECIDE_KW],
        "Get the control node kind (fork, join, merge, or decide).");

    first_child_method!(name, Name);
    first_child_method!(body, NamespaceBody);

    has_token_method!(is_fork, FORK_KW, "fork forkNode;");
    has_token_method!(is_join, JOIN_KW, "join joinNode;");
    has_token_method!(is_merge, MERGE_KW, "merge mergeNode;");
    has_token_method!(is_decide, DECIDE_KW, "decide decideNode;");
}

// ============================================================================
// Requirement Verification (satisfy/verify)
// ============================================================================

ast_node!(RequirementVerification, REQUIREMENT_VERIFICATION);

impl RequirementVerification {
    has_token_method!(is_satisfy, SATISFY_KW, "satisfy requirement R;");
    has_token_method!(is_verify, VERIFY_KW, "verify requirement R;");
    has_token_method!(is_negated, NOT_KW, "not satisfy requirement R;");
    has_token_method!(is_asserted, ASSERT_KW, "assert satisfy requirement R;");
    first_child_method!(requirement, QualifiedName);
    first_child_method!(typing, Typing);

    child_after_keyword_method!(by_target, QualifiedName, BY_KW,
        "Get the 'by' target (e.g., `vehicle_b` in `satisfy R by vehicle_b`).");
}

// ============================================================================
// KerML Connector (standalone connector, not SysML Connection)
// ============================================================================

ast_node!(Connector, CONNECTOR);

impl Connector {
    first_child_method!(name, Name);
    first_child_method!(typing, Typing);
    first_child_method!(connector_part, ConnectorPart);

    /// Get connector endpoints directly
    /// Returns iterator over connector ends for `from ... to ...` or `connect ... to ...`
    /// Looks in both CONNECTOR_PART (if present) and direct CONNECTION_END children
    pub fn ends(&self) -> impl Iterator<Item = ConnectorEnd> + '_ {
        // First try CONNECTOR_PART, then direct CONNECTION_END children
        let from_part: Vec<_> = self
            .connector_part()
            .into_iter()
            .flat_map(|cp| cp.ends().collect::<Vec<_>>())
            .collect();

        let direct: Vec<_> = if from_part.is_empty() {
            self.0.children().filter_map(ConnectorEnd::cast).collect()
        } else {
            Vec::new()
        };

        from_part.into_iter().chain(direct)
    }

    first_child_method!(body, NamespaceBody);
}

// ============================================================================
// Connect Usage
// ============================================================================

ast_node!(ConnectUsage, CONNECT_USAGE);

impl ConnectUsage {
    first_child_method!(connector_part, ConnectorPart);
}

ast_node!(ConnectorPart, CONNECTOR_PART);

impl ConnectorPart {
    children_method!(ends, ConnectorEnd);
    source_target_pair!(source, target, ends, ConnectorEnd);
}

// ConnectorEnd can be either CONNECTION_END (KerML) or CONNECTOR_END (SysML)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectorEnd(SyntaxNode);

impl AstNode for ConnectorEnd {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CONNECTION_END || kind == SyntaxKind::CONNECTOR_END
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

    /// Get the endpoint name (LHS of ::> if present).
    /// For patterns like `cause1 ::> a`, returns `cause1`.
    /// For simple patterns like `comp.lugNutPort`, returns None.
    pub fn endpoint_name(&self) -> Option<QualifiedName> {
        // Check if we have a CONNECTOR_END_REFERENCE child
        if let Some(ref_node) = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::CONNECTOR_END_REFERENCE)
        {
            // Check if there's a ::> or references keyword
            let has_references = ref_node.children_with_tokens().any(|n| {
                n.kind() == SyntaxKind::COLON_COLON_GT || n.kind() == SyntaxKind::REFERENCES_KW
            });

            if has_references {
                // Return the first QN (endpoint name before ::>)
                return ref_node.children().filter_map(QualifiedName::cast).next();
            }
        }
        // No endpoint name in simple patterns
        None
    }
}

// ============================================================================
// Binding Connector
// ============================================================================

ast_node!(BindingConnector, BINDING_CONNECTOR);

impl BindingConnector {
    children_method!(qualified_names, QualifiedName);
    source_target_pair!(source, target, qualified_names, QualifiedName);
}

// ============================================================================
// Succession
// ============================================================================

ast_node!(Succession, SUCCESSION);

impl Succession {
    children_method!(items, SuccessionItem);
    source_target_pair!(source, target, items, SuccessionItem);
    children_method!(inline_usages, Usage);
}

ast_node!(SuccessionItem, SUCCESSION_ITEM);

impl SuccessionItem {
    first_child_method!(target, QualifiedName);
    first_child_method!(usage, Usage);
}

// ============================================================================
// Constraint Body
// ============================================================================

ast_node!(ConstraintBody, CONSTRAINT_BODY);

impl ConstraintBody {
    first_child_method!(expression, Expression);
    children_method!(members, NamespaceMember);
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
                    for child in children.iter().skip(i + 1) {
                        if let Some(qn_node) = child.as_node() {
                            if qn_node.kind() == SyntaxKind::QUALIFIED_NAME {
                                type_name = Some(qn_node.text().to_string());
                                break;
                            }
                        }
                    }

                    // Find ARGUMENT_LIST and extract named arguments
                    if let Some(type_name) = type_name {
                        for child in children.iter().skip(i + 1) {
                            if let Some(arg_list_node) = child.as_node() {
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

        // Check for bare identifier tokens that are not inside a QUALIFIED_NAME
        // This handles cases like `[n]` in multiplicities where `n` is just an IDENT
        for child in node.children_with_tokens() {
            match &child {
                rowan::NodeOrToken::Token(token) if token.kind() == SyntaxKind::IDENT => {
                    // Found a bare identifier - treat as a single-part chain
                    let parts = vec![(token.text().to_string(), token.text_range())];
                    chains.push(FeatureChainRef {
                        parts,
                        full_range: token.text_range(),
                    });
                }
                rowan::NodeOrToken::Node(child_node) => {
                    // Recurse into child nodes
                    self.collect_feature_chains(child_node, chains);
                }
                _ => {}
            }
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
