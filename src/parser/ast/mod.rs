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
/// This includes contextual keywords that can appear as names in short name contexts like `<member>`.
/// Many SysML keywords can also be used as regular names in appropriate contexts
/// (e.g., `in frame : Integer` where `frame` is a feature name, not a keyword).
#[inline]
fn is_name_token(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::IDENT
            | SyntaxKind::START_KW
            | SyntaxKind::END_KW
            | SyntaxKind::DONE_KW
            | SyntaxKind::THIS_KW
            | SyntaxKind::MEMBER_KW
            | SyntaxKind::FRAME_KW // Used as name: `in frame : SpatialFrame`
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

/// Find the first token that can be used as a name (identifier or contextual keyword).
#[inline]
fn find_name_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| is_name_token(t.kind()))
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

/// Macro to generate a method that returns a Vec of children of a specific AST type.
///
/// Use this when the result needs to be collected (e.g., for iteration after borrowing ends).
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     children_vec_method!(targets, QualifiedName);
/// }
/// ```
macro_rules! children_vec_method {
    ($name:ident, $type:ident) => {
        #[doc = concat!("Get all `", stringify!($type), "` children of this node as a Vec.")]
        pub fn $name(&self) -> Vec<$type> {
            self.0.children().filter_map($type::cast).collect()
        }
    };
}

/// Macro to generate a method that returns an iterator over descendants of a specific AST type.
///
/// Unlike `children_method!`, this traverses the entire subtree, not just direct children.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     descendants_method!(expressions, Expression);
/// }
/// ```
macro_rules! descendants_method {
    ($name:ident, $type:ident) => {
        #[doc = concat!("Get all `", stringify!($type), "` descendants of this node.")]
        pub fn $name(&self) -> impl Iterator<Item = $type> + '_ {
            self.0.descendants().filter_map($type::cast)
        }
    };
    ($name:ident, $type:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(&self) -> impl Iterator<Item = $type> + '_ {
            self.0.descendants().filter_map($type::cast)
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

/// Macro to generate a method that maps token kinds to enum variants.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     token_to_enum_method!(direction, Direction, [
///         IN_KW => In,
///         OUT_KW => Out,
///         INOUT_KW => InOut,
///     ]);
/// }
/// ```
macro_rules! token_to_enum_method {
    ($name:ident, $enum_type:ident, [$($token:ident => $variant:ident),+ $(,)?]) => {
        pub fn $name(&self) -> Option<$enum_type> {
            for token in self.0.children_with_tokens().filter_map(|e| e.into_token()) {
                match token.kind() {
                    $(SyntaxKind::$token => return Some($enum_type::$variant),)+
                    _ => {}
                }
            }
            None
        }
    };
}

/// Macro to generate `prefix_metadata()` method that collects metadata from preceding siblings.
///
/// Usage:
/// ```ignore
/// impl MyStruct {
///     prefix_metadata_method!();
/// }
/// ```
macro_rules! prefix_metadata_method {
    () => {
        /// Get prefix metadata references from preceding siblings.
        pub fn prefix_metadata(&self) -> Vec<PrefixMetadata> {
            collect_prefix_metadata(&self.0)
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

/// Split children of a node at a keyword token.
///
/// Returns `(before, after)` where:
/// - `before` contains all nodes of type `T` before the keyword
/// - `after` contains all nodes of type `T` after the keyword
fn split_at_keyword<T: AstNode>(node: &SyntaxNode, keyword: SyntaxKind) -> (Vec<T>, Vec<T>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut found_keyword = false;

    for elem in node.children_with_tokens() {
        if let Some(token) = elem.as_token() {
            if token.kind() == keyword {
                found_keyword = true;
            }
        } else if let Some(child) = elem.as_node() {
            if let Some(item) = T::cast(child.clone()) {
                if found_keyword {
                    after.push(item);
                } else {
                    before.push(item);
                }
            }
        }
    }
    (before, after)
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
        pub struct $name(pub(crate) SyntaxNode);

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

// Submodules — declared after macros so macro_rules! are in scope
mod elements;
mod expressions;
mod namespace;
mod relationships;

// Re-export all public types so external code sees a flat namespace
pub use self::elements::*;
pub use self::expressions::*;
pub use self::namespace::*;
pub use self::relationships::*;

#[cfg(test)]
mod tests;
