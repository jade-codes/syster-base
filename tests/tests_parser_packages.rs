//! Parser Tests - Packages and Imports
//!
//! Phase 1: Parser/AST Layer
//! Tests for package and import parsing.
//!
//! Test data from tests_kerml_import_detection.rs.archived and
//! tests_multiple_packages.rs.archived.

use rstest::rstest;
use syster::parser::{AstNode, Import, NamespaceMember, SourceFile, parse_kerml, parse_sysml};

/// Helper to count packages in parsed content
fn count_packages(input: &str) -> usize {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax()).expect("Should cast");
    file.members()
        .filter(|m| {
            matches!(
                m,
                NamespaceMember::Package(_) | NamespaceMember::LibraryPackage(_)
            )
        })
        .count()
}

/// Helper to get the first import from a classifier
fn get_first_import(input: &str) -> Option<Import> {
    let parsed = parse_kerml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    for member in file.members() {
        if let NamespaceMember::Definition(def) = member {
            if let Some(body) = def.body() {
                for m in body.members() {
                    if let NamespaceMember::Import(imp) = m {
                        return Some(imp);
                    }
                }
            }
        }
    }
    None
}

/// Helper to get the first package's name
fn get_package_name(input: &str) -> Option<String> {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax())?;
    for member in file.members() {
        match member {
            NamespaceMember::Package(p) => return p.name().and_then(|n| n.text()),
            NamespaceMember::LibraryPackage(p) => return p.name().and_then(|n| n.text()),
            _ => continue,
        }
    }
    None
}

/// Helper to check if first package is a library package
fn is_library_package(input: &str) -> bool {
    let parsed = parse_sysml(input);
    let file = SourceFile::cast(parsed.syntax()).expect("Should cast");
    file.members()
        .any(|m| matches!(m, NamespaceMember::LibraryPackage(_)))
}

// ============================================================================
// Multiple Packages
// ============================================================================

#[rstest]
#[case("package Vehicle; package Engine; package Transmission;", 3)]
#[case("package SinglePackage;", 1)]
#[case("part def MyPart;", 0)]
fn test_package_count(#[case] input: &str, #[case] expected_count: usize) {
    assert_eq!(count_packages(input), expected_count);
}

// ============================================================================
// Package Properties
// ============================================================================

#[rstest]
#[case("package MyPackage;", "MyPackage", false)]
#[case("library package MyLib;", "MyLib", true)]
fn test_package_properties(
    #[case] input: &str,
    #[case] expected_name: &str,
    #[case] expected_library: bool,
) {
    assert_eq!(get_package_name(input), Some(expected_name.to_string()));
    assert_eq!(is_library_package(input), expected_library);
}

// ============================================================================
// Import Detection
// ============================================================================

#[rstest]
#[case("classifier Vehicle { import Base::DataValue; }", false, false, false)]
#[case(
    "classifier Vehicle { import all Base::DataValue; }",
    true,
    false,
    false
)]
#[case(
    "classifier Vehicle { import Base::DataValue::*; }",
    false,
    true,
    false
)]
#[case(
    "classifier Vehicle { import Base::DataValue::**; }",
    false,
    false,
    true
)]
fn test_import_properties(
    #[case] input: &str,
    #[case] expected_all: bool,
    #[case] expected_wildcard: bool,
    #[case] expected_recursive: bool,
) {
    let imp = get_first_import(input).expect("Should parse");
    assert_eq!(imp.is_all(), expected_all, "is_all mismatch for: {}", input);
    assert_eq!(
        imp.is_wildcard(),
        expected_wildcard,
        "is_wildcard mismatch for: {}",
        input
    );
    assert_eq!(
        imp.is_recursive(),
        expected_recursive,
        "is_recursive mismatch for: {}",
        input
    );
}
