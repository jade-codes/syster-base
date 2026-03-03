use super::*;

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
    /// Find the IDENT token (the name after #)
    fn ident_token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }

    /// Get the metadata type name (e.g., `mop` in `#mop attribute mass : Real;`)
    pub fn name(&self) -> Option<String> {
        self.ident_token().map(|t| t.text().to_string())
    }

    /// Get the text range of the identifier (for hover/goto)
    pub fn name_range(&self) -> Option<rowan::TextRange> {
        self.ident_token().map(|t| t.text_range())
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

    token_to_enum_method!(definition_kind, DefinitionKind, [
        PART_KW => Part,
        ATTRIBUTE_KW => Attribute,
        PORT_KW => Port,
        ITEM_KW => Item,
        ACTION_KW => Action,
        STATE_KW => State,
        CONSTRAINT_KW => Constraint,
        REQUIREMENT_KW => Requirement,
        CASE_KW => Case,
        CALC_KW => Calc,
        CONNECTION_KW => Connection,
        INTERFACE_KW => Interface,
        ALLOCATION_KW => Allocation,
        FLOW_KW => Flow,
        VIEW_KW => View,
        VIEWPOINT_KW => Viewpoint,
        RENDERING_KW => Rendering,
        METADATA_KW => Metadata,
        OCCURRENCE_KW => Occurrence,
        ENUM_KW => Enum,
        ANALYSIS_KW => Analysis,
        VERIFICATION_KW => Verification,
        USE_KW => UseCase,
        CONCERN_KW => Concern,
        // KerML definition keywords
        CLASS_KW => Class,
        STRUCT_KW => Struct,
        ASSOC_KW => Assoc,
        BEHAVIOR_KW => Behavior,
        FUNCTION_KW => Function,
        PREDICATE_KW => Predicate,
        INTERACTION_KW => Interaction,
        DATATYPE_KW => Datatype,
        CLASSIFIER_KW => Classifier,
        TYPE_KW => Type,
        METACLASS_KW => Metaclass,
    ]);

    first_child_method!(name, Name);
    children_method!(specializations, Specialization);
    first_child_method!(body, NamespaceBody);
    first_child_method!(constraint_body, ConstraintBody);

    body_members_method!();
    prefix_metadata_method!();
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
pub struct Usage(pub(crate) SyntaxNode);

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
    has_token_method!(
        is_individual,
        INDIVIDUAL_KW,
        "individual part earth : Earth;"
    );
    has_token_method!(is_end, END_KW, "end part wheel : Wheel[4];");
    has_token_method!(is_default, DEFAULT_KW, "default attribute rgb : RGB;");
    has_token_method!(is_ordered, ORDERED_KW, "ordered part wheels : Wheel[4];");
    has_token_method!(
        is_nonunique,
        NONUNIQUE_KW,
        "nonunique attribute scores : Integer[*];"
    );
    has_token_method!(is_portion, PORTION_KW, "portion part fuelLoad : Fuel;");

    token_to_enum_method!(direction, Direction, [
        IN_KW => In,
        OUT_KW => Out,
        INOUT_KW => InOut,
    ]);

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
    children_vec_method!(names, Name);

    first_child_method!(typing, Typing);

    child_after_keyword_method!(
        of_type,
        QualifiedName,
        OF_KW,
        "Get the 'of Type' qualified name for messages/items (e.g., `message sendCmd of SensedSpeed`)."
    );

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

    token_to_enum_method!(usage_kind, UsageKind, [
        PART_KW => Part,
        ATTRIBUTE_KW => Attribute,
        PORT_KW => Port,
        ITEM_KW => Item,
        ACTION_KW => Action,
        STATE_KW => State,
        CONSTRAINT_KW => Constraint,
        REQUIREMENT_KW => Requirement,
        CASE_KW => Case,
        CALC_KW => Calc,
        CONNECTION_KW => Connection,
        INTERFACE_KW => Interface,
        ALLOCATION_KW => Allocation,
        FLOW_KW => Flow,
        MESSAGE_KW => Flow,
        OCCURRENCE_KW => Occurrence,
        REF_KW => Ref,
        // KerML usage keywords
        FEATURE_KW => Feature,
        STEP_KW => Step,
        EXPR_KW => Expr,
        CONNECTOR_KW => Connector,
    ]);
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
        find_name_token(&self.0).map(|t| t.text().to_string())
    }
}

ast_node!(ShortName, SHORT_NAME);

impl ShortName {
    pub fn text(&self) -> Option<String> {
        find_name_token(&self.0).map(|t| strip_unrestricted_name(t.text()))
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
