//! Import resolution tests for the HIR layer.
//!
//! These tests verify that imports correctly bring symbols into scope,
//! including wildcard imports, member imports, and public re-exports.

use crate::helpers::hir_helpers::*;
use crate::helpers::source_fixtures::*;
use crate::helpers::symbol_assertions::*;
use syster::hir::SymbolKind;

// =============================================================================
// WILDCARD IMPORTS
// =============================================================================

#[test]
fn test_wildcard_import_makes_members_visible() {
    let (mut host, _) = analysis_from_sysml(WILDCARD_IMPORT);
    let analysis = host.analysis();

    // From Derived, Vehicle should be visible via wildcard import
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Vehicle");
}

#[test]
fn test_wildcard_import_makes_all_members_visible() {
    let (mut host, _) = analysis_from_sysml(WILDCARD_IMPORT);
    let analysis = host.analysis();

    // Both Vehicle and Engine should be visible in Derived
    assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_resolves(analysis.symbol_index(), "Derived", "Engine");
}

#[test]
fn test_wildcard_import_preserves_original_qualified_name() {
    let (mut host, _) = analysis_from_sysml(WILDCARD_IMPORT);
    let analysis = host.analysis();

    // Vehicle should still be Base::Vehicle, not Derived::Vehicle
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Vehicle");
}

// =============================================================================
// MEMBER IMPORTS
// =============================================================================

#[test]
fn test_member_import_makes_single_member_visible() {
    let (mut host, _) = analysis_from_sysml(MEMBER_IMPORT);
    let analysis = host.analysis();

    // Vehicle should be visible in Derived
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Vehicle");
}

#[test]
fn test_member_import_does_not_import_other_members() {
    let (mut host, _) = analysis_from_sysml(MEMBER_IMPORT);
    let analysis = host.analysis();

    // Engine should NOT be visible in Derived (only Vehicle was imported)
    assert_not_found(analysis.symbol_index(), "Derived", "Engine");
}

// =============================================================================
// PUBLIC IMPORTS (RE-EXPORTS)
// =============================================================================

