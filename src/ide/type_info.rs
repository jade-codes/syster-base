//! Type information at cursor position.
//!
//! Provides detailed information about type annotations,
//! including resolution and navigation.

use std::sync::Arc;

use crate::base::FileId;
use crate::hir::{HirSymbol, ResolveResult, SymbolIndex, TypeRef, TypeRefKind};

/// Information about a type reference at a position.
#[derive(Clone, Debug)]
pub struct TypeInfo {
    /// The target type name as written in source.
    pub target_name: Arc<str>,
    /// The type reference span information.
    pub type_ref: TypeRef,
    /// The resolved target symbol (if found).
    pub resolved_symbol: Option<HirSymbol>,
    /// The containing symbol's qualified name (for context).
    pub container: Option<Arc<str>>,
}

impl TypeInfo {
    /// Get the resolved qualified name, falling back to the written name.
    pub fn resolved_name(&self) -> &str {
        self.type_ref
            .resolved_target
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or(self.target_name.as_ref())
    }
}

/// Context for a type reference found at a position
pub struct TypeRefContext<'a> {
    /// The target name of this part
    pub target_name: Arc<str>,
    /// The TypeRef for this part
    pub type_ref: &'a TypeRef,
    /// The containing symbol
    pub containing_symbol: Option<&'a HirSymbol>,
    /// If part of a chain, the previous parts (for resolving member access)
    pub chain_prefix: Vec<&'a TypeRef>,
}

/// Get type information at a specific position.
///
/// Returns info if the cursor is on a type annotation (`:`, `:>`, `::>`, etc.).
///
/// # Arguments
/// * `index` - The symbol index to search
/// * `file` - The file containing the cursor
/// * `line` - Cursor line (0-indexed)
/// * `col` - Cursor column (0-indexed)
///
/// # Returns
/// Type information if cursor is on a type reference, None otherwise.
pub fn type_info_at(index: &SymbolIndex, file: FileId, line: u32, col: u32) -> Option<TypeInfo> {
    let ctx = find_type_ref_at_position(index, file, line, col)?;

    // Try to resolve the target symbol
    let resolved_symbol = resolve_type_ref_with_chain(index, &ctx);

    Some(TypeInfo {
        target_name: ctx.target_name,
        type_ref: ctx.type_ref.clone(),
        resolved_symbol,
        container: ctx.containing_symbol.map(|s| s.qualified_name.clone()),
    })
}

/// Resolve a type reference to its target symbol, handling feature chains.
pub fn resolve_type_ref_with_chain(
    index: &SymbolIndex,
    ctx: &TypeRefContext<'_>,
) -> Option<HirSymbol> {
    // Use pre-resolved target if available
    if let Some(resolved) = &ctx.type_ref.resolved_target {
        return index.lookup_qualified(resolved).cloned();
    }

    // If this is part of a chain (not the first element), resolve through the chain
    if !ctx.chain_prefix.is_empty() {
        return resolve_chain_member(index, ctx);
    }

    // Fallback: try to resolve at query time from containing scope
    // Use parent scope (strip the containing symbol's name) for synthetic symbols like flows/binds
    let containing_qn = ctx
        .containing_symbol
        .map(|s| s.qualified_name.as_ref())
        .unwrap_or("");
    let scope = containing_qn
        .rsplit_once("::")
        .map(|(s, _)| s)
        .unwrap_or(containing_qn);

    // Try direct qualified name first (child of scope)
    let direct_qn = format!("{}::{}", scope, ctx.target_name);
    if let Some(sym) = index.lookup_qualified(&direct_qn).cloned() {
        return Some(sym);
    }

    let resolver = index.resolver_for_scope(scope);

    match resolver.resolve(&ctx.target_name) {
        ResolveResult::Found(sym) => Some(sym),
        ResolveResult::Ambiguous(syms) => syms.into_iter().next(),
        ResolveResult::NotFound => None,
    }
}

/// Resolve a chain member like `mass` in `fuelTank.mass` by following the chain.
fn resolve_chain_member(index: &SymbolIndex, ctx: &TypeRefContext<'_>) -> Option<HirSymbol> {
    // Start from the containing symbol's parent scope (not the symbol itself)
    // For a bind like <bind:...>, we want to resolve from the parent (e.g., vehicle_b)
    let containing_qn = ctx
        .containing_symbol
        .map(|s| s.qualified_name.as_ref())
        .unwrap_or("");

    // Strip the last component to get parent scope
    let base_scope = containing_qn
        .rsplit_once("::")
        .map(|(s, _)| s)
        .unwrap_or(containing_qn);

    // Resolve the first part of the chain
    let first_part = ctx.chain_prefix.first()?;
    let resolver = index.resolver_for_scope(base_scope);

    let mut current_symbol = match resolver.resolve(&first_part.target) {
        ResolveResult::Found(sym) => sym,
        ResolveResult::Ambiguous(syms) => syms.into_iter().next()?,
        ResolveResult::NotFound => return None,
    };

    // Follow the chain through symbol members (handles redefinitions and type inheritance)
    for part in ctx.chain_prefix.iter().skip(1) {
        current_symbol = resolve_member_of_symbol(index, &current_symbol, &part.target)?;
    }

    // Finally, resolve the target in the last symbol
    resolve_member_of_symbol(index, &current_symbol, &ctx.target_name)
}

