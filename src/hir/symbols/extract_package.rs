//! Package, library package, and filter extractors.

use std::sync::Arc;

use crate::parser::{self, AstNode, ElementFilter, LibraryPackage, Package as AstPackage};

use super::context::{ExtractionContext, strip_quotes};
use super::extract::extract_from_ast_member;
use super::types::{
    ExtractionResult, HirSymbol, RefKind, SymbolKind, TypeRef, TypeRefKind, new_element_id,
};

/// Extract a package symbol directly from the AST Package node.
pub(super) fn extract_package_from_ast(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    pkg: &AstPackage,
) {
    let name = match pkg.name().and_then(|n| n.text()) {
        Some(n) => strip_quotes(&n),
        None => return,
    };

    let short_name = pkg
        .name()
        .and_then(|n| n.short_name())
        .and_then(|sn| sn.text());

    let name_range = pkg.name().map(|n| n.syntax().text_range());
    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(name_range.or(Some(pkg.syntax().text_range())));
    let doc = parser::extract_doc_comment(pkg.syntax()).map(|s| Arc::from(s.trim()));

    result.symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: short_name.as_deref().map(Arc::from),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Package,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc,
        supertypes: Vec::new(),
        relationships: Vec::new(),
        type_refs: Vec::new(),
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

    // Recurse into children
    ctx.push_scope(&name);
    if let Some(body) = pkg.body() {
        for child in body.members() {
            extract_from_ast_member(result, ctx, &child);
        }
    }
    ctx.pop_scope();
}

/// Extract a library package symbol directly from the AST LibraryPackage node.
pub(super) fn extract_library_package_from_ast(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    pkg: &LibraryPackage,
) {
    // Library packages are treated as regular packages
    let name = match pkg.name().and_then(|n| n.text()) {
        Some(n) => strip_quotes(&n),
        None => return,
    };

    let short_name = pkg
        .name()
        .and_then(|n| n.short_name())
        .and_then(|sn| sn.text());

    let name_range = pkg.name().map(|n| n.syntax().text_range());
    let qualified_name = ctx.qualified_name(&name);
    let span = ctx.range_to_info(name_range.or(Some(pkg.syntax().text_range())));
    let doc = parser::extract_doc_comment(pkg.syntax()).map(|s| Arc::from(s.trim()));

    result.symbols.push(HirSymbol {
        name: Arc::from(name.as_str()),
        short_name: short_name.as_deref().map(Arc::from),
        qualified_name: Arc::from(qualified_name.as_str()),
        element_id: new_element_id(),
        kind: SymbolKind::Package,
        file: ctx.file,
        start_line: span.start_line,
        start_col: span.start_col,
        end_line: span.end_line,
        end_col: span.end_col,
        short_name_start_line: None,
        short_name_start_col: None,
        short_name_end_line: None,
        short_name_end_col: None,
        doc,
        supertypes: Vec::new(),
        relationships: Vec::new(),
        type_refs: Vec::new(),
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

    // Recurse into children
    ctx.push_scope(&name);
    if let Some(body) = pkg.body() {
        for child in body.members() {
            extract_from_ast_member(result, ctx, &child);
        }
    }
    ctx.pop_scope();
}

/// Extract a filter directly from the AST ElementFilter node.
pub(super) fn extract_filter_from_ast(
    result: &mut ExtractionResult,
    ctx: &mut ExtractionContext,
    filter: &ElementFilter,
) {
    let metadata_refs = filter.metadata_refs();
    let all_refs = filter.all_qualified_refs();

    // Store filter for current scope (for import filtering)
    let scope = ctx.current_scope_name();
    if !metadata_refs.is_empty() {
        result.scope_filters.push((
            Arc::from(scope.as_str()),
            metadata_refs.iter().map(|s| s.to_string()).collect(),
        ));
    }

    // Create type_refs for all qualified names in the filter expression
    if !all_refs.is_empty() {
        let type_refs: Vec<TypeRefKind> = all_refs
            .iter()
            .map(|(name, range)| {
                let start = ctx.line_index.line_col(range.start());
                let end = ctx.line_index.line_col(range.end());
                TypeRefKind::Simple(TypeRef {
                    target: Arc::from(name.as_str()),
                    resolved_target: None,
                    kind: RefKind::Other,
                    start_line: start.line,
                    start_col: start.col,
                    end_line: end.line,
                    end_col: end.col,
                })
            })
            .collect();

        let span = ctx.range_to_info(Some(filter.syntax().text_range()));
        let filter_qname = ctx.qualified_name(&format!("<filter@L{}>", span.start_line));
        result.symbols.push(HirSymbol {
            name: Arc::from("<filter>"),
            short_name: None,
            qualified_name: Arc::from(filter_qname.as_str()),
            element_id: new_element_id(),
            kind: SymbolKind::Other,
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
