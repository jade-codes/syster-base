//! KerML symbol extraction tests for the HIR layer.
//!
//! These tests verify that KerML constructs are correctly extracted as symbols.
//! KerML types are mapped to their closest SysML equivalents where applicable.

use crate::helpers::hir_helpers::*;
use crate::helpers::symbol_assertions::*;
use syster::hir::SymbolKind;

// =============================================================================
// KERML PACKAGES
// =============================================================================

#[test]
fn test_kerml_package_extraction() {
    // Note: Empty packages with braces work; semicolon-terminated packages may have different behavior
    let (mut host, _) = analysis_from_kerml("package KerMLPkg {}");
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "KerMLPkg");
    let sym = get_symbol(analysis.symbol_index(), "KerMLPkg");
    assert_symbol_kind(sym, SymbolKind::Package);
}

#[test]
fn test_kerml_nested_packages() {
    let source = r#"
        package Outer {
            package Inner {
                class MyClass;
            }
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Outer");
    assert_symbol_exists(analysis.symbol_index(), "Outer::Inner");
    assert_symbol_exists(analysis.symbol_index(), "Outer::Inner::MyClass");
}

// =============================================================================
// KERML CLASSIFIERS
// =============================================================================

#[test]
fn test_kerml_class_extraction() {
    let source = r#"
        package Pkg {
            class MyClass;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::MyClass");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::MyClass");
    // KerML class maps to PartDef
    assert_symbol_kind(sym, SymbolKind::PartDefinition);
}

#[test]
fn test_kerml_datatype_extraction() {
    let source = r#"
        package Types {
            datatype ScalarValue;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Types::ScalarValue");
    let sym = get_symbol(analysis.symbol_index(), "Types::ScalarValue");
    // KerML datatype maps to AttributeDef
    assert_symbol_kind(sym, SymbolKind::AttributeDefinition);
}

#[test]
fn test_kerml_struct_extraction() {
    let source = r#"
        package Pkg {
            struct MyStruct;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::MyStruct");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::MyStruct");
    // KerML struct maps to PartDef
    assert_symbol_kind(sym, SymbolKind::PartDefinition);
}

#[test]
fn test_kerml_behavior_extraction() {
    let source = r#"
        package Behaviors {
            behavior Process;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Behaviors::Process");
    let sym = get_symbol(analysis.symbol_index(), "Behaviors::Process");
    // KerML behavior maps to ActionDef
    assert_symbol_kind(sym, SymbolKind::ActionDefinition);
}

#[test]
fn test_kerml_function_extraction() {
    let source = r#"
        package Funcs {
            function Calculate;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Funcs::Calculate");
    let sym = get_symbol(analysis.symbol_index(), "Funcs::Calculate");
    // KerML function maps to CalculationDef
    assert_symbol_kind(sym, SymbolKind::CalculationDefinition);
}

#[test]
fn test_kerml_association_extraction() {
    let source = r#"
        package Pkg {
            assoc Link;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::Link");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::Link");
    // KerML association maps to ConnectionDef
    assert_symbol_kind(sym, SymbolKind::ConnectionDefinition);
}

// =============================================================================
// KERML FEATURES
// =============================================================================

#[test]
fn test_kerml_feature_extraction() {
    let source = r#"
        package Pkg {
            class Container {
                feature data;
            }
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::Container::data");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::Container::data");
    // KerML feature maps to AttributeUsage
    assert_symbol_kind(sym, SymbolKind::AttributeUsage);
}

#[test]
fn test_kerml_step_extraction() {
    let source = r#"
        package Pkg {
            behavior Process {
                step execute;
            }
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::Process::execute");
}

#[test]
fn test_kerml_interaction_extraction() {
    let source = r#"
        package Pkg {
            interaction Communicate;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::Communicate");
    // Interactions map to a definition kind
    let sym = get_symbol(analysis.symbol_index(), "Pkg::Communicate");
    // Interactions are like behaviors (message sequences)
    assert!(
        sym.kind.is_definition(),
        "Interaction should be a definition"
    );
}

#[test]
fn test_kerml_metaclass_extraction() {
    let source = r#"
        package Pkg {
            metaclass SpecialClass;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::SpecialClass");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::SpecialClass");
    // Metaclasses are classifiers
    assert!(sym.kind.is_definition(), "Metaclass should be a definition");
}

#[test]
fn test_kerml_connector_extraction() {
    let source = r#"
        package Pkg {
            class System {
                connector link;
            }
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::System::link");
    let sym = get_symbol(analysis.symbol_index(), "Pkg::System::link");
    // Connectors are usages
    assert!(sym.kind.is_usage(), "Connector should be a usage");
}

#[test]
fn test_kerml_succession_extraction() {
    // The succession syntax requires 'first' keyword before the first endpoint
    // and 'then' keyword before the second endpoint
    let source = r#"
        package Pkg {
            behavior Process {
                step a;
                step b;
                succession first a then b;
            }
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    // At minimum, the steps should exist (succession may or may not create a symbol)
    assert_symbol_exists(analysis.symbol_index(), "Pkg::Process::a");
    assert_symbol_exists(analysis.symbol_index(), "Pkg::Process::b");
}

// =============================================================================
// KERML SPECIALIZATION
// =============================================================================

#[test]
fn test_kerml_class_specialization() {
    let source = r#"
        package Pkg {
            class Base;
            class Derived :> Base;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    assert_symbol_exists(analysis.symbol_index(), "Pkg::Base");
    assert_symbol_exists(analysis.symbol_index(), "Pkg::Derived");

    let derived = get_symbol(analysis.symbol_index(), "Pkg::Derived");
    assert_specializes(derived, "Base");
}

// =============================================================================
// KERML IMPORTS
// =============================================================================

#[test]
fn test_kerml_import_resolution() {
    // Match the SysML pattern - use braces and include something that uses the import
    let source = r#"
        package Source {
            class MyClass {}
        }
        package Target {
            import Source::*;
            feature myFeature : MyClass;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    // MyClass should be resolvable from Target
    let sym = assert_resolves(analysis.symbol_index(), "Target", "MyClass");
    assert_eq!(sym.qualified_name.as_ref(), "Source::MyClass");
}

#[test]
fn test_kerml_public_import_reexport() {
    let source = r#"
        package A {
            class ClassA {}
        }
        package B {
            public import A::*;
        }
        package C {
            import B::*;
        }
    "#;
    let (mut host, _) = analysis_from_kerml(source);
    let analysis = host.analysis();

    // ClassA should be visible from C via B's public re-export
    let sym = assert_resolves(analysis.symbol_index(), "C", "ClassA");
    assert_eq!(sym.qualified_name.as_ref(), "A::ClassA");
}
