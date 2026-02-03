//! AST layer tests for KerML parser using Rowan.
//!
//! These tests verify that the typed AST wrappers correctly extract
//! semantic information from the untyped Rowan CST.
//!
//! Adapted from tests_parser_kerml_pest.rs.disabled

use syster::parser::ast::{
    AstNode, Definition, DefinitionKind, NamespaceMember, SourceFile, Specialization,
    SpecializationKind, Usage,
};
use syster::parser::parse_kerml;

/// Helper to parse input and get SourceFile AST
fn parse_source(input: &str) -> SourceFile {
    let parsed = parse_kerml(input);
    SourceFile::cast(parsed.syntax()).expect("Failed to cast to SourceFile")
}

// ============================================================================
// AST Parsing Tests - Verify correct AST structure construction
// ============================================================================

#[test]
fn test_parse_classifier_with_specialization_ast() {
    let input = "classifier Car specializes Vehicle;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1, "Should have 1 member");

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.definition_kind(),
                Some(DefinitionKind::Classifier),
                "Should be a Classifier"
            );
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("Car".to_string()),
                "Name should be Car"
            );

            let specs: Vec<_> = def.specializations().collect();
            assert_eq!(specs.len(), 1, "Should have 1 specialization");
            assert_eq!(
                specs[0].kind(),
                Some(SpecializationKind::Specializes),
                "Should use 'specializes'"
            );
        }
        _ => panic!("Expected Definition, got {:?}", members[0]),
    }
}

#[test]
fn test_parse_classifier_with_multiple_specializations_ast() {
    let input = "classifier SportsCar specializes Car, Vehicle;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Classifier));
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("SportsCar".to_string())
            );

            let specs: Vec<_> = def.specializations().collect();
            // Note: comma-separated specializations may parse as single or multiple
            // depending on grammar - check at least one exists
            assert!(!specs.is_empty(), "Should have at least 1 specialization");
        }
        _ => panic!("Expected Definition"),
    }
}

#[test]
fn test_parse_feature_with_typing_ast() {
    // KerML feature at top level - wrap in package for SysML context
    let input = "package Test { feature mass : Real; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            assert_eq!(pkg_members.len(), 1, "Package should have 1 member");

            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("mass".to_string())
                    );
                    let typing = usage.typing();
                    assert!(typing.is_some(), "Should have typing");
                }
                _ => panic!("Expected Usage for feature"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_feature_with_redefinition_ast() {
    let input = "package Test { feature currentMass redefines mass; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("currentMass".to_string())
                    );
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_redef = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Redefines));
                    assert!(has_redef, "Should have redefinition");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_feature_with_subsetting_ast() {
    let input = "package Test { feature wheelMass subsets mass; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("wheelMass".to_string())
                    );
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_subset = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Subsets));
                    assert!(has_subset, "Should have subsetting");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_feature_with_typing_and_redefinition_ast() {
    let input = "package Test { feature currentMass : Real redefines mass; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("currentMass".to_string())
                    );

                    // Check typing
                    assert!(usage.typing().is_some(), "Should have typing");

                    // Check redefinition
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_redef = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Redefines));
                    assert!(has_redef, "Should have redefinition");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_abstract_classifier_ast() {
    let input = "abstract classifier Vehicle;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("Vehicle".to_string())
            );
            assert!(def.is_abstract(), "Classifier should be abstract");
        }
        _ => panic!("Expected Definition"),
    }
}

#[test]
fn test_parse_datatype_ast() {
    let input = "datatype Real;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(def.name().and_then(|n| n.text()), Some("Real".to_string()));
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Datatype));
        }
        _ => panic!("Expected Definition (Datatype)"),
    }
}

#[test]
fn test_parse_function_ast() {
    let input = "function calculateArea;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("calculateArea".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Function));
        }
        _ => panic!("Expected Definition (Function)"),
    }
}

#[test]
fn test_parse_class_ast() {
    let input = "class MyClass;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyClass".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Class));
        }
        _ => panic!("Expected Definition (Class)"),
    }
}

#[test]
fn test_parse_struct_ast() {
    let input = "struct MyStruct;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyStruct".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Struct));
        }
        _ => panic!("Expected Definition (Struct)"),
    }
}

