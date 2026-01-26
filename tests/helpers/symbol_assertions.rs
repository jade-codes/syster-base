//! Symbol assertion helpers for HIR tests.

use syster::hir::{HirSymbol, RelationshipKind, ResolveResult, Resolver, SymbolIndex, SymbolKind};

/// Assert a symbol exists with the given qualified name.
pub fn assert_symbol_exists(index: &SymbolIndex, qname: &str) {
    assert!(
        index.lookup_qualified(qname).is_some(),
        "Expected symbol '{}' to exist in index",
        qname
    );
}

/// Assert a symbol exists and return it for further assertions.
pub fn get_symbol<'a>(index: &'a SymbolIndex, qname: &str) -> &'a HirSymbol {
    index
        .lookup_qualified(qname)
        .unwrap_or_else(|| panic!("Expected symbol '{}' to exist", qname))
}

/// Assert a symbol has the expected kind.
pub fn assert_symbol_kind(symbol: &HirSymbol, expected: SymbolKind) {
    assert_eq!(
        symbol.kind, expected,
        "Expected symbol '{}' to have kind {:?}, got {:?}",
        symbol.qualified_name, expected, symbol.kind
    );
}

/// Assert no duplicate symbols by qualified name.
pub fn assert_no_duplicate_symbols(symbols: &[HirSymbol]) {
    let mut seen = std::collections::HashSet::new();
    for sym in symbols {
        if !seen.insert(&sym.qualified_name) {
            panic!(
                "Duplicate symbol found: {} (kind: {:?})",
                sym.qualified_name, sym.kind
            );
        }
    }
}

/// Resolve a name from a scope and assert it's found. Returns the resolved symbol.
pub fn assert_resolves(index: &SymbolIndex, scope: &str, name: &str) -> HirSymbol {
    let resolver = if scope.is_empty() {
        Resolver::new(index)
    } else {
        Resolver::new(index).with_scope(scope)
    };

    match resolver.resolve(name) {
        ResolveResult::Found(sym) => sym.clone(),
        ResolveResult::NotFound => {
            panic!("'{}' did not resolve from scope '{}'", name, scope)
        }
        ResolveResult::Ambiguous(candidates) => {
            panic!(
                "'{}' is ambiguous from scope '{}': {:?}",
                name,
                scope,
                candidates
                    .iter()
                    .map(|s| s.qualified_name.as_ref())
                    .collect::<Vec<_>>()
            )
        }
    }
}

/// Assert a name does NOT resolve from a scope.
pub fn assert_not_found(index: &SymbolIndex, scope: &str, name: &str) {
    let resolver = if scope.is_empty() {
        Resolver::new(index)
    } else {
        Resolver::new(index).with_scope(scope)
    };

    match resolver.resolve(name) {
        ResolveResult::NotFound => (),
        ResolveResult::Found(sym) => {
            panic!(
                "'{}' should not resolve from '{}', but found: {}",
                name, scope, sym.qualified_name
            )
        }
        ResolveResult::Ambiguous(candidates) => {
            panic!(
                "'{}' should not resolve from '{}', but was ambiguous: {:?}",
                name,
                scope,
                candidates
                    .iter()
                    .map(|s| s.qualified_name.as_ref())
                    .collect::<Vec<_>>()
            )
        }
    }
}

/// Assert a symbol has a type reference to a specific target.
pub fn assert_typed_by(symbol: &HirSymbol, type_name: &str) {
    let has_type = symbol.type_refs.iter().any(|tr| {
        tr.as_refs().iter().any(|r| {
            r.target.as_ref() == type_name || r.resolved_target.as_deref() == Some(type_name)
        })
    });
    assert!(
        has_type,
        "Expected '{}' to be typed by '{}', but type_refs: {:?}",
        symbol.name,
        type_name,
        symbol
            .type_refs
            .iter()
            .map(|tr| tr.first_target().as_ref())
            .collect::<Vec<_>>()
    );
}

/// Assert a symbol has a relationship of a specific kind to a target.
pub fn assert_has_relationship(symbol: &HirSymbol, kind: RelationshipKind, target: &str) {
    let has_rel = symbol.relationships.iter().any(|r| {
        r.kind == kind
            && (r.target.as_ref() == target || r.resolved_target.as_deref() == Some(target))
    });
    assert!(
        has_rel,
        "Expected '{}' to have {:?} relationship to '{}', but relationships: {:?}",
        symbol.name,
        kind,
        target,
        symbol
            .relationships
            .iter()
            .map(|r| format!("{:?} -> {}", r.kind, r.target))
            .collect::<Vec<_>>()
    );
}

/// Assert a symbol specializes another.
pub fn assert_specializes(symbol: &HirSymbol, parent_name: &str) {
    assert_has_relationship(symbol, RelationshipKind::Specializes, parent_name);
}

/// Assert a symbol has NO relationships.
pub fn assert_no_relationships(symbol: &HirSymbol) {
    assert!(
        symbol.relationships.is_empty(),
        "Expected '{}' to have no relationships, but has: {:?}",
        symbol.name,
        symbol
            .relationships
            .iter()
            .map(|r| format!("{:?} -> {}", r.kind, r.target))
            .collect::<Vec<_>>()
    );
}

/// Assert a symbol has a non-zero span.
pub fn assert_has_span(symbol: &HirSymbol) {
    assert!(
        symbol.start_line != 0
            || symbol.start_col != 0
            || symbol.end_line != 0
            || symbol.end_col != 0,
        "Expected '{}' to have a span, but all positions are 0",
        symbol.name
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::hir_helpers::*;

    #[test]
    fn test_assert_symbol_exists_passes() {
        let (mut host, _) = analysis_from_sysml("part def Car;");
        let analysis = host.analysis();
        assert_symbol_exists(analysis.symbol_index(), "Car");
    }

    #[test]
    #[should_panic(expected = "Expected symbol 'NonExistent' to exist")]
    fn test_assert_symbol_exists_fails() {
        let (mut host, _) = analysis_from_sysml("part def Car;");
        let analysis = host.analysis();
        assert_symbol_exists(analysis.symbol_index(), "NonExistent");
    }

    #[test]
    fn test_assert_resolves_works() {
        let (mut host, _) = analysis_from_sysml("package Pkg { part def Car; }");
        let analysis = host.analysis();
        let sym = assert_resolves(analysis.symbol_index(), "Pkg", "Car");
        assert_eq!(sym.qualified_name.as_ref(), "Pkg::Car");
    }

    #[test]
    fn test_assert_no_duplicate_symbols_passes() {
        let symbols = symbols_from_sysml("part def A; part def B;");
        assert_no_duplicate_symbols(&symbols);
    }
}
