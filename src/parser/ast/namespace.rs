use super::*;

// ============================================================================
// Root
// ============================================================================

ast_node!(SourceFile, SOURCE_FILE);

impl SourceFile {
    children_method!(members, NamespaceMember);
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
                        .collect()
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
    first_child_method!(target, QualifiedName);
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
    children_vec_method!(targets, QualifiedName);
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
    children_method!(qualified_names, QualifiedName);

    /// Get the source qualified name(s) - everything before "to"
    /// For `dependency a, b to c` returns [a, b]
    pub fn sources(&self) -> Vec<QualifiedName> {
        split_at_keyword(&self.0, SyntaxKind::TO_KW).0
    }

    /// Get the target qualified name - after "to"
    /// For `dependency a to c` returns c
    pub fn target(&self) -> Option<QualifiedName> {
        split_at_keyword::<QualifiedName>(&self.0, SyntaxKind::TO_KW)
            .1
            .into_iter()
            .next()
    }

    prefix_metadata_method!();
}

// ============================================================================