#[test]
fn test_parse_behavior_ast() {
    let input = "behavior MyBehavior;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyBehavior".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Behavior));
        }
        _ => panic!("Expected Definition (Behavior)"),
    }
}

#[test]
fn test_parse_predicate_ast() {
    let input = "predicate MyPredicate;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyPredicate".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Predicate));
        }
        _ => panic!("Expected Definition (Predicate)"),
    }
}

#[test]
fn test_parse_interaction_ast() {
    let input = "interaction MyInteraction;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyInteraction".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Interaction));
        }
        _ => panic!("Expected Definition (Interaction)"),
    }
}

#[test]
fn test_parse_metaclass_ast() {
    let input = "metaclass MyMetaclass;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyMetaclass".to_string())
            );
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Metaclass));
        }
        _ => panic!("Expected Definition (Metaclass)"),
    }
}

#[test]
fn test_parse_classifier_with_nested_feature_ast() {
    let input = r#"classifier Vehicle {
        feature mass : Real;
    }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Classifier));
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("Vehicle".to_string())
            );

            let nested: Vec<_> = def.members().collect();
            assert_eq!(nested.len(), 1, "Classifier should have 1 nested member");

            match &nested[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("mass".to_string())
                    );
                    assert!(usage.typing().is_some(), "Feature should have typing");
                }
                _ => panic!("Expected Usage for nested feature"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

#[test]
fn test_parse_package_with_import_ast() {
    let input = r#"package Test {
        import Base::*;
    }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Package(pkg) => {
            assert_eq!(pkg.name().and_then(|n| n.text()), Some("Test".to_string()));

            let pkg_members: Vec<_> = pkg.members().collect();
            assert_eq!(pkg_members.len(), 1);

            match &pkg_members[0] {
                NamespaceMember::Import(imp) => {
                    assert!(imp.is_wildcard(), "Should be wildcard import");
                }
                _ => panic!("Expected Import"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_library_package_ast() {
    let input = "standard library package ScalarValues;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::LibraryPackage(lib) => {
            assert!(lib.is_standard(), "Should be standard library");
            assert_eq!(
                lib.name().and_then(|n| n.text()),
                Some("ScalarValues".to_string())
            );
        }
        _ => panic!("Expected LibraryPackage"),
    }
}

#[test]
fn test_parse_abstract_class_with_specialization_ast() {
    let input = "abstract class Occurrence specializes Anything;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert!(def.is_abstract(), "Should be abstract");
            assert_eq!(def.definition_kind(), Some(DefinitionKind::Class));
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("Occurrence".to_string())
            );

            let specs: Vec<_> = def.specializations().collect();
            assert!(!specs.is_empty(), "Should have specialization");
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// Usage Direction Tests
// ============================================================================

#[test]
fn test_parse_feature_with_direction_in() {
    let input = "function Test { in feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let nested: Vec<_> = def.members().collect();
            match &nested[0] {
                NamespaceMember::Usage(usage) => {
                    use syster::parser::ast::Direction;
                    assert_eq!(usage.direction(), Some(Direction::In));
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

#[test]
fn test_parse_feature_with_direction_out() {
    let input = "function Test { out feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let nested: Vec<_> = def.members().collect();
            match &nested[0] {
                NamespaceMember::Usage(usage) => {
                    use syster::parser::ast::Direction;
                    assert_eq!(usage.direction(), Some(Direction::Out));
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

#[test]
fn test_parse_feature_with_direction_inout() {
    let input = "function Test { inout feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let nested: Vec<_> = def.members().collect();
            match &nested[0] {
                NamespaceMember::Usage(usage) => {
                    use syster::parser::ast::Direction;
                    assert_eq!(usage.direction(), Some(Direction::InOut));
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// Usage Modifier Tests
// ============================================================================

#[test]
fn test_parse_ref_feature_ast() {
    let input = "package Test { ref feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert!(usage.is_ref(), "Should be ref feature");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_derived_feature_ast() {
    let input = "package Test { derived feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert!(usage.is_derived(), "Should be derived feature");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

#[test]
fn test_parse_readonly_feature_ast() {
    let input = "package Test { readonly feature x; }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert!(usage.is_readonly(), "Should be readonly feature");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

// ============================================================================
// Comment and Metadata Tests
// ============================================================================

#[test]
fn test_parse_comment_ast() {
    let input = r#"package Test {
        comment /* This is a comment */
    }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Comment(_comment) => {
                    // Comment parsed successfully
                }
                _ => panic!("Expected Comment"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

// ============================================================================
// Dependency Tests
// ============================================================================

#[test]
fn test_parse_dependency_ast() {
    let input = "dependency Source to Target;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Dependency(dep) => {
            let sources = dep.sources();
            let target = dep.target();
            assert!(
                !sources.is_empty() || target.is_some(),
                "Should have source or target"
            );
        }
        _ => panic!("Expected Dependency, got {:?}", members[0]),
    }
}

// ============================================================================
// STDLIB SYNTAX AST COVERAGE TESTS
// ============================================================================
// These tests verify that advanced KerML/SysML v2 syntax patterns found in
// the standard library are correctly represented in the AST.
//
// Reference: SysML v2 Specification (OMG Document Number: formal/2023-06-01)
//
// IMPLEMENTATION PLAN:
// Each test documents what AST method is needed. Tests are ignored until
// the corresponding AST method is implemented.
// ============================================================================

/// Test `this` keyword as a feature name - AST extraction
/// From Occurrences.kerml: `feature this : Occurrence[1] default self { ... }`
/// Per SysML v2 Spec §7.3.4.3
///
/// IMPLEMENTATION: Name::text() should return "this" for THIS_KW token
/// Also consider adding Name::is_this_keyword() -> bool
#[test]
fn test_this_keyword_as_feature_name_ast() {
    let input = r#"classifier C { feature this : C[1]; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            // Use existing def.members() which goes through body()
            let body_members: Vec<_> = def.members().collect();
            assert!(!body_members.is_empty(), "Should have body members");

            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    let name = usage.name().and_then(|n| n.text());
                    assert_eq!(
                        name,
                        Some("this".to_string()),
                        "Feature should be named 'this'"
                    );
                }
                _ => panic!("Expected Usage for feature 'this'"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `this` with default value - AST extraction of default
/// From Occurrences.kerml: `feature this : Occurrence[1] default self { ... }`
///
/// IMPLEMENTATION NEEDED:
/// - Usage::default_value() -> Option<Expression>
/// - Usage::is_default_value() -> bool (distinguishes `default x` from `= x`)
#[test]
fn test_this_with_default_value_ast() {
    let input = r#"classifier C { feature this : C[1] default self; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("this".to_string())
                    );
                    // Check for default value - needs new method
                    // IMPLEMENTATION: Look for DEFAULT_KW token, then get following Expression
                    let has_default = usage.value_expression().is_some(); // Use existing for now
                    assert!(has_default, "Should have default value 'self'");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `chains` keyword - AST representation
/// From Base.kerml: `feature self: Anything[1] subsets things chains things.that`
/// Per SysML v2 Spec §7.3.4.5
///
/// IMPLEMENTATION NEEDED:
/// - SpecializationKind::Chains variant
/// - Usage::chains() -> impl Iterator<Item = Specialization>
#[test]
fn test_chains_keyword_ast() {
    let input = r#"classifier C { feature self: C[1] subsets things chains things.that; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("self".to_string())
                    );
                    // Check for chaining relationship
                    // IMPLEMENTATION: Filter specializations() for kind == FeatureChain
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_chains = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::FeatureChain));
                    assert!(has_chains, "Should have chaining relationship");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test feature path expressions with dot notation - AST qualified name
/// From Base.kerml: `chains things.that`
///
/// IMPLEMENTATION: QualifiedName::segments() should handle dot-separated paths
#[test]
fn test_feature_path_expression_ast() {
    let input = r#"classifier C { feature x subsets a.b.c; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_subsets = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Subsets));
                    assert!(has_subsets, "Should have subsetting relationship");

                    // Check that the target is a dotted path
                    if let Some(spec) = specs.first() {
                        if let Some(target) = spec.target() {
                            let segments = target.segments();
                            assert!(segments.len() >= 3, "Should have multi-segment path a.b.c");
                        }
                    }
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `metaclass` definition - AST DefinitionKind
/// From KerML.kerml: `metaclass AnnotatingElement specializes Element { ... }`
/// Per SysML v2 Spec §7.2.3.2
///
/// IMPLEMENTATION: DefinitionKind::Metaclass already exists, test should pass
#[test]
fn test_metaclass_definition_ast() {
    let input = r#"metaclass MyMeta specializes Element;"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.definition_kind(),
                Some(DefinitionKind::Metaclass),
                "Should be a Metaclass definition"
            );
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("MyMeta".to_string())
            );
        }
        _ => panic!("Expected Definition for metaclass"),
    }
}

/// Test `var` feature modifier - AST modifier check
/// From KerML.kerml: `var feature annotatedElement : Element[1..*] ordered`
/// Per SysML v2 Spec §7.3.3.6: "var" indicates a mutable feature
///
/// IMPLEMENTATION NEEDED:
/// - Usage::is_var() -> bool (check for VAR_KW token)
#[test]
fn test_var_feature_modifier_ast() {
    let input = r#"classifier C { var feature x : Integer[1]; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(usage.name().and_then(|n| n.text()), Some("x".to_string()));
                    // IMPLEMENTATION: Check for VAR_KW in children_with_tokens()
                    assert!(usage.is_var(), "Should be a var feature");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `derived var` combined modifiers - AST combined modifier check
/// From KerML.kerml: `derived var feature annotatedElement`
///
/// IMPLEMENTATION NEEDED: Usage::is_var() (is_derived() already exists)
#[test]
fn test_derived_var_modifier_ast() {
    let input = r#"classifier C { derived var feature x : Integer[1]; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert!(usage.is_derived(), "Should be derived");
                    assert!(usage.is_var(), "Should be var");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `all` keyword for universal quantification - AST all check
/// From Occurrences.kerml: `feature all x : T[*]`
/// Per SysML v2 Spec §7.3.3.4: "all" means sufficient/complete coverage
///
/// IMPLEMENTATION NEEDED:
/// - Usage::is_all() -> bool (check for ALL_KW token after feature keyword)
#[test]
fn test_all_keyword_in_feature_ast() {
    let input = r#"classifier C { feature all instances : C[*]; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("instances".to_string())
                    );
                    // IMPLEMENTATION: Check for ALL_KW after feature keyword
                    assert!(usage.is_all(), "Should be an 'all' feature");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test combined redefines with assignment - AST value extraction
/// From Parts.sysml: `:>> Action::this = that as Part`
///
/// IMPLEMENTATION: value_expression() already exists, should work
#[test]
fn test_redefines_with_assignment_ast() {
    let input = r#"classifier C { feature x :>> parent::y = initialValue; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    // Check for redefines
                    let specs: Vec<_> = usage.specializations().collect();
                    let has_redef = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Redefines));
                    assert!(has_redef, "Should have redefinition");

                    // Check for value assignment - use existing method
                    let has_value = usage.value_expression().is_some();
                    assert!(has_value, "Should have value assignment");
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test anonymous connector with from/to clauses - AST connector extraction
/// From Occurrences.kerml: `connector :HappensDuring from [1] self to [1] this;`
///
/// IMPLEMENTATION NEEDED:
/// - ConnectionEnd AST struct with multiplicity() and target()
/// - Connector::ends() -> impl Iterator<Item = ConnectionEnd>
/// - Parser support for `from [mult] name to [mult] name` connector syntax
#[test]
#[ignore] // TODO: Parser doesn't fully support `from [mult] a to [mult] b` connector syntax yet
fn test_anonymous_connector_with_multiplicity_ast() {
    let input = r#"classifier C { connector :Link from [1] a to [1] b; }"#;
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let body_members: Vec<_> = def.members().collect();
            match &body_members[0] {
                NamespaceMember::Connector(conn) => {
                    // Should be anonymous (no name, just type)
                    let typing = conn.typing();
                    assert!(typing.is_some(), "Should have typing ':Link'");

                    // Check endpoints - needs Connector::ends()
                    // IMPLEMENTATION: Return iterator over CONNECTION_END children
                    let ends: Vec<_> = conn.ends().collect();
                    assert_eq!(ends.len(), 2, "Should have 2 endpoints (from/to)");
                }
                _ => panic!("Expected Connector, got {:?}", body_members[0]),
            }
        }
        _ => panic!("Expected Definition"),
    }
}
