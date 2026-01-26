//! Name resolution tests for the HIR layer.
//!
//! These tests verify that names resolve correctly through scope walking,
//! qualified name lookup, and visibility rules.

use crate::helpers::hir_helpers::*;
use crate::helpers::source_fixtures::*;
use crate::helpers::symbol_assertions::*;

// =============================================================================
// SIMPLE NAME RESOLUTION
// =============================================================================

#[test]
fn test_resolve_simple_name_global_scope() {
    let (mut host, _) = analysis_from_sysml("part def Vehicle;");
    let analysis = host.analysis();

    // From global scope (empty string), should find Vehicle
    let sym = assert_resolves(analysis.symbol_index(), "", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Vehicle");
}

#[test]
fn test_resolve_nonexistent_returns_not_found() {
    let (mut host, _) = analysis_from_sysml("part def Vehicle;");
    let analysis = host.analysis();

    assert_not_found(analysis.symbol_index(), "", "DoesNotExist");
}

#[test]
fn test_resolve_from_nested_scope() {
    let source = r#"
        package Outer {
            part def OuterPart;
            package Inner {
                part def InnerPart;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From Inner scope, should find InnerPart
    let inner = assert_resolves(analysis.symbol_index(), "Outer::Inner", "InnerPart");
    assert_eq!(inner.qualified_name.as_ref(), "Outer::Inner::InnerPart");
}

// =============================================================================
// QUALIFIED NAME RESOLUTION
// =============================================================================

#[test]
fn test_resolve_qualified_name() {
    let (mut host, _) = analysis_from_sysml(NESTED_PACKAGE);
    let analysis = host.analysis();

    // Resolve fully qualified name
    let sym = assert_resolves(analysis.symbol_index(), "", "Vehicles::Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Vehicles::Vehicle");
}

#[test]
fn test_resolve_deeply_nested_qualified_name() {
    let (mut host, _) = analysis_from_sysml(DEEPLY_NESTED_PACKAGES);
    let analysis = host.analysis();

    let sym = assert_resolves(
        analysis.symbol_index(),
        "",
        "Level1::Level2::Level3::DeepPart",
    );
    assert_eq!(
        sym.qualified_name.as_ref(),
        "Level1::Level2::Level3::DeepPart"
    );
}

// =============================================================================
// SCOPE WALKING (PARENT SCOPE LOOKUP)
// =============================================================================

#[test]
fn test_child_scope_can_see_parent_symbols() {
    let source = r#"
        package Outer {
            part def SharedDef;
            package Inner {
                part usage : SharedDef;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From Inner scope, should find SharedDef defined in Outer
    let sym = assert_resolves(analysis.symbol_index(), "Outer::Inner", "SharedDef");
    assert_eq!(sym.qualified_name.as_ref(), "Outer::SharedDef");
}

#[test]
fn test_grandchild_scope_can_see_grandparent_symbols() {
    let source = r#"
        package Level1 {
            part def Level1Def;
            package Level2 {
                package Level3 {
                    part usage : Level1Def;
                }
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From Level3, should find Level1Def
    let sym = assert_resolves(
        analysis.symbol_index(),
        "Level1::Level2::Level3",
        "Level1Def",
    );
    assert_eq!(sym.qualified_name.as_ref(), "Level1::Level1Def");
}

// =============================================================================
// SHADOWING
// =============================================================================

#[test]
fn test_local_definition_shadows_outer() {
    let source = r#"
        package Outer {
            part def Thing;
            package Inner {
                part def Thing;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From Inner, Thing should resolve to Inner's definition (shadowing)
    let sym = assert_resolves(analysis.symbol_index(), "Outer::Inner", "Thing");
    assert_eq!(sym.qualified_name.as_ref(), "Outer::Inner::Thing");

    // From Outer, Thing should resolve to Outer's definition
    let outer_sym = assert_resolves(analysis.symbol_index(), "Outer", "Thing");
    assert_eq!(outer_sym.qualified_name.as_ref(), "Outer::Thing");
}

// =============================================================================
// RESOLUTION WITH SPECIALIZATION
// =============================================================================

#[test]
fn test_specialization_target_resolves() {
    let (mut host, _) = analysis_from_sysml(SIMPLE_SPECIALIZATION);
    let analysis = host.analysis();

    // Car specializes Vehicle - Vehicle should be resolvable
    let car = get_symbol(analysis.symbol_index(), "Car");
    assert_specializes(car, "Vehicle");

    // Vehicle should resolve from Car's parent scope
    let vehicle = assert_resolves(analysis.symbol_index(), "", "Vehicle");
    assert_eq!(vehicle.qualified_name.as_ref(), "Vehicle");
}

// =============================================================================
// CROSS-SCOPE RESOLUTION
// =============================================================================

#[test]
fn test_sibling_scope_not_directly_visible() {
    let source = r#"
        package Parent {
            package Sibling1 {
                part def S1Part;
            }
            package Sibling2 {
                // S1Part not directly visible here without import
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From Sibling2, S1Part should not be found (it's in sibling scope)
    assert_not_found(analysis.symbol_index(), "Parent::Sibling2", "S1Part");

    // But qualified name should work
    let sym = assert_resolves(
        analysis.symbol_index(),
        "Parent::Sibling2",
        "Sibling1::S1Part",
    );
    assert_eq!(sym.qualified_name.as_ref(), "Parent::Sibling1::S1Part");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_resolve_package_itself() {
    // Note: Packages with content are indexed and resolvable
    let (mut host, _) = analysis_from_sysml("package MyPackage { part def Inner; }");
    let analysis = host.analysis();

    let sym = assert_resolves(analysis.symbol_index(), "", "MyPackage");
    assert_eq!(sym.qualified_name.as_ref(), "MyPackage");
}

#[test]
fn test_resolve_from_definition_scope() {
    let source = r#"
        part def Vehicle {
            part engine : Engine;
        }
        part def Engine;
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From inside Vehicle, Engine should resolve via parent scope
    let engine = assert_resolves(analysis.symbol_index(), "Vehicle", "Engine");
    assert_eq!(engine.qualified_name.as_ref(), "Engine");
}

// =============================================================================
// AMBIGUOUS RESOLUTION
// =============================================================================

// Note: Full ambiguity detection would require tracking multiple import candidates per name.
// Currently the resolver uses HashMap which only stores one import per name (last wins).
// This test documents the current behavior.

#[test]
fn test_multiple_imports_same_name_last_wins() {
    // When the same name is imported from multiple packages,
    // the current implementation picks the last one (HashMap insert overwrites)
    let source = r#"
        package A {
            part def Thing;
        }
        package B {
            part def Thing;
        }
        package Consumer {
            import A::*;
            import B::*;
            // Thing is imported from B last, so B::Thing wins
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Current behavior: last import wins, no ambiguity error
    let sym = assert_resolves(analysis.symbol_index(), "Consumer", "Thing");
    // B::Thing should be picked since it's imported last
    assert_eq!(sym.qualified_name.as_ref(), "B::Thing");
}

#[test]
fn test_not_ambiguous_when_shadowed() {
    let source = r#"
        package A {
            part def Thing;
        }
        package Consumer {
            import A::*;
            part def Thing;  // Local definition shadows imported
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Local definition should shadow, not be ambiguous
    let sym = assert_resolves(analysis.symbol_index(), "Consumer", "Thing");
    assert_eq!(sym.qualified_name.as_ref(), "Consumer::Thing");
}