/// Resolve a member name in a symbol - checks direct children first, then type members.
///
/// For example, given `vehicleToRoadPort.wheelToRoadPort1`:
/// 1. If `vehicleToRoadPort` has a direct child `wheelToRoadPort1` (e.g., added via redefinition), return it
/// 2. Otherwise, look for `wheelToRoadPort1` as a member of `vehicleToRoadPort`'s type (e.g., `VehicleToRoadPort`)
///
/// This handles SysML v2 redefinitions where a usage can add new nested elements.
fn resolve_member_of_symbol(
    index: &SymbolIndex,
    symbol: &HirSymbol,
    member_name: &str,
) -> Option<HirSymbol> {
    // First: check for direct children (handles redefinitions that add new members)
    let direct_child = format!("{}::{}", symbol.qualified_name, member_name);
    if let Some(sym) = index.lookup_qualified(&direct_child).cloned() {
        return Some(sym);
    }

    // Second: look through the type hierarchy for inherited members
    resolve_member_in_type(index, symbol, member_name)
}

/// Resolve a member name within the type of a symbol.
/// e.g., for `fuelTank : FuelTank`, looking up `mass` should find `FuelTank::mass`
fn resolve_member_in_type(
    index: &SymbolIndex,
    symbol: &HirSymbol,
    member_name: &str,
) -> Option<HirSymbol> {
    use crate::hir::RelationshipKind;

    // Get the type of the symbol from:
    // 1. supertypes (the first one is usually the type)
    // 2. type_refs with TypedBy kind
    // 3. relationships with domain-specific kinds (Performs, Exhibits, Includes, etc.)
    let type_name = symbol
        .supertypes
        .first()
        .map(|s| s.as_ref())
        .or_else(|| {
            symbol
                .type_refs
                .iter()
                .filter_map(|tr| tr.as_refs().into_iter().next())
                .find(|tr| matches!(tr.kind, crate::hir::RefKind::TypedBy))
                .and_then(|tr| {
                    tr.resolved_target
                        .as_ref()
                        .map(|s| s.as_ref())
                        .or(Some(tr.target.as_ref()))
                })
        })
        .or_else(|| {
            // Check relationships for domain-specific kinds that establish a type relationship:
            // - Performs: perform action (e.g., `perform takePicture :> TakePicture;`)
            // - Exhibits: exhibit state (e.g., `exhibit state running :> Running;`)
            // - Includes: include use case (e.g., `include use case login :> Login;`)
            // - Satisfies: satisfy requirement (e.g., `satisfy requirement safety :> SafetyReq;`)
            // - Asserts: assert constraint (e.g., `assert constraint limit :> SpeedLimit;`)
            // - Verifies: verify requirement (e.g., `verify requirement safety :> SafetyReq;`)
            symbol
                .relationships
                .iter()
                .find(|r| {
                    matches!(
                        r.kind,
                        RelationshipKind::Performs
                            | RelationshipKind::Exhibits
                            | RelationshipKind::Includes
                            | RelationshipKind::Satisfies
                            | RelationshipKind::Asserts
                            | RelationshipKind::Verifies
                    )
                })
                .map(|r| r.target.as_ref())
        })?;

    // Look up the member in the type's scope
    // First try qualified, then definition, then resolve from symbol's scope
    let type_symbol = index
        .lookup_qualified(type_name)
        .or_else(|| index.lookup_definition(type_name))
        .cloned()
        .or_else(|| {
            // Try resolving from the containing symbol's scope
            let scope = symbol
                .qualified_name
                .rsplit_once("::")
                .map(|(s, _)| s)
                .unwrap_or("");
            let resolver = index.resolver_for_scope(scope);
            match resolver.resolve(type_name) {
                ResolveResult::Found(sym) => Some(sym),
                ResolveResult::Ambiguous(syms) => syms.into_iter().next(),
                ResolveResult::NotFound => None,
            }
        })?;

    // The member should be qualified as TypeName::memberName
    let member_qualified = format!("{}::{}", type_symbol.qualified_name, member_name);

    index
        .lookup_qualified(&member_qualified)
        .cloned()
        .or_else(|| {
            // Try looking in the type's scope with resolver
            let resolver = index.resolver_for_scope(&type_symbol.qualified_name);
            match resolver.resolve(member_name) {
                ResolveResult::Found(sym) => Some(sym),
                ResolveResult::Ambiguous(syms) => syms.into_iter().next(),
                ResolveResult::NotFound => None,
            }
        })
}

