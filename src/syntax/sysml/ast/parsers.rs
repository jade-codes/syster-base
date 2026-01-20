//! SysML AST parsing.
//!
//! This module provides single-pass parsing for efficient AST construction.
//! All information (name, span, relationships, flags, body members) is extracted
//! in one traversal instead of multiple passes.

use super::enums::{DefinitionMember, Element, UsageKind, UsageMember};
use super::types::{
    Alias, Comment, CrossRel, Definition, Import, MetaRel, NamespaceDeclaration, Package,
    RedefinitionRel, ReferenceRel, Relationships, SatisfyRel, SpecializationRel, SubsettingRel,
    SysMLFile, Usage,
};
use super::utils::{
    extract_name_from_identification, find_in, is_body_rule, is_definition_rule, is_usage_rule,
    to_def_kind, to_usage_kind,
};
use crate::core::Span;
use crate::parser::sysml::Rule;
use pest::iterators::{Pair, Pairs};

/// Parse error type for AST construction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn no_match() -> Self {
        Self {
            message: "No matching rule".to_string(),
        }
    }

    pub fn invalid_rule(rule: &str) -> Self {
        Self {
            message: format!("Invalid rule: {rule}"),
        }
    }
}

// ============================================================================
// Parse Context - accumulates all extracted data in single pass
// ============================================================================

/// Context for accumulating parsed data during single-pass traversal
#[derive(Debug, Default)]
struct ParseContext {
    // Identity
    name: Option<String>,
    name_span: Option<Span>,
    short_name: Option<String>,
    short_name_span: Option<Span>,

    // Flags
    is_abstract: bool,
    is_variation: bool,
    is_derived: bool,
    is_const: bool,

    // Relationships
    relationships: Relationships,

    // Body members (for definitions)
    def_members: Vec<DefinitionMember>,

    // Body members (for usages)
    usage_members: Vec<UsageMember>,

    // Expression references (for value expressions)
    expression_refs: Vec<ExtractedRef>,
}

impl ParseContext {
    fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Span conversion
// ============================================================================

#[inline]
fn to_span(pest_span: pest::Span) -> Span {
    let (sl, sc) = pest_span.start_pos().line_col();
    let (el, ec) = pest_span.end_pos().line_col();
    Span::from_coords(sl - 1, sc - 1, el - 1, ec - 1)
}

// ============================================================================
// Reference extraction helpers
// ============================================================================

fn strip_quotes(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Strip quotes from each part of a qualified name like "'Foo'::'Bar'" -> "Foo::Bar"
/// Also handles single identifiers like "'packet header'" -> "packet header"
fn strip_qualified_name_quotes(s: &str) -> String {
    // Split on :: and strip quotes from each part
    s.split("::")
        .map(|part| strip_quotes(part.trim()))
        .collect::<Vec<_>>()
        .join("::")
}

/// Extract a single reference with span from a pair
pub(super) fn ref_with_span_from(pair: &Pair<Rule>) -> Option<(String, Span)> {
    for inner in pair.clone().into_inner() {
        match inner.as_rule() {
            Rule::qualified_name | Rule::feature_reference | Rule::owned_feature_chain => {
                // Build from parts, stripping quotes where needed
                let parts: Vec<String> = inner
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::identifier || p.as_rule() == Rule::quoted_name)
                    .map(|p| {
                        if p.as_rule() == Rule::quoted_name {
                            strip_quotes(p.as_str())
                        } else {
                            p.as_str().to_string()
                        }
                    })
                    .collect();
                if !parts.is_empty() {
                    return Some((parts.join("::"), to_span(inner.as_span())));
                }
                return Some((inner.as_str().trim().to_string(), to_span(inner.as_span())));
            }
            Rule::identifier => {
                return Some((inner.as_str().trim().to_string(), to_span(inner.as_span())));
            }
            Rule::quoted_name => {
                return Some((strip_quotes(inner.as_str()), to_span(inner.as_span())));
            }
            _ => {
                if let Some(result) = ref_with_span_from(&inner) {
                    return Some(result);
                }
            }
        }
    }
    None
}

/// Reference extracted from parsing, with optional chain context
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractedRef {
    pub name: String,
    pub span: Option<Span>,
    /// If part of a feature chain, contains (all_parts, index_in_chain)
    pub chain_context: Option<(Vec<String>, usize)>,
}

/// Extract all references with spans from a pair
pub(super) fn all_refs_with_spans_from(pair: &Pair<Rule>) -> Vec<ExtractedRef> {
    let mut refs = Vec::new();
    collect_refs_recursive(pair, &mut refs);
    refs
}

fn collect_refs_recursive(pair: &Pair<Rule>, refs: &mut Vec<ExtractedRef>) {
    match pair.as_rule() {
        Rule::owned_feature_chain => {
            // For feature chains like `pwrCmd.pwrLevel`, emit each part as a separate reference
            // with chain context for proper resolution.
            let raw = pair.as_str().trim();
            let base_span = pair.as_span();
            let (base_line, base_col) = base_span.start_pos().line_col();

            // Collect all parts first for the chain context
            let chain_parts: Vec<String> = raw.split('.').map(|p| strip_quotes(p.trim())).collect();

            let mut offset = 0;
            for (chain_index, part) in raw.split('.').enumerate() {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }

                // Calculate the span for this part
                let part_start = offset;
                let part_end = part_start + part.len();
                let part_span = Span::from_coords(
                    base_line - 1,
                    base_col - 1 + part_start,
                    base_line - 1,
                    base_col - 1 + part_end,
                );

                // Strip quotes if present
                let name = strip_quotes(part);
                refs.push(ExtractedRef {
                    name,
                    span: Some(part_span),
                    chain_context: Some((chain_parts.clone(), chain_index)),
                });

                // Move offset past this part and the dot separator
                offset = part_end + 1; // +1 for the '.'
            }
        }
        Rule::qualified_name => {
            // For qualified names like `SysML::Usage`, emit as a single reference
            // Build the qualified name from parts, stripping quotes where needed
            let parts: Vec<String> = pair
                .clone()
                .into_inner()
                .filter(|p| p.as_rule() == Rule::identifier || p.as_rule() == Rule::quoted_name)
                .map(|p| {
                    if p.as_rule() == Rule::quoted_name {
                        strip_quotes(p.as_str())
                    } else {
                        p.as_str().to_string()
                    }
                })
                .collect();
            if !parts.is_empty() {
                refs.push(ExtractedRef {
                    name: parts.join("::"),
                    span: Some(to_span(pair.as_span())),
                    chain_context: None,
                });
            } else {
                // Fallback for atomic rules: use the raw string but strip quotes if needed
                let raw = pair.as_str().trim();
                let name = strip_qualified_name_quotes(raw);
                refs.push(ExtractedRef {
                    name,
                    span: Some(to_span(pair.as_span())),
                    chain_context: None,
                });
            }
        }
        Rule::identifier => {
            refs.push(ExtractedRef {
                name: pair.as_str().trim().to_string(),
                span: Some(to_span(pair.as_span())),
                chain_context: None,
            });
        }
        Rule::quoted_name => {
            refs.push(ExtractedRef {
                name: strip_quotes(pair.as_str()),
                span: Some(to_span(pair.as_span())),
                chain_context: None,
            });
        }
        _ => {
            for inner in pair.clone().into_inner() {
                collect_refs_recursive(&inner, refs);
            }
        }
    }
}

