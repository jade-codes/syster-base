//! SysML AST parsing.
//!
//! This module provides single-pass parsing for efficient AST construction.
//! All information (name, span, relationships, flags, body members) is extracted
//! in one traversal instead of multiple passes.

use super::enums::{DefinitionMember, Element, UsageKind, UsageMember};
use super::types::{
    Alias, Comment, CrossRel, Definition, Dependency, DependencyRef, Filter, Import, MetaRel,
    NamespaceDeclaration, Package, RedefinitionRel, ReferenceRel, Relationships, SatisfyRel,
    SpecializationRel, SubsettingRel, SysMLFile, Usage,
};
use super::utils::{
    extract_full_identification, extract_name_from_identification, find_in, is_body_rule, is_definition_rule, is_usage_rule,
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

/// Reference extracted from parsing - can be simple or a chain
#[derive(Debug, Clone, PartialEq)]
pub enum ExtractedRef {
    /// A simple reference (identifier, qualified name)
    Simple {
        name: String,
        span: Option<Span>,
    },
    /// A feature chain like `providePower.distributeTorque`
    Chain(super::types::FeatureChain),
}

impl Default for ExtractedRef {
    fn default() -> Self {
        ExtractedRef::Simple {
            name: String::new(),
            span: None,
        }
    }
}

impl ExtractedRef {
    /// Create a simple reference
    pub fn simple(name: String, span: Option<Span>) -> Self {
        ExtractedRef::Simple { name, span }
    }
    
    /// Create a chain reference
    pub fn chain(chain: super::types::FeatureChain) -> Self {
        ExtractedRef::Chain(chain)
    }
    
    /// Get the name (for simple refs) or dotted string (for chains)
    /// For backwards compatibility
    pub fn name(&self) -> String {
        match self {
            ExtractedRef::Simple { name, .. } => name.clone(),
            ExtractedRef::Chain(chain) => chain.as_dotted_string(),
        }
    }
    
    /// Get the span
    pub fn span(&self) -> Option<Span> {
        match self {
            ExtractedRef::Simple { span, .. } => *span,
            ExtractedRef::Chain(chain) => chain.span,
        }
    }
    
    /// Check if this is a chain
    pub fn is_chain(&self) -> bool {
        matches!(self, ExtractedRef::Chain(_))
    }
    
    /// Get chain parts if this is a chain
    pub fn chain_parts(&self) -> Option<&[super::types::FeatureChainPart]> {
        match self {
            ExtractedRef::Chain(chain) => Some(&chain.parts),
            _ => None,
        }
    }
    
    /// Legacy: get chain_context as (parts, index) - always returns index 0
    /// DEPRECATED: Use chain_parts() instead
    pub fn chain_context(&self) -> Option<(Vec<String>, usize)> {
        match self {
            ExtractedRef::Chain(chain) => {
                let parts: Vec<String> = chain.parts.iter().map(|p| p.name.clone()).collect();
                Some((parts, 0))
            }
            _ => None,
        }
    }
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
            // For feature chains like `pwrCmd.pwrLevel`, emit as a structured FeatureChain
            let raw = pair.as_str().trim();
            let base_span = pair.as_span();
            let (base_line, base_col) = base_span.start_pos().line_col();
            

            let mut parts = Vec::new();
            let mut offset = 0;
            
            for part in raw.split('.') {
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
                parts.push(super::types::FeatureChainPart {
                    name,
                    span: Some(part_span),
                });

                // Move offset past this part and the dot separator
                offset = part_end + 1; // +1 for the '.'
            }
            
            if !parts.is_empty() {
                refs.push(ExtractedRef::Chain(super::types::FeatureChain {
                    parts,
                    span: Some(to_span(base_span)),
                }));
            }
        }
        // Handle primary_expression which may have chained access like `driver.p1`
        // primary_expression = { base_expression ~ ("." ~ feature_chain_member ~ ...)* }
        Rule::primary_expression => {
            // Collect the full chain from this expression
            let raw = pair.as_str().trim();

            // Check if this contains dots (indicating a chain)
            if raw.contains('.') && !raw.contains("::") {
                // This looks like a feature chain - extract as structured FeatureChain
                let base_span = pair.as_span();
                let (base_line, base_col) = base_span.start_pos().line_col();

                // Split by dots, being careful about method calls
                let raw_parts: Vec<&str> = raw
                    .split('.')
                    .filter(|p| !p.trim().is_empty() && !p.contains('('))
                    .collect();

                if raw_parts.len() > 1 {
                    let mut parts = Vec::new();
                    let mut offset = 0;
                    
                    for part in raw.split('.') {
                        let part = part.trim();
                        if part.is_empty() || part.contains('(') {
                            offset += part.len() + 1;
                            continue;
                        }

                        let part_start = offset;
                        let part_end = part_start + part.len();
                        let part_span = Span::from_coords(
                            base_line - 1,
                            base_col - 1 + part_start,
                            base_line - 1,
                            base_col - 1 + part_end,
                        );

                        let name = strip_quotes(part);
                        parts.push(super::types::FeatureChainPart {
                            name,
                            span: Some(part_span),
                        });

                        offset = part_end + 1;
                    }
                    
                    if parts.len() > 1 {
                        refs.push(ExtractedRef::Chain(super::types::FeatureChain {
                            parts,
                            span: Some(to_span(base_span)),
                        }));
                        return; // Don't recurse - we've handled it
                    }
                }
            }

            // Fall through to normal recursion for non-chain cases
            for inner in pair.clone().into_inner() {
                collect_refs_recursive(&inner, refs);
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
                refs.push(ExtractedRef::simple(parts.join("::"), Some(to_span(pair.as_span()))));
            } else {
                // Fallback for atomic rules: use the raw string but strip quotes if needed
                let raw = pair.as_str().trim();
                let name = strip_qualified_name_quotes(raw);
                refs.push(ExtractedRef::simple(name, Some(to_span(pair.as_span()))));
            }
        }
        Rule::identifier => {
            refs.push(ExtractedRef::simple(
                pair.as_str().trim().to_string(),
                Some(to_span(pair.as_span())),
            ));
        }
        Rule::quoted_name => {
            refs.push(ExtractedRef::simple(
                strip_quotes(pair.as_str()),
                Some(to_span(pair.as_span())),
            ));
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
        // Handle "new Type()" instantiation expressions
        // instantiation_expression = { "new" ~ owned_feature_typing ~ argument_list? }
        Rule::instantiation_expression => {
            // Extract the type from owned_feature_typing
            for inner in pair.clone().into_inner() {
                if inner.as_rule() == Rule::owned_feature_typing {
                    if let Some((target, span)) = ref_with_span_from(&inner) {
                        metas.push(MetaRel::new(ExtractedRef::simple(target, Some(span))));
                    }
                }
            }
            // Also recurse into argument_list for any nested instantiations
            for inner in pair.clone().into_inner() {
                if inner.as_rule() == Rule::argument_list {
                    collect_meta_types_recursive(&inner, metas, false);
                }
            }
            return; // Don't double-recurse
        }
        Rule::type_result_member | Rule::type_reference_member | Rule::type_reference => {
            if saw_type_operator {
                // This is the type after a meta or as operator
                if let Some((target, span)) = ref_with_span_from(pair) {
                    metas.push(MetaRel::new(ExtractedRef::simple(target, Some(span))));
                }
            }
        }
        Rule::classification_expression => {
            // Look for meta_operator, as_operator, or classification_test_operator (@ @@ hastype istype) followed by type
            let children: Vec<_> = pair.clone().into_inner().collect();
            for (i, child) in children.iter().enumerate() {
                let child_rule = child.as_rule();
                if child_rule == Rule::meta_operator
                    || child_rule == Rule::as_operator
                    || child_rule == Rule::classification_test_operator
                {
                    // Next child should be the type reference
                    if let Some(type_child) = children.get(i + 1)
                        && let Some((target, span)) = ref_with_span_from(type_child)
                    {
                        metas.push(MetaRel::new(ExtractedRef::simple(target, Some(span))));
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
                    refs.push(ExtractedRef::simple(name.clone(), Some(span)));
                }
            }
        }

        // primary_expression contains feature_chain_member after "." operators
        // e.g., elapseTime.num where "num" is a feature_chain_member
        Rule::primary_expression => {
            // Process children to find base_expression and feature_chain_members
            let children: Vec<_> = pair.clone().into_inner().collect();
            let mut current_base: Option<(String, Span)> = None;
            let mut chain_parts_raw: Vec<(String, Span)> = Vec::new();

            for child in &children {
                match child.as_rule() {
                    Rule::base_expression => {
                        // Find the feature_reference_expression inside base_expression
                        if let Some(feat_ref) = find_feature_ref_in_base(child) {
                            current_base = Some(feat_ref.clone());
                            chain_parts_raw.push(feat_ref);
                        }
                        // Also recurse to handle nested expressions
                        collect_expression_refs_recursive(child, refs, None);
                    }
                    Rule::feature_chain_member => {
                        // This is part of a chain like .num
                        let name = strip_quotes(child.as_str().trim());
                        let span = to_span(child.as_span());
                        chain_parts_raw.push((name, span));
                    }
                    _ => {
                        // Recurse into other children
                        collect_expression_refs_recursive(child, refs, current_base.clone());
                    }
                }
            }

            // Now emit as a structured chain if we have multiple parts
            if chain_parts_raw.len() > 1 {
                let parts: Vec<super::types::FeatureChainPart> = chain_parts_raw
                    .iter()
                    .map(|(name, span)| super::types::FeatureChainPart {
                        name: name.clone(),
                        span: Some(*span),
                    })
                    .collect();
                
                // Calculate overall span
                let first_span = chain_parts_raw.first().map(|(_, s)| *s);
                let last_span = chain_parts_raw.last().map(|(_, s)| *s);
                let overall_span = match (first_span, last_span) {
                    (Some(f), Some(l)) => Some(Span::from_coords(
                        f.start.line, f.start.column,
                        l.end.line, l.end.column,
                    )),
                    _ => None,
                };
                
                refs.push(ExtractedRef::Chain(super::types::FeatureChain {
                    parts,
                    span: overall_span,
                }));
            }
            // Single reference case already handled by feature_reference_expression recursion
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
                        ctx.relationships.specializes.push(SpecializationRel::new(extracted));
                    }
                }
            }
        }

        Rule::redefinition_part => {
            for p in pair.clone().into_inner() {
                if p.as_rule() == Rule::owned_subclassification {
                    for extracted in all_refs_with_spans_from(&p) {
                        ctx.relationships.redefines.push(RedefinitionRel::new(extracted));
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
                            ctx.relationships.subsets.push(SubsettingRel::new(extracted));
                        }
                    }
                    Rule::redefinitions => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships.redefines.push(RedefinitionRel::new(extracted));
                        }
                    }
                    Rule::references => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships.references.push(ReferenceRel::new(extracted));
                        }
                    }
                    Rule::crosses => {
                        for extracted in all_refs_with_spans_from(&spec) {
                            ctx.relationships.crosses.push(CrossRel::new(extracted));
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
                ctx.relationships.subsets.push(SubsettingRel::new(extracted));
            }
        }

        // Domain-specific relationships
        Rule::satisfaction_subject_member => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.relationships.satisfies.push(SatisfyRel::new(extracted));
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

        // Message event endpoints - extract feature chains from message_declaration
        // message_declaration contains: from_token ~ message_event_member ~ to_token ~ message_event_member
        // message_event_member contains message_event which contains owned_reference_subsetting
        // which can be owned_feature_chain or feature_reference
        Rule::message_event_member | Rule::message_event => {
            // Extract all feature chain references from message event endpoints
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Interface endpoints - can have named endpoints like `lugNutCompositePort ::> wheel1.lugNutCompositePort`
        // interface_end = { owned_cross_multiplicity_member? ~ (identifier ~ (::> | =>) ~ target | target) }
        // When there's a named endpoint with ::> or =>, we need to create a nested Usage
        Rule::interface_end_member | Rule::interface_end => {
            // Recursively find the actual interface_end content
            let actual_inner: Vec<_> = if pair.as_rule() == Rule::interface_end_member {
                // interface_end_member wraps interface_end, so we need to go one level deeper
                pair.clone()
                    .into_inner()
                    .flat_map(|inner| inner.into_inner())
                    .collect()
            } else {
                pair.clone().into_inner().collect()
            };

            // Find identifier and what follows it
            let mut endpoint_name: Option<(String, Span)> = None;
            let mut has_references_op = false;
            let mut has_crosses_op = false;

            for (i, inner) in actual_inner.iter().enumerate() {
                if inner.as_rule() == Rule::identifier {
                    // Check if the next rule is references_operator or crosses_operator
                    if i + 1 < actual_inner.len() {
                        let next = &actual_inner[i + 1];
                        if next.as_rule() == Rule::references_operator {
                            endpoint_name =
                                Some((inner.as_str().to_string(), to_span(inner.as_span())));
                            has_references_op = true;
                        } else if next.as_rule() == Rule::crosses_operator {
                            endpoint_name =
                                Some((inner.as_str().to_string(), to_span(inner.as_span())));
                            has_crosses_op = true;
                        }
                    }
                }
            }

            if let Some((name, name_span)) = endpoint_name {
                // Create a nested Usage for the named endpoint
                let mut nested_usage = Usage::new(
                    UsageKind::Reference,
                    Some(name.clone()),
                    Relationships::default(),
                    Vec::new(),
                );
                nested_usage.span = Some(name_span);

                // Extract the target reference from owned_reference_subsetting as a full chain
                for inner in actual_inner.iter() {
                    if inner.as_rule() == Rule::owned_reference_subsetting {
                        // Extract properly as ExtractedRef to preserve chain structure
                        for extracted in all_refs_with_spans_from(inner) {
                            if has_references_op {
                                nested_usage.relationships.references.push(
                                    super::types::ReferenceRel::new(extracted),
                                );
                            } else if has_crosses_op {
                                nested_usage
                                    .relationships
                                    .crosses
                                    .push(super::types::CrossRel::new(extracted));
                            }
                        }
                        break;
                    }
                }

                // Add to usage_members
                ctx.usage_members
                    .push(UsageMember::Usage(Box::new(nested_usage)));
            } else {
                // No named endpoint - just extract references as expression_refs
                for extracted in all_refs_with_spans_from(pair) {
                    ctx.expression_refs.push(extracted);
                }
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

        // Transition source - extract the source state/action reference
        // transition_source_member = { owned_feature_chain | feature_reference }
        Rule::transition_source_member => {
            // Extract all feature chain references from transition source
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Transition guard expression - extract references from the condition
        // guard_expression_member = { guard_feature_kind ~ owned_expression }
        // e.g., "if ignitionCmd.ignitionOnOff==IgnitionOnOff::on and brakePedalDepressed"
        Rule::guard_expression_member => {
            // Extract meta type references from the guard expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the guard expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // Transition effect behavior - extract references from the do action
        // effect_behavior_member = { effect_feature_kind ~ effect_behavior_usage }
        // effect_behavior_usage contains performed_action_usage or accept_node_declaration
        // e.g., "do send new StartSignal() to controller"
        Rule::effect_behavior_member | Rule::effect_behavior_usage => {
            // Extract meta type references (e.g., StartSignal in "new StartSignal()")
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references (e.g., controller in "to controller")
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);

            // Recurse into children for any nested relationships
            for inner in pair.clone().into_inner() {
                visit_pair(&inner, ctx, depth + 1, in_body);
            }
        }

        // performed_action_usage, satisfy_requirement_usage, exhibit_state_usage all use
        // typed_reference which contains owned_reference_subsetting ~ feature_specialization_part?
        // Let these fall through to the default case which recurses into children

        // Accept node - extract via port reference AND create symbol for payload parameter
        // accept_node_declaration = { action_node_usage_declaration? ~ accept_token ~ accept_parameter_part }
        // accept_parameter_part = { payload_parameter_member ~ (via_token ~ node_parameter_member)? }
        // e.g., "accept ignitionCmd:IgnitionCmd via ignitionCmdPort"
        // Here, 'ignitionCmd' is a declaration that should become a resolvable symbol,
        // 'IgnitionCmd' is its type, and 'ignitionCmdPort' is the via port reference.
        Rule::accept_node_declaration | Rule::accept_parameter_part => {
            for inner in pair.clone().into_inner() {
                if inner.as_rule() == Rule::node_parameter_member {
                    // Extract via port as expression reference
                    for extracted in all_refs_with_spans_from(&inner) {
                        ctx.expression_refs.push(extracted);
                    }
                } else if inner.as_rule() == Rule::payload_parameter_member {
                    // Create a symbol for the payload parameter (e.g., ignitionCmd:IgnitionCmd)
                    // payload_parameter_member -> payload_parameter -> payload
                    // payload = { identification? ~ payload_feature_specialization_part ~ value_part? | ... }
                    let mut payload_name: Option<(String, crate::core::Span)> = None;
                    let mut payload_type: Option<(String, crate::core::Span)> = None;

                    // Navigate to find identification and type
                    fn find_payload_parts(
                        pair: &pest::iterators::Pair<'_, Rule>,
                        name: &mut Option<(String, crate::core::Span)>,
                        typ: &mut Option<(String, crate::core::Span)>,
                    ) {
                        match pair.as_rule() {
                            Rule::identification | Rule::regular_name | Rule::short_name => {
                                for id_inner in pair.clone().into_inner() {
                                    if id_inner.as_rule() == Rule::identifier {
                                        if name.is_none() {
                                            *name = Some((
                                                id_inner.as_str().to_string(),
                                                to_span(id_inner.as_span()),
                                            ));
                                        }
                                    } else {
                                        find_payload_parts(&id_inner, name, typ);
                                    }
                                }
                            }
                            Rule::feature_typing | Rule::owned_feature_typing => {
                                // feature_typing = { owned_feature_typing | conjugated_port_typing }
                                // owned_feature_typing = { conjugation_operator? ~ feature_reference }
                                // Extract the type name from feature_reference
                                if typ.is_none() {
                                    // Try to find the feature_reference for the type name
                                    if let Some((type_name, type_span)) = ref_with_span_from(pair) {
                                        *typ = Some((type_name, type_span));
                                    } else {
                                        // Fallback: use the raw text
                                        let text = pair.as_str().trim();
                                        if !text.is_empty() {
                                            *typ =
                                                Some((text.to_string(), to_span(pair.as_span())));
                                        }
                                    }
                                }
                            }
                            _ => {
                                for child in pair.clone().into_inner() {
                                    find_payload_parts(&child, name, typ);
                                }
                            }
                        }
                    }

                    find_payload_parts(&inner, &mut payload_name, &mut payload_type);

                    if let Some((name, span)) = payload_name {
                        // Create a nested Usage for the payload parameter
                        let mut nested_usage = Usage::new(
                            UsageKind::Reference,
                            Some(name.clone()),
                            Relationships::default(),
                            Vec::new(),
                        );
                        nested_usage.span = Some(span);

                        // Set the typed_by relationship if we found a type
                        if let Some((type_name, type_span)) = payload_type {
                            nested_usage.relationships.typed_by = Some(type_name);
                            nested_usage.relationships.typed_by_span = Some(type_span);
                        }

                        ctx.usage_members
                            .push(UsageMember::Usage(Box::new(nested_usage)));
                    } else if let Some((type_name, type_span)) = payload_type {
                        // No name but we have a type - this is a bare type reference like "accept MySignal"
                        // Add the type as an expression reference so it can be resolved
                        ctx.expression_refs.push(ExtractedRef::simple(type_name, Some(type_span)));
                    }
                } else if inner.as_rule() == Rule::accept_parameter_part {
                    // Recurse into accept_parameter_part
                    visit_pair(&inner, ctx, depth + 1, in_body);
                } else if inner.as_rule() == Rule::action_node_usage_declaration {
                    // Recurse to extract the name (e.g., "trigger" in "action trigger accept ...")
                    visit_pair(&inner, ctx, depth + 1, in_body);
                }
            }
        }

        // node_parameter_member is the via port reference - extract it
        Rule::node_parameter_member => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Send node - extract references from via/to parts AND the action name AND parse body
        // send_node = { occurrence_usage_prefix ~ action_node_usage_declaration? ~ send_token ~ (action_body | (node_parameter_member ~ sender_receiver_part? | ...) ~ action_body) }
        // send_node_declaration = { action_node_usage_declaration? ~ send_token ~ node_parameter_member ~ sender_receiver_part? }
        // sender_receiver_part = { via_token ~ node_parameter_member ~ (to_token ~ node_parameter_member)? | ... }
        // e.g., "action turnVehicleOn send ignitionCmd via driver.p1" - we want to extract 'turnVehicleOn' (name) and 'driver.p1' (reference)
        Rule::send_node | Rule::send_node_declaration => {
            // Find and extract references from sender_receiver_part (the via/to clauses)
            // Also extract from node_parameter_member (the thing being sent, e.g., "new OtherSignal()")
            // Also recurse into action_node_usage_declaration to extract the name
            // Also parse action_body for nested parameters
            for inner in pair.clone().into_inner() {
                if inner.as_rule() == Rule::node_parameter_member {
                    // Extract from the send payload (e.g., "new OtherSignal()" or just "mySignal")
                    for extracted in all_refs_with_spans_from(&inner) {
                        ctx.expression_refs.push(extracted);
                    }
                } else if inner.as_rule() == Rule::sender_receiver_part {
                    for extracted in all_refs_with_spans_from(&inner) {
                        ctx.expression_refs.push(extracted);
                    }
                } else if inner.as_rule() == Rule::action_node_usage_declaration {
                    // Recurse to extract the name (e.g., "turnVehicleOn" in "action turnVehicleOn send ...")
                    visit_pair(&inner, ctx, depth + 1, in_body);
                } else if inner.as_rule() == Rule::action_body {
                    // Parse the action body to extract any directed parameters (in/out/inout)
                    // e.g., "action sendStatus send es via vehicle.statusPort { in es : EngineStatus; }"
                    for body_item in inner.clone().into_inner() {
                        if body_item.as_rule() == Rule::action_body_item {
                            for item_inner in body_item.clone().into_inner() {
                                if item_inner.as_rule() == Rule::directed_parameter_member {
                                    let param_usage =
                                        parse_usage_with_kind(item_inner, UsageKind::Reference);
                                    ctx.usage_members
                                        .push(UsageMember::Usage(Box::new(param_usage)));
                                }
                            }
                        }
                    }
                }
            }
        }

        // sender_receiver_part contains the via/to port references
        Rule::sender_receiver_part => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Transition succession - extract the target state reference
        // transition_succession_member = { transition_succession }
        // transition_succession = { empty_source_end_member ~ connector_end_member }
        // The connector_end_member contains the target state reference
        // Note: This is already handled by connector_end_member above, but adding for explicitness
        Rule::transition_succession_member | Rule::transition_succession => {
            for extracted in all_refs_with_spans_from(pair) {
                ctx.expression_refs.push(extracted);
            }
        }

        // Transition target - extract the target state reference from "then X" patterns
        // transition_target = { then_token ~ connector_end_member | guarded_target_succession | default_target_succession }
        // Used in succession_as_usage: "first X then Y;"
        Rule::transition_target
        | Rule::guarded_target_succession
        | Rule::default_target_succession
        | Rule::target_succession_member => {
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
        // Calculation body expressions (constraints, calculations)
        // result_expression_member = { member_prefix ~ owned_expression }
        // ====================================================================
        Rule::result_expression_member => {
            // Extract meta type references from the expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // ====================================================================
        // Constraint body expressions
        // constraint_body_part = { definition_body_item* ~ (visible_annotating_member* ~ owned_expression)? }
        // e.g., "require constraint {massActual <= massRequired}"
        // ====================================================================
        Rule::constraint_body_part | Rule::constraint_body => {
            // First, collect body items (like `in mass = m;` parameter bindings)
            for inner in pair.clone().into_inner() {
                visit_body_member(&inner, ctx);
            }

            // Extract meta type references from the constraint expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the constraint expression
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

/// Visit a body member and add it to the appropriate collection.
/// Uses an explicit work stack to avoid stack overflow on deeply nested ASTs.
fn visit_body_member(pair: &Pair<Rule>, ctx: &mut ParseContext) {
    let mut work_stack: Vec<Pair<Rule>> = vec![pair.clone()];

    while let Some(current) = work_stack.pop() {
        visit_body_member_single(&current, ctx, &mut work_stack);
    }
}

/// Process a single body member pair, pushing children to the work stack instead of recursing.
fn visit_body_member_single<'a>(
    pair: &Pair<'a, Rule>,
    ctx: &mut ParseContext,
    work_stack: &mut Vec<Pair<'a, Rule>>,
) {
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

        // Annotation wrappers - push children to work stack
        Rule::visible_annotating_member
        | Rule::annotating_element
        | Rule::annotating_member
        | Rule::owned_annotation => {
            let children: Vec<_> = pair.clone().into_inner().collect();
            for inner in children.into_iter().rev() {
                work_stack.push(inner);
            }
        }

        // Element filter members - `filter @Safety;` or `filter someExpr;`
        // element_filter_member = { visibility? ~ filter_token ~ owned_expression ~ semi_colon }
        Rule::element_filter_member => {
            // Extract meta type references from the expression (e.g., @Safety)
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // Expose members - `expose PartsTree::**;`
        // expose = { (namespace_expose | membership_expose) ~ filter_package? ~ relationship_body }
        Rule::expose | Rule::namespace_expose | Rule::membership_expose => {
            // Expose is like import - it has a namespace reference
            // We extract refs from imported_namespace or imported_membership
            for inner in pair.clone().into_inner() {
                let inner_rule = inner.as_rule();
                if inner_rule == Rule::imported_namespace || inner_rule == Rule::imported_membership
                {
                    // These contain qualified_name references
                    let refs = all_refs_with_spans_from(&inner);
                    for r in refs {
                        ctx.expression_refs.push(r);
                    }
                } else if inner_rule == Rule::namespace_expose
                    || inner_rule == Rule::membership_expose
                {
                    // Push to work stack instead of recursing
                    work_stack.push(inner);
                }
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

        // Transition usage members - extract the inner transition_usage
        Rule::transition_usage_member | Rule::target_transition_usage_member => {
            // Find the inner transition_usage or target_transition_usage
            for inner in pair.clone().into_inner() {
                let inner_rule = inner.as_rule();
                if inner_rule == Rule::transition_usage
                    || inner_rule == Rule::target_transition_usage
                {
                    let usage = parse_usage_with_kind(inner.clone(), UsageKind::Transition);
                    ctx.def_members
                        .push(DefinitionMember::Usage(Box::new(usage.clone())));
                    ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
                    return; // Don't recurse further
                }
            }
        }

        // Entry/do/exit action members - extract the action name from state_action_usage
        Rule::entry_action_member | Rule::do_action_member | Rule::exit_action_member => {
            let usage = parse_state_action_member(pair.clone());
            ctx.def_members
                .push(DefinitionMember::Usage(Box::new(usage.clone())));
            ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
        }

        // Requirement constraint members - extract the inner constraint usage
        // requirement_constraint_member = { member_prefix ~ requirement_constraint_kind ~ requirement_constraint_usage }
        // e.g., "require constraint {massActual <= massRequired}"
        Rule::requirement_constraint_member => {
            // Find the inner requirement_constraint_usage
            for inner in pair.clone().into_inner() {
                let inner_rule = inner.as_rule();
                if inner_rule == Rule::requirement_constraint_usage {
                    let usage = parse_usage_with_kind(inner.clone(), UsageKind::Constraint);
                    ctx.def_members
                        .push(DefinitionMember::Usage(Box::new(usage.clone())));
                    ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
                    return; // Don't recurse further
                }
            }
        }

        // Framed concern members - parse as concern usage (creates symbol + type reference)
        // framed_concern_member = { member_prefix ~ framed_concern_kind ~ framed_concern_usage }
        // e.g., "frame concern vs:VehicleSafety;" or "frame vs:VehicleSafety;"
        Rule::framed_concern_member => {
            // Find the inner framed_concern_usage and parse it as a usage
            for inner in pair.clone().into_inner() {
                let inner_rule = inner.as_rule();
                if inner_rule == Rule::framed_concern_usage {
                    // Parse as a concern usage - this creates a symbol and extracts type references
                    let usage = parse_usage_with_kind(inner.clone(), UsageKind::Concern);
                    ctx.def_members
                        .push(DefinitionMember::Usage(Box::new(usage.clone())));
                    ctx.usage_members.push(UsageMember::Usage(Box::new(usage)));
                    return; // Don't recurse further
                }
            }
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

        // Result expression members (constraint/calculation bodies)
        // result_expression_member = { member_prefix ~ owned_expression }
        Rule::result_expression_member => {
            // Extract meta type references from the expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // View rendering members - `render asTreeDiagram;`
        // view_rendering_member = { member_prefix ~ render_token ~ view_rendering_usage }
        // view_rendering_usage = { owned_reference_subsetting ~ ... | ... }
        Rule::view_rendering_member => {
            // Extract the reference from owned_reference_subsetting (the rendering reference)
            let refs = all_refs_with_spans_from(pair);
            ctx.expression_refs.extend(refs);
        }

        // Value expressions within body members (e.g., metadata annotation values)
        // value_part = { "=" ~ owned_expression }
        // feature_value = { ("=" | ":=") ~ owned_expression }
        Rule::value_part | Rule::feature_value => {
            // Extract meta type references from the expression
            let meta_refs = extract_meta_types_from_expression(pair);
            ctx.relationships.meta.extend(meta_refs);

            // Extract feature references from the expression
            let expr_refs = extract_expression_refs(pair);
            ctx.expression_refs.extend(expr_refs);
        }

        // Recurse into containers - push children to work stack
        _ => {
            let children: Vec<_> = pair.clone().into_inner().collect();
            for inner in children.into_iter().rev() {
                work_stack.push(inner);
            }
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Parse an entry/do/exit action member, extracting the action name from state_action_usage
fn parse_state_action_member(pair: Pair<Rule>) -> Usage {
    let mut name: Option<String> = None;
    let mut name_span: Option<Span> = None;
    let mut expression_refs: Vec<ExtractedRef> = Vec::new();
    let mut has_action_keyword = false;
    let mut body_items: Vec<UsageMember> = Vec::new();

    // Check if this is a declaration (has action keyword) or a reference
    fn extract_action_info(
        p: &Pair<Rule>,
        name: &mut Option<String>,
        name_span: &mut Option<Span>,
        expression_refs: &mut Vec<ExtractedRef>,
        has_action_keyword: &mut bool,
        body_items: &mut Vec<UsageMember>,
    ) {
        let rule = p.as_rule();
        match rule {
            Rule::state_action_usage => {
                // state_action_usage = { action_keyword ~ (identifier ~ semi_colon | ...) | identifier ~ ... | qualified_name ~ ... }
                // First, check if there's an action_keyword
                let children: Vec<_> = p.clone().into_inner().collect();
                let has_keyword = children.iter().any(|c| c.as_rule() == Rule::action_keyword);
                *has_action_keyword = has_keyword;

                for inner in children {
                    extract_action_info(
                        &inner,
                        name,
                        name_span,
                        expression_refs,
                        has_action_keyword,
                        body_items,
                    );
                }
            }
            Rule::action_keyword => {
                *has_action_keyword = true;
            }
            Rule::identifier if name.is_none() => {
                let id_name = p.as_str().to_string();
                let id_span = to_span(p.as_span());

                if *has_action_keyword {
                    // This is a declaration like `entry action initial;`
                    *name = Some(id_name);
                    *name_span = Some(id_span);
                } else {
                    // This is a reference like `entry performSelfTest;`
                    // Also set the name so we know what action is being performed
                    *name = Some(id_name.clone());
                    *name_span = Some(id_span);
                    expression_refs.push(ExtractedRef::simple(id_name, Some(id_span)));
                }
            }
            Rule::quoted_name if name.is_none() => {
                let qname = strip_quotes(p.as_str());
                let qspan = to_span(p.as_span());

                if *has_action_keyword {
                    *name = Some(qname);
                    *name_span = Some(qspan);
                } else {
                    *name = Some(qname.clone());
                    *name_span = Some(qspan);
                    expression_refs.push(ExtractedRef::simple(qname, Some(qspan)));
                }
            }
            Rule::qualified_name if name.is_none() => {
                // For qualified_name, extract the full path - always a reference
                let parts: Vec<_> = p
                    .clone()
                    .into_inner()
                    .filter(|i| i.as_rule() == Rule::identifier || i.as_rule() == Rule::quoted_name)
                    .map(|i| {
                        if i.as_rule() == Rule::quoted_name {
                            strip_quotes(i.as_str())
                        } else {
                            i.as_str().to_string()
                        }
                    })
                    .collect();
                if !parts.is_empty() {
                    let joined = parts.join("::");
                    *name = Some(joined.clone());
                    *name_span = Some(to_span(p.as_span()));
                    expression_refs.push(ExtractedRef::simple(joined, Some(to_span(p.as_span()))));
                }
            }
            Rule::action_body => {
                // Parse the action body to extract any directed parameters (in/out/inout)
                // These contain references that need to be resolved in the context of the performed action
                for inner in p.clone().into_inner() {
                    if inner.as_rule() == Rule::action_body_item {
                        // Check for directed_parameter_member (in/out/inout param)
                        for item_inner in inner.clone().into_inner() {
                            if item_inner.as_rule() == Rule::directed_parameter_member {
                                // Parse this as a usage and add to body
                                let param_usage =
                                    parse_usage_with_kind(item_inner, UsageKind::Reference);
                                body_items.push(UsageMember::Usage(Box::new(param_usage)));
                            }
                        }
                    }
                }
            }
            _ => {
                for inner in p.clone().into_inner() {
                    extract_action_info(
                        &inner,
                        name,
                        name_span,
                        expression_refs,
                        has_action_keyword,
                        body_items,
                    );
                }
            }
        }
    }

    // Find state_action_usage in the member
    for inner in pair.clone().into_inner() {
        if inner.as_rule() == Rule::state_action_usage {
            extract_action_info(
                &inner,
                &mut name,
                &mut name_span,
                &mut expression_refs,
                &mut has_action_keyword,
                &mut body_items,
            );
        }
    }

    Usage {
        kind: UsageKind::Action,
        name,
        short_name: None,
        short_name_span: None,
        relationships: Relationships::default(),
        body: body_items,
        span: name_span,
        expression_refs,
        is_derived: false,
        is_const: false,
    }
}

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
    
    // For anonymous usages with redefines (like perform/satisfy/exhibit),
    // derive the name from the redefines target.
    // E.g., "perform ActionTree::providePower redefines providePower;" 
    // should create a symbol named "providePower"
    let name = ctx.name.or_else(|| {
        ctx.relationships.redefines.first().map(|r| {
            // Get the last part of the target (after any "::" or ".")
            let target = r.target();
            let simple_name = target
                .rsplit("::")
                .next()
                .unwrap_or(&target)
                .rsplit('.')
                .next()
                .unwrap_or(&target);
            simple_name.to_string()
        })
    });
    
    // For anonymous usages, use the redefines span as the symbol span
    // This ensures hover/go-to-definition works for `:>> name` syntax
    let span = ctx.name_span.or_else(|| {
        ctx.relationships.redefines.first().and_then(|r| r.span())
    });

    Usage {
        kind,
        name,
        short_name: ctx.short_name,
        short_name_span: ctx.short_name_span,
        relationships: ctx.relationships,
        body: ctx.usage_members,
        span,
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
    let mut short_name = None;
    let mut elements = Vec::new();
    let mut span = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::package_declaration => {
                if let Some(p) = find_in(&pair, Rule::identification) {
                    let (extracted_short, short_span, extracted_name, extracted_span) = extract_full_identification(p);
                    short_name = extracted_short;
                    // If there's a regular name, use it as the primary name
                    // Otherwise fall back to short_name as the name (SysML behavior)
                    if extracted_name.is_some() {
                        name = extracted_name;
                        span = extracted_span;
                    } else if short_name.is_some() {
                        // Use short_name as the name when no regular name is provided
                        name = short_name.clone();
                        span = short_span;
                    }
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
        short_name,
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
                        if trimmed.starts_with('\'')
                            && trimmed.ends_with('\'')
                            && trimmed.len() >= 2
                        {
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

/// Parse a dependency from a pest pair
/// Grammar: dependency = { prefix_metadata? ~ dependency_token ~ ((identification ~ from_token) | from_token)? ~ element_reference ~ ("," ~ element_reference)* ~ to_token ~ element_reference ~ ("," ~ element_reference)* ~ relationship_body }
pub fn parse_dependency(pair: Pair<Rule>) -> Result<Dependency, ParseError> {
    let span = Some(to_span(pair.as_span()));
    let mut name = None;
    let mut name_span = None;
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    let mut seen_to = false;

    // Handle relationship_member_element wrapper
    let inner = if pair.as_rule() == Rule::relationship_member_element {
        // Find the dependency inside
        pair.into_inner()
            .find(|p| p.as_rule() == Rule::dependency)
            .ok_or(ParseError::no_match())?
    } else {
        pair
    };

    for child in inner.into_inner() {
        match child.as_rule() {
            Rule::identification => {
                let (extracted_name, extracted_span) = extract_name_from_identification(child);
                name = extracted_name;
                name_span = extracted_span;
            }
            Rule::to_token => {
                seen_to = true;
            }
            Rule::element_reference => {
                let ref_span = Some(to_span(child.as_span()));
                let ref_path = child.as_str().to_string();
                let dep_ref = DependencyRef {
                    path: ref_path,
                    span: ref_span,
                };
                if seen_to {
                    targets.push(dep_ref);
                } else {
                    sources.push(dep_ref);
                }
            }
            _ => {}
        }
    }

    Ok(Dependency {
        name,
        name_span,
        sources,
        targets,
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
        Rule::visible_annotating_member
        | Rule::annotating_element
        | Rule::annotating_member
        | Rule::owned_annotation => parse_element(&mut pair.into_inner())?,
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
        Rule::relationship_member_element | Rule::dependency => {
            Element::Dependency(parse_dependency(pair)?)
        }
        // Element filter member: `filter @Safety;` or `filter @Safety or @Security;`
        Rule::element_filter_member => {
            let span = Some(to_span(pair.as_span()));
            let meta_refs = extract_meta_types_from_expression(&pair);
            let expression_refs = extract_expression_refs(&pair);
            Element::Filter(Filter {
                meta_refs,
                expression_refs,
                span,
            })
        }
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
            def.relationships.specializes[0].target(), "SemanticMetadata",
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
            usage.relationships.redefines[0].target(), "packet header",
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
        assert_eq!(def.relationships.specializes[0].target(), "Vehicle");
        assert!(def.relationships.specializes[0].span().is_some());
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
        assert_eq!(usage.relationships.satisfies[0].target(), "system");
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
        assert_eq!(usage.relationships.subsets[0].target(), "SafetyReq");
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
            usage.relationships.meta[0].target(), "SysML::Usage",
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
    fn test_owned_feature_chain_extracts_as_chain() {
        // Test that owned_feature_chain like `pwrCmd.pwrLevel` extracts as a FeatureChain
        let source = "attribute :>> pwrCmd.pwrLevel = 0;";
        let pair = SysMLParser::parse(Rule::attribute_usage, source)
            .unwrap()
            .next()
            .unwrap();

        // Use the all_refs_with_spans_from function to check extracted references
        let refs = all_refs_with_spans_from(&pair);

        // Should have exactly one Chain ref
        assert_eq!(refs.len(), 1, "Should have exactly one chain reference, got: {:?}", refs);
        
        let chain_ref = &refs[0];
        assert!(chain_ref.is_chain(), "Reference should be a chain");
        
        // Get the chain parts
        let parts = chain_ref.chain_parts().expect("Should have chain parts");
        assert_eq!(parts.len(), 2, "Chain should have 2 parts");
        
        // Check first part (pwrCmd)
        assert_eq!(parts[0].name, "pwrCmd");
        if let Some(span) = &parts[0].span {
            // "attribute :>> pwrCmd.pwrLevel = 0;"
            //               ^ pwrCmd starts here (column 15, 0-indexed = 14)
            assert_eq!(span.start.column, 14, "pwrCmd should start at column 14");
            assert_eq!(span.end.column, 20, "pwrCmd should end at column 20");
        }
        
        // Check second part (pwrLevel)
        assert_eq!(parts[1].name, "pwrLevel");
        if let Some(span) = &parts[1].span {
            // "attribute :>> pwrCmd.pwrLevel = 0;"
            //                      ^ pwrLevel starts here (column 21)
            assert_eq!(span.start.column, 21, "pwrLevel should start at column 21");
            assert_eq!(span.end.column, 29, "pwrLevel should end at column 29");
        }
        
        // Check overall chain span
        if let Some(span) = chain_ref.span() {
            assert_eq!(span.start.column, 14, "Chain should start at column 14");
            assert_eq!(span.end.column, 29, "Chain should end at column 29");
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
        let usages: Vec<_> = def
            .body
            .iter()
            .filter_map(|m| {
                if let crate::syntax::sysml::ast::enums::DefinitionMember::Usage(u) = m {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();

        println!(
            "Usages found: {:?}",
            usages.iter().map(|u| &u.name).collect::<Vec<_>>()
        );

        // Should have at least 2 usages (energy and capacity)
        assert!(
            usages.len() >= 2,
            "Expected at least 2 usages, got {}",
            usages.len()
        );

        // Check that energy and capacity are found
        let names: Vec<_> = usages.iter().filter_map(|u| u.name.as_ref()).collect();
        assert!(
            names.contains(&&"energy".to_string()),
            "energy not found in {:?}",
            names
        );
        assert!(
            names.contains(&&"capacity".to_string()),
            "capacity not found in {:?}",
            names
        );
    }

    #[test]
    fn test_parse_state_def_with_transitions() {
        let input = r#"state def TestState {
            state off;
            state on;
            transition initial then off;
            transition t1 first off then on;
        }"#;

        let pairs = crate::parser::sysml::SysMLParser::parse(Rule::state_definition, input)
            .expect("Failed to parse state def");

        let pair = pairs.into_iter().next().unwrap();
        let def = parse_definition(pair).unwrap();

        assert_eq!(def.name, Some("TestState".to_string()));

        // Print all body members for debugging
        println!("State definition body members:");
        for member in &def.body {
            println!("  {:?}", member);
        }

        // Find usages in the body
        let usages: Vec<_> = def
            .body
            .iter()
            .filter_map(|m| {
                if let crate::syntax::sysml::ast::enums::DefinitionMember::Usage(u) = m {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();

        println!("Usages found:");
        for u in &usages {
            println!("  name: {:?}, kind: {:?}", u.name, u.kind);
        }

        // Should have states (off, on) and transitions (initial, t1)
        assert!(
            usages.len() >= 4,
            "Expected at least 4 usages (2 states + 2 transitions), got {}",
            usages.len()
        );
    }

    #[test]
    fn test_parse_state_with_entry_action() {
        let input = r#"state def TestState {
            entry action initial;
            state off;
            state on;
            entry performSelfTest;
            do providePower;
            exit applyParkingBrake;
        }"#;

        let pairs = crate::parser::sysml::SysMLParser::parse(Rule::state_definition, input)
            .expect("Failed to parse state def");

        let pair = pairs.into_iter().next().unwrap();
        let def = parse_definition(pair).unwrap();

        assert_eq!(def.name, Some("TestState".to_string()));

        // Print all body members for debugging
        println!("State definition body members:");
        for member in &def.body {
            match member {
                crate::syntax::sysml::ast::enums::DefinitionMember::Usage(u) => {
                    println!(
                        "  Usage: name={:?}, kind={:?}, expression_refs={:?}",
                        u.name, u.kind, u.expression_refs
                    );
                }
                crate::syntax::sysml::ast::enums::DefinitionMember::Comment(c) => {
                    println!("  Comment: {:?}", c.content);
                }
                crate::syntax::sysml::ast::enums::DefinitionMember::Import(i) => {
                    println!("  Import: {:?}", i.path);
                }
            }
        }

        // Find usages in the body
        let usages: Vec<_> = def
            .body
            .iter()
            .filter_map(|m| {
                if let crate::syntax::sysml::ast::enums::DefinitionMember::Usage(u) = m {
                    Some(u)
                } else {
                    None
                }
            })
            .collect();

        // Should have: entry action initial, state off, state on, entry performSelfTest, do providePower, exit applyParkingBrake
        // At minimum: 2 states (off, on)
        assert!(
            usages.len() >= 2,
            "Expected at least 2 usages, got {}",
            usages.len()
        );
    }

    #[test]
    fn test_parse_message_with_typed_payload() {
        let input =
            r#"message of ignitionCmd:IgnitionCmd from driver.turnVehicleOn to vehicle.trigger1;"#;

        let pairs = crate::parser::sysml::SysMLParser::parse(Rule::message, input)
            .expect("Failed to parse message");

        let pair = pairs.into_iter().next().unwrap();
        let usage = parse_usage(pair);

        println!("Message usage:");
        println!("  name: {:?}", usage.name);
        println!("  kind: {:?}", usage.kind);
        println!("  typed_by: {:?}", usage.relationships.typed_by);
        println!("  typed_by_span: {:?}", usage.relationships.typed_by_span);
        println!("  expression_refs: {:?}", usage.expression_refs);

        // The typed payload should extract IgnitionCmd as the type
        // Either as typed_by or in expression_refs
        let has_ignition_cmd = usage
            .relationships
            .typed_by
            .as_ref()
            .is_some_and(|t| t == "IgnitionCmd")
            || usage
                .expression_refs
                .iter()
                .any(|r| r.name() == "IgnitionCmd");

        assert!(
            has_ignition_cmd,
            "IgnitionCmd type should be captured from message payload"
        );
    }
}
