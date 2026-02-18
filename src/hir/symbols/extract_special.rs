//! Special variant extraction helpers — bind, succession, connector, etc.
//!
//! Each NamespaceMember variant that isn't a plain Usage/Definition gets a
//! dedicated adapter function here. They all delegate to `push_special_usage_symbol`.

use std::sync::Arc;

use rowan::TextRange;

use crate::parser::{self, AstNode, NamespaceMember, SyntaxKind};

use super::context::{ExtractionContext, strip_quotes};
use super::extract::extract_from_ast_member_into_symbols;
use super::helpers::{
    extract_expression_chains, extract_hir_relationships, extract_metadata_from_ast_context,
    extract_type_refs, make_chain_or_simple, rel_kind_to_anon_prefix,
};
use super::types::{
    ExtractedRel, HirSymbol, InternalUsageKind, RefKind, RelKind, RelTarget, SymbolKind,
    TypeRefKind, new_element_id,
};

/// Common helper for special usage variants (bind, succession, connector, etc.).
///
/// These variants have simpler AST structure than general usages — no boolean
/// flags, no direction/multiplicity/value. This function handles the
/// anonymous/named split logic once, so each variant only needs to provide
/// the name, kind, relationships, range, body members, and optional doc.
#[allow(clippy::too_many_arguments)]
fn push_special_usage_symbol(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    name_node: Option<parser::Name>,
    kind: InternalUsageKind,
    rels: Vec<ExtractedRel>,
    syntax_range: TextRange,
    body_members: Vec<NamespaceMember>,
    doc: Option<String>,
) {
    let doc = doc.map(|d| Arc::from(d.as_str()));
    let type_refs = extract_type_refs(&rels, &ctx.line_index);
    let relationships = extract_hir_relationships(&rels, &ctx.line_index);
    let metadata_annotations =
        extract_metadata_from_ast_context(&rels, body_members.iter().cloned());

    let name_text = name_node.as_ref().and_then(|n| n.text());
    let name_text = name_text.map(|n| strip_quotes(&n));

    match name_text {
        Some(name) => {
            // Named special usage
            let qualified_name = ctx.qualified_name(&name);
            let sym_kind = SymbolKind::from_usage_kind(kind);
            let name_range = name_node.as_ref().map(|n| n.syntax().text_range());
            let short_name_range = name_node
                .as_ref()
                .and_then(|n| n.short_name())
                .map(|sn| sn.syntax().text_range());
            let span = ctx.range_to_info(name_range.or(Some(syntax_range)));
            let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) =
                ctx.range_to_optional(short_name_range);
            let short_name = name_node
                .as_ref()
                .and_then(|n| n.short_name())
                .and_then(|sn| sn.text());

            let supertypes: Vec<Arc<str>> = rels
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        RelKind::TypedBy
                            | RelKind::Subsets
                            | RelKind::Specializes
                            | RelKind::Redefines
                    )
                })
                .map(|r| Arc::from(r.target.as_str().as_ref()))
                .collect();

            symbols.push(HirSymbol {
                file: ctx.file,
                name: Arc::from(name.as_str()),
                short_name: short_name.map(|s| Arc::from(s.as_str())),
                qualified_name: Arc::from(qualified_name.as_str()),
                element_id: new_element_id(),
                kind: sym_kind,
                start_line: span.start_line,
                start_col: span.start_col,
                end_line: span.end_line,
                end_col: span.end_col,
                short_name_start_line: sn_start_line,
                short_name_start_col: sn_start_col,
                short_name_end_line: sn_end_line,
                short_name_end_col: sn_end_col,
                supertypes,
                relationships,
                type_refs,
                doc,
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

            ctx.push_scope(&name);
            for child in &body_members {
                extract_from_ast_member_into_symbols(symbols, ctx, child);
            }
            ctx.pop_scope();
        }
        None => {
            // Anonymous special usage
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

            let line = ctx.line_index.line_col(syntax_range.start()).line;
            let anon_scope = rels
                .iter()
                .find(|r| !matches!(r.kind, RelKind::Expression))
                .map(|r| {
                    let prefix = rel_kind_to_anon_prefix(r.kind);
                    ctx.next_anon_scope(prefix, &r.target.as_str(), line)
                })
                .unwrap_or_else(|| ctx.next_anon_scope("anon", "", line));

            let qualified_name = ctx.qualified_name(&anon_scope);
            let sym_kind = SymbolKind::from_usage_kind(kind);
            let anon_span_range = rels
                .iter()
                .find(|r| !matches!(r.kind, RelKind::Expression))
                .and_then(|r| r.range)
                .unwrap_or(syntax_range);
            let span = ctx.range_to_info(Some(anon_span_range));

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

            let is_expression_scope = rels.iter().all(|r| matches!(r.kind, RelKind::Expression));
            let is_connection_kind = matches!(
                kind,
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
                kind: sym_kind,
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
                doc,
                is_public: false,
                view_data: None,
                metadata_annotations: metadata_annotations.clone(),
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

            ctx.push_scope(&anon_scope);
            for child in &body_members {
                extract_from_ast_member_into_symbols(symbols, ctx, child);
            }
            ctx.pop_scope();
        }
    }
}