/// Extract type references from expressions (e.g., "= effects meta SysML::Usage" or "= causes as SysML::Usage")
/// Walks the expression tree looking for `meta_operator ~ type_result_member` or `as_operator ~ type_result_member` patterns
fn extract_meta_types_from_expression(pair: &Pair<Rule>) -> Vec<MetaRel> {
    let mut metas = Vec::new();
    collect_meta_types_recursive(pair, &mut metas, false);
    metas
}

fn collect_meta_types_recursive(
    pair: &Pair<Rule>,
    metas: &mut Vec<MetaRel>,
    saw_type_operator: bool,
) {
    let rule = pair.as_rule();

    match rule {
        Rule::meta_operator | Rule::as_operator => {
            // Next sibling should be the type reference
            // We handle this by setting a flag and looking for the type in children
        }
        Rule::type_result_member | Rule::type_reference_member | Rule::type_reference => {
            if saw_type_operator {
                // This is the type after a meta or as operator
                if let Some((target, span)) = ref_with_span_from(pair) {
                    metas.push(MetaRel {
                        target,
                        span: Some(span),
                        chain_context: None,
                    });
                }
            }
        }
        Rule::classification_expression => {
            // Look for meta_operator or as_operator followed by type
            let children: Vec<_> = pair.clone().into_inner().collect();
            for (i, child) in children.iter().enumerate() {
                if child.as_rule() == Rule::meta_operator || child.as_rule() == Rule::as_operator {
                    // Next child should be the type
                    if let Some(type_child) = children.get(i + 1)
                        && let Some((target, span)) = ref_with_span_from(type_child)
                    {
                        metas.push(MetaRel {
                            target,
                            span: Some(span),
                            chain_context: None,
                        });
                    }
                } else {
                    collect_meta_types_recursive(child, metas, false);
                }
            }
            return; // Don't recurse again
        }
        _ => {}
    }

    // Recurse into children
    for inner in pair.clone().into_inner() {
        collect_meta_types_recursive(&inner, metas, saw_type_operator);
    }
}

/// Extract feature references from value expressions (e.g., "= 2*elapseTime.num").
/// This finds all feature_reference_expression and feature_chain_member nodes in expressions.
fn extract_expression_refs(pair: &Pair<Rule>) -> Vec<ExtractedRef> {
    let mut refs = Vec::new();
    collect_expression_refs_recursive(pair, &mut refs, None);
    refs
}

fn collect_expression_refs_recursive(
    pair: &Pair<Rule>,
    refs: &mut Vec<ExtractedRef>,
    chain_base: Option<(String, Span)>,
) {
    let rule = pair.as_rule();

    match rule {
        // feature_reference_expression wraps qualified_name - extract as base reference
        Rule::feature_reference_expression => {
            // This is a starting point of a chain - extract the qualified_name
            for inner in pair.clone().into_inner() {
                if inner.as_rule() == Rule::qualified_name {
                    let name = strip_qualified_name_quotes(inner.as_str().trim());
                    let span = to_span(inner.as_span());
                    refs.push(ExtractedRef {
                        name: name.clone(),
                        span: Some(span),
                        chain_context: None,
                    });
                }
            }
        }

        // primary_expression contains feature_chain_member after "." operators
        // e.g., elapseTime.num where "num" is a feature_chain_member
        Rule::primary_expression => {
            // Process children to find base_expression and feature_chain_members
            let children: Vec<_> = pair.clone().into_inner().collect();
            let mut current_base: Option<(String, Span)> = None;
            let mut chain_parts: Vec<String> = Vec::new();
            let mut chain_spans: Vec<Span> = Vec::new();

            for child in &children {
                match child.as_rule() {
                    Rule::base_expression => {
                        // Find the feature_reference_expression inside base_expression
                        if let Some(feat_ref) = find_feature_ref_in_base(child) {
                            current_base = Some(feat_ref.clone());
                            chain_parts.push(feat_ref.0.clone());
                            chain_spans.push(feat_ref.1);
                        }
                        // Also recurse to handle nested expressions
                        collect_expression_refs_recursive(child, refs, None);
                    }
                    Rule::feature_chain_member => {
                        // This is part of a chain like .num
                        let name = strip_quotes(child.as_str().trim());
                        let span = to_span(child.as_span());
                        chain_parts.push(name.clone());
                        chain_spans.push(span);
                    }
                    _ => {
                        // Recurse into other children
                        collect_expression_refs_recursive(child, refs, current_base.clone());
                    }
                }
            }

            // Now emit refs for each part of the chain with proper context
            if chain_parts.len() > 1 {
                for (idx, (name, span)) in chain_parts.iter().zip(chain_spans.iter()).enumerate() {
                    refs.push(ExtractedRef {
                        name: name.clone(),
                        span: Some(*span),
                        chain_context: Some((chain_parts.clone(), idx)),
                    });
                }
            } else if chain_parts.len() == 1 {
                // Single reference, no chain context needed - already added by feature_reference_expression
            }
            return; // Don't recurse again, we handled it
        }

        _ => {}
    }

    // Recurse into children for other rules
    for inner in pair.clone().into_inner() {
        collect_expression_refs_recursive(&inner, refs, chain_base.clone());
    }
}

