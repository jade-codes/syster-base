//! Standard Library HIR tests.
//!
//! These tests verify that the standard library loads correctly and that
//! key symbols are extracted without duplicates.

use crate::helpers::symbol_assertions::*;
use std::fs;
use std::path::PathBuf;
use syster::hir::SymbolKind;
use syster::ide::AnalysisHost;

// =============================================================================
// HELPERS
// =============================================================================

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn load_stdlib_file(relative_path: &str) -> (AnalysisHost, String) {
    let path = stdlib_path().join(relative_path);
    assert!(path.exists(), "Stdlib file not found: {:?}", path);

    let content = fs::read_to_string(&path).expect("Failed to read stdlib file");
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut host = AnalysisHost::new();
    let errors = host.set_file_content(filename, &content);
    // Stdlib files should parse without errors
    assert!(
        errors.is_empty(),
        "Parse errors in stdlib file '{}': {:?}",
        relative_path,
        errors
    );

    (host, filename.to_string())
}

fn load_all_stdlib_files() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();

    // Load all .kerml and .sysml files
    load_files_recursive(&mut host, &stdlib);

    // Trigger index rebuild
    let _ = host.analysis();
    host
}

fn load_files_recursive(host: &mut AnalysisHost, dir: &PathBuf) {
    if !dir.exists() {
        return;
    }

    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.is_dir() {
            load_files_recursive(host, &path);
        } else if let Some(ext) = path.extension() {
            if ext == "kerml" || ext == "sysml" {
                let content = fs::read_to_string(&path).expect("Failed to read file");
                let filename = path.to_string_lossy().to_string();
                let _ = host.set_file_content(&filename, &content);
            }
        }
    }
}

// =============================================================================
// INDIVIDUAL FILE LOADING TESTS
// =============================================================================

#[test]
fn test_performances_kerml_loads_without_duplicates() {
    let (mut host, filename) =
        load_stdlib_file("Kernel Libraries/Kernel Semantic Library/Performances.kerml");
    let analysis = host.analysis();

    // Count 'thisPerformance' symbols - should be exactly 1
    let this_perf_count = analysis
        .symbol_index()
        .all_symbols()
        .filter(|s| s.name.as_ref() == "thisPerformance")
        .count();

    assert_eq!(
        this_perf_count, 1,
        "Should have exactly one 'thisPerformance' definition in {}, got {}",
        filename, this_perf_count
    );
}

#[test]
fn test_observation_kerml_loads_without_duplicates() {
    let (mut host, filename) =
        load_stdlib_file("Kernel Libraries/Kernel Semantic Library/Observation.kerml");
    let analysis = host.analysis();

    // Count 'observations' symbols - there are 2 in the file:
    // 1. private composite feature observations[0..*] : ObserveChange; (line 108)
    // 2. private feature observations[0..*] : ObserveChange = ... (line 151, inside cancelObservation)
    let obs_count = analysis
        .symbol_index()
        .all_symbols()
        .filter(|s| s.name.as_ref() == "observations")
        .count();

    assert_eq!(
        obs_count, 2,
        "Should have exactly two 'observations' definitions in {}, got {}",
        filename, obs_count
    );
}

#[test]
fn test_objects_kerml_loads_without_duplicates() {
    let (mut host, filename) =
        load_stdlib_file("Kernel Libraries/Kernel Semantic Library/Objects.kerml");
    let analysis = host.analysis();

    // Count 'StructuredSpaceObject' symbols
    let sso_count = analysis
        .symbol_index()
        .all_symbols()
        .filter(|s| s.name.as_ref() == "StructuredSpaceObject")
        .count();

    assert_eq!(
        sso_count, 1,
        "Should have exactly one 'StructuredSpaceObject' definition in {}, got {}",
        filename, sso_count
    );
}

#[test]
fn test_measurement_references_sysml_loads_without_duplicates() {
    let (mut host, filename) =
        load_stdlib_file("Domain Libraries/Quantities and Units/MeasurementReferences.sysml");
    let analysis = host.analysis();

    // Count 'MeasurementReferences' package symbols
    let mr_count = analysis
        .symbol_index()
        .all_symbols()
        .filter(|s| s.name.as_ref() == "MeasurementReferences" && s.kind == SymbolKind::Package)
        .count();

    assert_eq!(
        mr_count, 1,
        "Should have exactly one 'MeasurementReferences' package in {}, got {}",
        filename, mr_count
    );
}

