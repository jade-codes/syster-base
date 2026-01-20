use crate::core::Span;
use crate::semantic::graphs::{FeatureChainContext, ReferenceIndex};
use crate::semantic::symbol_table::SymbolTable;
use crate::semantic::types::SemanticError;
use std::path::PathBuf;

pub struct SysmlAdapter<'a> {
    pub(super) symbol_table: &'a mut SymbolTable,
    pub(super) reference_index: Option<&'a mut ReferenceIndex>,
    pub(super) current_namespace: Vec<String>,
    pub(super) errors: Vec<SemanticError>,
}

impl<'a> SysmlAdapter<'a> {
    pub fn new(symbol_table: &'a mut SymbolTable) -> Self {
        Self {
            symbol_table,
            reference_index: None,
            current_namespace: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn with_index(
        symbol_table: &'a mut SymbolTable,
        reference_index: &'a mut ReferenceIndex,
    ) -> Self {
        Self {
            symbol_table,
            reference_index: Some(reference_index),
            current_namespace: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Index a reference from source to target for reverse lookups.
    ///
    /// If the target contains `.` (feature chain like `takePicture.focus`),
    /// each part is indexed separately with chain context to enable proper resolution.
    pub(super) fn index_reference(&mut self, source_qname: &str, target: &str, span: Option<Span>) {
        self.index_reference_with_type(source_qname, target, span, None);
    }

    /// Index a reference with an explicit token type for semantic highlighting.
    ///
    /// Use this for references where the default Type token is not appropriate:
    /// - Property for redefines/subsets targets (they reference usages/features)
    /// - Type for typed_by targets (they reference definitions/classifiers)
    pub(super) fn index_reference_with_type(
        &mut self,
        source_qname: &str,
        target: &str,
        span: Option<Span>,
        token_type: Option<crate::semantic::types::TokenType>,
    ) {
        self.index_reference_with_chain_context(source_qname, target, span, token_type, None);
    }

    /// Index a reference with chain context (from parser).
    ///
    /// When chain_context is Some, it means this reference is part of a feature chain
    /// like `takePicture.focus`. The chain_context provides all parts and the index
    /// of this reference within the chain, enabling proper resolution at lookup time.
    pub(super) fn index_reference_with_chain_context(
        &mut self,
        source_qname: &str,
        target: &str,
        span: Option<Span>,
        token_type: Option<crate::semantic::types::TokenType>,
        chain_context: Option<(Vec<String>, usize)>,
    ) {
        // Check if this is a feature chain (contains `.`) - legacy handling
        if chain_context.is_none() && target.contains('.') {
            self.index_feature_chain(source_qname, target, span, token_type);
            return;
        }

        // First resolve the target (needs immutable borrow of self)
        let resolved_target = self.resolve_reference_target(target);
        let file = self.symbol_table.current_file().map(PathBuf::from);
        
        // Get the current scope ID for proper resolution at hover time
        let scope_id = self.symbol_table.current_scope_id();

        // Convert chain context format
        let feature_chain_ctx = chain_context.map(|(parts, idx)| FeatureChainContext {
            chain_parts: parts,
            chain_index: idx,
        });

        // Then add to index (needs mutable borrow of reference_index)
        if let Some(index) = &mut self.reference_index {
            index.add_reference_full(
                source_qname,
                &resolved_target,
                file.as_ref(),
                span,
                token_type,
                feature_chain_ctx,
                Some(scope_id),
            );
        }
    }

    /// Index a feature chain (e.g., `takePicture.focus`) with chain context.
    ///
    /// Each part is indexed separately with:
    /// - Its own span (computed from the overall span)
    /// - Chain context (all parts + index) for proper resolution at lookup time
    fn index_feature_chain(
        &mut self,
        source_qname: &str,
        target: &str,
        span: Option<Span>,
        token_type: Option<crate::semantic::types::TokenType>,
    ) {
        let file = self.symbol_table.current_file().map(PathBuf::from);
        let scope_id = self.symbol_table.current_scope_id();

        let parts: Vec<&str> = target.split('.').map(|s| s.trim()).collect();
        if parts.is_empty() {
            return;
        }

        let chain_parts: Vec<String> = parts.iter().map(|s| s.to_string()).collect();

        // Calculate span for each part based on the overall span
        let base_span = match span {
            Some(s) => s,
            None => {
                // No span info - index without chain context (fallback)
                if let Some(index) = &mut self.reference_index {
                    for part in &parts {
                        index.add_reference_with_type(
                            source_qname,
                            part,
                            file.as_ref(),
                            None,
                            token_type,
                        );
                    }
                }
                return;
            }
        };

        let mut offset = 0;
        for (chain_index, part) in parts.iter().enumerate() {
            // Calculate the span for this part
            let part_start = offset;
            let part_end = part_start + part.len();
            let part_span = Span::from_coords(
                base_span.start.line,
                base_span.start.column + part_start,
                base_span.start.line,
                base_span.start.column + part_end,
            );

            // Create chain context
            let chain_context = FeatureChainContext {
                chain_parts: chain_parts.clone(),
                chain_index,
            };

            // Add reference with chain context and scope_id
            if let Some(index) = &mut self.reference_index {
                index.add_reference_full(
                    source_qname,
                    part, // Store simple part name, resolution uses chain context
                    file.as_ref(),
                    Some(part_span),
                    token_type,
                    Some(chain_context),
                    Some(scope_id),
                );
            }

            // Move offset past this part and the dot separator
            offset = part_end + 1; // +1 for the '.'
        }
    }

    /// Resolve a reference target to its full qualified name.
    ///
    /// Tries multiple resolution strategies:
    /// 1. Check if target already exists as a fully qualified name
    /// 2. Try prefixing with current namespace parts (walking up the chain)
    fn resolve_reference_target(&self, target: &str) -> String {
        // Strategy 1: Check if it's already a valid fully qualified name
        if self.symbol_table.find_by_qualified_name(target).is_some() {
            return target.to_string();
        }

        // Strategy 2: Try prefixing with current namespace, walking up the chain
        // e.g., if we're in Outer and target is Inner::Vehicle,
        // try Outer::Inner::Vehicle
        let mut namespace = self.current_namespace.clone();
        while !namespace.is_empty() {
            let candidate = format!("{}::{}", namespace.join("::"), target);
            if self
                .symbol_table
                .find_by_qualified_name(&candidate)
                .is_some()
            {
                return candidate;
            }
            namespace.pop();
        }

        // Strategy 3: If target contains ::, try resolving just the first segment
        // and build the full path from there
        if let Some(first_segment) = target.split("::").next() {
            let mut ns = self.current_namespace.clone();
            while !ns.is_empty() {
                let candidate_prefix = format!("{}::{}", ns.join("::"), first_segment);
                if self
                    .symbol_table
                    .find_by_qualified_name(&candidate_prefix)
                    .is_some()
                {
                    // Found the first segment, now build the full path
                    let full_target = format!("{}::{}", ns.join("::"), target);
                    if self
                        .symbol_table
                        .find_by_qualified_name(&full_target)
                        .is_some()
                    {
                        return full_target;
                    }
                }
                ns.pop();
            }
        }

        // Fallback: return original target (unresolved references)
        target.to_string()
    }
}