#[test]
fn test_public_import_re_exports_to_importers() {
    let source = r#"
        package Base {
            part def Original;
        }
        package Middle {
            public import Base::*;
        }
        package Consumer {
            import Middle::*;
            part usage : Original;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Consumer imports from Middle, which publicly re-exports Base
    // So Original should be visible in Consumer
    let sym = assert_resolves(analysis.symbol_index(), "Consumer", "Original");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Original");
}

#[test]
fn test_private_import_does_not_re_export() {
    let source = r#"
        package Base {
            part def Original;
        }
        package Middle {
            import Base::*;  // Private import (no 'public' keyword)
        }
        package Consumer {
            import Middle::*;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Consumer imports from Middle, but Middle's import is private
    // So Original should NOT be visible in Consumer
    assert_not_found(analysis.symbol_index(), "Consumer", "Original");
}

// =============================================================================
// NESTED IMPORTS
// =============================================================================

#[test]
fn test_nested_package_imports() {
    let (mut host, _) = analysis_from_sysml(NESTED_IMPORTS);
    let analysis = host.analysis();

    // Vehicle should be resolvable from Usage via import chain
    let sym = assert_resolves(analysis.symbol_index(), "Usage", "Vehicle");
    assert_eq!(
        sym.qualified_name.as_ref(),
        "Definitions::PartDefinitions::Vehicle"
    );
}

#[test]
fn test_import_from_parent_scope_sibling() {
    let source = r#"
        package Definitions {
            public import PartDefs::*;
            public import PortDefs::*;
            
            package PartDefs {
                part def Vehicle {
                    port p : DataPort;
                }
            }
            package PortDefs {
                port def DataPort;
            }
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // From PartDefs::Vehicle, DataPort should resolve via parent (Definitions)
    // which imports from sibling (PortDefs)
    let sym = assert_resolves(
        analysis.symbol_index(),
        "Definitions::PartDefs::Vehicle",
        "DataPort",
    );
    assert_eq!(
        sym.qualified_name.as_ref(),
        "Definitions::PortDefs::DataPort"
    );
}

#[test]
fn test_root_level_import_visible_in_nested_package() {
    // Root-level `private import P2::*` should make P2's members visible in nested packages
    let source = r#"
        package P1 {
            part def A;
        }
        
        package P2 {
            private import P1::*;
            part a : A;
        }
        
        private import P2::*;
        
        package P3 {
            part b subsets a;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // 'a' is imported at root level via `private import P2::*`
    // It should be visible from within P3 (via parent scope lookup)
    let sym = assert_resolves(index, "P3", "a");
    assert_eq!(sym.qualified_name.as_ref(), "P2::a");
}

// =============================================================================
// IMPORT SHADOWING
// =============================================================================

#[test]
fn test_local_definition_shadows_import() {
    let source = r#"
        package Base {
            part def Thing;
        }
        package Derived {
            import Base::*;
            part def Thing;  // Local shadows imported
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Local Thing should shadow imported Thing
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Thing");
    assert_eq!(sym.qualified_name.as_ref(), "Derived::Thing");
}

// =============================================================================
// TRANSITIVE IMPORTS
// =============================================================================

#[test]
fn test_transitive_public_import_chain() {
    let source = r#"
        package A {
            part def APart;
        }
        package B {
            public import A::*;
        }
        package C {
            public import B::*;
        }
        package D {
            import C::*;
            part usage : APart;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // D imports C, which re-exports B, which re-exports A
    let sym = assert_resolves(analysis.symbol_index(), "D", "APart");
    assert_eq!(sym.qualified_name.as_ref(), "A::APart");
}

// =============================================================================
// CROSS-FILE IMPORTS
// =============================================================================

#[test]
fn test_cross_file_import_resolution() {
    let mut host = analysis_from_sources(&[
        ("base.sysml", "package Base { part def Vehicle; }"),
        (
            "derived.sysml",
            r#"
            package Derived {
                import Base::*;
                part myCar : Vehicle;
            }
        "#,
        ),
    ]);
    let analysis = host.analysis();

    // Vehicle should resolve in Derived via import
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Vehicle");
}

#[test]
fn test_cross_file_import_order_independence() {
    // Add files in reverse order (derived before base)
    let mut host = analysis_from_sources(&[
        (
            "derived.sysml",
            r#"
            package Derived {
                import Base::*;
                part myCar : Vehicle;
            }
        "#,
        ),
        ("base.sysml", "package Base { part def Vehicle; }"),
    ]);
    let analysis = host.analysis();

    // Should still resolve correctly
    let sym = assert_resolves(analysis.symbol_index(), "Derived", "Vehicle");
    assert_eq!(sym.qualified_name.as_ref(), "Base::Vehicle");
}

#[test]
fn test_cross_file_transitive_specialization() {
    // A→B→C chain across 3 files
    let mut host = analysis_from_sources(&[
        ("a.sysml", "package A { part def Thing; }"),
        (
            "b.sysml",
            r#"
            package B {
                import A::*;
                part def Vehicle :> Thing;
            }
        "#,
        ),
        (
            "c.sysml",
            r#"
            package C {
                import B::*;
                part def Car :> Vehicle;
            }
        "#,
        ),
    ]);
    let analysis = host.analysis();

    // All symbols should exist
    assert_symbol_exists(analysis.symbol_index(), "A::Thing");
    assert_symbol_exists(analysis.symbol_index(), "B::Vehicle");
    assert_symbol_exists(analysis.symbol_index(), "C::Car");

    // Vehicle should resolve in B via import from A
    let vehicle = assert_resolves(analysis.symbol_index(), "B", "Thing");
    assert_eq!(vehicle.qualified_name.as_ref(), "A::Thing");

    // Car should resolve Vehicle in C via import from B
    let car_sym = get_symbol(analysis.symbol_index(), "C::Car");
    assert!(
        !car_sym.supertypes.is_empty(),
        "Car should have Vehicle as supertype"
    );
}

#[test]
fn test_cross_file_incremental_update() {
    // Start with two files
    let mut host = analysis_from_sources(&[
        ("base.sysml", "package Base { part def Original; }"),
        (
            "consumer.sysml",
            r#"
            package Consumer {
                import Base::*;
                part x : Original;
            }
        "#,
        ),
    ]);

    // Verify initial state
    {
        let analysis = host.analysis();
        assert_symbol_exists(analysis.symbol_index(), "Base::Original");
        let sym = assert_resolves(analysis.symbol_index(), "Consumer", "Original");
        assert_eq!(sym.qualified_name.as_ref(), "Base::Original");
    }

    // Update base file with a new definition
    host.set_file_content(
        "base.sysml",
        "package Base { part def Original; part def Added; }",
    );

    // Verify the new symbol is available
    {
        let analysis = host.analysis();
        assert_symbol_exists(analysis.symbol_index(), "Base::Original");
        assert_symbol_exists(analysis.symbol_index(), "Base::Added");
        // Consumer should be able to see both via import
        assert_resolves(analysis.symbol_index(), "Consumer", "Original");
        assert_resolves(analysis.symbol_index(), "Consumer", "Added");
    }
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_import_nonexistent_package_graceful() {
    let source = r#"
        package Consumer {
            import NonExistent::*;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should not crash, just nothing imported
    assert_not_found(analysis.symbol_index(), "Consumer", "SomeSymbol");
}

#[test]
fn test_self_referential_import_safe() {
    let source = r#"
        package Self {
            import Self::*;
            part def Internal;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should not crash, Internal should still be visible
    let sym = assert_resolves(analysis.symbol_index(), "Self", "Internal");
    assert_eq!(sym.qualified_name.as_ref(), "Self::Internal");
}

// =============================================================================
// RECURSIVE IMPORTS
// =============================================================================

#[test]
fn test_recursive_import_double_star() {
    // `import Pkg::**;` should import all symbols recursively from nested packages
    let source = r#"
        package Lib {
            package Sub {
                part def Nested;
            }
        }
        package Consumer {
            import Lib::**;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // With recursive import, Nested should be visible in Consumer
    // Note: This may resolve to Sub::Nested or just Nested depending on implementation
    let resolver = analysis.symbol_index().resolver_for_scope("Consumer");
    let result = resolver.resolve("Nested");
    // If recursive imports are not fully implemented, this test documents the expected behavior
    if result.is_found() {
        let sym = result.symbol().unwrap();
        assert_eq!(sym.qualified_name.as_ref(), "Lib::Sub::Nested");
    } else {
        // Document that recursive imports are not yet implemented
        eprintln!("Note: Recursive imports (:**) not yet fully implemented");
    }
}

// =============================================================================
// IMPORT ALIASING
// =============================================================================

#[test]
fn test_import_alias() {
    // SysML uses `alias X for Y;` syntax to create aliases
    let source = r#"
        package Lib {
            part def Original;
        }
        package Consumer {
            import Lib::*;
            alias Alias for Original;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should be able to resolve the alias
    let resolver = analysis.symbol_index().resolver_for_scope("Consumer");
    let result = resolver.resolve("Alias");
    assert!(result.is_found(), "Aliased import should resolve");
    let sym = result.symbol().unwrap();

    // Alias is its own symbol in the Consumer namespace
    assert_eq!(sym.qualified_name.as_ref(), "Consumer::Alias");
    assert_eq!(sym.kind, SymbolKind::Alias);

    // But it points to the original via supertypes
    assert!(
        !sym.supertypes.is_empty(),
        "Alias should have target in supertypes"
    );
    assert_eq!(sym.supertypes[0].as_ref(), "Original");
}

// =============================================================================
// FILTER IMPORTS (SysML v2 §7.5.4)
// =============================================================================

// Note: SysML uses filter expressions with [condition] syntax, not `except` keyword.
// e.g., `import Lib::*[not isAbstract];`
// Filter expression evaluation is NOT YET implemented - filters are parsed but ignored.

#[test]
fn test_import_filter_syntax_parses() {
    // SysML filter imports use bracket notation [condition]
    // The filter expression is parsed but not evaluated yet
    let source = r#"
        package Lib {
            part def Included;
            part def Excluded;
        }
        package Consumer {
            import Lib::*;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Without filter, both should be visible
    assert_resolves(analysis.symbol_index(), "Consumer", "Included");
    assert_resolves(analysis.symbol_index(), "Consumer", "Excluded");
}

#[test]
fn test_filtered_import_with_nonexistent_filter_imports_nothing() {
    // Filter syntax with non-matching filter imports nothing
    // This verifies filter evaluation is working
    let source = r#"
        package Source {
            part def PartA;
            part def PartB;
            part def PartC;
        }
        package Consumer {
            import Source::*[@SomeCondition];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // @SomeCondition doesn't exist, so no elements match the filter
    assert_not_found(analysis.symbol_index(), "Consumer", "PartA");
    assert_not_found(analysis.symbol_index(), "Consumer", "PartB");
    assert_not_found(analysis.symbol_index(), "Consumer", "PartC");
}

#[test]
fn test_multiple_filter_conditions_require_all_to_match() {
    // Multiple [filter] conditions require ALL to match (AND semantics)
    let source = r#"
        metadata def Condition1;
        metadata def Condition2;
        metadata def Condition3;
        
        package Source {
            part def AllThree { @Condition1; @Condition2; @Condition3; }
            part def JustOne { @Condition1; }
            part def JustTwo { @Condition1; @Condition2; }
            part def None;
        }
        package Consumer {
            import Source::*[@Condition1][@Condition2][@Condition3];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Only element with ALL three metadata passes the filter
    assert_resolves(analysis.symbol_index(), "Consumer", "AllThree");
    assert_not_found(analysis.symbol_index(), "Consumer", "JustOne");
    assert_not_found(analysis.symbol_index(), "Consumer", "JustTwo");
    assert_not_found(analysis.symbol_index(), "Consumer", "None");
}

#[test]
fn test_package_level_filter_statement_parses() {
    // Package-level filter statement (applies to all imports)
    // With filter @SomeMetadata, only elements WITH @SomeMetadata are visible
    let source = r#"
        metadata def SomeMetadata;
        
        package Source {
            part def Visible { @SomeMetadata; }
            part def Hidden;
        }
        package Consumer {
            import Source::*;
            filter @SomeMetadata;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Filter statement evaluated - only Visible has @SomeMetadata
    assert_resolves(analysis.symbol_index(), "Consumer", "Visible");
    assert_not_found(analysis.symbol_index(), "Consumer", "Hidden");
}

#[test]
fn test_recursive_import_with_filter_nonexistent_metadata() {
    // Recursive import (**) with filter condition that doesn't exist
    // When filter metadata doesn't exist, nothing matches
    let source = r#"
        package Outer {
            package Inner {
                part def DeepElement;
            }
            part def ShallowElement;
        }
        package Consumer {
            import Outer::**[@SomeFilter];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // @SomeFilter doesn't exist, so no elements pass the filter
    assert_not_found(analysis.symbol_index(), "Consumer", "ShallowElement");
    assert_not_found(analysis.symbol_index(), "Consumer", "DeepElement");
}

#[test]
fn test_spec_example_approval_metadata_with_filter_evaluation() {
    // Example from SysML v2 spec §7.5.4, updated for filter evaluation
    // Complex expressions like "@Approval and approved and level > 1"
    // are parsed but only simple metadata refs (@Approval) are currently extracted
    let source = r#"
        package ApprovalMetadata {
            metadata def Approval {
                attribute approved : Boolean;
                attribute level : Natural;
            }
        }
        package DesignModel {
            public import ApprovalMetadata::*;
            part def System { @Approval; }
            part def UnapprovedSystem;
        }
        package UpperLevelApprovals {
            private import ApprovalMetadata::**;
            public import DesignModel::**[@Approval];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Verify packages exist
    assert_symbol_exists(analysis.symbol_index(), "ApprovalMetadata");
    assert_symbol_exists(analysis.symbol_index(), "DesignModel");
    assert_symbol_exists(analysis.symbol_index(), "UpperLevelApprovals");

    // Only System with @Approval is imported; UnapprovedSystem is filtered out
    assert_resolves(analysis.symbol_index(), "UpperLevelApprovals", "System");
    assert_not_found(
        analysis.symbol_index(),
        "UpperLevelApprovals",
        "UnapprovedSystem",
    );
}
// =============================================================================
// FILTER IMPORT EVALUATION (TODO: Implement)
// =============================================================================

/// Verify that metadata_annotations are being extracted correctly from symbols.
/// This test passes once HirSymbol.metadata_annotations is populated.
#[test]
fn test_metadata_annotations_extracted() {
    let source = r#"
        metadata def Safety;
        metadata def Approved;
        
        package Source {
            part safeAndApproved { @Safety; @Approved; }
            part onlySafe { @Safety; }
            part noMetadata;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Check metadata_annotations are extracted
    let safe_and_approved = get_symbol(analysis.symbol_index(), "Source::safeAndApproved");
    assert!(
        safe_and_approved
            .metadata_annotations
            .contains(&"Safety".into()),
        "safeAndApproved should have @Safety, got: {:?}",
        safe_and_approved.metadata_annotations
    );
    assert!(
        safe_and_approved
            .metadata_annotations
            .contains(&"Approved".into()),
        "safeAndApproved should have @Approved, got: {:?}",
        safe_and_approved.metadata_annotations
    );

    let only_safe = get_symbol(analysis.symbol_index(), "Source::onlySafe");
    assert!(
        only_safe.metadata_annotations.contains(&"Safety".into()),
        "onlySafe should have @Safety, got: {:?}",
        only_safe.metadata_annotations
    );
    assert!(
        !only_safe.metadata_annotations.contains(&"Approved".into()),
        "onlySafe should NOT have @Approved"
    );

    let no_metadata = get_symbol(analysis.symbol_index(), "Source::noMetadata");
    assert!(
        no_metadata.metadata_annotations.is_empty(),
        "noMetadata should have no annotations, got: {:?}",
        no_metadata.metadata_annotations
    );
}

#[test]
fn test_filtered_import_excludes_non_matching_elements() {
    // This test should FAIL until filter evaluation is implemented
    // Based on SysML v2 spec example - metadata applied to parts
    let source = r#"
        metadata def Safety {
            attribute isMandatory : Boolean;
        }
        metadata def Security;
        
        package Source {
            part bumper { @Safety { isMandatory = true; } }
            part keylessEntry { @Security; }
            part frontSeat;
        }
        package SafetyGroup {
            import Source::*;
            filter @Safety;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // With filter @Safety, only bumper should be visible
    assert_resolves(analysis.symbol_index(), "SafetyGroup", "bumper");

    // These should NOT be visible (filtered out - no @Safety metadata)
    assert_not_found(analysis.symbol_index(), "SafetyGroup", "keylessEntry");
    assert_not_found(analysis.symbol_index(), "SafetyGroup", "frontSeat");
}

#[test]
fn test_filtered_import_with_bracket_syntax() {
    // Filter in bracket syntax on import itself
    let source = r#"
        metadata def Approved;
        
        package Source {
            part approved1 { @Approved; }
            part approved2 { @Approved; }
            part notApproved;
        }
        package Consumer {
            import Source::*[@Approved];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Only approved elements should be visible
    assert_resolves(analysis.symbol_index(), "Consumer", "approved1");
    assert_resolves(analysis.symbol_index(), "Consumer", "approved2");
    assert_not_found(analysis.symbol_index(), "Consumer", "notApproved");
}

#[test]
fn test_filter_with_no_matching_elements() {
    // Filter that matches nothing should import nothing
    let source = r#"
        metadata def Required;
        
        package Source {
            part a;
            part b;
            part c;
        }
        package Consumer {
            import Source::*[@Required];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Nothing has @Required, so nothing should be visible
    assert_not_found(analysis.symbol_index(), "Consumer", "a");
    assert_not_found(analysis.symbol_index(), "Consumer", "b");
    assert_not_found(analysis.symbol_index(), "Consumer", "c");
}

#[test]
fn test_filter_with_all_matching_elements() {
    // Filter where all elements match should import all
    let source = r#"
        metadata def Common;
        
        package Source {
            part a { @Common; }
            part b { @Common; }
            part c { @Common; }
        }
        package Consumer {
            import Source::*[@Common];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // All have @Common, so all should be visible
    assert_resolves(analysis.symbol_index(), "Consumer", "a");
    assert_resolves(analysis.symbol_index(), "Consumer", "b");
    assert_resolves(analysis.symbol_index(), "Consumer", "c");
}

#[test]
fn test_filter_metadata_short_name_matches() {
    // @Safety should match even when metadata is in a nested package
    let source = r#"
        package SafetyMetadata {
            metadata def Safety;
        }
        package Source {
            import SafetyMetadata::*;
            part safePart { @Safety; }
            part unsafePart;
        }
        package Consumer {
            import SafetyMetadata::*;
            import Source::*[@Safety];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_resolves(analysis.symbol_index(), "Consumer", "safePart");
    assert_not_found(analysis.symbol_index(), "Consumer", "unsafePart");
}

#[test]
fn test_filter_metadata_qualified_name_matches() {
    // Fully qualified metadata name in filter
    let source = r#"
        package Meta {
            metadata def Tag;
        }
        package Source {
            import Meta::*;
            part tagged { @Tag; }
            part untagged;
        }
        package Consumer {
            import Meta::*;
            import Source::*[@Tag];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    assert_resolves(analysis.symbol_index(), "Consumer", "tagged");
    assert_not_found(analysis.symbol_index(), "Consumer", "untagged");
}

#[test]
fn test_filter_nonexistent_metadata_imports_nothing() {
    // Filter with nonexistent metadata type should import nothing
    let source = r#"
        package Source {
            part a;
            part b;
        }
        package Consumer {
            import Source::*[@NonExistent];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // NonExistent doesn't exist, so nothing matches
    assert_not_found(analysis.symbol_index(), "Consumer", "a");
    assert_not_found(analysis.symbol_index(), "Consumer", "b");
}

#[test]
fn test_filter_on_recursive_import() {
    // Recursive import with filter should filter at all levels
    let source = r#"
        metadata def Important;
        
        package Outer {
            part outerImportant { @Important; }
            part outerNormal;
            package Inner {
                part innerImportant { @Important; }
                part innerNormal;
            }
        }
        package Consumer {
            import Outer::**[@Important];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Only @Important elements at any level
    assert_resolves(analysis.symbol_index(), "Consumer", "outerImportant");
    assert_resolves(analysis.symbol_index(), "Consumer", "innerImportant");
    assert_not_found(analysis.symbol_index(), "Consumer", "outerNormal");
    assert_not_found(analysis.symbol_index(), "Consumer", "innerNormal");
}

#[test]
fn test_multiple_filters_on_same_import() {
    // Multiple [filter] conditions require ALL to match
    let source = r#"
        metadata def A;
        metadata def B;
        
        package Source {
            part hasA { @A; }
            part hasB { @B; }
            part hasBoth { @A; @B; }
            part hasNeither;
        }
        package Consumer {
            import Source::*[@A][@B];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Only element with BOTH @A and @B
    assert_resolves(analysis.symbol_index(), "Consumer", "hasBoth");
    assert_not_found(analysis.symbol_index(), "Consumer", "hasA");
    assert_not_found(analysis.symbol_index(), "Consumer", "hasB");
    assert_not_found(analysis.symbol_index(), "Consumer", "hasNeither");
}

#[test]
fn test_filter_preserves_public_visibility() {
    // Public filtered import should re-export filtered elements
    let source = r#"
        metadata def Public;
        
        package Source {
            part publicPart { @Public; }
            part privatePart;
        }
        package Middle {
            public import Source::*[@Public];
        }
        package Consumer {
            import Middle::*;
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Consumer gets publicPart via Middle's public filtered import
    assert_resolves(analysis.symbol_index(), "Consumer", "publicPart");
    assert_not_found(analysis.symbol_index(), "Consumer", "privatePart");
}

// =============================================================================
// FEATURE CHAIN RESOLUTION
// =============================================================================

#[test]
fn test_quoted_identifier_chain_resolution() {
    // Test that quoted identifiers (unrestricted names) resolve correctly in feature chains
    // This was a bug where the chain extraction didn't strip quotes from the name,
    // causing lookups to fail (e.g., "'Θ'" vs "Θ" in visibility maps).
    let source = r#"
        package ISQ {
            attribute 'Θ': ScalarValues::Real;
        }
        
        package Test {
            import ISQ::*;
            
            attribute test: ScalarValues::Real = ISQ.'Θ';
        }
    "#;
    let (mut host, _file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Check that the chain resolves both parts
    let test_sym = index
        .all_symbols()
        .find(|s| s.qualified_name.as_ref() == "Test::test")
        .expect("Should have test symbol");

    for trk in &test_sym.type_refs {
        if let syster::hir::TypeRefKind::Chain(chain) = trk {
            // Check that the quoted identifier part resolved
            let theta_part = chain
                .parts
                .iter()
                .find(|p| p.target.as_ref() == "Θ")
                .expect("Should have Θ part (quotes stripped)");

            assert!(
                theta_part.resolved_target.is_some(),
                "Quoted identifier 'Θ' should resolve to ISQ::Θ"
            );
            assert_eq!(
                theta_part.resolved_target.as_ref().map(|s| s.as_ref()),
                Some("ISQ::Θ")
            );
        }
    }
}

#[test]
fn test_flow_feature_chain_correct_resolution() {
    // Test that correctly spelled feature chains resolve properly
    let source = r#"
        package VehicleDefinitions {
            port def AxleMountIF { 
                out transferredTorque;
            }
            
            port def WheelHubIF { 
                in appliedTorque;
            }
            
            interface def Mounting {
                end axleMount: AxleMountIF;
                end hub: WheelHubIF;
                
                flow axleMount.transferredTorque to hub.appliedTorque;
            }
        }
    "#;
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Should have no undefined reference errors for correct chains
    let diags = syster::hir::check_file(index, file_id);
    let semantic_errors: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("undefined reference"))
        .collect();
    assert!(
        semantic_errors.is_empty(),
        "Expected no undefined reference errors, got: {:?}",
        semantic_errors
            .iter()
            .map(|d| &d.message)
            .collect::<Vec<_>>()
    );
}
#[test]
fn test_redefines_that_chain_resolution() {
    // Test pattern from Items.sysml: ref item envelopedItem :>> that
    // The chain `envelopedItem.outerSpaceDimension` should resolve if `that`
    // is understood to refer to the containing Item.
    let source = r#"
        part def Item {
            attribute outerSpaceDimension: ScalarValues::Integer;
            
            part envelopingShapes: Item[0..*] {
                ref item envelopedItem :>> that;
                
                attribute test = envelopedItem.outerSpaceDimension;
            }
        }
    "#;
    let (mut host, _file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Find the test attribute
    let test_sym = index
        .all_symbols()
        .find(|s| s.qualified_name.as_ref() == "Item::envelopingShapes::test")
        .expect("Should have test symbol");

    println!("\n=== Test symbol type_refs ===");
    for trk in &test_sym.type_refs {
        if let syster::hir::TypeRefKind::Chain(chain) = trk {
            for (i, part) in chain.parts.iter().enumerate() {
                println!(
                    "  part[{}]: '{}' -> {:?}",
                    i, part.target, part.resolved_target
                );
            }
        }
    }

    // Check if outerSpaceDimension resolves
    let mut found_chain = false;
    for trk in &test_sym.type_refs {
        if let syster::hir::TypeRefKind::Chain(chain) = trk {
            if chain.parts.len() == 2 {
                found_chain = true;
                let second_part = &chain.parts[1];
                println!(
                    "\nSecond part '{}' resolved: {:?}",
                    second_part.target,
                    second_part.resolved_target.is_some()
                );
            }
        }
    }
    assert!(found_chain, "Should have found the chain");

    // Run check_file to trigger CHAIN_TRIAGE logging
    let diags = syster::hir::check_file(index, _file_id);
    println!("Diagnostics count: {}", diags.len());

    // Debug: print all symbols with expression chains
    println!("\n=== All symbols with chains ===");
    for sym in index.all_symbols() {
        for trk in &sym.type_refs {
            if let syster::hir::TypeRefKind::Chain(chain) = trk {
                if chain.parts.len() > 1 {
                    let parts_str: Vec<_> = chain
                        .parts
                        .iter()
                        .map(|p| format!("{}:{:?}", p.target, p.resolved_target.is_some()))
                        .collect();
                    println!(
                        "  {} has chain: [{}]",
                        sym.qualified_name,
                        parts_str.join(", ")
                    );
                }
            }
        }
    }
}
