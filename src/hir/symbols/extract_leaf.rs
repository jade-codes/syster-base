//! Leaf node extractors — comment, alias, import, dependency.

use std::sync::Arc;

use crate::parser::{
    Alias as AstAlias, AstNode, Comment as AstComment,
    Dependency as AstDependency, Import as AstImport,
};

use super::context::{strip_quotes, ExtractionContext};
use super::helpers::{extract_type_refs, make_chain_or_simple};
use super::types::{
    ExtractionResult, ExtractedRel, HirSymbol, RefKind, RelKind, RelTarget, SymbolKind, TypeRef,
    TypeRefKind, new_element_id,
};

/// Extract a comment symbol directly from the AST Comment node.
pub(super) fn extract_comment_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    comment: &AstComment,
) {
    // Build about relationships directly from the AST
    let about_rels: Vec<ExtractedRel> = comment
        .about_targets()
        .map(|qn| {
            let target_str = qn.to_string();
            ExtractedRel {
                kind: RelKind::About,
                target: RelTarget::Simple(target_str),
                range: Some(qn.syntax().text_range()),
            }
        })
        .collect();

    let type_refs = extract_type_refs(&about_rels, &ctx.line_index);

    let (name, is_anonymous) = match comment.name().and_then(|n| n.text()) {
        Some(n) => (strip_quotes(&n), false),
        None => {
            if type_refs.is_empty() {
                return; // Nothing to track
            }
            let range = comment.syntax().text_range();
            let pos = ctx.line_index.line_col(range.start());
            (
                format!("<anonymous_comment_{}_{}>", pos.line, pos.col),
                true,
            )
        }
    };

    let short_name = comment
        .name()
        .and_then(|n| n.short_name())
        .and_then(|sn| sn.text());

    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(Some(comment.syntax().text_range()));

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: short_name.as_deref().map(Arc::from),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Comment,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc: if is_anonymous {
            None
        } else {
            Some(Arc::from("")) // TODO: Extract comment content
        },
        supertypes: Vec::new(),
        relationships: Vec::new(),
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

/// Extract an alias symbol directly from the AST Alias node.
pub(super) fn extract_alias_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    alias: &AstAlias,
) {
    let name = match alias.name().and_then(|n| n.text()) {
        Some(n) => strip_quotes(&n),
        None => return,
    };

    let short_name = alias
        .name()
        .and_then(|n| n.short_name())
        .and_then(|sn| sn.text());

    let target_str = alias
        .target()
        .map(|t| t.to_string())
        .unwrap_or_default();
    let target_range = alias.target().map(|t| t.syntax().text_range());
    let name_range = alias.name().map(|n| n.syntax().text_range());

    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(name_range.or(Some(alias.syntax().text_range())));

    // Create type_ref for the alias target so hover works on it
    let type_refs = if let Some(r) = target_range {
        let start = ctx.line_index.line_col(r.start());
        let end = ctx.line_index.line_col(r.end());
        vec![TypeRefKind::Simple(TypeRef {
            target: Arc::from(target_str.as_str()),
            resolved_target: None,
            kind: RefKind::Other,
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col,
        })]
    } else {
        Vec::new()
    };

    symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: short_name.as_deref().map(Arc::from),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Alias,
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
        supertypes: vec![Arc::from(target_str.as_str())],
        relationships: Vec::new(),
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

/// Extract an import symbol directly from the AST Import node.
pub(super) fn extract_import_from_ast(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    import: &AstImport,
) {
    // Build the path from AST accessors
    let target = import.target();
    let path_range = target.as_ref().map(|t| t.syntax().text_range());
    let path = target
        .map(|t| {
            let mut path = t.to_string();
            if import.is_wildcard() {
                path.push_str("::*");
            }
            if import.is_recursive() {
                if path.ends_with("::*") {
                    path.push('*');
                } else {
                    path.push_str("::**");
                }
            }
            path
        })
        .unwrap_or_default();

    // Extract filter metadata from bracket syntax [@Filter]
    let filters: Vec<String> = import
        .filter()
        .map(|fp| fp.targets().into_iter().map(|qn| qn.to_string()).collect())
        .unwrap_or_default();

    let qualified_name = ctx.qualified_name(&format!("import:{}", path));

    let span = path_range
        .map(|r| ctx.range_to_info(Some(r)))
        .unwrap_or_else(|| ctx.range_to_info(Some(import.syntax().text_range())));

    // Create type_ref for the import target
    let target_path = path
        .strip_suffix("::**")
        .or_else(|| path.strip_suffix("::*"))
        .unwrap_or(&path);

    let type_refs = if let Some(r) = path_range {
        let start = ctx.line_index.line_col(r.start());
        let end = ctx.line_index.line_col(r.end());
        vec![TypeRefKind::Simple(TypeRef {
            target: Arc::from(target_path),
            resolved_target: None,
            kind: RefKind::Other,
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col,
        })]
    } else {
        Vec::new()
    };

    let is_public = import.is_public();

    result.symbols.push(HirSymbol {
        name: Arc::from(path.as_str()),
        short_name: None,
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Import,
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
        relationships: Vec::new(),
        type_refs,
        is_public,
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

    // Store bracket filters if present
    if !filters.is_empty() {
        let import_qname = ctx.qualified_name(&format!("import:{}", path));
        result
            .import_filters
            .push((Arc::from(import_qname.as_str()), filters));
    }
}

/// Extract a dependency symbol directly from the AST Dependency node.
pub(super) fn extract_dependency_from_ast(
    symbols: &mut Vec<HirSymbol>,
    ctx: &mut ExtractionContext,
    dep: &AstDependency,
) {
    // Build source relationships from AST
    let mut rels: Vec<ExtractedRel> = Vec::new();

    for source in dep.sources() {
        let target_str = source.to_string();
        let rel_target = make_chain_or_simple(&target_str, &source);
        rels.push(ExtractedRel {
            kind: RelKind::DependencySource,
            target: rel_target,
            range: Some(source.syntax().text_range()),
        });
    }

    if let Some(target) = dep.target() {
        let target_str = target.to_string();
        let rel_target = make_chain_or_simple(&target_str, &target);
        rels.push(ExtractedRel {
            kind: RelKind::DependencyTarget,
            target: rel_target,
            range: Some(target.syntax().text_range()),
        });
    }

    // Extract prefix metadata
    for prefix_meta in dep.prefix_metadata() {
        if let (Some(name), Some(range)) = (prefix_meta.name(), prefix_meta.name_range()) {
            rels.push(ExtractedRel {
                kind: RelKind::Meta,
                target: RelTarget::Simple(name),
                range: Some(range),
            });
        }
    }

    let type_refs = extract_type_refs(&rels, &ctx.line_index);
    let span = ctx.range_to_info(Some(dep.syntax().text_range()));

    // Dependencies typically don't have names — create anonymous symbol to hold refs
    if !type_refs.is_empty() {
        symbols.push(HirSymbol {
            name: Arc::from("<anonymous-dependency>"),
            short_name: None,
            qualified_name: Arc::from(format!("{}::<anonymous-dependency>", ctx.prefix)),
            element_id: new_element_id(),
            kind: SymbolKind::Dependency,
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
            relationships: Vec::new(),
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