/// Resolve a type reference to its target symbol.
pub fn resolve_type_ref(
    index: &SymbolIndex,
    type_ref: &TypeRef,
    target_name: &str,
    containing_symbol: Option<&HirSymbol>,
) -> Option<HirSymbol> {
    // Use pre-resolved target if available (computed during semantic analysis)
    if let Some(resolved) = &type_ref.resolved_target {
        return index.lookup_qualified(resolved).cloned();
    }

    // Fallback: try to resolve at query time
    let scope = containing_symbol
        .map(|s| s.qualified_name.as_ref())
        .unwrap_or("");
    let resolver = index.resolver_for_scope(scope);

    match resolver.resolve(target_name) {
        ResolveResult::Found(sym) => Some(sym),
        ResolveResult::Ambiguous(syms) => syms.into_iter().next(),
        ResolveResult::NotFound => {
            // Try qualified name directly
            index.lookup_qualified(target_name).cloned()
        }
    }
}

/// Find a type reference at a specific position in a file.
///
/// Returns context including the chain prefix if the position is on part of a feature chain.
/// Prefers the most specific (smallest) containing symbol to ensure correct scope resolution.
pub fn find_type_ref_at_position(
    index: &SymbolIndex,
    file: FileId,
    line: u32,
    col: u32,
) -> Option<TypeRefContext<'_>> {
    let symbols = index.symbols_in_file(file);

    // Find all matches, then pick the most specific one
    let mut best_match: Option<(&HirSymbol, usize, &TypeRef, Vec<&TypeRef>)> = None;

    for symbol in symbols {
        for type_ref_kind in symbol.type_refs.iter() {
            if type_ref_kind.contains(line, col) {
                // Find which part contains the position
                if let Some((part_idx, tr)) = type_ref_kind.part_at(line, col) {
                    // Collect chain prefix (all parts before the current one)
                    let chain_prefix: Vec<&TypeRef> = match type_ref_kind {
                        TypeRefKind::Simple(_) => Vec::new(),
                        TypeRefKind::Chain(chain) => chain.parts.iter().take(part_idx).collect(),
                    };

                    // Check if this is a more specific match
                    let is_better = match &best_match {
                        None => true,
                        Some((best_sym, _, _, _)) => {
                            // Prefer the symbol with the longer qualified name (more specific)
                            // This typically means a nested/child symbol
                            symbol.qualified_name.len() > best_sym.qualified_name.len()
                        }
                    };

                    if is_better {
                        best_match = Some((symbol, part_idx, tr, chain_prefix));
                    }
                }
            }
        }
    }

    best_match.map(|(symbol, _part_idx, tr, chain_prefix)| TypeRefContext {
        target_name: tr.target.clone(),
        type_ref: tr,
        containing_symbol: Some(symbol),
        chain_prefix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{RefKind, SymbolKind, new_element_id};

    fn make_symbol_with_type_ref(
        name: &str,
        qualified: &str,
        kind: SymbolKind,
        type_ref_target: &str,
        line: u32,
    ) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            element_id: new_element_id(),
            kind,
            file: FileId::new(0),
            start_line: line,
            start_col: 0,
            end_line: line + 1,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: vec![Arc::from(type_ref_target)],
            relationships: Vec::new(),
            type_refs: vec![crate::hir::TypeRefKind::Simple(TypeRef::new(
                type_ref_target,
                RefKind::TypedBy,
                line,
                10,
                line,
                20,
            ))],
            is_public: false,
            view_data: None,
            metadata_annotations: Vec::new(),
            is_abstract: false,
            is_variation: false,
            is_readonly: false,
            is_derived: false,
            is_parallel: false,
            direction: None,
            multiplicity: None,
        }
    }

    #[test]
    fn test_type_info_at_type_ref() {
        let mut index = SymbolIndex::new();

        // Add a definition
        let def = HirSymbol {
            name: Arc::from("Engine"),
            short_name: None,
            qualified_name: Arc::from("Engine"),
            element_id: new_element_id(),
            kind: SymbolKind::PartDefinition,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 5,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
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
            direction: None,
            multiplicity: None,
        };

        // Add a usage with type_ref
        let usage =
            make_symbol_with_type_ref("engine", "Car::engine", SymbolKind::PartUsage, "Engine", 10);

        index.add_file(FileId::new(0), vec![def, usage]);

        // Query at the type_ref position
        let info = type_info_at(&index, FileId::new(0), 10, 15);
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.target_name.as_ref(), "Engine");
        assert!(info.resolved_symbol.is_some());
        assert_eq!(
            info.resolved_symbol.unwrap().qualified_name.as_ref(),
            "Engine"
        );
    }

    #[test]
    fn test_type_info_not_on_type_ref() {
        let mut index = SymbolIndex::new();

        let symbol = HirSymbol {
            name: Arc::from("Car"),
            short_name: None,
            qualified_name: Arc::from("Car"),
            element_id: new_element_id(),
            kind: SymbolKind::PartDefinition,
            file: FileId::new(0),
            start_line: 0,
            start_col: 0,
            end_line: 10,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
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
            direction: None,
            multiplicity: None,
        };

        index.add_file(FileId::new(0), vec![symbol]);

        // Query at position without type_ref
        let info = type_info_at(&index, FileId::new(0), 5, 5);
        assert!(info.is_none());
    }
}
