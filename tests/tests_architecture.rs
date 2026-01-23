//! Architecture Layer Dependency Tests
//!
//! These tests enforce the layered architecture dependency rules:
//!
//! ```
//! CLI/LSP (Delivery)
//!       ↓
//! Project/IDE
//!       ↓
//! HIR (Semantic)
//!       ↓
//! Syntax (AST)
//!       ↓
//! Parser
//!       ↓
//! Base (Foundation)
//! ```
//!
//! Dependency Rules:
//! - base → no imports (only std + external crates)
//! - parser → only base
//! - syntax → base, parser
//! - hir → base, parser, syntax
//! - ide → base, parser, syntax, hir
//! - project → base, parser, syntax, hir, ide
//! - CLI/LSP → everything
//! - No layer depends on CLI/LSP

mod tests_architecture_helpers;

use std::path::Path;
use tests_architecture_helpers::*;

#[test]
fn test_base_layer_has_no_dependencies() {
    let violations = collect_layer_violations(Path::new("src/base"), &[], "base");
    assert!(
        violations.is_empty(),
        "\n❌ Base layer should not depend on any other crate modules (only std).\nViolations:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn test_parser_layer_only_depends_on_base() {
    let violations = collect_layer_violations(Path::new("src/parser"), &["base"], "parser");
    assert!(
        violations.is_empty(),
        "\n❌ Parser layer should only depend on base.\nViolations:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn test_syntax_layer_has_minimal_dependencies() {
    let violations =
        collect_layer_violations(Path::new("src/syntax"), &["base", "parser"], "syntax");
    assert!(
        violations.is_empty(),
        "\n❌ Syntax layer should only depend on base and parser.\nViolations:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn test_project_layer_dependencies() {
    let violations = collect_layer_violations(
        Path::new("src/project"),
        &["base", "parser", "syntax", "hir", "ide"],
        "project",
    );
    assert!(
        violations.is_empty(),
        "\n❌ Project layer should only depend on base, parser, syntax, hir, and ide.\nViolations:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn test_no_layer_depends_on_lsp() {
    let violations = check_no_reverse_dependency(Path::new("src"), "syster_lsp", "LSP");
    assert!(
        violations.is_empty(),
        "\n❌ No layer in syster-base should depend on LSP.\nViolations:\n{}\n",
        violations.join("\n")
    );
}

#[test]
fn test_no_layer_depends_on_cli() {
    let violations = check_no_reverse_dependency(Path::new("src"), "syster_cli", "CLI");
    assert!(
        violations.is_empty(),
        "\n❌ No layer in syster-base should depend on CLI.\nViolations:\n{}\n",
        violations.join("\n")
    );
}

/// Helper test to show current architecture state
#[test]
fn test_show_architecture_violations_summary() {
    let layers = vec![
        ("base", vec![], "src/base"),
        ("parser", vec!["base"], "src/parser"),
        ("syntax", vec!["base", "parser"], "src/syntax"),
        (
            "project",
            vec!["base", "parser", "syntax", "hir", "ide"],
            "src/project",
        ),
    ];

    let mut total_violations = 0;

    for (layer_name, allowed, path) in layers {
        let violations = collect_layer_violations(Path::new(path), &allowed, layer_name);

        if violations.is_empty() {
        } else {
            total_violations += violations.len();
        }
    }

    assert_eq!(
        total_violations, 0,
        "Found {total_violations} architecture violations. Run with --nocapture to see details."
    );
}

// ============================================================================
// PHASE 6: Semantic Adapter Separation Tests
// ============================================================================

// NOTE: test_semantic_layer_only_adapters_import_syntax removed - semantic module was deleted

/// Verifies that all required constants are defined in base/constants.rs
#[test]
fn test_base_constants_defined() {
    let content = read_required_file(Path::new("src/base/constants.rs"));

    let required_constants = [
        "pub const REL_SATISFY",
        "pub const REL_PERFORM",
        "pub const REL_EXHIBIT",
        "pub const REL_INCLUDE",
        "pub const ROLE_REQUIREMENT",
        "pub const ROLE_ACTION",
        "pub const ROLE_STATE",
        "pub const ROLE_USE_CASE",
    ];

    let missing: Vec<_> = required_constants
        .iter()
        .filter(|constant| !content.contains(*constant))
        .collect();

    assert!(
        missing.is_empty(),
        "\n❌ Missing required constants in parser/constants.rs:\n{}\n",
        format_violation_list(&missing)
    );
}