/// Extract a BindingConnector (bind x = y) directly from AST.
pub(super) fn extract_bind_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    bind: &parser::BindingConnector,
) {
    let mut rels = Vec::new();
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
    push_special_usage_symbol(
        symbols,
        ctx,
        None, // Bind statements are anonymous
        InternalUsageKind::Connection,
        rels,
        bind.syntax().text_range(),
        Vec::new(),
        None,
    );
}

/// Extract a Succession (first x then y) directly from AST.
pub(super) fn extract_succession_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    succ: &parser::Succession,
) {
    let mut rels = Vec::new();
    let mut body_members = Vec::new();

    let items: Vec<_> = succ.items().collect();
    if !items.is_empty() {
        // First item is the source
        if let Some(qn) = items[0].target() {
            let target_str = qn.to_string();
            rels.push(ExtractedRel {
                kind: RelKind::SuccessionSource,
                target: make_chain_or_simple(&target_str, &qn),
                range: Some(qn.syntax().text_range()),
            });
        } else if let Some(usage) = items[0].usage() {
            body_members.push(NamespaceMember::Usage(usage));
        }
    }
    // Remaining items are targets
    for item in items.iter().skip(1) {
        if let Some(qn) = item.target() {
            let target_str = qn.to_string();
            rels.push(ExtractedRel {
                kind: RelKind::SuccessionTarget,
                target: make_chain_or_simple(&target_str, &qn),
                range: Some(qn.syntax().text_range()),
            });
        } else if let Some(usage) = item.usage() {
            body_members.push(NamespaceMember::Usage(usage));
        }
    }
    // Inline usages directly inside succession (not wrapped in SUCCESSION_ITEM)
    for usage in succ.inline_usages() {
        body_members.push(NamespaceMember::Usage(usage));
    }
    // Accept and send actions inside succession
    for accept in succ
        .syntax()
        .children()
        .filter_map(parser::AcceptActionUsage::cast)
    {
        body_members.push(NamespaceMember::AcceptAction(accept));
    }
    for send in succ
        .syntax()
        .children()
        .filter_map(parser::SendActionUsage::cast)
    {
        body_members.push(NamespaceMember::SendAction(send));
    }

    // Compute a tighter range that excludes trailing whitespace
    let range = {
        let full_range = succ.syntax().text_range();
        let mut last_significant_end = full_range.start();
        for token in succ.syntax().descendants_with_tokens() {
            if let Some(tok) = token.as_token() {
                if tok.kind() != SyntaxKind::WHITESPACE
                    && tok.kind() != SyntaxKind::LINE_COMMENT
                    && tok.kind() != SyntaxKind::BLOCK_COMMENT
                {
                    let tok_end = tok.text_range().end();
                    if tok_end > last_significant_end {
                        last_significant_end = tok_end;
                    }
                }
            }
        }
        TextRange::new(full_range.start(), last_significant_end)
    };

    push_special_usage_symbol(
        symbols,
        ctx,
        None, // Succession statements are anonymous
        InternalUsageKind::Other,
        rels,
        range,
        body_members,
        None,
    );
}

