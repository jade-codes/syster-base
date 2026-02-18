//! Usage and metadata member extraction from AST.

use std::sync::Arc;

use crate::parser::{
    self, AstNode, Expression, MetadataUsage, Multiplicity, NamespaceMember, QualifiedName,
    SpecializationKind, SyntaxKind, Usage,
};
use rowan::TextRange;

use super::context::{ExtractionContext, strip_quotes};
use super::extract::extract_from_ast_member_into_symbols;
use super::helpers::{
    determine_usage_kind, extract_expression_chains, extract_hir_relationships,
    extract_metadata_from_ast_context, extract_type_refs,
    implicit_supertype_for_internal_usage_kind, make_chain_or_simple, rel_kind_to_anon_prefix,
};
use super::types::{
    ExtractedRel, FeatureChain, FeatureChainPart, HirSymbol, InternalUsageKind, RefKind, RelKind,
    RelTarget, SymbolKind, TypeRefKind, new_element_id,
};

/// Extract relationships from a Usage AST node into ExtractedRel values.
fn extract_usage_rels_from_ast(usage: &Usage) -> Vec<ExtractedRel> {
    let mut rels = Vec::new();

    // Typing (possibly from nested perform action)
    let typing = usage
        .typing()
        .or_else(|| usage.perform_action_usage().and_then(|p| p.typing()));
    if let Some(typing) = typing {
        if let Some(target) = typing.target() {
            rels.push(ExtractedRel {
                kind: RelKind::TypedBy,
                target: RelTarget::Simple(target.to_string()),
                range: Some(target.syntax().text_range()),
            });
        }
    }

    // Prefix metadata
    for prefix_meta in usage.prefix_metadata() {
        if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
            rels.push(ExtractedRel {
                kind: RelKind::Meta,
                target: RelTarget::Simple(name),
                range: Some(range),
            });
        }
    }

    // "of Type" clause
    if let Some(of_type) = usage.of_type() {
        rels.push(ExtractedRel {
            kind: RelKind::TypedBy,
            target: RelTarget::Simple(of_type.to_string()),
            range: Some(of_type.syntax().text_range()),
        });
    }

    // Specializations
    for spec in usage.specializations() {
        let rel_kind = match spec.kind() {
            Some(SpecializationKind::Specializes) => RelKind::Specializes,
            Some(SpecializationKind::Subsets) => RelKind::Subsets,
            Some(SpecializationKind::Redefines) => RelKind::Redefines,
            Some(SpecializationKind::References) => RelKind::References,
            Some(SpecializationKind::Conjugates) => RelKind::Specializes,
            Some(SpecializationKind::FeatureChain) => RelKind::FeatureChain,
            None => RelKind::Subsets, // Comma-continuation for usages
        };
        if let Some(target) = spec.target() {
            let target_str = target.to_string();
            let target_range = target.syntax().text_range();
            let rel_target = if target_str.contains('.') {
                let segments = target.segments_with_ranges();
                let parts: Vec<FeatureChainPart> = segments
                    .iter()
                    .map(|(name, range)| FeatureChainPart {
                        name: name.clone(),
                        range: Some(*range),
                    })
                    .collect();
                RelTarget::Chain(FeatureChain {
                    parts,
                    range: Some(target_range),
                })
            } else {
                RelTarget::Simple(target_str)
            };
            rels.push(ExtractedRel {
                kind: rel_kind,
                target: rel_target,
                range: Some(target_range),
            });
        }
    }

    // Expression chains (only NOT inside nested scopes)
    for expr in usage.descendants::<Expression>() {
        let mut is_in_nested_scope = false;
        let mut ancestor = expr.syntax().parent();
        let usage_range = usage.syntax().text_range();
        while let Some(ref node) = ancestor {
            if node.text_range() == usage_range {
                break;
            }
            if matches!(
                node.kind(),
                SyntaxKind::NAMESPACE_BODY | SyntaxKind::USAGE | SyntaxKind::DEFINITION
            ) {
                is_in_nested_scope = true;
                break;
            }
            ancestor = node.parent();
        }
        if is_in_nested_scope {
            continue;
        }

        extract_expression_chains(&expr, &mut rels);

        // Named constructor args
        for (type_name, arg_name, arg_range) in expr.named_constructor_args() {
            let parts = vec![
                FeatureChainPart {
                    name: type_name,
                    range: None,
                },
                FeatureChainPart {
                    name: arg_name,
                    range: Some(arg_range),
                },
            ];
            rels.push(ExtractedRel {
                kind: RelKind::Expression,
                target: RelTarget::Chain(FeatureChain {
                    parts,
                    range: Some(arg_range),
                }),
                range: Some(arg_range),
            });
        }
    }

    // From-to clause (for flow/message)
    if let Some(from_to) = usage.from_to_clause() {
        if let Some(source) = from_to.source() {
            if let Some(qn) = source.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::FlowSource,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        if let Some(target) = from_to.target() {
            if let Some(qn) = target.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::FlowTarget,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Direct flow endpoints
    let (direct_source, direct_target) = usage.direct_flow_endpoints();
    if let Some(qn) = direct_source {
        let target_str = qn.to_string();
        rels.push(ExtractedRel {
            kind: RelKind::FlowSource,
            target: make_chain_or_simple(&target_str, &qn),
            range: Some(qn.syntax().text_range()),
        });
    }
    if let Some(qn) = direct_target {
        let target_str = qn.to_string();
        rels.push(ExtractedRel {
            kind: RelKind::FlowTarget,
            target: make_chain_or_simple(&target_str, &qn),
            range: Some(qn.syntax().text_range()),
        });
    }

    // Transition source/target
    if let Some(transition) = usage.transition_usage() {
        if let Some(source_spec) = transition.source() {
            if let Some(qn) = source_spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::TransitionSource,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        if let Some(target_spec) = transition.target() {
            if let Some(qn) = target_spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::TransitionTarget,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Succession source/target
    if let Some(succession) = usage.succession() {
        let items: Vec<_> = succession.items().collect();
        if let Some(first) = items.first() {
            if let Some(qn) = first.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::SuccessionSource,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        for item in items.iter().skip(1) {
            if let Some(qn) = item.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::SuccessionTarget,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Perform action
    if let Some(perform) = usage.perform_action_usage() {
        if let Some(spec) = perform.performed() {
            if let Some(qn) = spec.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::Performs,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        // Additional redefines/subsets from perform action
        for spec in perform.specializations().skip(1) {
            let rel_kind = match spec.kind() {
                Some(SpecializationKind::Redefines) => RelKind::Redefines,
                Some(SpecializationKind::Subsets) => RelKind::Subsets,
                Some(SpecializationKind::Specializes) => RelKind::Specializes,
                Some(SpecializationKind::References) => RelKind::References,
                _ => continue,
            };
            if let Some(qn) = spec.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: rel_kind,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Satisfy/verify
    if let Some(req_ver) = usage.requirement_verification() {
        let kind = if req_ver.is_satisfy() {
            RelKind::Satisfies
        } else {
            RelKind::Verifies
        };
        if let Some(qn) = req_ver.requirement() {
            rels.push(ExtractedRel {
                kind,
                target: RelTarget::Simple(qn.to_string()),
                range: Some(qn.syntax().text_range()),
            });
        }
        if let Some(typing) = req_ver.typing() {
            if let Some(target) = typing.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }
        if let Some(by_target) = req_ver.by_target() {
            let target_str = by_target.to_string();
            rels.push(ExtractedRel {
                kind: RelKind::References,
                target: make_chain_or_simple(&target_str, &by_target),
                range: Some(by_target.syntax().text_range()),
            });
        }
    }

    // Connect endpoints
    let connector_part = if let Some(connect) = usage.connect_usage() {
        connect.connector_part()
    } else {
        usage.connector_part()
    };
    if let Some(part) = connector_part {
        for end in part.ends() {
            if end.endpoint_name().is_none() {
                if let Some(qn) = end.target() {
                    let target_str = qn.to_string();
                    rels.push(ExtractedRel {
                        kind: RelKind::ConnectTarget,
                        target: make_chain_or_simple(&target_str, &qn),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
            // Named endpoints are handled as children, not relationships
        }
    }

    // Bind endpoints
    if let Some(bind) = usage.binding_connector() {
        if let Some(qn) = bind.source() {
            let target_str = qn.to_string();
            rels.push(ExtractedRel {
                kind: RelKind::BindSource,
                target: make_chain_or_simple(&target_str, &qn),
                range: Some(qn.syntax().text_range()),
            });
        }
        if let Some(qn) = bind.target() {
            let target_str = qn.to_string();
            rels.push(ExtractedRel {
                kind: RelKind::BindTarget,
                target: make_chain_or_simple(&target_str, &qn),
                range: Some(qn.syntax().text_range()),
            });
        }
    }

    // Exhibit
    if usage.is_exhibit() {
        for spec in usage.specializations() {
            if let Some(qn) = spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::Exhibits,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Include
    if usage.is_include() {
        for spec in usage.specializations() {
            if let Some(qn) = spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::Includes,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Assert
    if usage.is_assert() && usage.requirement_verification().is_none() {
        for spec in usage.specializations() {
            if let Some(qn) = spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::Asserts,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Assume
    if usage.is_assume() {
        for spec in usage.specializations() {
            if let Some(qn) = spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::Assumes,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Require
    if usage.is_require() {
        for spec in usage.specializations() {
            if let Some(qn) = spec.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::Requires,
                    target: RelTarget::Simple(qn.to_string()),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    // Allocate
    if usage.is_allocate() {
        let qnames: Vec<_> = usage
            .syntax()
            .children()
            .filter_map(QualifiedName::cast)
            .collect();
        if !qnames.is_empty() {
            let source_str = qnames[0].to_string();
            rels.push(ExtractedRel {
                kind: RelKind::AllocateSource,
                target: make_chain_or_simple(&source_str, &qnames[0]),
                range: Some(qnames[0].syntax().text_range()),
            });
        }
        if qnames.len() >= 2 {
            let target_str = qnames[1].to_string();
            rels.push(ExtractedRel {
                kind: RelKind::AllocateTo,
                target: make_chain_or_simple(&target_str, &qnames[1]),
                range: Some(qnames[1].syntax().text_range()),
            });
        }
    }

    rels
}

/// Get children from a Usage AST node — handles perform/constraint/normal body.
fn usage_body_members(usage: &Usage) -> Vec<NamespaceMember> {
    if let Some(perform) = usage.perform_action_usage() {
        perform
            .body()
            .map(|b| b.members().collect())
            .unwrap_or_default()
    } else if let Some(constraint_body) = usage.constraint_body() {
        constraint_body.members().collect()
    } else {
        usage
            .body()
            .map(|b| b.members().collect())
            .unwrap_or_default()
    }
}

/// Collect connector endpoint children from a Usage AST node.
fn collect_endpoint_children_from_ast(
    usage: &Usage,
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
) {
    let connector_part = if let Some(connect) = usage.connect_usage() {
        connect.connector_part()
    } else {
        usage.connector_part()
    };
    if let Some(part) = connector_part {
        for end in part.ends() {
            if let Some(endpoint_qn) = end.endpoint_name() {
                let endpoint_name = endpoint_qn.to_string();
                let mut endpoint_rels = Vec::new();
                if let Some(target_qn) = end.target() {
                    let target_str = target_qn.to_string();
                    endpoint_rels.push(ExtractedRel {
                        kind: RelKind::References,
                        target: make_chain_or_simple(&target_str, &target_qn),
                        range: Some(target_qn.syntax().text_range()),
                    });
                }
                let type_refs = extract_type_refs(&endpoint_rels, &ctx.line_index);
                let relationships = extract_hir_relationships(&endpoint_rels, &ctx.line_index);
                let span = ctx.range_to_info(Some(endpoint_qn.syntax().text_range()));
                let qn = ctx.qualified_name(&endpoint_name);
                symbols.push(HirSymbol {
                    name: Arc::from(endpoint_name.as_str()),
                    short_name: None,
                    qualified_name: Arc::from(qn.as_str()),
                    element_id: new_element_id(),
                    kind: SymbolKind::from_usage_kind(InternalUsageKind::End),
                    file: ctx.file,
                    start_line: span.start_line,
                    start_col: span.start_col,
                    end_line: span.end_line,
                    end_col: span.end_col,
                    short_name_start_line: None,
                    short_name_start_col: None,
                    short_name_end_line: None,
                    short_name_end_col: None,
                    doc: None,
                    supertypes: Vec::new(),
                    relationships,
                    type_refs,
                    is_public: false,
                    view_data: None,
                    metadata_annotations: Vec::new(),
                    is_abstract: false,
                    is_variation: false,
                    is_readonly: false,
                    is_derived: false,
                    is_parallel: false,
                    is_individual: false,
                    is_end: true,
                    is_default: false,
                    is_ordered: false,
                    is_nonunique: false,
                    is_portion: false,
                    direction: None,
                    multiplicity: None,
                    value: None,
                });
            }
        }
    }
}

/// Collect transition accept payload as a child.
fn collect_transition_payload_from_ast(
    usage: &Usage,
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
) {
    if let Some(trans) = usage.transition_usage() {
        if let Some(accept_name) = trans.accept_payload_name() {
            let payload_text = accept_name.text();
            let payload_short = accept_name.short_name().and_then(|sn| sn.text());
            let payload_range = accept_name.syntax().text_range();
            let payload_short_range = accept_name.short_name().map(|sn| sn.syntax().text_range());

            let mut payload_rels = Vec::new();
            if let Some(typing) = trans.accept_typing() {
                if let Some(target) = typing.target() {
                    payload_rels.push(ExtractedRel {
                        kind: RelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
            if let Some(via_target) = trans.accept_via() {
                let target_str = via_target.to_string();
                payload_rels.push(ExtractedRel {
                    kind: RelKind::AcceptVia,
                    target: make_chain_or_simple(&target_str, &via_target),
                    range: Some(via_target.syntax().text_range()),
                });
            }

            let type_refs = extract_type_refs(&payload_rels, &ctx.line_index);
            let relationships = extract_hir_relationships(&payload_rels, &ctx.line_index);
            let supertypes: Vec<Arc<str>> = payload_rels
                .iter()
                .filter(|r| matches!(r.kind, RelKind::TypedBy))
                .map(|r| Arc::from(r.target.as_str().as_ref()))
                .collect();

            if let Some(name) = payload_text {
                let span = ctx.range_to_info(Some(payload_range));
                let (sn_start, sn_start_col, sn_end, sn_end_col) =
                    ctx.range_to_optional(payload_short_range);
                let qn = ctx.qualified_name(&name);
                symbols.push(HirSymbol {
                    name: Arc::from(name.as_str()),
                    short_name: payload_short.as_deref().map(Arc::from),
                    qualified_name: Arc::from(qn.as_str()),
                    element_id: new_element_id(),
                    kind: SymbolKind::from_usage_kind(InternalUsageKind::Accept),
                    file: ctx.file,
                    start_line: span.start_line,
                    start_col: span.start_col,
                    end_line: span.end_line,
                    end_col: span.end_col,
                    short_name_start_line: sn_start,
                    short_name_start_col: sn_start_col,
                    short_name_end_line: sn_end,
                    short_name_end_col: sn_end_col,
                    doc: None,
                    supertypes,
                    relationships,
                    type_refs,
                    is_public: false,
                    view_data: None,
                    metadata_annotations: Vec::new(),
                    is_abstract: false,
                    is_variation: false,
                    is_readonly: false,
                    is_derived: false,
                    is_parallel: false,
                    is_individual: false,
                    is_end: false,
                    is_default: false,
                    is_ordered: false,
                    is_nonunique: false,
                    is_portion: false,
                    direction: None,
                    multiplicity: None,
                    value: None,
                });
            }
        }
    }
}

/// Extract a usage symbol directly from the AST Usage node.
pub(super) fn extract_usage_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    usage: &Usage,
) {
    let usage_kind = determine_usage_kind(usage);
    let rels = extract_usage_rels_from_ast(usage);
    let type_refs = extract_type_refs(&rels, &ctx.line_index);
    let relationships = extract_hir_relationships(&rels, &ctx.line_index);

    let body_members = usage_body_members(usage);
    let metadata_annotations =
        extract_metadata_from_ast_context(&rels, body_members.iter().cloned());

    // For nested special usages (transition, perform), the NAME is inside the
    // inner node (TRANSITION_USAGE, PERFORM_ACTION_USAGE), not on the outer
    // USAGE wrapper. Resolve the effective name once and use it throughout.
    //
    // Multi-name handling: KerML patterns like `end self2 [1] feature sameThing: Anything`
    // have two Name children. The first is an identification/short name, and the second
    // (after the `feature` keyword) is the actual feature name.
    let all_names = usage.names();
    let (effective_name, multi_name_short) = if all_names.len() >= 2 {
        // Multiple names: second is the feature name, first is the short name
        (Some(all_names[1].clone()), Some(all_names[0].clone()))
    } else {
        let name = usage.name().or_else(|| {
            if let Some(trans) = usage.transition_usage() {
                trans.name()
            } else if let Some(perf) = usage.perform_action_usage() {
                perf.name()
            } else {
                None
            }
        });
        (name, None)
    };

    // Shorthand redefines naming: In SysML, `:>> name` (shorthand redefines)
    // on an otherwise anonymous usage effectively names that usage with `name`.
    // e.g., `:>> samples : TimeStateRecord` → usage named "samples".
    // This matches the normalize layer behavior. Only applies to `:>>` shorthand,
    // not the `redefines` keyword, and only for simple names (not qualified).
    let shorthand_redefines_name: Option<(String, TextRange)> =
        if effective_name.as_ref().and_then(|n| n.text()).is_none() {
            usage.specializations().find_map(|spec| {
                if spec.is_shorthand_redefines() {
                    spec.target().and_then(|t| {
                        let target_str = t.to_string();
                        if !target_str.contains("::") && !target_str.contains('.') {
                            Some((target_str, t.syntax().text_range()))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
        } else {
            None
        };

    // Anonymous usage handling
    let name = if let Some((ref redef_name, _)) = shorthand_redefines_name {
        strip_quotes(redef_name)
    } else {
        match effective_name.as_ref().and_then(|n| n.text()) {
            Some(n) => strip_quotes(&n),
            None => {
                // Attach typing refs to parent for anonymous usages
                if !type_refs.is_empty() {
                    if let Some(parent) = symbols
                        .iter_mut()
                        .rev()
                        .find(|s| s.qualified_name.as_ref() == ctx.prefix)
                    {
                        if parent.kind != SymbolKind::Package {
                            let typing_refs: Vec<_> = type_refs
                            .iter()
                            .filter(
                                |tr| {
                                    matches!(tr, TypeRefKind::Simple(r) if r.kind == RefKind::TypedBy)
                                },
                            )
                            .cloned()
                            .collect();
                            parent.type_refs.extend(typing_refs);
                        }
                    }
                }

                // Generate unique anonymous scope
                let line = ctx
                    .line_index
                    .line_col(usage.syntax().text_range().start())
                    .line;
                let anon_scope = rels
                    .iter()
                    .find(|r| !matches!(r.kind, RelKind::Expression))
                    .map(|r| {
                        let prefix = rel_kind_to_anon_prefix(r.kind);
                        ctx.next_anon_scope(prefix, &r.target.as_str(), line)
                    })
                    .unwrap_or_else(|| ctx.next_anon_scope("anon", "", line));

                let qualified_name = ctx.qualified_name(&anon_scope);
                let kind = SymbolKind::from_usage_kind(usage_kind);
                let anon_span_range = rels
                    .iter()
                    .find(|r| !matches!(r.kind, RelKind::Expression))
                    .and_then(|r| r.range)
                    .or(Some(usage.syntax().text_range()));
                let span = ctx.range_to_info(anon_span_range);

                // Build supertypes for anonymous symbol
                let mut anon_supertypes: Vec<Arc<str>> = rels
                    .iter()
                    .filter(|r| {
                        matches!(
                            r.kind,
                            RelKind::TypedBy
                                | RelKind::Subsets
                                | RelKind::Specializes
                                | RelKind::Redefines
                                | RelKind::Satisfies
                                | RelKind::Verifies
                        )
                    })
                    .map(|r| Arc::from(r.target.as_str().as_ref()))
                    .collect();

                let is_expression_scope =
                    rels.iter().all(|r| matches!(r.kind, RelKind::Expression));
                let is_connection_kind = matches!(
                    usage_kind,
                    InternalUsageKind::Connection
                        | InternalUsageKind::Flow
                        | InternalUsageKind::Interface
                        | InternalUsageKind::Allocation
                );

                if !is_expression_scope && !is_connection_kind {
                    if let Some(parent) = symbols
                        .iter()
                        .rev()
                        .find(|s| s.qualified_name.as_ref() == ctx.prefix)
                    {
                        for supertype in &parent.supertypes {
                            if !anon_supertypes.contains(supertype) {
                                anon_supertypes.push(supertype.clone());
                            }
                        }
                    }
                }

                symbols.push(HirSymbol {
                    file: ctx.file,
                    name: Arc::from(anon_scope.as_str()),
                    short_name: None,
                    qualified_name: Arc::from(qualified_name.as_str()),
                    element_id: new_element_id(),
                    kind,
                    start_line: span.start_line,
                    start_col: span.start_col,
                    end_line: span.end_line,
                    end_col: span.end_col,
                    short_name_start_line: None,
                    short_name_start_col: None,
                    short_name_end_line: None,
                    short_name_end_col: None,
                    supertypes: anon_supertypes,
                    relationships: relationships.clone(),
                    type_refs,
                    doc: None,
                    is_public: false,
                    view_data: None,
                    metadata_annotations: metadata_annotations.clone(),
                    is_abstract: usage.is_abstract(),
                    is_variation: usage.is_variation(),
                    is_readonly: usage.is_readonly(),
                    is_derived: usage.is_derived(),
                    is_parallel: usage.is_parallel(),
                    is_individual: usage.is_individual(),
                    is_end: usage.is_end(),
                    is_default: usage.is_default(),
                    is_ordered: usage.is_ordered(),
                    is_nonunique: usage.is_nonunique(),
                    is_portion: usage.is_portion(),
                    direction: usage.direction(),
                    multiplicity: usage.multiplicity().map(|(lo, hi)| Multiplicity {
                        lower: lo,
                        upper: hi,
                    }),
                    value: None,
                });

                ctx.push_scope(&anon_scope);
                collect_endpoint_children_from_ast(usage, symbols, ctx);
                collect_transition_payload_from_ast(usage, symbols, ctx);
                for child in &body_members {
                    extract_from_ast_member_into_symbols(symbols, ctx, child);
                }
                ctx.pop_scope();
                return;
            }
        }
    };

    // Named usage
    let qualified_name = ctx.qualified_name(&name);
    let kind = SymbolKind::from_usage_kind(usage_kind);
    // For shorthand redefines, use the redefines target range as the name range
    let name_range = if let Some((_, ref range)) = shorthand_redefines_name {
        Some(*range)
    } else {
        effective_name.as_ref().map(|n| n.syntax().text_range())
    };

    // For multi-name usages (e.g., `end self2 [1] feature sameThing`), the short name
    // comes from the first Name (identification), not from the feature name's .short_name().
    let short_name_range = if let Some(ref id_name) = multi_name_short {
        Some(id_name.syntax().text_range())
    } else {
        effective_name
            .as_ref()
            .and_then(|n| n.short_name())
            .map(|sn| sn.syntax().text_range())
    };
    let span = ctx.range_to_info(name_range.or(Some(usage.syntax().text_range())));
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) =
        ctx.range_to_optional(short_name_range);

    let short_name = if let Some(ref id_name) = multi_name_short {
        id_name.text()
    } else {
        effective_name
            .as_ref()
            .and_then(|n| n.short_name())
            .and_then(|sn| sn.text())
    };

    // Build supertypes
    let mut supertypes: Vec<Arc<str>> = rels
        .iter()
        .filter(|r| {
            matches!(
                r.kind,
                RelKind::TypedBy
                    | RelKind::Subsets
                    | RelKind::Specializes
                    | RelKind::Redefines
                    | RelKind::Performs
                    | RelKind::Exhibits
                    | RelKind::Includes
                    | RelKind::Satisfies
                    | RelKind::Asserts
                    | RelKind::Verifies
            )
        })
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();

    // Implicit redefinition detection
    if supertypes.is_empty() && !ctx.prefix.is_empty() {
        if let Some(parent) = symbols
            .iter()
            .rev()
            .find(|s| s.qualified_name.as_ref() == ctx.prefix)
        {
            if let Some(parent_type) = parent.supertypes.first() {
                let parent_type_qualified = symbols
                    .iter()
                    .find(|s| {
                        s.name.as_ref() == parent_type.as_ref()
                            || s.qualified_name.as_ref() == parent_type.as_ref()
                    })
                    .map(|s| s.qualified_name.clone());

                if let Some(type_qname) = parent_type_qualified {
                    let potential_redef = format!("{}::{}", type_qname, name);
                    if symbols
                        .iter()
                        .any(|s| s.qualified_name.as_ref() == potential_redef)
                    {
                        supertypes.push(Arc::from(potential_redef));
                    }
                }
            }
        }
    }

    if supertypes.is_empty() {
        if let Some(implicit) = implicit_supertype_for_internal_usage_kind(usage_kind) {
            supertypes.push(Arc::from(implicit));
        }
    }

    let doc = parser::extract_doc_comment(usage.syntax()).map(|s| Arc::from(s.trim()));

    // View data
    let typed_by = supertypes.first();
    let view_data = match usage_kind {
        InternalUsageKind::View => {
            use crate::hir::views::{ViewData, ViewUsage};
            Some(ViewData::ViewUsage(ViewUsage::new(typed_by.cloned())))
        }
        InternalUsageKind::Viewpoint => Some(crate::hir::views::ViewData::ViewpointUsage(
            crate::hir::views::ViewpointUsage {
                viewpoint_def: typed_by.cloned(),
                span: None,
            },
        )),
        InternalUsageKind::Rendering => Some(crate::hir::views::ViewData::RenderingUsage(
            crate::hir::views::RenderingUsage {
                rendering_def: typed_by.cloned(),
                span: None,
            },
        )),
        _ => None,
    };

    // Value expression
    let value = usage
        .value_expression()
        .map(|e| crate::parser::extract_value_expression(&e));

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: short_name.as_deref().map(Arc::from),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: sn_start_line,
        short_name_start_col: sn_start_col,
        short_name_end_line: sn_end_line,
        short_name_end_col: sn_end_col,
        doc,
        supertypes,
        relationships,
        type_refs,
        is_public: false,
        view_data,
        metadata_annotations,
        is_abstract: usage.is_abstract(),
        is_variation: usage.is_variation(),
        is_readonly: usage.is_readonly(),
        is_derived: usage.is_derived(),
        is_parallel: usage.is_parallel(),
        is_individual: usage.is_individual(),
        is_end: usage.is_end(),
        is_default: usage.is_default(),
        is_ordered: usage.is_ordered(),
        is_nonunique: usage.is_nonunique(),
        is_portion: usage.is_portion(),
        direction: usage.direction(),
        multiplicity: usage.multiplicity().map(|(lo, hi)| Multiplicity {
            lower: lo,
            upper: hi,
        }),
        value,
    });

    // Recurse into children
    ctx.push_scope(&name);
    collect_endpoint_children_from_ast(usage, symbols, ctx);
    collect_transition_payload_from_ast(usage, symbols, ctx);
    for child in &body_members {
        extract_from_ast_member_into_symbols(symbols, ctx, child);
    }
    ctx.pop_scope();
}

/// Extract metadata member (@Type) directly from AST.
pub(super) fn extract_metadata_member_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    meta: &MetadataUsage,
) {
    let type_name = meta.target().map(|t| t.to_string()).unwrap_or_default();
    let mut rels = Vec::new();

    if !type_name.is_empty() {
        rels.push(ExtractedRel {
            kind: RelKind::TypedBy,
            target: RelTarget::Simple(type_name),
            range: meta.target().map(|t| t.syntax().text_range()),
        });
    }

    for qn in meta.about_targets() {
        let target_str = qn.to_string();
        rels.push(ExtractedRel {
            kind: RelKind::About,
            target: make_chain_or_simple(&target_str, &qn),
            range: Some(qn.syntax().text_range()),
        });
    }

    let type_refs = extract_type_refs(&rels, &ctx.line_index);
    let relationships = extract_hir_relationships(&rels, &ctx.line_index);

    let children: Vec<NamespaceMember> = meta
        .body()
        .map(|b| b.members().collect())
        .unwrap_or_default();
    let metadata_annotations = extract_metadata_from_ast_context(&rels, children.iter().cloned());

    // Metadata usages are anonymous — attach typing refs to parent
    if !type_refs.is_empty() {
        if let Some(parent) = symbols
            .iter_mut()
            .rev()
            .find(|s| s.qualified_name.as_ref() == ctx.prefix)
        {
            if parent.kind != SymbolKind::Package {
                let typing_refs: Vec<_> = type_refs
                    .iter()
                    .filter(|tr| matches!(tr, TypeRefKind::Simple(r) if r.kind == RefKind::TypedBy))
                    .cloned()
                    .collect();
                parent.type_refs.extend(typing_refs);
            }
        }
    }

    let line = ctx
        .line_index
        .line_col(meta.syntax().text_range().start())
        .line;
    let anon_scope = rels
        .iter()
        .find(|r| !matches!(r.kind, RelKind::Expression))
        .map(|r| {
            let prefix = rel_kind_to_anon_prefix(r.kind);
            ctx.next_anon_scope(prefix, &r.target.as_str(), line)
        })
        .unwrap_or_else(|| ctx.next_anon_scope("anon", "", line));

    let qualified_name = ctx.qualified_name(&anon_scope);
    let span = ctx.range_to_info(Some(meta.syntax().text_range()));

    let mut anon_supertypes: Vec<Arc<str>> = rels
        .iter()
        .filter(|r| {
            matches!(
                r.kind,
                RelKind::TypedBy | RelKind::Subsets | RelKind::Specializes
            )
        })
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();

    // Add parent supertypes
    if let Some(parent) = symbols
        .iter()
        .rev()
        .find(|s| s.qualified_name.as_ref() == ctx.prefix)
    {
        for supertype in &parent.supertypes {
            if !anon_supertypes.contains(supertype) {
                anon_supertypes.push(supertype.clone());
            }
        }
    }

    symbols.push(HirSymbol {
        file: ctx.file,
        name: Arc::from(anon_scope.as_str()),
        short_name: None,
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::from_usage_kind(InternalUsageKind::Attribute),
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        supertypes: anon_supertypes,
        relationships,
        type_refs,
        doc: None,
        is_public: false,
        view_data: None,
        metadata_annotations,
        is_abstract: false,
        is_variation: false,
        is_readonly: false,
        is_derived: false,
        is_parallel: false,
        is_individual: false,
        is_end: false,
        is_default: false,
        is_ordered: false,
        is_nonunique: false,
        is_portion: false,
        direction: None,
        multiplicity: None,
        value: None,
    });

    if !children.is_empty() {
        ctx.push_scope(&anon_scope);
        for child in &children {
            extract_from_ast_member_into_symbols(symbols, ctx, child);
        }
        ctx.pop_scope();
    }
}
