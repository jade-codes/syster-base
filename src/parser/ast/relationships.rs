use super::*;

// Typing and Specialization
// ============================================================================

ast_node!(Typing, TYPING);

impl Typing {
    first_child_method!(target, QualifiedName);
}

ast_node!(Specialization, SPECIALIZATION);

impl Specialization {
    token_to_enum_method!(kind, SpecializationKind, [
        COLON_GT => Specializes,
        COLON_GT_GT => Redefines,
        COLON_COLON_GT => FeatureChain,
        SPECIALIZES_KW => Specializes,
        SUBSETS_KW => Subsets,
        REDEFINES_KW => Redefines,
        REFERENCES_KW => References,
        TILDE => Conjugates,
        FROM_KW => FeatureChain,
        TO_KW => FeatureChain,
        CHAINS_KW => FeatureChain,
    ]);

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

    children_method!(specializations, Specialization);
    source_target_pair!(source, target, specializations, Specialization);

    child_after_keyword_method!(
        accept_payload_name,
        Name,
        ACCEPT_KW,
        "Get the accept payload name (e.g., `ignitionCmd` in `accept ignitionCmd:IgnitionCmd`)."
    );

    first_child_method!(accept_typing, Typing);

    child_after_keyword_method!(
        accept_via,
        QualifiedName,
        VIA_KW,
        "Get the 'via' target for the accept trigger (e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`)."
    );
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
    first_child_method!(name, Name);
    first_child_method!(trigger, Expression);
    first_child_method!(accepted, QualifiedName);

    child_after_keyword_method!(
        via,
        QualifiedName,
        VIA_KW,
        "Get the 'via' target port (e.g., `ignitionCmdPort` in `accept ignitionCmd via ignitionCmdPort`)."
    );
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
    descendants_method!(
        expressions,
        Expression,
        "Get descendant expressions (condition and then/else targets)."
    );
    children_method!(qualified_names, QualifiedName);
    first_child_method!(body, NamespaceBody);
}

// ============================================================================
// While Loop Action Usage
// ============================================================================

ast_node!(WhileLoopActionUsage, WHILE_LOOP_ACTION_USAGE);

impl WhileLoopActionUsage {
    descendants_method!(
        expressions,
        Expression,
        "Get descendant expressions (condition)."
    );
    first_child_method!(body, NamespaceBody);
    body_members_method!();
}

// ============================================================================
// State Subaction (entry/do/exit)
// ============================================================================

ast_node!(StateSubaction, STATE_SUBACTION);

impl StateSubaction {
    find_token_kind_method!(
        kind,
        [ENTRY_KW, DO_KW, EXIT_KW],
        "Get the state subaction kind (entry, do, or exit)."
    );

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
    find_token_kind_method!(
        kind,
        [FORK_KW, JOIN_KW, MERGE_KW, DECIDE_KW],
        "Get the control node kind (fork, join, merge, or decide)."
    );

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

    child_after_keyword_method!(
        by_target,
        QualifiedName,
        BY_KW,
        "Get the 'by' target (e.g., `vehicle_b` in `satisfy R by vehicle_b`)."
    );
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
pub struct ConnectorEnd(pub(crate) SyntaxNode);

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
    /// Find the CONNECTOR_END_REFERENCE child and check if it has a ::> or references keyword.
    fn end_reference_info(&self) -> Option<(SyntaxNode, bool)> {
        let ref_node = self
            .0
            .children()
            .find(|n| n.kind() == SyntaxKind::CONNECTOR_END_REFERENCE)?;
        let has_references = ref_node.children_with_tokens().any(|n| {
            n.kind() == SyntaxKind::COLON_COLON_GT || n.kind() == SyntaxKind::REFERENCES_KW
        });
        Some((ref_node, has_references))
    }

    /// Get the qualified name target reference.
    /// For patterns like `p ::> comp.lugNutPort`, returns `comp.lugNutPort`.
    /// For simple patterns like `comp.lugNutPort`, returns `comp.lugNutPort`.
    pub fn target(&self) -> Option<QualifiedName> {
        if let Some((ref_node, has_references)) = self.end_reference_info() {
            let qns: Vec<_> = ref_node
                .children()
                .filter_map(QualifiedName::cast)
                .collect();
            // If there's a ::> or references keyword, return the second QN (the target)
            // Otherwise return the first/only QN
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
        if let Some((ref_node, has_references)) = self.end_reference_info() {
            if has_references {
                // Return the first QN (endpoint name before ::>)
                return ref_node.children().filter_map(QualifiedName::cast).next();
            }
        }
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