/// Extract a bare TransitionUsage (target transitions) directly from AST.
/// Named transitions via `transition t first S1 then S2` go through
/// NamespaceMember::Usage → extract_usage_from_ast instead.
pub(super) fn extract_bare_transition_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    trans: &parser::TransitionUsage,
) {
    let mut rels = Vec::new();

    // Source (first specialization)
    if let Some(source_spec) = trans.source() {
        if let Some(qn) = source_spec.target() {
            rels.push(ExtractedRel {
                kind: RelKind::TransitionSource,
                target: RelTarget::Simple(qn.to_string()),
                range: Some(qn.syntax().text_range()),
            });
        }
    }
    // Target (second specialization)
    if let Some(target_spec) = trans.target() {
        if let Some(qn) = target_spec.target() {
            rels.push(ExtractedRel {
                kind: RelKind::TransitionTarget,
                target: RelTarget::Simple(qn.to_string()),
                range: Some(qn.syntax().text_range()),
            });
        }
    }
    // Accept typing and qualified names
    for child in trans.syntax().children() {
        if let Some(typing) = parser::Typing::cast(child.clone()) {
            if let Some(target) = typing.target() {
                rels.push(ExtractedRel {
                    kind: RelKind::TypedBy,
                    target: RelTarget::Simple(target.to_string()),
                    range: Some(target.syntax().text_range()),
                });
            }
        }
        if child.kind() == SyntaxKind::QUALIFIED_NAME {
            if let Some(qn) = parser::QualifiedName::cast(child.clone()) {
                let target_str = qn.to_string();
                let already_exists = rels
                    .iter()
                    .any(|r| matches!(&r.target, RelTarget::Simple(t) if t == &target_str));
                if !already_exists {
                    rels.push(ExtractedRel {
                        kind: RelKind::TransitionTarget,
                        target: RelTarget::Simple(target_str),
                        range: Some(qn.syntax().text_range()),
                    });
                }
            }
        }
    }
    // Implicit typing
    let has_typing = rels.iter().any(|r| matches!(r.kind, RelKind::TypedBy));
    if !has_typing {
        rels.push(ExtractedRel {
            kind: RelKind::TypedBy,
            target: RelTarget::Simple("Actions::TransitionAction".to_string()),
            range: None,
        });
    }

    // Accept payload is handled as an anonymous child via push_special_usage_symbol
    let body_members = Vec::new();

    let name_node = trans.name();

    push_special_usage_symbol(
        symbols,
        ctx,
        name_node,
        InternalUsageKind::Transition,
        rels,
        trans.syntax().text_range(),
        body_members,
        None,
    );

    // Extract accept payload as a child (after symbol is pushed and scope is set)
    // This needs special handling since it's not a NamespaceMember
    // For now, the payload is part of the parent's type_refs via TypedBy
}

/// Extract a KerML Connector directly from AST.
pub(super) fn extract_connector_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    conn: &parser::Connector,
) {
    let mut rels = Vec::new();

    if let Some(conn_part) = conn.connector_part() {
        let ends: Vec<_> = conn_part.ends().collect();
        if let Some(first) = ends.first() {
            if let Some(qn) = first.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::ConnectSource,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
        for end in ends.iter().skip(1) {
            if let Some(qn) = end.target() {
                let target_str = qn.to_string();
                rels.push(ExtractedRel {
                    kind: RelKind::ConnectTarget,
                    target: make_chain_or_simple(&target_str, &qn),
                    range: Some(qn.syntax().text_range()),
                });
            }
        }
    }

    let body_members: Vec<NamespaceMember> = conn
        .body()
        .map(|b| b.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        conn.name(),
        InternalUsageKind::Connection,
        rels,
        conn.syntax().text_range(),
        body_members,
        None,
    );
}

/// Extract a SysML ConnectUsage (connect x to y) directly from AST.
/// Handles both binary (connect a to b) and n-ary (connect (a, b, c, ...)) forms.
pub(super) fn extract_connect_usage_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    conn: &parser::ConnectUsage,
) {
    let mut rels = Vec::new();

    // Collect all endpoints, not just source/target (supports n-ary connectors)
    if let Some(conn_part) = conn.connector_part() {
        for end in conn_part.ends() {
            // Named endpoints (e.g., `cause1 ::> a`) are handled as children, not rels
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
        }
    }

    // Extract body members (connect can have a body with nested definitions)
    let body_members: Vec<NamespaceMember> = conn
        .syntax()
        .children()
        .find_map(parser::NamespaceBody::cast)
        .map(|body| body.members().collect())
        .unwrap_or_default();

    // Extract name from NAME child
    let name_node = conn.syntax().children().find_map(parser::Name::cast);

    // Track symbol count before push to find the parent scope afterwards
    let sym_count_before = symbols.len();

    push_special_usage_symbol(
        symbols,
        ctx,
        name_node,
        InternalUsageKind::Connection,
        rels,
        conn.syntax().text_range(),
        body_members,
        None,
    );

    // Extract named endpoint children (e.g., `cause1 ::> causer1` in `connection { end cause1 ::> causer1; }`)
    // We need to re-enter the scope that push_special_usage_symbol created and then popped
    if let Some(conn_part) = conn.connector_part() {
        let has_named_endpoints = conn_part.ends().any(|e| e.endpoint_name().is_some());
        if has_named_endpoints {
            // Find the scope name from the symbol that was just pushed
            if let Some(parent_sym) = symbols.get(sym_count_before) {
                let scope_name = parent_sym.name.to_string();
                ctx.push_scope(&scope_name);
                for end in conn_part.ends() {
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
                        let relationships =
                            extract_hir_relationships(&endpoint_rels, &ctx.line_index);
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
                ctx.pop_scope();
            }
        }
    }
}

/// Extract a SendActionUsage (send msg via port) directly from AST.
pub(super) fn extract_send_action_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    send: &parser::SendActionUsage,
) {
    let body_members: Vec<NamespaceMember> = send
        .syntax()
        .children()
        .find_map(parser::NamespaceBody::cast)
        .map(|body| body.members().collect())
        .unwrap_or_default();

    // Extract name — first inside the node, then check preceding sibling
    let mut name_node = send.syntax().children().find_map(parser::Name::cast);

    if name_node.is_none() {
        if let Some(prev_sibling) = send.syntax().prev_sibling() {
            name_node = parser::Name::cast(prev_sibling);
        }
    }

    push_special_usage_symbol(
        symbols,
        ctx,
        name_node,
        InternalUsageKind::Action,
        Vec::new(),
        send.syntax().text_range(),
        body_members,
        None,
    );
}