/// Find feature_reference_expression inside a base_expression
fn find_feature_ref_in_base(pair: &Pair<Rule>) -> Option<(String, Span)> {
    for inner in pair.clone().into_inner() {
        match inner.as_rule() {
            Rule::feature_reference_expression => {
                for qn in inner.clone().into_inner() {
                    if qn.as_rule() == Rule::qualified_name {
                        let name = strip_qualified_name_quotes(qn.as_str().trim());
                        let span = to_span(qn.as_span());
                        return Some((name, span));
                    }
                }
            }
            _ => {
                if let Some(result) = find_feature_ref_in_base(&inner) {
                    return Some(result);
                }
            }
        }
    }
    None
}

// ============================================================================
// Single-pass visitor
// ============================================================================

/// Visit a pair and extract all relevant information into the context.
/// This is the core single-pass algorithm.
fn visit_pair(pair: &Pair<Rule>, ctx: &mut ParseContext, depth: usize, in_body: bool) {
    let rule = pair.as_rule();

    // Don't descend into nested definitions/usages when extracting relationships
    // But we DO need to collect them as body members
    if depth > 0 && !in_body && (is_definition_rule(rule) || is_usage_rule(rule)) {
        return;
    }

    match rule {
        // ====================================================================
        // Identity extraction
        // ====================================================================
        Rule::identification => {
            for inner in pair.clone().into_inner() {
                visit_pair(&inner, ctx, depth + 1, in_body);
            }
        }

        Rule::regular_name => {
            if ctx.name.is_none() {
                for inner in pair.clone().into_inner() {
                    match inner.as_rule() {
                        Rule::identifier => {
                            ctx.name = Some(inner.as_str().to_string());
                            ctx.name_span = Some(to_span(inner.as_span()));
                        }
                        Rule::quoted_name => {
                            ctx.name = Some(strip_quotes(inner.as_str()));
                            ctx.name_span = Some(to_span(inner.as_span()));
                        }
                        _ => {}
                    }
                }
            }
        }

        Rule::short_name => {
            for inner in pair.clone().into_inner() {
                match inner.as_rule() {
                    Rule::identifier => {
                        ctx.short_name = Some(inner.as_str().to_string());
                        ctx.short_name_span = Some(to_span(inner.as_span()));
                    }
                    Rule::quoted_name => {
                        ctx.short_name = Some(strip_quotes(inner.as_str()));
                        ctx.short_name_span = Some(to_span(inner.as_span()));
                    }
                    _ => {}
                }
            }
        }

        // Fallback: direct identifier at top level (for simple declarations)
        Rule::identifier if ctx.name.is_none() && depth <= 2 => {
            ctx.name = Some(pair.as_str().to_string());
            ctx.name_span = Some(to_span(pair.as_span()));
        }

        // ====================================================================
        // Flag extraction
        // ====================================================================
        Rule::abstract_token => ctx.is_abstract = true,
        Rule::variation_token => ctx.is_variation = true,
        Rule::derived_token => ctx.is_derived = true,
        Rule::constant_token => ctx.is_const = true,

        // Also check in prefix rules
        Rule::basic_definition_prefix | Rule::definition_prefix | Rule::ref_prefix => {
            for inner in pair.clone().into_inner() {
                visit_pair(&inner, ctx, depth + 1, in_body);
            }
        }

        // ====================================================================
        // Relationship extraction
        // ====================================================================
        Rule::subclassification_part => {
            for p in pair.clone().into_inner() {
                if p.as_rule() == Rule::owned_subclassification {
                    for extracted in all_refs_with_spans_from(&p) {
                        ctx.relationships
                            .specializes
                            .push(SpecializationRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                    }
                }
            }
        }

        Rule::redefinition_part => {
            for p in pair.clone().into_inner() {
                if p.as_rule() == Rule::owned_subclassification {
                    for extracted in all_refs_with_spans_from(&p) {
                        ctx.relationships
                            .redefines
                            .push(RedefinitionRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                    }
                }
            }
        }

        Rule::feature_specialization => {
            for spec in pair.clone().into_inner() {
                match spec.as_rule() {
                    Rule::typings => {
                        if let Some((name, span)) = ref_with_span_from(&spec) {
                            ctx.relationships.typed_by = Some(name);
                            ctx.relationships.typed_by_span = Some(span);
                        }
                    }
                    Rule::subsettings => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships
                                .subsets
                                .push(SubsettingRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                        }
                    }
                    Rule::redefinitions => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships
                                .redefines
                                .push(RedefinitionRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                        }
                    }
                    Rule::references => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships
                                .references
                                .push(ReferenceRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                        }
                    }
                    Rule::crosses => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships.crosses.push(CrossRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Also handle feature_specialization_part which wraps feature_specialization
        Rule::feature_specialization_part => {
            for inner in pair.clone().into_inner() {
                visit_pair(&inner, ctx, depth + 1, in_body);
            }
        }

        // Handle owned_feature_typing for parameter_binding
        Rule::owned_feature_typing => {
            if let Some((name, span)) = ref_with_span_from(pair) {
                ctx.relationships.typed_by = Some(name);
                ctx.relationships.typed_by_span = Some(span);
            }
        }

        // Handle owned_reference_subsetting (used in short-form usages like "satisfy SafetyReq;")
        // This captures the reference as a subsetting relationship
        Rule::owned_reference_subsetting => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.relationships
                    .subsets
                    .push(SubsettingRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
            }
        }

        // Domain-specific relationships
        Rule::satisfaction_subject_member => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.relationships
                    .satisfies
                    .push(SatisfyRel { target: extracted.name, span: extracted.span, chain_context: extracted.chain_context });
            }
        }

        // Flow connection endpoints - extract feature chains from flow_part
        // flow_part contains: from_token ~ flow_end_member ~ to_token ~ flow_end_member
        // or: flow_end_member ~ to_token ~ flow_end_member
        // flow_end_member contains owned_feature_chain or flow_feature_member
        Rule::flow_part | Rule::flow_end_member | Rule::flow_end | Rule::flow_feature_member => {
            // Extract all feature chain references from flow endpoints
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Connection endpoints - extract references from connector_end_reference
        // connector_end_reference can be:
        // - owned_feature_chain (e.g., `a.b.c`)
        // - identifier ~ ::> ~ (owned_feature_chain | feature_reference) (e.g., `cause1 ::> causer1`)
        // - feature_reference (e.g., `causer1`)
        Rule::connector_end_member | Rule::connector_end | Rule::connector_end_reference => {
            // Extract all feature chain references from connection endpoints
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // ====================================================================
        // Value expressions - extract meta type and feature references
        // ====================================================================
        Rule::value_part | Rule::feature_value => {
            // Extract meta type references from the expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // ====================================================================
        // Body extraction
        // ====================================================================
        _ if is_body_rule(rule) => {
            // Enter body context and collect members
            for inner in pair.clone().into_inner() {
                visit_body_member(&inner, ctx);
            }
        }

        // ====================================================================
        // Default: recurse into children
        // ====================================================================
        _ => {
            for inner in pair.clone().into_inner() {
                visit_pair(&inner, ctx, depth + 1, in_body);
            }
        }
    }
}

/// Visit a body member and add it to the appropriate collection
fn visit_body_member(pair: &Pair<Rule>, ctx: &mut ParseContext) {
    let rule = pair.as_rule();

    match rule {
        // Comments
        Rule::documentation | Rule::block_comment => {
            let comment = Comment {
                name: None,
                name_span: None,
                content: pair.as_str().to_string(),
                about: Vec::new(),
                span: Some(to_span(pair.as_span())),
            };
            ctx.def_members
                .push(DefinitionMember::Comment(Box::new(comment.clone())));
            ctx.usage_members.push(UsageMember::Comment(comment));
        }
        
        // Named comments with optional about clause
        Rule::comment_annotation => {
            if let Ok(comment) = parse_comment_from_pair(pair.clone()) {
                ctx.def_members
                    .push(DefinitionMember::Comment(Box::new(comment.clone())));
                ctx.usage_members.push(UsageMember::Comment(comment));
            }
        }
        
        // Annotation wrappers - recurse into them to find comment_annotation
        Rule::visible_annotating_member | Rule::annotating_element | Rule::annotating_member | Rule::owned_annotation => {
            for inner in pair.clone().into_inner() {
                visit_body_member(&inner, ctx);
            }
        }

        // Imports inside definitions (e.g., `part def Camera { private import X::*; }`)
        Rule::membership_import | Rule::namespace_import => {
            if let Ok(import) = parse_import(&mut pair.clone().into_inner()) {
                ctx.def_members
                    .push(DefinitionMember::Import(Box::new(import)));
            }
        }

        // Parameter binding (for in/out/inout parameters)
        Rule::parameter_binding => {
            let usage = parse_usage_with_kind(pair.clone(), UsageKind::Reference);
            ctx.def_members
                .push(DefinitionMember::Usage(Box::new(usage.clone())));
            ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
        }

        // Nested usages
        _ if is_usage_rule(rule) => {
            let usage = parse_usage_with_kind(
                pair.clone(),
                to_usage_kind(rule).unwrap_or(UsageKind::Reference),
            );
            ctx.def_members
                .push(DefinitionMember::Usage(Box::new(usage.clone())));
            ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
        }

        // Recurse into containers
        _ => {
            for inner in pair.clone().into_inner() {
                visit_body_member(&inner, ctx);
            }
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Parse a definition from a pest pair using single-pass extraction
pub fn parse_definition(pair: Pair<Rule>) -> Result<Definition, ParseError> {
    let kind = to_def_kind(pair.as_rule()).map_err(|_| ParseError::invalid_rule("definition"))?;

    let mut ctx = ParseContext::new();
    visit_pair(&pair, &mut ctx, 0, false);

    Ok(Definition {
        kind,
        name: ctx.name,
        short_name: ctx.short_name,
        short_name_span: ctx.short_name_span,
        relationships: ctx.relationships,
        body: ctx.def_members,
        span: ctx.name_span,
        is_abstract: ctx.is_abstract,
        is_variation: ctx.is_variation,
    })
}

/// Parse a usage from a pest pair using single-pass extraction
fn parse_usage_with_kind(pair: Pair<Rule>, kind: UsageKind) -> Usage {
    let mut ctx = ParseContext::new();
    visit_pair(&pair, &mut ctx, 0, false);

    Usage {
        kind,
        name: ctx.name,
        short_name: ctx.short_name,
        short_name_span: ctx.short_name_span,
        relationships: ctx.relationships,
        body: ctx.usage_members,
        span: ctx.name_span,
        is_derived: ctx.is_derived,
        is_const: ctx.is_const,
        expression_refs: ctx.expression_refs,
    }
}

/// Parse a usage, inferring kind from the rule
pub fn parse_usage(pair: Pair<Rule>) -> Usage {
    let kind = to_usage_kind(pair.as_rule()).unwrap_or(UsageKind::Reference);
    parse_usage_with_kind(pair, kind)
}

// ============================================================================
// Parse functions for other AST types
// ============================================================================

/// Parse a package from pest pairs
pub fn parse_package(pairs: &mut Pairs<Rule>) -> Result<Package, ParseError> {
    let mut name = None;
    let mut elements = Vec::new();
    let mut span = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::package_declaration => {
                if let Some(p) = find_in(&pair, Rule::identification) {
                    let (extracted_name, extracted_span) = extract_name_from_identification(p);
                    name = extracted_name;
                    span = extracted_span;
                }
            }
            Rule::package_body => {
                elements = pair
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::package_body_items)
                    .flat_map(|p| p.into_inner())
                    .filter(|p| p.as_rule() == Rule::package_body_element)
                    .filter_map(|p| parse_element(&mut p.into_inner()).ok())
                    .collect();
            }
            _ => {}
        }
    }

    Ok(Package {
        name,
        elements,
        span,
    })
}

/// Parse a comment from a single pair (used by parse_element)
/// Grammar: comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference ~ ("," ~ element_reference)*)? ~ (block_comment | semi_colon)? }
pub fn parse_comment_from_pair(pair: Pair<Rule>) -> Result<Comment, ParseError> {
    if pair.as_rule() != Rule::comment_annotation {
        return Err(ParseError::no_match());
    }
    
    let content = pair.as_str().to_string();
    let span = Some(to_span(pair.as_span()));
    let mut name = None;
    let mut name_span = None;
    let mut about = Vec::new();
    
    for child in pair.into_inner() {
        match child.as_rule() {
            Rule::identifier => {
                name = Some(child.as_str().to_string());
                name_span = Some(to_span(child.as_span()));
            }
            Rule::element_reference => {
                // element_reference can contain qualified_name or feature_chain_expression
                let ref_text = child.as_str().to_string();
                let ref_span = Some(to_span(child.as_span()));
                about.push(crate::syntax::sysml::ast::types::AboutReference {
                    name: ref_text,
                    span: ref_span,
                });
            }
            _ => {}
        }
    }
    
    Ok(Comment {
        name,
        name_span,
        content,
        about,
        span,
    })
}

/// Parse a comment from pest pairs (legacy, used by visit_body_member)
/// Grammar: comment_annotation = { comment_token ~ identifier? ~ (locale_token ~ quoted_name)? ~ (about_token ~ element_reference ~ ("," ~ element_reference)*)? ~ (block_comment | semi_colon)? }
pub fn parse_comment(pairs: &mut Pairs<Rule>) -> Result<Comment, ParseError> {
    let pair = pairs.next().ok_or(ParseError::no_match())?;
    parse_comment_from_pair(pair)
}

/// Parse an import from pest pairs
pub fn parse_import(pairs: &mut Pairs<Rule>) -> Result<Import, ParseError> {
    let mut is_recursive = false;
    let mut is_public = false;
    let mut path = String::new();
    let mut path_span = None;
    let mut span = None;

    fn process_pair(
        pair: Pair<Rule>,
        path: &mut String,
        path_span: &mut Option<Span>,
        span: &mut Option<Span>,
        is_public: &mut bool,
        is_recursive: &mut bool,
    ) {
        match pair.as_rule() {
            Rule::import_prefix => {
                for child in pair.into_inner() {
                    if child.as_rule() == Rule::visibility {
                        *is_public = child.as_str().trim() == "public";
                    }
                }
            }
            Rule::imported_membership | Rule::imported_namespace => {
                // Normalize the path by stripping quotes from each component
                // e.g., "'Robotic Vacuum Cleaner'::*" -> "Robotic Vacuum Cleaner::*"
                let raw_path = pair.as_str();
                let normalized = raw_path
                    .split("::")
                    .map(|part| {
                        let trimmed = part.trim();
                        if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
                            trimmed[1..trimmed.len() - 1].to_string()
                        } else {
                            trimmed.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("::");
                *path = normalized;
                *span = Some(to_span(pair.as_span()));
                *path_span = Some(to_span(pair.as_span()));
                *is_recursive = pair
                    .clone()
                    .into_inner()
                    .any(|p| p.as_rule() == Rule::recursive_marker);
            }
            Rule::membership_import | Rule::namespace_import => {
                // These contain import_prefix and imported_membership/imported_namespace
                for child in pair.into_inner() {
                    process_pair(child, path, path_span, span, is_public, is_recursive);
                }
            }
            _ => {}
        }
    }

    for pair in pairs {
        process_pair(
            pair,
            &mut path,
            &mut path_span,
            &mut span,
            &mut is_public,
            &mut is_recursive,
        );
    }

    Ok(Import {
        path,
        path_span,
        is_recursive,
        is_public,
        span,
    })
}

/// Parse an alias from pest pairs
pub fn parse_alias(pairs: &mut Pairs<Rule>) -> Result<Alias, ParseError> {
    let mut name = None;
    let mut target = String::new();
    let mut target_span = None;
    let mut span = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::identification => {
                let (extracted_name, extracted_span) = extract_name_from_identification(pair);
                name = extracted_name;
                span = extracted_span;
            }
            Rule::element_reference => {
                target = pair.as_str().to_string();
                target_span = Some(to_span(pair.as_span()));
            }
            _ => {}
        }
    }

    Ok(Alias {
        name,
        target,
        target_span,
        span,
    })
}

/// Parse an element from pest pairs
pub fn parse_element(pairs: &mut Pairs<Rule>) -> Result<Element, ParseError> {
    let mut pair = pairs.next().ok_or(ParseError::no_match())?;

    // Check for visibility prefix (public/private/protected)
    if pair.as_rule() == Rule::visibility {
        pair = pairs.next().ok_or(ParseError::no_match())?;
    }

    Ok(match pair.as_rule() {
        Rule::package | Rule::library_package | Rule::package_declaration => {
            Element::Package(parse_package(&mut pair.into_inner())?)
        }
        Rule::definition_member_element
        | Rule::usage_member
        | Rule::definition_element
        | Rule::usage_element
        | Rule::occurrence_usage_element
        | Rule::structure_usage_element
        | Rule::behavior_usage_element
        | Rule::non_occurrence_usage_element => parse_element(&mut pair.into_inner())?,
        r if is_definition_rule(r) => Element::Definition(parse_definition(pair)?),
        r if is_usage_rule(r) => Element::Usage(parse_usage(pair)),
        Rule::comment_annotation => {
            // Don't call into_inner() - parse_comment expects to receive the comment_annotation pair directly
            Element::Comment(parse_comment_from_pair(pair)?)
        }
        // Handle annotation wrappers - recurse into them to find comment_annotation
        Rule::visible_annotating_member | Rule::annotating_element | Rule::annotating_member | Rule::owned_annotation => {
            parse_element(&mut pair.into_inner())?
        }
        // Handle documentation as a comment (doc comments)
        Rule::documentation => {
            let comment = Comment {
                name: None,
                name_span: None,
                content: pair.as_str().to_string(),
                about: Vec::new(),
                span: Some(to_span(pair.as_span())),
            };
            Element::Comment(comment)
        }
        Rule::import => Element::Import(parse_import(&mut pair.into_inner())?),
        Rule::alias_member_element => Element::Alias(parse_alias(&mut pair.into_inner())?),
        _ => return Err(ParseError::no_match()),
    })
}

/// Parse a SysML file from pest pairs (main entry point)
pub fn parse_file(pairs: &mut Pairs<Rule>) -> Result<SysMLFile, ParseError> {
    let model = pairs.next().ok_or(ParseError::no_match())?;
    if model.as_rule() != Rule::file {
        return Err(ParseError::no_match());
    }

    let mut elements = Vec::new();
    let mut namespace = None;
    let mut namespaces = Vec::new();

    // Grammar structure: model = { SOI ~ root_namespace ~ EOI }
    // root_namespace = { package_body_element* }
    // package_body_element = { package | library_package | import | ... }
    // So we need to find root_namespace, then iterate its package_body_element children
    for pair in model.into_inner() {
        if pair.as_rule() == Rule::root_namespace {
            for body_element in pair.into_inner() {
                // body_element is package_body_element, which contains the actual element
                // We need to iterate its inner to get the actual rule (package, import, etc.)
                if let Ok(element) = parse_element(&mut body_element.into_inner()) {
                    // Track all package declarations (Issue #10)
                    if let Element::Package(ref pkg) = element
                        && pkg.elements.is_empty()
                        && let Some(ref name) = pkg.name
                    {
                        let ns = NamespaceDeclaration {
                            name: name.clone(),
                            span: pkg.span,
                        };

                        // Keep first namespace for backward compatibility
                        if namespace.is_none() {
                            namespace = Some(ns.clone());
                        }

                        // Collect all namespaces
                        namespaces.push(ns);
                    }
                    elements.push(element);
                }
            }
        }
    }

    Ok(SysMLFile {
        namespace,
        namespaces,
        elements,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::sysml::SysMLParser;
    use crate::syntax::sysml::ast::DefinitionKind;
    use pest::Parser;

    #[test]
    fn test_parse_metadata_def_with_short_name() {
        let source = "metadata def <original> OriginalRequirementMetadata :> SemanticMetadata;";
        let pair = SysMLParser::parse(Rule::metadata_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.kind, DefinitionKind::Metadata);
        // The main name should be OriginalRequirementMetadata, NOT original
        assert_eq!(
            def.name,
            Some("OriginalRequirementMetadata".to_string()),
            "Expected regular name 'OriginalRequirementMetadata', got {:?}",
            def.name
        );
        // The short name should be original
        assert_eq!(
            def.short_name,
            Some("original".to_string()),
            "Expected short name 'original', got {:?}",
            def.short_name
        );
        // Specialization should be captured
        assert_eq!(
            def.relationships.specializes.len(),
            1,
            "Expected 1 specialization"
        );
        assert_eq!(
            def.relationships.specializes[0].target, "SemanticMetadata",
            "Expected specialization target 'SemanticMetadata'"
        );
    }

    #[test]
    fn test_parse_quoted_name_redefines() {
        let source = r#"attribute 'packet primary header' redefines 'packet header';"#;
        let pair = SysMLParser::parse(Rule::attribute_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage(pair);

        assert_eq!(
            usage.name,
            Some("packet primary header".to_string()),
            "Name should not have quotes"
        );
        assert_eq!(
            usage.relationships.redefines.len(),
            1,
            "Expected 1 redefinition"
        );
        assert_eq!(
            usage.relationships.redefines[0].target, "packet header",
            "Redefines target should not have quotes"
        );
    }

    #[test]
    fn test_parse_part_def() {
        let source = "part def Vehicle;";
        let pair = SysMLParser::parse(Rule::part_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.kind, DefinitionKind::Part);
        assert_eq!(def.name, Some("Vehicle".to_string()));
        assert!(def.span.is_some());
    }

    #[test]
    fn test_parse_part_def_with_specialization() {
        let source = "part def Car :> Vehicle;";
        let pair = SysMLParser::parse(Rule::part_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.name, Some("Car".to_string()));
        assert_eq!(def.relationships.specializes.len(), 1);
        assert_eq!(def.relationships.specializes[0].target, "Vehicle");
        assert!(def.relationships.specializes[0].span.is_some());
    }

    #[test]
    fn test_parse_abstract_part_def() {
        let source = "abstract part def AbstractVehicle;";
        let pair = SysMLParser::parse(Rule::part_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.name, Some("AbstractVehicle".to_string()));
        assert!(def.is_abstract);
    }

    #[test]
    fn test_parse_part_usage_with_typing() {
        let source = "part myCar : Car;";
        let pair = SysMLParser::parse(Rule::part_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage(pair);

        assert_eq!(usage.kind, UsageKind::Part);
        assert_eq!(usage.name, Some("myCar".to_string()));
        assert_eq!(usage.relationships.typed_by, Some("Car".to_string()));
        assert!(usage.relationships.typed_by_span.is_some());
    }

    #[test]
    fn test_parse_constraint_def_with_parameters() {
        let source = r#"constraint def MassConstraint {
            in totalMass : MassValue;
        }"#;
        let pair = SysMLParser::parse(Rule::constraint_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.kind, DefinitionKind::Constraint);
        assert_eq!(def.name, Some("MassConstraint".to_string()));
        assert_eq!(def.body.len(), 1);

        // Check the parameter was extracted
        if let DefinitionMember::Usage(usage) = &def.body[0] {
            assert_eq!(usage.name, Some("totalMass".to_string()));
            assert_eq!(usage.relationships.typed_by, Some("MassValue".to_string()));
        } else {
            panic!("Expected Usage member");
        }
    }

    #[test]
    fn test_parse_satisfy_requirement_usage() {
        let source = "satisfy requirement req1 : Req1 by system;";
        let pair = SysMLParser::parse(Rule::satisfy_requirement_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage(pair);

        assert_eq!(usage.kind, UsageKind::SatisfyRequirement);
        assert_eq!(usage.name, Some("req1".to_string()));
        // Typing should be extracted
        assert_eq!(usage.relationships.typed_by, Some("Req1".to_string()));
        // Satisfies should contain "system"
        assert_eq!(usage.relationships.satisfies.len(), 1);
        assert_eq!(usage.relationships.satisfies[0].target, "system");
    }

    #[test]
    fn test_parse_satisfy_short_form() {
        // This is the short form: "satisfy SafetyReq;" without explicit typing or by clause
        let source = "satisfy SafetyReq;";
        let pair = SysMLParser::parse(Rule::satisfy_requirement_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage(pair);

        assert_eq!(usage.kind, UsageKind::SatisfyRequirement);
        // The target should be captured in subsets
        assert_eq!(usage.relationships.subsets.len(), 1);
        assert_eq!(usage.relationships.subsets[0].target, "SafetyReq");
    }

    #[test]
    fn test_parse_satisfy_with_requirement_keyword() {
        // Syntax: "satisfy requirement SafetyReq;"
        // SafetyReq is the NAME of the satisfy usage, not a type reference
        let source = "satisfy requirement SafetyReq;";
        let pair = SysMLParser::parse(Rule::satisfy_requirement_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage(pair);

        assert_eq!(usage.kind, UsageKind::SatisfyRequirement);
        // SafetyReq should be the name of the satisfy usage
        assert_eq!(
            usage.name,
            Some("SafetyReq".to_string()),
            "Expected SafetyReq to be the name of the satisfy usage"
        );
    }

    #[test]
    fn test_parse_reference_usage_with_meta_expression() {
        // Parse a reference usage that includes a meta expression in its value
        let source = "ref :>> baseType = causations meta SysML::Usage;";
        let pair = SysMLParser::parse(Rule::reference_usage, source)
            .unwrap()
            .next()
            .unwrap();

        let usage = parse_usage_with_kind(pair, UsageKind::Reference);

        // Debug output
        println!("name: {:?}", usage.name);
        println!("references: {:?}", usage.relationships.references);
        println!("meta: {:?}", usage.relationships.meta);

        // The meta relationship should be captured
        assert!(
            !usage.relationships.meta.is_empty(),
            "Expected meta relationship to be extracted from expression, got: {:?}",
            usage.relationships.meta
        );
        assert_eq!(
            usage.relationships.meta[0].target, "SysML::Usage",
            "Expected meta target to be SysML::Usage"
        );
    }

    #[test]
    fn test_parse_connection_def_with_end_usages() {
        // Test that end usages in connection definitions capture type references
        let source = r#"connection def Req1_Derivation {
            end r1 : Req1;
            end r1_1 : Req1_1;
        }"#;
        let pair = SysMLParser::parse(Rule::connection_definition, source)
            .unwrap()
            .next()
            .unwrap();

        let def = parse_definition(pair).unwrap();

        assert_eq!(def.kind, DefinitionKind::Connection);
        assert_eq!(def.name, Some("Req1_Derivation".to_string()));

        // Check that we have 2 end usages in the body
        let usages: Vec<_> = def
            .body
            .iter()
            .filter_map(|m| match m {
                DefinitionMember::Usage(u) => Some(u.as_ref()),
                _ => None,
            })
            .collect();

        assert_eq!(usages.len(), 2, "Expected 2 end usages");

        // Check that type references are captured
        assert_eq!(
            usages[0].relationships.typed_by,
            Some("Req1".to_string()),
            "First end should be typed by Req1"
        );
        assert!(
            usages[0].relationships.typed_by_span.is_some(),
            "First end should have typed_by_span"
        );
        assert_eq!(
            usages[1].relationships.typed_by,
            Some("Req1_1".to_string()),
            "Second end should be typed by Req1_1"
        );
        assert!(
            usages[1].relationships.typed_by_span.is_some(),
            "Second end should have typed_by_span"
        );
    }

    #[test]
    fn test_owned_feature_chain_extracts_separate_references() {
        // Test that owned_feature_chain like `pwrCmd.pwrLevel` extracts each part separately
        let source = "attribute :>> pwrCmd.pwrLevel = 0;";
        let pair = SysMLParser::parse(Rule::attribute_usage, source)
            .unwrap()
            .next()
            .unwrap();

        // Use the all_refs_with_spans_from function to check extracted references
        let refs = all_refs_with_spans_from(&pair);

        // Find the references from the owned_feature_chain
        let pwr_cmd_refs: Vec<_> = refs.iter().filter(|r| r.name == "pwrCmd").collect();
        let pwr_level_refs: Vec<_> = refs.iter().filter(|r| r.name == "pwrLevel").collect();

        assert!(
            !pwr_cmd_refs.is_empty(),
            "Should have a reference for 'pwrCmd', got: {:?}",
            refs
        );
        assert!(
            !pwr_level_refs.is_empty(),
            "Should have a reference for 'pwrLevel', got: {:?}",
            refs
        );

        // Check that pwrCmd span is correct (starts at position of 'p' in pwrCmd)
        if let Some(extracted) = pwr_cmd_refs.first() {
            if let Some(span) = &extracted.span {
                // "attribute :>> pwrCmd.pwrLevel = 0;"
                //               ^ pwrCmd starts here (column 15, 0-indexed = 14)
                assert_eq!(span.start.column, 14, "pwrCmd should start at column 14");
                // pwrCmd is 6 characters long, so end column should be 14+6=20
                assert_eq!(span.end.column, 20, "pwrCmd should end at column 20");
            }
            // Check chain context
            assert!(extracted.chain_context.is_some(), "pwrCmd should have chain context");
            if let Some((parts, index)) = &extracted.chain_context {
                assert_eq!(parts, &vec!["pwrCmd".to_string(), "pwrLevel".to_string()]);
                assert_eq!(*index, 0, "pwrCmd should be at index 0");
            }
        }

        // Check that pwrLevel span is correct
        if let Some(extracted) = pwr_level_refs.first() {
            if let Some(span) = &extracted.span {
                // "attribute :>> pwrCmd.pwrLevel = 0;"
                //                      ^ pwrLevel starts here (column 21, 0-indexed = 20+1=21)
                assert_eq!(span.start.column, 21, "pwrLevel should start at column 21");
                // pwrLevel is 8 characters long, so end column should be 21+8=29
                assert_eq!(span.end.column, 29, "pwrLevel should end at column 29");
            }
            // Check chain context for pwrLevel
            assert!(extracted.chain_context.is_some(), "pwrLevel should have chain context");
            if let Some((parts, index)) = &extracted.chain_context {
                assert_eq!(parts, &vec!["pwrCmd".to_string(), "pwrLevel".to_string()]);
                assert_eq!(*index, 1, "pwrLevel should be at index 1");
            }
        }
    }

    // ========================================================================
    // ParseError tests
    // ========================================================================

    #[test]
    fn test_parse_error_no_match() {
        let error = ParseError::no_match();
        assert_eq!(error.message, "No matching rule");
    }

    #[test]
    fn test_parse_error_invalid_rule() {
        let error = ParseError::invalid_rule("some_rule");
        assert_eq!(error.message, "Invalid rule: some_rule");
    }

    #[test]
    fn test_parse_error_invalid_rule_empty() {
        let error = ParseError::invalid_rule("");
        assert_eq!(error.message, "Invalid rule: ");
    }

    #[test]
    fn test_parse_error_equality() {
        let error1 = ParseError::no_match();
        let error2 = ParseError::no_match();
        assert_eq!(error1, error2);

        let error3 = ParseError::invalid_rule("rule_a");
        let error4 = ParseError::invalid_rule("rule_a");
        assert_eq!(error3, error4);

        // Different errors should not be equal
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_parse_error_clone() {
        let error = ParseError::invalid_rule("test");
        let cloned = error.clone();
        assert_eq!(error, cloned);
        assert_eq!(cloned.message, "Invalid rule: test");
    }

    #[test]
    fn test_parse_calc_def_with_in_params() {
        use crate::parser::sysml::Rule;
        use pest::Parser;
        
        let input = r#"calc def CalcBatteryLevel{
            in energy : Real; 
            in capacity : Real; 
            
            energy / capacity
        }"#;
        
        let pairs = crate::parser::sysml::SysMLParser::parse(Rule::calculation_definition, input)
            .expect("Failed to parse calc def");
        
        let pair = pairs.into_iter().next().unwrap();
        let def = parse_definition(pair).unwrap();
        
        assert_eq!(def.name, Some("CalcBatteryLevel".to_string()));
        
        // Check that body members include the in parameters
        println!("Definition body members: {:#?}", def.body);
        
        // Find usages in the body
        let usages: Vec<_> = def.body.iter()
            .filter_map(|m| {
                if let crate::syntax::sysml::ast::enums::DefinitionMember::Usage(u) = m {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();
        
        println!("Usages found: {:?}", usages.iter().map(|u| &u.name).collect::<Vec<_>>());
        
        // Should have at least 2 usages (energy and capacity)
        assert!(usages.len() >= 2, "Expected at least 2 usages, got {}", usages.len());
        
        // Check that energy and capacity are found
        let names: Vec<_> = usages.iter()
            .filter_map(|u| u.name.as_ref())
            .collect();
        assert!(names.contains(&&"energy".to_string()), "energy not found in {:?}", names);
        assert!(names.contains(&&"capacity".to_string()), "capacity not found in {:?}", names);
    }
}
