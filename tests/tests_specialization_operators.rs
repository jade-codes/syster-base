//! Tests for specialization operator parsing and semantics.
//!
//! These tests verify correct behavior for:
//! - `:>` (Specializes/Subsets)
//! - `:>>` (Redefines shorthand) - critically different from `~` (Conjugates)
//! - `redefines` keyword vs `:>>` shorthand distinction
//! - Quoted/unrestricted name handling
//!
//! ## SysML v2 Specification Reference
//!
//! From the SysML v2 specification:
//! - `:>` denotes specialization (inheritance) or subsetting for features
//! - `:>>` denotes redefinition - the feature redefines one from a supertype
//! - `~` denotes conjugation (port conjugation, etc.)
//!
//! The `:>>` vs `~` distinction was a critical bug fix - `:>>` was incorrectly
//! mapped to Conjugates instead of Redefines.

use syster::parser::{
    AstNode, Definition, NamespaceMember, SourceFile, SpecializationKind, parse_sysml,
};

/// Helper to parse input and get SourceFile AST
fn parse_source(input: &str) -> SourceFile {
    let parsed = parse_sysml(input);
    SourceFile::cast(parsed.syntax()).expect("Failed to cast to SourceFile")
}

/// Helper to get members from a definition's body
fn def_members(def: &Definition) -> Vec<NamespaceMember> {
    def.body()
        .map(|b| b.members().collect())
        .unwrap_or_default()
}

// ============================================================================
// `:>>` (Redefines shorthand) vs `~` (Conjugates) - Critical distinction
// ============================================================================

/// Test that `:>>` parses as Redefines, NOT Conjugates
/// This was the root cause of many hover failures.
#[test]
fn test_colon_gt_gt_is_redefines_not_conjugates() {
    let input = "part def Vehicle {
        attribute mass : Real;
    }
    part def Car :> Vehicle {
        attribute :>> mass = 1500;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 2, "Should have Vehicle and Car");

    // Get Car definition
    match &members[1] {
        NamespaceMember::Definition(car_def) => {
            assert_eq!(
                car_def.name().and_then(|n| n.text()),
                Some("Car".to_string())
            );

            // Find the attribute usage (mass redefinition)
            let car_members = def_members(car_def);
            assert_eq!(car_members.len(), 1, "Car should have 1 member");

            match &car_members[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    assert!(!specs.is_empty(), "Should have specialization");

                    // THIS IS THE CRITICAL TEST: `:>>` must be Redefines
                    let has_redefines = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Redefines));
                    assert!(
                        has_redefines,
                        "`:>>` should parse as Redefines, not Conjugates"
                    );

                    // Verify it's NOT Conjugates
                    let has_conjugates = specs
                        .iter()
                        .any(|s| s.kind() == Some(SpecializationKind::Conjugates));
                    assert!(
                        !has_conjugates,
                        "`:>>` should NOT be Conjugates - that's for `~`"
                    );
                }
                _ => panic!("Expected Usage for attribute"),
            }
        }
        _ => panic!("Expected Definition for Car"),
    }
}