// =============================================================================
// KEY SYMBOL EXISTENCE TESTS
// =============================================================================

#[test]
fn test_scalar_values_real_exists() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    // Real should exist
    let real = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| s.name.as_ref() == "Real");

    assert!(real.is_some(), "ScalarValues::Real should exist");
}

#[test]
fn test_scalar_values_integer_exists() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let integer = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| s.name.as_ref() == "Integer");

    assert!(integer.is_some(), "ScalarValues::Integer should exist");
}

#[test]
fn test_scalar_values_boolean_exists() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let boolean = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| s.name.as_ref() == "Boolean");

    assert!(boolean.is_some(), "ScalarValues::Boolean should exist");
}

#[test]
fn test_scalar_values_string_exists() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let string = analysis
        .symbol_index()
        .all_symbols()
        .find(|s| s.name.as_ref() == "String");

    assert!(string.is_some(), "ScalarValues::String should exist");
}

// =============================================================================
// SYMBOL COUNT VALIDATION
// =============================================================================

#[test]
fn test_scalar_values_has_expected_symbols() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let symbol_names: Vec<_> = analysis
        .symbol_index()
        .all_symbols()
        .map(|s| s.name.as_ref().to_string())
        .collect();

    // Should have common scalar types
    assert!(
        symbol_names.len() >= 5,
        "Expected at least 5 symbols, got {}: {:?}",
        symbol_names.len(),
        symbol_names
    );

    // Check for key types
    assert!(
        symbol_names.contains(&"Real".to_string()),
        "Should have Real"
    );
    assert!(
        symbol_names.contains(&"Integer".to_string()),
        "Should have Integer"
    );
}

#[test]
fn test_single_stdlib_file_symbol_count() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let count = analysis.symbol_index().all_symbols().count();

    // ScalarValues.kerml should have a reasonable number of symbols
    assert!(
        count >= 5,
        "Expected at least 5 symbols in ScalarValues.kerml, got {}",
        count
    );
    assert!(
        count < 500,
        "Unexpectedly high symbol count {} - possible duplication",
        count
    );
}

// =============================================================================
// NO DUPLICATE SYMBOLS TESTS
// =============================================================================

#[test]
fn test_stdlib_file_no_duplicate_qualified_names() {
    let (mut host, _) =
        load_stdlib_file("Kernel Libraries/Kernel Data Type Library/ScalarValues.kerml");
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();
    assert_no_duplicate_symbols(&symbols);
}

// =============================================================================
// FULL STDLIB TESTS (slower, run last)
// =============================================================================

#[test]
fn test_full_stdlib_loads_without_errors() {
    let mut host = load_all_stdlib_files();
    let analysis = host.analysis();

    let count = analysis.symbol_index().all_symbols().count();

    // Full stdlib should have 1400+ symbols
    assert!(
        count >= 1400,
        "Expected at least 1400 symbols in full stdlib, got {}",
        count
    );
}

#[test]
fn test_full_stdlib_si_package_exists() {
    let mut host = load_all_stdlib_files();
    let analysis = host.analysis();

    let si = analysis.symbol_index().lookup_qualified("SI");
    assert!(si.is_some(), "SI package should exist in full stdlib");
}

#[test]
fn test_full_stdlib_isq_package_exists() {
    let mut host = load_all_stdlib_files();
    let analysis = host.analysis();

    let isq = analysis.symbol_index().lookup_qualified("ISQ");
    assert!(isq.is_some(), "ISQ package should exist in full stdlib");
}

#[test]
fn test_full_stdlib_no_duplicate_symbols() {
    let mut host = load_all_stdlib_files();
    let analysis = host.analysis();

    let symbols: Vec<_> = analysis.symbol_index().all_symbols().cloned().collect();

    // Check for duplicates
    let mut seen = std::collections::HashMap::new();
    let mut duplicates = Vec::new();

    for sym in &symbols {
        let key = sym.qualified_name.as_ref();
        if let Some(prev) = seen.insert(key, sym) {
            duplicates.push((prev.qualified_name.clone(), sym.qualified_name.clone()));
        }
    }

    assert!(
        duplicates.is_empty(),
        "Found {} duplicate qualified names in stdlib: {:?}",
        duplicates.len(),
        duplicates.iter().take(10).collect::<Vec<_>>()
    );
}
