//! AST → internal type conversion helpers.
//!
//! Functions that convert AST data into internal extraction types (RelTarget,
//! ExtractedRel, TypeRefKind, HirRelationship, etc.).

use std::sync::Arc;

use crate::parser::{
    DefinitionKind, Expression, QualifiedName, Usage, UsageKind,
};

use super::types::{
    ExtractedRel, FeatureChain, FeatureChainPart, HirRelationship, InternalUsageKind, RefKind,
    RelKind, RelTarget, RelationshipKind, TypeRef, TypeRefChain, TypeRefKind,
};

/// Helper to create a chain or simple RelTarget from a dotted qualified name.
pub(super) fn make_chain_or_simple(target_str: &str, qn: &QualifiedName) -> RelTarget {
    use crate::parser::AstNode;
    if target_str.contains('.') {
        let segments_with_ranges = qn.segments_with_ranges();
        let parts: Vec<FeatureChainPart> = segments_with_ranges
            .into_iter()
            .map(|(name, range)| FeatureChainPart {
                name,
                range: Some(range),
            })
            .collect();
        RelTarget::Chain(FeatureChain {
            parts,
            range: Some(qn.syntax().text_range()),
        })
    } else {
        RelTarget::Simple(target_str.to_string())
    }
}

/// Extract feature chain expression references from an Expression AST node.
pub(super) fn extract_expression_chains(
    expr: &Expression,
    relationships: &mut Vec<ExtractedRel>,
) {
    for chain in expr.feature_chains() {
        if chain.parts.len() == 1 {
            let (name, range) = &chain.parts[0];
            relationships.push(ExtractedRel {
                kind: RelKind::Expression,
                target: RelTarget::Simple(name.clone()),
                range: Some(*range),
            });
        } else {
            let parts: Vec<FeatureChainPart> = chain
                .parts
                .iter()
                .map(|(name, range)| FeatureChainPart {
                    name: name.clone(),
                    range: Some(*range),
                })
                .collect();
            relationships.push(ExtractedRel {
                kind: RelKind::Expression,
                target: RelTarget::Chain(FeatureChain {
                    parts,
                    range: Some(chain.full_range),
                }),
                range: Some(chain.full_range),
            });
        }
    }
}

/// Determine the internal usage kind for a Usage AST node.
pub(super) fn determine_usage_kind(usage: &Usage) -> InternalUsageKind {
    // Check for nested transition first, then perform action
    if usage.transition_usage().is_some() {
        InternalUsageKind::Transition
    } else if usage.perform_action_usage().is_some() {
        InternalUsageKind::Action
    } else {
        match usage.usage_kind() {
            Some(UsageKind::Part) => InternalUsageKind::Part,
            Some(UsageKind::Attribute) => InternalUsageKind::Attribute,
            Some(UsageKind::Port) => InternalUsageKind::Port,
            Some(UsageKind::Item) => InternalUsageKind::Item,
            Some(UsageKind::Action) => InternalUsageKind::Action,
            Some(UsageKind::State) => InternalUsageKind::State,
            Some(UsageKind::Constraint) => InternalUsageKind::Constraint,
            Some(UsageKind::Requirement) => InternalUsageKind::Requirement,
            Some(UsageKind::Calc) => InternalUsageKind::Calculation,
            Some(UsageKind::Connection) => InternalUsageKind::Connection,
            Some(UsageKind::Interface) => InternalUsageKind::Interface,
            Some(UsageKind::Allocation) => InternalUsageKind::Allocation,
            Some(UsageKind::Flow) => InternalUsageKind::Flow,
            Some(UsageKind::Occurrence) => InternalUsageKind::Occurrence,
            Some(UsageKind::Ref) => InternalUsageKind::Reference,
            Some(UsageKind::Feature) => InternalUsageKind::Attribute,
            Some(UsageKind::Step) => InternalUsageKind::Action,
            Some(UsageKind::Expr) => InternalUsageKind::Calculation,
            Some(UsageKind::Connector) => InternalUsageKind::Connection,
            Some(UsageKind::Case) => InternalUsageKind::Other,
            None => InternalUsageKind::Part, // Default to Part for usages without keyword
        }
    }
}