/// Test that `~` parses as Conjugates (port conjugation)
#[test]
fn test_tilde_is_conjugates() {
    let input = "port def FuelPort;
    port def ConjugateFuelPort ~ FuelPort;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 2);

    match &members[1] {
        NamespaceMember::Definition(def) => {
            assert_eq!(
                def.name().and_then(|n| n.text()),
                Some("ConjugateFuelPort".to_string())
            );

            let specs: Vec<_> = def.specializations().collect();
            assert!(!specs.is_empty(), "Should have specialization");

            // `~` should be Conjugates
            let has_conjugates = specs
                .iter()
                .any(|s| s.kind() == Some(SpecializationKind::Conjugates));
            assert!(has_conjugates, "`~` should parse as Conjugates");

            // Should NOT be Redefines
            let has_redefines = specs
                .iter()
                .any(|s| s.kind() == Some(SpecializationKind::Redefines));
            assert!(!has_redefines, "`~` should NOT be Redefines");
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// `:>` (Specializes/Subsets) operator
// ============================================================================

/// Test that `:>` on definitions parses as Specializes
#[test]
fn test_colon_gt_on_definition_is_specializes() {
    let input = "part def Car :> Vehicle;";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 1);

    match &members[0] {
        NamespaceMember::Definition(def) => {
            let specs: Vec<_> = def.specializations().collect();
            assert_eq!(specs.len(), 1, "Should have 1 specialization");
            assert_eq!(
                specs[0].kind(),
                Some(SpecializationKind::Specializes),
                "`:>` on definition should be Specializes"
            );
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test that `:>` on usages parses correctly
#[test]
fn test_colon_gt_on_usage() {
    let input = "part def Vehicle {
        part wheels : Wheel[4];
    }
    part def Car :> Vehicle {
        part frontWheels :> wheels;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[1] {
        NamespaceMember::Definition(car_def) => {
            let car_members = def_members(car_def);
            match &car_members[0] {
                NamespaceMember::Usage(usage) => {
                    assert_eq!(
                        usage.name().and_then(|n| n.text()),
                        Some("frontWheels".to_string())
                    );

                    let specs: Vec<_> = usage.specializations().collect();
                    // `:>` on usage is typically Subsets in SysML
                    // The grammar may interpret this as Specializes at AST level
                    // Either is acceptable - the key distinction is with `:>>`
                    let has_specializes_or_subsets = specs.iter().any(|s| {
                        matches!(
                            s.kind(),
                            Some(SpecializationKind::Specializes)
                                | Some(SpecializationKind::Subsets)
                        )
                    });
                    assert!(
                        has_specializes_or_subsets,
                        "`:>` should parse as Specializes or Subsets"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// `is_shorthand_redefines()` - distinguishing `:>>` from `redefines` keyword
// ============================================================================

/// Test that `:>> name` is detected as shorthand redefines
#[test]
fn test_is_shorthand_redefines_colon_gt_gt() {
    let input = "part def Car {
        attribute :>> mass;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let members_list = def_members(def);
            match &members_list[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    assert!(!specs.is_empty());

                    // Should be detected as shorthand redefines
                    let spec = &specs[0];
                    assert!(
                        spec.is_shorthand_redefines(),
                        "`:>>` should be detected as shorthand redefines"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test that `redefines name` is NOT detected as shorthand redefines
#[test]
fn test_is_not_shorthand_redefines_keyword() {
    let input = "part def Car {
        attribute currentMass redefines mass;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let members_list = def_members(def);
            match &members_list[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    assert!(!specs.is_empty());

                    // Find the redefines spec
                    let redef_spec = specs
                        .iter()
                        .find(|s| s.kind() == Some(SpecializationKind::Redefines));
                    assert!(redef_spec.is_some(), "Should have redefines");

                    // `redefines` keyword is NOT shorthand
                    assert!(
                        !redef_spec.unwrap().is_shorthand_redefines(),
                        "`redefines` keyword should NOT be detected as shorthand"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test the practical difference: shorthand `:>> name` vs keyword `redefines name`
/// With shorthand, the redefines target becomes the usage name.
/// With keyword, the usage has its own explicit name.
#[test]
fn test_shorthand_vs_keyword_name_extraction() {
    // Shorthand: `:>> mass` - the usage is named after the redefined feature
    let shorthand = "part def A { attribute :>> mass; }";
    let keyword = "part def B { attribute myMass redefines mass; }";

    let file_a = parse_source(shorthand);
    let file_b = parse_source(keyword);

    // In shorthand case, the usage typically gets no explicit name
    // (the name comes from the redefines target during semantic analysis)
    let members_a: Vec<_> = file_a.members().collect();
    match &members_a[0] {
        NamespaceMember::Definition(def) => {
            let members_list = def_members(def);
            match &members_list[0] {
                NamespaceMember::Usage(usage) => {
                    // Shorthand typically has no explicit name
                    let name = usage.name().and_then(|n| n.text());
                    // Name may be None or may be extracted from target - either is valid
                    eprintln!("Shorthand name: {:?}", name);
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }

    // In keyword case, the usage has an explicit name
    let members_b: Vec<_> = file_b.members().collect();
    match &members_b[0] {
        NamespaceMember::Definition(def) => {
            let members_list = def_members(def);
            match &members_list[0] {
                NamespaceMember::Usage(usage) => {
                    let name = usage.name().and_then(|n| n.text());
                    assert_eq!(
                        name,
                        Some("myMass".to_string()),
                        "Keyword redefines should preserve explicit name"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// Quote stripping for unrestricted names
// ============================================================================

/// Test that `QualifiedName::segments()` strips quotes from unrestricted names
#[test]
fn test_qualified_name_strips_quotes() {
    // Reference to a quoted name in a typing context
    let input = "package Test {
        part myPart : 'Quoted Type Name';
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    let typing = usage.typing();
                    assert!(typing.is_some(), "Should have typing");

                    let qualified_name = typing.unwrap().target();
                    assert!(qualified_name.is_some(), "Should have target");

                    let segments = qualified_name.unwrap().segments();
                    assert_eq!(segments.len(), 1);
                    assert_eq!(
                        segments[0], "Quoted Type Name",
                        "Quotes should be stripped from unrestricted name"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

/// Test that `QualifiedName::segments()` handles regular names unchanged
#[test]
fn test_qualified_name_regular_name_unchanged() {
    let input = "package Test {
        part myPart : RegularType;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    let typing = usage.typing();
                    let qualified_name = typing.unwrap().target();
                    let segments = qualified_name.unwrap().segments();
                    assert_eq!(segments.len(), 1);
                    assert_eq!(
                        segments[0], "RegularType",
                        "Regular names should be unchanged"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

/// Test quote stripping with qualified paths
#[test]
fn test_qualified_name_strips_quotes_in_path() {
    let input = "package Test {
        part myPart : SomePackage::'Quoted Name';
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    let typing = usage.typing();
                    let qualified_name = typing.unwrap().target();
                    let segments = qualified_name.unwrap().segments();
                    assert_eq!(segments.len(), 2);
                    assert_eq!(segments[0], "SomePackage");
                    assert_eq!(
                        segments[1], "Quoted Name",
                        "Quotes should be stripped from path segment"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

/// Test `segments_with_ranges()` also strips quotes
#[test]
fn test_segments_with_ranges_strips_quotes() {
    let input = "package Test {
        part myPart : 'Quoted Type';
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            match &pkg_members[0] {
                NamespaceMember::Usage(usage) => {
                    let typing = usage.typing();
                    let qualified_name = typing.unwrap().target();
                    let segments_with_ranges = qualified_name.unwrap().segments_with_ranges();
                    assert_eq!(segments_with_ranges.len(), 1);
                    assert_eq!(
                        segments_with_ranges[0].0, "Quoted Type",
                        "Quotes should be stripped in segments_with_ranges too"
                    );
                    // Range should still cover the full quoted token
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

// ============================================================================
// Specialization kinds mapping completeness
// ============================================================================

/// Test `specializes` keyword
#[test]
fn test_specializes_keyword() {
    let input = "classifier A specializes B;";
    let file = parse_source(input);
    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Definition(def) => {
            let specs: Vec<_> = def.specializations().collect();
            assert!(
                specs
                    .iter()
                    .any(|s| s.kind() == Some(SpecializationKind::Specializes))
            );
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `subsets` keyword
#[test]
fn test_subsets_keyword() {
    // Using attribute instead of feature for SysML context
    let input = "part def Container {
        attribute a;
    }
    part def SubContainer :> Container {
        attribute a2 subsets a;
    }";
    let file = parse_source(input);
    let members: Vec<_> = file.members().collect();
    match &members[1] {
        NamespaceMember::Definition(def) => {
            let body_members = def_members(def);
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    assert!(
                        specs
                            .iter()
                            .any(|s| s.kind() == Some(SpecializationKind::Subsets))
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

/// Test `references` keyword
#[test]
fn test_references_keyword() {
    // Using ref/item for SysML references syntax
    let input = "part def Container {
        item a;
    }
    part def Referencer :> Container {
        ref item a2 references a;
    }";
    let file = parse_source(input);
    let members: Vec<_> = file.members().collect();
    match &members[1] {
        NamespaceMember::Definition(def) => {
            let body_members = def_members(def);
            match &body_members[0] {
                NamespaceMember::Usage(usage) => {
                    let specs: Vec<_> = usage.specializations().collect();
                    assert!(
                        specs
                            .iter()
                            .any(|s| s.kind() == Some(SpecializationKind::References))
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Definition"),
    }
}

// ============================================================================
// Real-world patterns from sysml.library
// ============================================================================

/// Test pattern from ScalarValues.sysml - quoted names like 'Ideal Gas Law'
#[test]
fn test_real_world_quoted_names() {
    let input = "package ScalarValues {
        part def 'Ideal Gas Law';
        part law : 'Ideal Gas Law';
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    match &members[0] {
        NamespaceMember::Package(pkg) => {
            let pkg_members: Vec<_> = pkg.members().collect();
            // Definition should have the quoted name
            match &pkg_members[0] {
                NamespaceMember::Definition(def) => {
                    // Name extraction may or may not strip quotes at definition level
                    let name = def.name();
                    eprintln!("Definition name node: {:?}", name);
                }
                _ => panic!("Expected Definition"),
            }
            // Usage's typing should resolve with stripped quotes
            match &pkg_members[1] {
                NamespaceMember::Usage(usage) => {
                    let typing = usage.typing();
                    let qualified = typing.unwrap().target().unwrap();
                    let segments = qualified.segments();
                    assert_eq!(
                        segments[0], "Ideal Gas Law",
                        "Type reference should strip quotes"
                    );
                }
                _ => panic!("Expected Usage"),
            }
        }
        _ => panic!("Expected Package"),
    }
}

/// Test pattern from KerML - shorthand redefines in context
#[test]
fn test_shorthand_redefines_in_context() {
    let input = "part def Vehicle {
        attribute mass : Real;
        attribute velocity : Real;
    }
    part def Car :> Vehicle {
        attribute :>> mass = 1500;
        attribute :>> velocity;
    }";
    let file = parse_source(input);

    let members: Vec<_> = file.members().collect();
    assert_eq!(members.len(), 2);

    match &members[1] {
        NamespaceMember::Definition(car_def) => {
            let car_members = def_members(car_def);
            assert_eq!(car_members.len(), 2);

            // Both should have shorthand redefines
            for (i, member) in car_members.iter().enumerate() {
                match member {
                    NamespaceMember::Usage(usage) => {
                        let specs: Vec<_> = usage.specializations().collect();
                        assert!(!specs.is_empty(), "Member {} should have specs", i);

                        // Should be Redefines
                        assert!(
                            specs
                                .iter()
                                .any(|s| s.kind() == Some(SpecializationKind::Redefines)),
                            "Member {} should have Redefines spec",
                            i
                        );

                        // Should be shorthand
                        assert!(
                            specs.iter().any(|s| s.is_shorthand_redefines()),
                            "Member {} should be shorthand redefines",
                            i
                        );
                    }
                    _ => panic!("Expected Usage at index {}", i),
                }
            }
        }
        _ => panic!("Expected Definition for Car"),
    }
}