/// Extract an AcceptActionUsage (accept sig : Signal via port) directly from AST.
pub(super) fn extract_accept_action_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    accept: &parser::AcceptActionUsage,
) {
    let mut rels = Vec::new();

    // Typing for the accepted signal
    if let Some(typing) = accept.syntax().children().find_map(parser::Typing::cast) {
        if let Some(target) = typing.target() {
            rels.push(ExtractedRel {
                kind: RelKind::TypedBy,
                target: RelTarget::Simple(target.to_string()),
                range: Some(target.syntax().text_range()),
            });
        }
    }

    // Via port
    if let Some(via_target) = accept.via() {
        let target_str = via_target.to_string();
        rels.push(ExtractedRel {
            kind: RelKind::AcceptVia,
            target: make_chain_or_simple(&target_str, &via_target),
            range: Some(via_target.syntax().text_range()),
        });
    }

    // Name resolution: check preceding sibling first (for `action <name> accept ...`),
    // then check inside node
    let payload_type = accept
        .syntax()
        .children()
        .find_map(parser::Typing::cast)
        .and_then(|t| t.target().map(|qn| qn.to_string()));

    let mut name_node: Option<parser::Name> = None;

    // Check preceding sibling (for `action <name> accept ...` pattern)
    if let Some(prev_sibling) = accept.syntax().prev_sibling() {
        if let Some(sibling_name) = parser::Name::cast(prev_sibling) {
            name_node = Some(sibling_name);
            // The NAME inside is the payload name, not the action name
            if let Some(inner_name) = accept.syntax().children().find_map(parser::Name::cast) {
                // Create payload as a child by adding a synthetic NamespaceMember
                // We handle this via push_special_usage_symbol's body_members
                let payload_name_text = inner_name.text();
                if let Some(_pname) = payload_name_text {
                    // Build a payload child as accept with TypedBy
                    let mut payload_rels = Vec::new();
                    if let Some(ptype) = &payload_type {
                        payload_rels.push(ExtractedRel {
                            kind: RelKind::TypedBy,
                            target: RelTarget::Simple(ptype.clone()),
                            range: None,
                        });
                    }
                    // We can't easily create a synthetic NamespaceMember, so we'll manually
                    // push the payload child after push_special_usage_symbol
                }
            }
        }
    }

    // If no sibling name, check inside node
    if name_node.is_none() {
        name_node = accept.syntax().children().find_map(parser::Name::cast);
    }

    // Extract body members
    let body_members: Vec<NamespaceMember> = accept
        .syntax()
        .children()
        .find_map(parser::NamespaceBody::cast)
        .map(|body| body.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        name_node.clone(),
        InternalUsageKind::Action,
        rels,
        accept.syntax().text_range(),
        body_members,
        None,
    );

    // Handle payload child for `action trigger1 accept ignitionCmd : IgnitionCmd` pattern
    if let Some(prev_sibling) = accept.syntax().prev_sibling() {
        if parser::Name::cast(prev_sibling).is_some() {
            if let Some(inner_name) = accept.syntax().children().find_map(parser::Name::cast) {
                if let Some(pname) = inner_name.text() {
                    let name_str = name_node
                        .as_ref()
                        .and_then(|n| n.text())
                        .unwrap_or_default();
                    let name_str = strip_quotes(&name_str);
                    // If the preceding sibling gave us the name, inner_name is payload
                    // Push the payload as a child of the accept action
                    let mut payload_rels = Vec::new();
                    if let Some(ptype) = &payload_type {
                        payload_rels.push(ExtractedRel {
                            kind: RelKind::TypedBy,
                            target: RelTarget::Simple(ptype.clone()),
                            range: None,
                        });
                    }
                    // After push_special_usage_symbol, ctx.prefix is back to the parent scope.
                    // The payload is a child of the accept action, so include the action name.
                    let pname = strip_quotes(&pname);
                    let payload_qn = format!("{}::{}::{}", ctx.prefix, name_str, pname);
                    let payload_type_refs = extract_type_refs(&payload_rels, &ctx.line_index);
                    let payload_hir_rels =
                        extract_hir_relationships(&payload_rels, &ctx.line_index);
                    let payload_supertypes: Vec<Arc<str>> = payload_rels
                        .iter()
                        .filter(|r| matches!(r.kind, RelKind::TypedBy))
                        .map(|r| Arc::from(r.target.as_str().as_ref()))
                        .collect();
                    let payload_span = ctx.range_to_info(Some(inner_name.syntax().text_range()));

                    // Only add if the name was from a sibling (pattern 1)
                    // and the inner name is different from the outer name
                    if name_str != pname {
                        symbols.push(HirSymbol {
                            file: ctx.file,
                            name: Arc::from(pname.as_str()),
                            short_name: None,
                            qualified_name: Arc::from(payload_qn.as_str()),
                            element_id: new_element_id(),
                            kind: SymbolKind::ItemUsage,
                            start_line: payload_span.start_line,
                            start_col: payload_span.start_col,
                            end_line: payload_span.end_line,
                            end_col: payload_span.end_col,
                            short_name_start_line: None,
                            short_name_start_col: None,
                            short_name_end_line: None,
                            short_name_end_col: None,
                            supertypes: payload_supertypes,
                            relationships: payload_hir_rels,
                            type_refs: payload_type_refs,
                            doc: None,
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
    }
}

/// Extract a StateSubaction (entry/do/exit) directly from AST.
pub(super) fn extract_state_subaction_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    subaction: &parser::StateSubaction,
) {
    let mut rels = Vec::new();

    // Typing if present
    if let Some(typing) = subaction.syntax().children().find_map(parser::Typing::cast) {
        if let Some(target) = typing.target() {
            rels.push(ExtractedRel {
                kind: RelKind::TypedBy,
                target: RelTarget::Simple(target.to_string()),
                range: Some(target.syntax().text_range()),
            });
        }
    }

    let body_members: Vec<NamespaceMember> = subaction
        .body()
        .map(|body| body.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        subaction.name(),
        InternalUsageKind::Action,
        rels,
        subaction.syntax().text_range(),
        body_members,
        None,
    );
}

/// Extract a ControlNode (fork/join/merge/decide) directly from AST.
pub(super) fn extract_control_node_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    node: &parser::ControlNode,
) {
    let kind = match node.kind() {
        Some(SyntaxKind::FORK_KW) => InternalUsageKind::Fork,
        Some(SyntaxKind::JOIN_KW) => InternalUsageKind::Join,
        Some(SyntaxKind::MERGE_KW) => InternalUsageKind::Merge,
        Some(SyntaxKind::DECIDE_KW) => InternalUsageKind::Decide,
        _ => InternalUsageKind::Other,
    };

    let body_members: Vec<NamespaceMember> = node
        .body()
        .map(|body| body.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        node.name(),
        kind,
        Vec::new(),
        node.syntax().text_range(),
        body_members,
        parser::extract_doc_comment(node.syntax()),
    );
}

/// Extract a ForLoopActionUsage directly from AST.
pub(super) fn extract_for_loop_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    for_loop: &parser::ForLoopActionUsage,
) {
    // The for loop variable becomes a child usage
    let mut body_members: Vec<NamespaceMember> = Vec::new();

    // Add body members
    if let Some(body) = for_loop.body() {
        for member in body.members() {
            body_members.push(member);
        }
    }

    // Push the for-loop as anonymous action
    push_special_usage_symbol(
        symbols,
        ctx,
        None,
        InternalUsageKind::Action,
        Vec::new(),
        for_loop.syntax().text_range(),
        body_members,
        parser::extract_doc_comment(for_loop.syntax()),
    );

    // Handle the loop variable as a child: for `for n : Integer in collection`
    // n becomes a child attribute symbol. We need to push it into the scope that
    // push_special_usage_symbol just created. But push_special_usage_symbol already
    // popped the scope. Instead, we inject the variable in the body_members.
    // Actually, we need to handle this differently. Let me inline the variable
    // creation BEFORE calling push_special_usage_symbol by passing it in body.
    // But body_members takes NamespaceMember and there's no variant for a loop variable.
    // We manually push the variable symbol here, after finding the scope.

    // We need to find the anonymous scope that was just created
    if let Some(var_name) = for_loop.variable_name() {
        if let Some(name_text) = var_name.text() {
            // Find the for-loop scope that was just created
            let scope_name = if let Some(last_sym) = symbols.last() {
                last_sym.qualified_name.clone()
            } else {
                return;
            };

            // Check if the last symbol ends with something anon-looking
            // (it should, since for loops are anonymous)
            let var_qualified_name = format!("{}::{}", scope_name, name_text);
            let var_span = ctx.range_to_info(Some(var_name.syntax().text_range()));

            let mut var_rels = Vec::new();
            if let Some(typing) = for_loop.typing() {
                if let Some(target) = typing.target() {
                    var_rels.push(ExtractedRel {
                        kind: RelKind::TypedBy,
                        target: RelTarget::Simple(target.to_string()),
                        range: Some(target.syntax().text_range()),
                    });
                }
            }
            let var_type_refs = extract_type_refs(&var_rels, &ctx.line_index);
            let var_hir_rels = extract_hir_relationships(&var_rels, &ctx.line_index);
            let var_supertypes: Vec<Arc<str>> = var_rels
                .iter()
                .filter(|r| matches!(r.kind, RelKind::TypedBy))
                .map(|r| Arc::from(r.target.as_str().as_ref()))
                .collect();

            symbols.push(HirSymbol {
                file: ctx.file,
                name: Arc::from(name_text.as_str()),
                short_name: None,
                qualified_name: Arc::from(var_qualified_name.as_str()),
                element_id: new_element_id(),
                kind: SymbolKind::AttributeUsage,
                start_line: var_span.start_line,
                start_col: var_span.start_col,
                end_line: var_span.end_line,
                end_col: var_span.end_col,
                short_name_start_line: None,
                short_name_start_col: None,
                short_name_end_line: None,
                short_name_end_col: None,
                supertypes: var_supertypes,
                relationships: var_hir_rels,
                type_refs: var_type_refs,
                doc: None,
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

/// Extract an IfActionUsage (if expr then ... else ...) directly from AST.
pub(super) fn extract_if_action_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    if_action: &parser::IfActionUsage,
) {
    let mut rels = Vec::new();

    // Expression references from condition
    for expr in if_action.expressions() {
        extract_expression_chains(&expr, &mut rels);
    }

    // Qualified name references (then/else action targets)
    for qn in if_action.qualified_names() {
        rels.push(ExtractedRel {
            kind: RelKind::Expression,
            target: RelTarget::Simple(qn.to_string()),
            range: Some(qn.syntax().text_range()),
        });
    }

    let body_members: Vec<NamespaceMember> = if_action
        .body()
        .map(|body| body.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        None,
        InternalUsageKind::Action,
        rels,
        if_action.syntax().text_range(),
        body_members,
        parser::extract_doc_comment(if_action.syntax()),
    );
}

/// Extract a WhileLoopActionUsage (while expr { ... }) directly from AST.
pub(super) fn extract_while_loop_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    while_loop: &parser::WhileLoopActionUsage,
) {
    let mut rels = Vec::new();

    // Expression references from condition
    for expr in while_loop.expressions() {
        extract_expression_chains(&expr, &mut rels);
    }

    let body_members: Vec<NamespaceMember> = while_loop
        .body()
        .map(|body| body.members().collect())
        .unwrap_or_default();

    push_special_usage_symbol(
        symbols,
        ctx,
        None,
        InternalUsageKind::Action,
        rels,
        while_loop.syntax().text_range(),
        body_members,
        parser::extract_doc_comment(while_loop.syntax()),
    );
}