/// Map DefinitionKind to implicit supertype name.
pub(super) fn implicit_supertype_for_definition_kind(kind: Option<DefinitionKind>) -> Option<&'static str> {
    match kind {
        Some(DefinitionKind::Part) | Some(DefinitionKind::Class) | Some(DefinitionKind::Struct)
        | Some(DefinitionKind::Classifier) => Some("Parts::Part"),
        Some(DefinitionKind::Item) => Some("Items::Item"),
        Some(DefinitionKind::Action) | Some(DefinitionKind::Behavior)
        | Some(DefinitionKind::Interaction) => Some("Actions::Action"),
        Some(DefinitionKind::State) => Some("States::StateAction"),
        Some(DefinitionKind::Constraint) | Some(DefinitionKind::Predicate) => {
            Some("Constraints::ConstraintCheck")
        }
        Some(DefinitionKind::Requirement) => Some("Requirements::RequirementCheck"),
        Some(DefinitionKind::Calc) | Some(DefinitionKind::Function) => {
            Some("Calculations::Calculation")
        }
        Some(DefinitionKind::Port) => Some("Ports::Port"),
        Some(DefinitionKind::Connection) | Some(DefinitionKind::Assoc) => {
            Some("Connections::BinaryConnection")
        }
        Some(DefinitionKind::Interface) => Some("Interfaces::Interface"),
        Some(DefinitionKind::Allocation) => Some("Allocations::Allocation"),
        Some(DefinitionKind::UseCase) | Some(DefinitionKind::Case) => Some("UseCases::UseCase"),
        Some(DefinitionKind::Analysis) | Some(DefinitionKind::Verification) => {
            Some("AnalysisCases::AnalysisCase")
        }
        Some(DefinitionKind::Attribute) | Some(DefinitionKind::Datatype) => {
            Some("Attributes::AttributeValue")
        }
        _ => None,
    }
}

/// Map InternalUsageKind to implicit supertype name.
pub(super) fn implicit_supertype_for_internal_usage_kind(kind: InternalUsageKind) -> Option<&'static str> {
    match kind {
        InternalUsageKind::Part => Some("Parts::Part"),
        InternalUsageKind::Item => Some("Items::Item"),
        InternalUsageKind::Action => Some("Actions::Action"),
        InternalUsageKind::State => Some("States::StateAction"),
        InternalUsageKind::Flow => Some("Flows::Message"),
        InternalUsageKind::Connection => Some("Connections::Connection"),
        InternalUsageKind::Interface => Some("Interfaces::Interface"),
        InternalUsageKind::Allocation => Some("Allocations::Allocation"),
        InternalUsageKind::Requirement => Some("Requirements::RequirementCheck"),
        InternalUsageKind::Constraint => Some("Constraints::ConstraintCheck"),
        InternalUsageKind::Calculation => Some("Calculations::Calculation"),
        InternalUsageKind::Port => Some("Ports::Port"),
        InternalUsageKind::Attribute => Some("Attributes::AttributeValue"),
        _ => None,
    }
}

/// Extract type references from ExtractedRel relationships.
pub(super) fn extract_type_refs(
    relationships: &[ExtractedRel],
    line_index: &crate::base::LineIndex,
) -> Vec<TypeRefKind> {
    let mut type_refs = Vec::new();

    for rel in relationships.iter() {
        let ref_kind = RefKind::from_rel_kind(rel.kind);

        match &rel.target {
            RelTarget::Chain(chain) => {
                let num_parts = chain.parts.len();
                let parts: Vec<TypeRef> = chain
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(idx, part)| {
                        let (start_line, start_col, end_line, end_col) = if let Some(r) = part.range
                        {
                            let start = line_index.line_col(r.start());
                            let end = line_index.line_col(r.end());
                            (start.line, start.col, end.line, end.col)
                        } else if idx == num_parts - 1 {
                            if let Some(r) = rel.range {
                                let start = line_index.line_col(r.start());
                                let end = line_index.line_col(r.end());
                                (start.line, start.col, end.line, end.col)
                            } else {
                                (0, 0, 0, 0)
                            }
                        } else {
                            (0, 0, 0, 0)
                        };
                        TypeRef {
                            target: Arc::from(part.name.as_str()),
                            resolved_target: None,
                            kind: ref_kind,
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        }
                    })
                    .collect();

                if !parts.is_empty() {
                    type_refs.push(TypeRefKind::Chain(TypeRefChain { parts }));
                }
            }
            RelTarget::Simple(target) => {
                if let Some(r) = rel.range {
                    let start = line_index.line_col(r.start());
                    let end = line_index.line_col(r.end());
                    type_refs.push(TypeRefKind::Simple(TypeRef {
                        target: Arc::from(target.as_str()),
                        resolved_target: None,
                        kind: ref_kind,
                        start_line: start.line,
                        start_col: start.col,
                        end_line: end.line,
                        end_col: end.col,
                    }));

                    // Also add prefix segments as references
                    let parts: Vec<&str> = target.split("::").collect();
                    if parts.len() > 1 {
                        let mut prefix = String::new();
                        for (i, part) in parts.iter().enumerate() {
                            if i == parts.len() - 1 {
                                break;
                            }
                            if !prefix.is_empty() {
                                prefix.push_str("::");
                            }
                            prefix.push_str(part);

                            type_refs.push(TypeRefKind::Simple(TypeRef {
                                target: Arc::from(prefix.as_str()),
                                resolved_target: None,
                                kind: ref_kind,
                                start_line: start.line,
                                start_col: start.col,
                                end_line: end.line,
                                end_col: end.col,
                            }));
                        }
                    }
                }
            }
        }
    }

    type_refs
}

