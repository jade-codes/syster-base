//! Definition extraction from AST.

use std::sync::Arc;

use crate::parser::{
    self, AstNode, Definition as AstDefinition, DefinitionKind, Expression, NamespaceMember,
    SpecializationKind, SyntaxKind,
};

use super::context::{ExtractionContext, strip_quotes};
use super::extract::extract_from_ast_member_into_symbols;
use super::helpers::{
    extract_expression_chains, extract_hir_relationships, extract_metadata_from_ast_context,
    extract_type_refs, implicit_supertype_for_definition_kind,
};
use super::types::{ExtractedRel, HirSymbol, RelKind, RelTarget, SymbolKind, new_element_id};

/// Extract relationships from a Definition AST node into ExtractedRel values.
fn extract_definition_rels_from_ast(def: &AstDefinition) -> Vec<ExtractedRel> {
    let mut rels = Vec::new();

    // Specializations
    for spec in def.specializations() {
        let rel_kind = match spec.kind() {
            Some(SpecializationKind::Specializes) => RelKind::Specializes,
            Some(SpecializationKind::Subsets) => RelKind::Subsets,
            Some(SpecializationKind::Redefines) => RelKind::Redefines,
            Some(SpecializationKind::References) => RelKind::References,
            Some(SpecializationKind::Conjugates) => RelKind::Specializes,
            Some(SpecializationKind::FeatureChain) => RelKind::Specializes,
            None => RelKind::Specializes, // Comma-continuation
        };
        if let Some(target) = spec.target() {
            rels.push(ExtractedRel {
                kind: rel_kind,
                target: RelTarget::Simple(target.to_string()),
                range: Some(target.syntax().text_range()),
            });
        }
    }

    // Expression references — only those NOT inside nested scopes
    for expr in def.descendants::<Expression>() {
        let mut is_in_nested_scope = false;
        let mut ancestor = expr.syntax().parent();
        let def_syntax = def.syntax();
        while let Some(ref node) = ancestor {
            if node.text_range().start() == def_syntax.text_range().start() {
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
    }

    // Prefix metadata
    for prefix_meta in def.prefix_metadata() {
        if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
            rels.push(ExtractedRel {
                kind: RelKind::Meta,
                target: RelTarget::Simple(name),
                range: Some(range),
            });
        }
    }

    rels
}

/// Extract view-specific data from a Definition AST node (for view/viewpoint/rendering).
fn extract_view_data_from_ast_definition(
    def: &AstDefinition,
    kind: Option<DefinitionKind>,
) -> Option<crate::hir::views::ViewData> {
    use crate::hir::views::{FilterCondition, ViewData, ViewDefinition};

    match kind {
        Some(DefinitionKind::View) => {
            let mut view_def = ViewDefinition::new();

            // Extract filters from children
            if let Some(body) = def.body() {
                for member in body.members() {
                    if let NamespaceMember::Filter(filter) = member {
                        for meta_ref in filter.metadata_refs() {
                            let filter_cond =
                                FilterCondition::metadata(Arc::from(meta_ref.as_str()));
                            view_def.add_filter(filter_cond);
                        }
                    }
                }
            }

            Some(ViewData::ViewDefinition(view_def))
        }
        Some(DefinitionKind::Viewpoint) => Some(ViewData::ViewpointDefinition(
            crate::hir::views::ViewpointDefinition {
                stakeholders: Vec::new(),
                concerns: Vec::new(),
                span: None,
            },
        )),
        Some(DefinitionKind::Rendering) => Some(ViewData::RenderingDefinition(
            crate::hir::views::RenderingDefinition {
                layout: None,
                span: None,
            },
        )),
        _ => None,
    }
}

/// Extract a definition symbol directly from the AST Definition node.
pub(super) fn extract_definition_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    def: &AstDefinition,
) {
    let name = match def.name().and_then(|n| n.text()) {
        Some(n) => strip_quotes(&n),
        None => return,
    };

    let short_name = def
        .name()
        .and_then(|n| n.short_name())
        .and_then(|sn| sn.text());

    let def_kind = def.definition_kind();
    let kind = SymbolKind::from_definition_kind(def_kind);
    let name_range = def.name().map(|n| n.syntax().text_range());
    let short_name_range = def
        .name()
        .and_then(|n| n.short_name())
        .map(|sn| sn.syntax().text_range());
    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(name_range.or(Some(def.syntax().text_range())));
    let (sn_start_line, sn_start_col, sn_end_line, sn_end_col) =
        ctx.range_to_optional(short_name_range);

    // Extract relationships
    let rels = extract_definition_rels_from_ast(def);

    // Supertypes
    let mut supertypes: Vec<Arc<str>> = rels
        .iter()
        .filter(|r| matches!(r.kind, RelKind::Specializes))
        .map(|r| Arc::from(r.target.as_str().as_ref()))
        .collect();

    if supertypes.is_empty() {
        if let Some(implicit) = implicit_supertype_for_definition_kind(def_kind) {
            supertypes.push(Arc::from(implicit));
        }
    }

    let type_refs = extract_type_refs(&rels, &ctx.line_index);
    let relationships = extract_hir_relationships(&rels, &ctx.line_index);

    // Metadata annotations from rels + body children
    let body_members_for_meta = def
        .body()
        .into_iter()
        .flat_map(|b| b.members().collect::<Vec<_>>())
        .chain(
            def.constraint_body()
                .into_iter()
                .flat_map(|cb| cb.members().collect::<Vec<_>>()),
        );
    let metadata_annotations = extract_metadata_from_ast_context(&rels, body_members_for_meta);

    let doc = parser::extract_doc_comment(def.syntax()).map(|s| Arc::from(s.trim()));
    let view_data = extract_view_data_from_ast_definition(def, def_kind);

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
        is_abstract: def.is_abstract(),
        is_variation: def.is_variation(),
        is_readonly: false,
        is_derived: false,
        is_parallel: false,
        is_individual: def.is_individual(),
        is_end: false,
        is_default: false,
        is_ordered: false,
        is_nonunique: false,
        is_portion: false,
        direction: None,
        multiplicity: None,
        value: None,
    });

    // Recurse into children
    ctx.push_scope(&name);
    let children = def
        .body()
        .into_iter()
        .flat_map(|b| b.members().collect::<Vec<_>>())
        .chain(
            def.constraint_body()
                .into_iter()
                .flat_map(|cb| cb.members().collect::<Vec<_>>()),
        );
    for child in children {
        extract_from_ast_member_into_symbols(symbols, ctx, &child);
    }
    ctx.pop_scope();
}
