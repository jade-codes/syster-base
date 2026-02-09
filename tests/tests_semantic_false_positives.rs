//! Tests for semantic analysis false positives
//!
//! This module tests that our semantic analysis (symbol resolution, type checking)
//! does NOT produce false positive errors when analyzing:
//! 1. The official SysML v2 standard library (sysml.library)
//! 2. Official SysML v2 Release examples
//!
//! These tests ensure that valid SysML/KerML code is not incorrectly flagged as having errors.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use syster::base::FileId;
use syster::hir::{Diagnostic, Severity, SymbolIndex, check_file, extract_symbols_unified};
use syster::syntax::parser::parse_content;

fn get_stdlib_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn get_examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sysml-examples")
}

/// Recursively collect all .sysml and .kerml files from a directory
fn collect_model_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_model_files(&path, files);
            } else if let Some(ext) = path.extension() {
                if ext == "sysml" || ext == "kerml" {
                    files.push(path);
                }
            }
        }
    }
}

/// Load all files from a directory into a SymbolIndex
fn load_directory_into_index(dir: &Path) -> (SymbolIndex, Vec<(FileId, PathBuf)>) {
    let mut files = Vec::new();
    collect_model_files(dir, &mut files);
    files.sort();

    let mut index = SymbolIndex::new();
    let mut file_info = Vec::new();

    for (i, path) in files.iter().enumerate() {
        let file_id = FileId::new(i as u32);
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let syntax = parse_content(&content, path).unwrap();
        let symbols = extract_symbols_unified(file_id, &syntax);
        index.add_file(file_id, symbols);
        file_info.push((file_id, path.clone()));
    }

    index.ensure_visibility_maps();
    (index, file_info)
}

/// Get all semantic errors (excluding warnings) for all files in an index
fn get_all_errors(
    index: &SymbolIndex,
    file_info: &[(FileId, PathBuf)],
) -> Vec<(PathBuf, Diagnostic)> {
    let mut all_errors = Vec::new();

    for (file_id, path) in file_info {
        let diagnostics = check_file(index, *file_id);
        for diag in diagnostics {
            if diag.severity == Severity::Error {
                all_errors.push((path.clone(), diag));
            }
        }
    }

    all_errors
}

/// Test that the SysML v2 standard library has zero semantic errors
///
/// This is the primary regression test for false positives.
/// The sysml.library contains the official KerML and SysML standard library files.
#[test]
fn test_stdlib_zero_semantic_errors() {
    let stdlib_dir = get_stdlib_dir();

    if !stdlib_dir.exists() {
        eprintln!("⏭️  Skipping: sysml.library not found at {stdlib_dir:?}");
        return;
    }

    let (index, file_info) = load_directory_into_index(&stdlib_dir);
    let errors = get_all_errors(&index, &file_info);

    if !errors.is_empty() {
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║  STDLIB SEMANTIC ANALYSIS ERRORS (FALSE POSITIVES)             ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");

        for (path, diag) in &errors {
            let relative = path.strip_prefix(&stdlib_dir).unwrap_or(path).display();
            eprintln!(
                "║  {}:{}:{}: {}",
                relative,
                diag.start_line + 1,
                diag.start_col + 1,
                diag.message
            );
        }

        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!(
            "║  Total: {} errors in {} files                                  ║",
            errors.len(),
            file_info.len()
        );
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
    }

    assert!(
        errors.is_empty(),
        "Expected 0 semantic errors in sysml.library, but found {}. \
         These are likely false positives that need to be fixed.",
        errors.len()
    );

    eprintln!(
        "✓ sysml.library: {} files analyzed, 0 semantic errors",
        file_info.len()
    );
}