/// Extract HirRelationship values from ExtractedRel relationships.
pub(super) fn extract_hir_relationships(
    relationships: &[ExtractedRel],
    line_index: &crate::base::LineIndex,
) -> Vec<HirRelationship> {
    relationships
        .iter()
        .filter_map(|rel| {
            RelationshipKind::from_rel_kind(rel.kind).map(|kind| {
                let (start_line, start_col, end_line, end_col) = rel
                    .range
                    .map(|r| {
                        let start = line_index.line_col(r.start());
                        let end = line_index.line_col(r.end());
                        (start.line, start.col, end.line, end.col)
                    })
                    .unwrap_or((0, 0, 0, 0));
                HirRelationship::with_span(
                    kind,
                    rel.target.as_str().as_ref(),
                    start_line,
                    start_col,
                    end_line,
                    end_col,
                )
            })
        })
        .collect()
}

/// Map a RelKind to a prefix string for anonymous scope naming.
pub(super) fn rel_kind_to_anon_prefix(kind: RelKind) -> &'static str {
    match kind {
        RelKind::Subsets => ":>",
        RelKind::TypedBy => ":",
        RelKind::Specializes => ":>:",
        RelKind::Redefines => ":>>",
        RelKind::About => "about:",
        RelKind::Performs => "perform:",
        RelKind::Satisfies => "satisfy:",
        RelKind::Exhibits => "exhibit:",
        RelKind::Includes => "include:",
        RelKind::Asserts => "assert:",
        RelKind::Verifies => "verify:",
        RelKind::References => "ref:",
        RelKind::Meta => "meta:",
        RelKind::Crosses => "crosses:",
        RelKind::Expression => "~",
        RelKind::FeatureChain => "chain:",
        RelKind::Conjugates => "~:",
        RelKind::TransitionSource => "from:",
        RelKind::TransitionTarget => "then:",
        RelKind::SuccessionSource => "first:",
        RelKind::SuccessionTarget => "then:",
        RelKind::AcceptedMessage => "accept:",
        RelKind::AcceptVia => "via:",
        RelKind::SentMessage => "send:",
        RelKind::SendVia => "via:",
        RelKind::SendTo => "to:",
        RelKind::MessageSource => "from:",
        RelKind::MessageTarget => "to:",
        RelKind::Assumes => "assume:",
        RelKind::Requires => "require:",
        RelKind::AllocateSource => "allocate:",
        RelKind::AllocateTo => "to:",
        RelKind::BindSource => "bind:",
        RelKind::BindTarget => "=:",
        RelKind::ConnectSource => "connect:",
        RelKind::ConnectTarget => "to:",
        RelKind::FlowItem => "flow:",
        RelKind::FlowSource => "from:",
        RelKind::FlowTarget => "to:",
        RelKind::InterfaceEnd => "end:",
        RelKind::Exposes => "expose:",
        RelKind::Renders => "render:",
        RelKind::Filters => "filter:",
        RelKind::DependencySource => "dep:",
        RelKind::DependencyTarget => "to:",
    }
}

/// Extract metadata annotations from ExtractedRel relationships and AST body members.
pub(super) fn extract_metadata_from_ast_context(
    rels: &[ExtractedRel],
    body_members: impl Iterator<Item = crate::parser::NamespaceMember>,
) -> Vec<Arc<str>> {
    let mut annotations = Vec::new();

    for rel in rels.iter() {
        if matches!(rel.kind, RelKind::Meta) {
            let target = rel.target.as_str();
            let simple_name = target.rsplit("::").next().unwrap_or(&target);
            annotations.push(Arc::from(simple_name));
        }
    }

    for member in body_members {
        if let crate::parser::NamespaceMember::Metadata(meta) = member {
            if let Some(target) = meta.target() {
                let name = target.to_string();
                let simple = name.rsplit("::").next().unwrap_or(&name);
                annotations.push(Arc::from(simple));
            }
        }
    }

    annotations
}
