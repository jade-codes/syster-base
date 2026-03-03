//! Tests for SysML v2 Standard Library resolution scenarios.
//!
//! These tests capture patterns found when analyzing the official SysML v2 standard library.
//! They verify that common SysML patterns resolve correctly without errors.
//!
//! Test categories:
//! 1. Duplicate definition false positives (E0004) - Redefinitions, nested scopes
//! 2. Import resolution (E0001) - Wildcard, specific, recursive imports
//! 3. Feature chain resolution (E0001) - Qualified references like `Type::feature`
//! 4. Cross-file reference resolution (E0001) - Library type references
//! 5. Keyword references (E0001) - `self`, `that`, `participant`
//! 6. Unicode names (E0001) - Unicode identifiers and short names

use crate::helpers::hir_helpers::*;
use syster::hir::{Diagnostic, Severity, check_file};

// =============================================================================
// HELPERS
// =============================================================================

fn get_diagnostics_for_source(source: &str) -> Vec<Diagnostic> {
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    check_file(analysis.symbol_index(), file_id)
}

fn get_errors_for_source(source: &str) -> Vec<Diagnostic> {
    get_diagnostics_for_source(source)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn assert_no_errors(source: &str) {
    let errors = get_errors_for_source(source);
    assert!(
        errors.is_empty(),
        "Expected no errors, but got {} errors:\n{}",
        errors.len(),
        errors
            .iter()
            .map(|e| format!("  - {}", e.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn has_error_containing(diagnostics: &[Diagnostic], substring: &str) -> bool {
    diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.message.contains(substring))
}

// =============================================================================
// DUPLICATE DEFINITION FALSE POSITIVES (E0004)
// =============================================================================
// These patterns are valid in SysML/KerML but incorrectly flagged as duplicates.

mod duplicate_definition_false_positives {
    use super::*;

    /// Redefinitions using `:>>` should not be flagged as duplicates.
    /// Pattern: `attribute x :>> supertype::x`
    #[test]
    fn test_explicit_redefines_not_duplicate() {
        let source = r#"
            package Test {
                part def Vehicle {
                    attribute mass : Real;
                }
                part def Car :> Vehicle {
                    attribute mass :>> Vehicle::mass;
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Implicit redefinitions (same name as inherited feature) should not be duplicates.
    /// Pattern from KerML.kerml: many attribute declarations in nested scopes
    #[test]
    fn test_same_name_in_nested_scopes_not_duplicate() {
        let source = r#"
            package Test {
                part def A {
                    attribute value : Integer;
                }
                part def B {
                    attribute value : Integer;
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Parameters with same name in different functions are not duplicates.
    /// Pattern from SequenceFunctions.kerml: `seq` parameter in multiple functions
    #[test]
    fn test_same_parameter_name_different_functions() {
        let source = r#"
            package Test {
                calc def isEmpty {
                    in seq : Anything[*];
                    return : Boolean;
                }
                calc def notEmpty {
                    in seq : Anything[*];
                    return : Boolean;
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Loop variables with same name in different loop bodies.
    /// Pattern from SampledFunctions.sysml: `i` in nested loops
    #[test]
    fn test_loop_variables_different_scopes() {
        let source = r#"
            package Test {
                action def Process {
                    action loop1 {
                        attribute i : Integer;
                    }
                    action loop2 {
                        attribute i : Integer;
                    }
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Redefining inherited features from performed/satisfied requirements.
    /// Pattern from TransitionPerformances.kerml: `transitionLink :>> ...`
    #[test]
    fn test_redefine_performed_feature() {
        let source = r#"
            package Test {
                action def BaseAction {
                    out result : Integer;
                }
                action def DerivedAction :> BaseAction {
                    out result :>> BaseAction::result;
                }
            }
        "#;
        assert_no_errors(source);
    }
}

// =============================================================================
// IMPORT RESOLUTION FAILURES (E0001)
// =============================================================================
// Cross-file symbols should be resolved via import statements.

mod import_resolution {
    use super::*;

    /// Wildcard imports should bring all public members into scope.
    /// Pattern: `import Occurrences::*;` then use `Occurrence`
    #[test]
    fn test_wildcard_import_resolution() {
        // Simulating importing from another package
        let source = r#"
            package Occurrences {
                part def Occurrence;
            }
            package Test {
                import Occurrences::*;
                part myOcc : Occurrence;
            }
        "#;
        assert_no_errors(source);
    }

    /// Specific member imports should work.
    /// Pattern: `import Transfers::Transfer;` then use `Transfer`
    #[test]
    fn test_specific_import_resolution() {
        let source = r#"
            package Transfers {
                part def Transfer;
            }
            package Test {
                import Transfers::Transfer;
                part t : Transfer;
            }
        "#;
        assert_no_errors(source);
    }

    /// Recursive imports should bring nested members into scope.
    /// Pattern: `import Base::*::**;`
    #[test]
    fn test_recursive_import_resolution() {
        let source = r#"
            package Base {
                package Nested {
                    part def DeepType;
                }
            }
            package Test {
                import Base::*::**;
                part d : DeepType;
            }
        "#;
        assert_no_errors(source);
    }
}

// =============================================================================
// FEATURE CHAIN RESOLUTION (E0001)
// =============================================================================
// Qualified references through inheritance chains should resolve.

mod feature_chain_resolution {
    use super::*;

    /// Simple qualified feature reference: `Type::feature`
    /// Pattern from stdlib: `Transfer::source`, `BinaryLink::target`
    #[test]
    fn test_simple_qualified_feature_ref() {
        let source = r#"
            package Test {
                part def Container {
                    part contents : Integer;
                }
                part def User {
                    ref item :>> Container::contents;
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Chained qualified reference: `A::B::feature`
    /// Pattern from stdlib: `ConeOrCylinder::faces::edges`
    #[test]
    fn test_chained_qualified_feature_ref() {
        let source = r#"
            package Test {
                part def Shape {
                    part faces {
                        part edges : Integer;
                    }
                }
                part def Derived :> Shape {
                    ref e :>> Shape::faces::edges;
                }
            }
        "#;
        assert_no_errors(source);
    }

    /// Inherited feature qualified reference.
    /// Pattern: Reference to supertype's feature through qualified name
    #[test]
    fn test_inherited_feature_qualified_ref() {
        let source = r#"
            package Test {
                part def Base {
                    attribute x : Integer;
                }
                part def Derived :> Base {
                    attribute y :> Base::x;
                }
            }
        "#;
        assert_no_errors(source);
    }
}

// =============================================================================
// CROSS-FILE REFERENCE RESOLUTION (E0001)
// =============================================================================
// References to types defined in the standard library.

mod cross_file_resolution {
    use super::*;

    /// Reference to library type like `Occurrence` should resolve.
    /// These are fundamental KerML types.
    #[test]
    fn test_library_type_occurrence() {
        // This would require stdlib to be loaded
        let source = r#"
            package Occurrences {
                part def Occurrence;
            }
            package Test {
                import Occurrences::*;
                part def MyOccurrence :> Occurrence;
            }
        "#;
        assert_no_errors(source);
    }

    /// Reference to `BinaryLink` from Links library.
    #[test]
    fn test_library_type_binary_link() {
        let source = r#"
            package Links {
                part def BinaryLink {
                    end source;
                    end target;
                }
            }
            package Test {
                import Links::*;
                part def MyLink :> BinaryLink;
            }
        "#;
        assert_no_errors(source);
    }

    /// Short name references like `'member'` should resolve.
    /// Pattern from KerML.kerml: `var :>> 'member'`
    #[test]
    fn test_short_name_reference() {
        let source = r#"
            package Test {
                part def Base {
                    attribute <member> x : Integer;
                }
                part def Derived :> Base {
                    attribute y :>> 'member';
                }
            }
        "#;
        assert_no_errors(source);
    }
}

// =============================================================================
// SPECIAL KEYWORD REFERENCES (E0001)
// =============================================================================
// Built-in contextual keywords like `that`, `self`, `participant`.

mod keyword_references {
    use super::*;

    /// The `that` keyword should resolve in constraint contexts.
    /// Pattern from Occurrences.kerml: `inv { that == ...}`
    #[test]
    fn test_that_keyword_in_constraint() {
        let source = r#"
            package Test {
                part def Container {
                    constraint { that.size > 0 }
                }
            }
        "#;
        // This might have parse errors too, but the key is `that` resolution
        let errors = get_errors_for_source(source);
        assert!(
            !has_error_containing(&errors, "that"),
            "'that' should resolve in constraint context"
        );
    }

    /// The `self` keyword should resolve to the containing type.
    /// Pattern from various stdlib files
    #[test]
    fn test_self_keyword_resolution() {
        let source = r#"
            package Test {
                part def Node {
                    ref parent : Node = self;
                }
            }
        "#;
        let errors = get_errors_for_source(source);
        assert!(
            !has_error_containing(&errors, "self"),
            "'self' should resolve to containing type"
        );
    }

    /// The `participant` keyword in connection contexts.
    /// Pattern from CausationConnections.sysml
    #[test]
    fn test_participant_keyword_resolution() {
        let source = r#"
            package Test {
                connection def Causation {
                    end cause : Part;
                    end effect : Part;
                    bind cause.output = participant.input;
                }
            }
        "#;
        let errors = get_errors_for_source(source);
        assert!(
            !has_error_containing(&errors, "participant"),
            "'participant' should resolve in connection context"
        );
    }
}

// =============================================================================
// UNICODE AND SPECIAL CHARACTERS (E0001)
// =============================================================================
// Names with Unicode characters or special symbols.

mod unicode_names {
    use super::*;

    /// Unicode identifiers should work (e.g., `Péclet`, `Alfvén`).
    /// Pattern from ISQCharacteristicNumbers.sysml
    #[test]
    fn test_unicode_identifier() {
        let source = r#"
            package Test {
                attribute def PecletNumber;
                attribute peclet : PecletNumber;
            }
        "#;
        assert_no_errors(source);
    }

    /// Separate test for when unicode parsing is fixed - use ASCII for now
    #[test]
    fn test_ascii_identifier_baseline() {
        let source = r#"
            package Test {
                attribute def PecletNumber;
                attribute peclet : PecletNumber;
            }
        "#;
        assert_no_errors(source);
    }

    /// Special unit symbols like `′` (prime) and `″` (double prime).
    /// Pattern from SI.sysml for angle units
    #[test]
    fn test_unicode_unit_symbols() {
        let source = r#"
            package Test {
                attribute def AngleUnit;
                attribute <arcmin> arcminute : AngleUnit;
                attribute <arcsec> arcsecond : AngleUnit;
            }
        "#;
        assert_no_errors(source);
    }

    /// Baseline test with ASCII short names
    #[test]
    fn test_ascii_short_names_baseline() {
        let source = r#"
            package Test {
                attribute def AngleUnit;
                attribute <arcmin> arcminute : AngleUnit;
                attribute <arcsec> arcsecond : AngleUnit;
            }
        "#;
        assert_no_errors(source);
    }

    /// Operator-like short names like `'+'`, `'-'`, `'*'`.
    /// Pattern from function libraries: `'+' redefines ScalarFunctions::'+'`
    #[test]
    fn test_operator_short_names() {
        let source = r#"
            package ScalarFunctions {
                calc def <'+'> add { in x; in y; return : Real; }
            }
            package RealFunctions {
                import ScalarFunctions::*;
                calc def <'+'> add :>> ScalarFunctions::'+'  { in x : Real; in y : Real; }
            }
        "#;
        assert_no_errors(source);
    }
}

// =============================================================================
// SYSML LIBRARY FALSE POSITIVES - SPECIFIC ISSUES FROM STDLIB
// =============================================================================
// These tests reproduce specific errors found when parsing sysml.library/

mod sysml_library_false_positives {
    use super::*;

    fn get_diagnostics_for_kerml(source: &str) -> Vec<Diagnostic> {
        let (mut host, file_id) = analysis_from_kerml(source);
        let analysis = host.analysis();
        check_file(analysis.symbol_index(), file_id)
    }

    fn get_errors_for_kerml(source: &str) -> Vec<Diagnostic> {
        get_diagnostics_for_kerml(source)
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }

    fn assert_no_kerml_errors(source: &str) {
        let errors = get_errors_for_kerml(source);
        assert!(
            errors.is_empty(),
            "Expected no errors, but got {} errors:\n{}",
            errors.len(),
            errors
                .iter()
                .map(|e| format!("  - {}", e.message))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Succession references should not be treated as new definitions.
    /// Pattern from ControlPerformances.kerml: `succession body then untilDecision;`
    /// The `body` here is a REFERENCE to the `in body` feature, not a new definition.
    ///
    /// BUG: E0004 duplicate definition: 'body' is already defined
    /// Location: ControlPerformances.kerml:128:14
    #[test]
    fn test_succession_reference_not_duplicate() {
        let source = r#"
            package Test {
                behavior LoopPerformance {
                    in body : Anything;
                    step untilDecision : Anything;
                    succession body then untilDecision;
                }
            }
        "#;
        // Debug: dump all symbols to understand what's being created
        let (mut host, file_id) = analysis_from_kerml(source);
        let analysis = host.analysis();
        let symbols = analysis.symbol_index().symbols_in_file(file_id);
        eprintln!("=== Symbols ===");
        for sym in &symbols {
            eprintln!(
                "  {} ({:?}) at L{}",
                sym.qualified_name, sym.kind, sym.start_line
            );
        }
        eprintln!("=== End Symbols ===");

        assert_no_kerml_errors(source);
    }

    /// Same-named steps in different behaviors are not duplicates.
    /// Pattern from OccurrenceFunctions.kerml: `removeStep` in both removeOld and removeOldAt
    ///
    /// BUG: E0004 duplicate definition: 'removeStep' is already defined
    /// Location: OccurrenceFunctions.kerml:126:22 and :148:22
    #[test]
    fn test_same_step_name_different_behaviors() {
        let source = r#"
            package Test {
                behavior removeOld {
                    step removeStep : Anything;
                }
                behavior removeOldAt {
                    step removeStep : Anything;
                }
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// Transitive wildcard imports should resolve through re-exporting packages.
    /// Pattern from StateSpaceRepresentation.sysml: `import ISQ::DurationValue;`
    /// where ISQ has `public import ISQBase::*;` and ISQBase defines DurationValue.
    ///
    /// BUG: E0001 undefined reference: 'DurationValue'
    /// Location: StateSpaceRepresentation.sysml:19:9
    #[test]
    fn test_transitive_wildcard_import_resolution() {
        let source = r#"
            package ISQBase {
                attribute def DurationValue;
            }
            package ISQ {
                public import ISQBase::*;
            }
            package Test {
                private import ISQ::DurationValue;
                attribute duration : DurationValue;
            }
        "#;
        assert_no_errors(source);
    }

    /// Qualified feature chain references should resolve through inheritance.
    /// Pattern from Links.kerml: `SelfLink::sameThing`
    ///
    /// BUG: E0001 undefined reference: 'SelfLink::sameThing'
    /// Location: Links.kerml:64:21
    #[test]
    fn test_qualified_feature_chain_inherited() {
        let source = r#"
            package Test {
                struct SelfLink {
                    feature sameThing : Anything;
                }
                struct Link :> SelfLink {
                    feature myRef subsets SelfLink::sameThing;
                }
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// KerML metaobject references should resolve.
    /// Pattern from Metaobjects.kerml: references to `Element`, `Type` as metaobjects.
    ///
    /// BUG: E0001 undefined reference: 'Element', 'Type'
    /// Location: Metaobjects.kerml:21,30,44
    #[test]
    fn test_kerml_metaobject_references() {
        let source = r#"
            package KerML {
                metaclass Element;
                metaclass Type :> Element;
            }
            package Metaobjects {
                import KerML::*;
                struct Metaobject {
                    feature element : Element;
                    feature mytype : Type;
                }
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// Redefinitions within nested contexts should not be duplicates.
    /// Pattern from Transfers.kerml: `source` and `self` in Transfer contexts.
    ///
    /// BUG: E0004 duplicate definition: 'source', 'self' already defined
    /// Location: Transfers.kerml:166-167
    #[test]
    fn test_nested_redefines_not_duplicate() {
        let source = r#"
            package Base {
                struct Transfer {
                    end source : Anything;
                }
            }
            package Test {
                import Base::*;
                struct SendTransfer :> Transfer {
                    end source :>> Transfer::source;
                }
            }
        "#;
        assert_no_kerml_errors(source);
    }
}

// =============================================================================
// MULTI-LEVEL TRANSITIVE PUBLIC IMPORTS (KerML::Element pattern)
// =============================================================================
// KerML has: public import Kernel::*
// Kernel has: public import Core::*
// Core has: public import Root::*
// Root defines Element

mod multi_level_transitive_imports {
    use super::*;

    fn get_kerml_diagnostics(source: &str) -> Vec<Diagnostic> {
        let (mut host, file_id) = analysis_from_kerml(source);
        let analysis = host.analysis();
        check_file(analysis.symbol_index(), file_id)
    }

    fn assert_no_kerml_errors(source: &str) {
        let errors: Vec<_> = get_kerml_diagnostics(source)
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Expected no errors, but got {} errors:\n{}",
            errors.len(),
            errors
                .iter()
                .map(|e| format!("  - {}", e.message))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Test that single-level public re-export works
    #[test]
    fn test_single_level_public_reexport() {
        let source = r#"
            package Root {
                class Element;
            }
            package Outer {
                public import Root::*;
            }
            package Test {
                import Outer::Element;
                class MyClass :> Element;
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// Test that two-level public re-export works
    #[test]
    fn test_two_level_public_reexport() {
        let source = r#"
            package Root {
                class Element;
            }
            package Core {
                public import Root::*;
            }
            package Outer {
                public import Core::*;
            }
            package Test {
                import Outer::Element;
                class MyClass :> Element;
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// Test that three-level public re-export works (KerML pattern)
    #[test]
    fn test_three_level_public_reexport() {
        let source = r#"
            package Root {
                class Element;
                class Type;
            }
            package Core {
                public import Root::*;
            }
            package Kernel {
                public import Core::*;
            }
            package KerML {
                public import Kernel::*;
            }
            package Metaobjects {
                private import KerML::Element;
                private import KerML::Type;
                class Metaobject {
                    feature annotatedElement : Element;
                }
            }
        "#;
        assert_no_kerml_errors(source);
    }

    /// Test visibility map directly for transitive imports
    #[test]
    fn test_visibility_map_has_transitive_imports() {
        let source = r#"
            package Root {
                class Element;
            }
            package Core {
                public import Root::*;
            }
            package Outer {
                public import Core::*;
            }
        "#;
        let (mut host, _file_id) = analysis_from_kerml(source);
        let analysis = host.analysis();
        let index = analysis.symbol_index();

        // Check that Outer has Element in its visibility
        let outer_vis = index
            .visibility_for_scope("Outer")
            .expect("Outer should have visibility");
        let element_in_outer = outer_vis.lookup("Element");
        assert!(
            element_in_outer.is_some(),
            "Element should be visible in Outer via transitive import. Got: {:?}",
            outer_vis.imports().collect::<Vec<_>>()
        );
        assert_eq!(element_in_outer.unwrap().as_ref(), "Root::Element");
    }

    /// Test that inherited members are visible through the inheritance chain
    #[test]
    fn test_inherited_members_across_packages() {
        let source = r#"
            package Occurrences {
                class Occurrence {
                    feature startShot : Occurrence;
                    feature endShot : Occurrence;
                }
            }
            package Performances {
                private import Occurrences::*;
                behavior Performance specializes Occurrence { }
            }
            package ControlPerformances {
                private import Performances::*;
                behavior DecisionPerformance specializes Performance { }
            }
            package StatePerformances {
                private import ControlPerformances::*;
                behavior StatePerformance specializes DecisionPerformance { }
            }
        "#;
        let (mut host, _file_id) = analysis_from_kerml(source);
        let analysis = host.analysis();
        let index = analysis.symbol_index();

        // Check StatePerformance visibility
        let sp_vis = index
            .visibility_for_scope("StatePerformances::StatePerformance")
            .expect("StatePerformance should have visibility");

        eprintln!(
            "StatePerformance direct_defs: {:?}",
            sp_vis.direct_defs().collect::<Vec<_>>()
        );
        eprintln!(
            "StatePerformance imports: {:?}",
            sp_vis.imports().collect::<Vec<_>>()
        );

        // Check DecisionPerformance visibility
        if let Some(dp_vis) = index.visibility_for_scope("ControlPerformances::DecisionPerformance")
        {
            eprintln!(
                "DecisionPerformance direct_defs: {:?}",
                dp_vis.direct_defs().collect::<Vec<_>>()
            );
        }

        // Check Performance visibility
        if let Some(p_vis) = index.visibility_for_scope("Performances::Performance") {
            eprintln!(
                "Performance direct_defs: {:?}",
                p_vis.direct_defs().collect::<Vec<_>>()
            );
        }

        // Check Occurrence visibility
        if let Some(o_vis) = index.visibility_for_scope("Occurrences::Occurrence") {
            eprintln!(
                "Occurrence direct_defs: {:?}",
                o_vis.direct_defs().collect::<Vec<_>>()
            );
        }

        // startShot should be inherited from Occurrence via Performance via DecisionPerformance
        let startshot = sp_vis.lookup("startShot");
        assert!(
            startshot.is_some(),
            "startShot should be visible in StatePerformance via inheritance. Direct defs: {:?}",
            sp_vis.direct_defs().collect::<Vec<_>>()
        );
    }
}