/// Test that SysML v2 Release examples have zero semantic errors
///
/// This test requires the examples to be set up first:
/// ```bash
/// git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml
/// cp -r /tmp/sysml/sysml/src/examples tests/sysml-examples
/// ```
#[test]
fn test_examples_zero_semantic_errors() {
    let examples_dir = get_examples_dir();

    if !examples_dir.exists() {
        eprintln!("⏭️  Skipping: sysml-examples not found at {examples_dir:?}");
        eprintln!("   To run this test, execute:");
        eprintln!(
            "   git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml"
        );
        eprintln!("   cp -r /tmp/sysml/sysml/src/examples tests/sysml-examples");
        return;
    }

    // Load stdlib first (examples depend on it)
    let stdlib_dir = get_stdlib_dir();
    let mut all_files = Vec::new();

    if stdlib_dir.exists() {
        collect_model_files(&stdlib_dir, &mut all_files);
    }

    // Then load examples
    let examples_start_idx = all_files.len();
    collect_model_files(&examples_dir, &mut all_files);
    all_files.sort();

    let mut index = SymbolIndex::new();
    let mut file_info = Vec::new();

    for (i, path) in all_files.iter().enumerate() {
        let file_id = FileId::new(i as u32);
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let syntax = parse_content(&content, path).unwrap();
        let symbols = extract_symbols_unified(file_id, &syntax);
        index.add_file(file_id, symbols);

        // Only track example files for error reporting
        if i >= examples_start_idx {
            file_info.push((file_id, path.clone()));
        }
    }

    index.ensure_visibility_maps();

    let errors = get_all_errors(&index, &file_info);

    if !errors.is_empty() {
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║  EXAMPLE SEMANTIC ANALYSIS ERRORS (FALSE POSITIVES)            ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");

        for (path, diag) in &errors {
            let relative = path.strip_prefix(&examples_dir).unwrap_or(path).display();
            eprintln!(
                "║  {}:{}:{}: {}",
                relative,
                diag.start_line + 1,
                diag.start_col + 1,
                diag.message
            );
        }

        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!(
            "║  Total: {} errors in {} example files                          ║",
            errors.len(),
            file_info.len()
        );
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
    }

    assert!(
        errors.is_empty(),
        "Expected 0 semantic errors in sysml-examples, but found {}. \
         These are likely false positives that need to be fixed.",
        errors.len()
    );

    eprintln!(
        "✓ sysml-examples: {} files analyzed, 0 semantic errors",
        file_info.len()
    );
}

/// Combined test that checks both stdlib and examples together
/// This is the most comprehensive false positive regression test.
#[test]
fn test_all_zero_semantic_errors() {
    let stdlib_dir = get_stdlib_dir();
    let examples_dir = get_examples_dir();

    if !stdlib_dir.exists() {
        eprintln!("⏭️  Skipping: sysml.library not found");
        return;
    }

    let mut all_files = Vec::new();
    collect_model_files(&stdlib_dir, &mut all_files);

    let has_examples = examples_dir.exists();
    if has_examples {
        collect_model_files(&examples_dir, &mut all_files);
    }

    all_files.sort();

    let mut index = SymbolIndex::new();
    let mut file_info = Vec::new();

    for (i, path) in all_files.iter().enumerate() {
        let file_id = FileId::new(i as u32);
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let syntax = parse_content(&content, path).unwrap();
        let symbols = extract_symbols_unified(file_id, &syntax);
        index.add_file(file_id, symbols);
        file_info.push((file_id, path.clone()));
    }

    index.ensure_visibility_maps();

    let errors = get_all_errors(&index, &file_info);
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    if !errors.is_empty() {
        eprintln!("\n╔════════════════════════════════════════════════════════════════╗");
        eprintln!("║  SEMANTIC ANALYSIS ERRORS (FALSE POSITIVES)                    ║");
        eprintln!("╠════════════════════════════════════════════════════════════════╣");

        for (path, diag) in &errors {
            let relative = path.strip_prefix(&base_dir).unwrap_or(path).display();
            eprintln!(
                "║  {}:{}:{}: {}",
                relative,
                diag.start_line + 1,
                diag.start_col + 1,
                diag.message
            );
        }

        eprintln!("╠════════════════════════════════════════════════════════════════╣");
        eprintln!(
            "║  Total: {} errors in {} files                                  ║",
            errors.len(),
            file_info.len()
        );
        eprintln!("╚════════════════════════════════════════════════════════════════╝\n");
    }

    assert!(
        errors.is_empty(),
        "Expected 0 semantic errors, but found {}. \
         These are likely false positives that need to be fixed.",
        errors.len()
    );

    eprintln!(
        "✓ All files: {} analyzed, 0 semantic errors{}",
        file_info.len(),
        if has_examples {
            " (stdlib + examples)"
        } else {
            " (stdlib only)"
        }
    );
}
