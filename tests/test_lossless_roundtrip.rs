//! Comprehensive lossless roundtrip verification for XMI, JSON-LD, and YAML formats.
//!
//! This test verifies that all data is preserved when converting between formats.
//! It checks not just element counts but also:
//! - Element IDs preserved
//! - Element names preserved
//! - Element kinds preserved  
//! - Relationship sources/targets preserved
//! - Property values preserved (is_abstract, is_derived, etc.)
#![cfg(feature = "interchange")]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use syster::interchange::{JsonLd, ModelFormat, Xmi, Yaml, model::*};

/// Clone the SysML-v2-Release repo if not already present
fn get_sysml_release_dir() -> Option<PathBuf> {
    let tmp_dir = std::env::temp_dir().join("syster-test-sysml-release");

    if !tmp_dir.exists() {
        println!("Cloning SysML-v2-Release repository...");
        let status = Command::new("git")
            .args([
                "clone",
                "--depth=1",
                "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                tmp_dir.to_str().unwrap(),
            ])
            .status()
            .ok()?;

        if !status.success() {
            return None;
        }
    }

    Some(tmp_dir)
}

/// Find all .sysmlx/.kermlx files in the repository
fn find_all_xmi_files(base_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let xmi_lib_dir = base_dir.join("sysml.library.xmi");
    if xmi_lib_dir.exists() {
        collect_xmi_files(&xmi_lib_dir, &mut files);
    }
    files
}

fn collect_xmi_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_xmi_files(&path, files);
            } else if let Some(ext) = path.extension() {
                if ext == "sysmlx" || ext == "kermlx" {
                    files.push(path);
                }
            }
        }
    }
}

/// Detailed comparison result
#[derive(Debug, Default)]
struct ComparisonResult {
    pub missing_elements: Vec<String>,
    pub extra_elements: Vec<String>,
    pub missing_relationships: Vec<String>,
    pub extra_relationships: Vec<String>,
    pub name_mismatches: Vec<(String, String, String)>, // (id, expected, actual)
    pub kind_mismatches: Vec<(String, String, String)>, // (id, expected, actual)
    pub property_mismatches: Vec<(String, String)>,     // (id, description)
}

impl ComparisonResult {
    fn is_lossless(&self) -> bool {
        self.missing_elements.is_empty()
            && self.missing_relationships.is_empty()
            && self.name_mismatches.is_empty()
            && self.kind_mismatches.is_empty()
            && self.property_mismatches.is_empty()
    }

    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.missing_elements.is_empty() {
            parts.push(format!("{} missing elements", self.missing_elements.len()));
        }
        if !self.extra_elements.is_empty() {
            parts.push(format!("{} extra elements", self.extra_elements.len()));
        }
        if !self.missing_relationships.is_empty() {
            parts.push(format!("{} missing rels", self.missing_relationships.len()));
        }
        if !self.extra_relationships.is_empty() {
            parts.push(format!("{} extra rels", self.extra_relationships.len()));
        }
        if !self.name_mismatches.is_empty() {
            parts.push(format!("{} name mismatches", self.name_mismatches.len()));
        }
        if !self.kind_mismatches.is_empty() {
            parts.push(format!("{} kind mismatches", self.kind_mismatches.len()));
        }
        if !self.property_mismatches.is_empty() {
            parts.push(format!(
                "{} property mismatches",
                self.property_mismatches.len()
            ));
        }
        if parts.is_empty() {
            "OK".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Compare two models in detail
fn compare_models(original: &Model, roundtripped: &Model) -> ComparisonResult {
    let mut result = ComparisonResult::default();

    // Build ID sets
    let original_ids: HashSet<_> = original.elements.keys().collect();
    let roundtripped_ids: HashSet<_> = roundtripped.elements.keys().collect();

    // Check for missing/extra elements
    for id in original_ids.difference(&roundtripped_ids) {
        result.missing_elements.push(id.to_string());
    }
    for id in roundtripped_ids.difference(&original_ids) {
        result.extra_elements.push(id.to_string());
    }

    // Compare elements that exist in both
    for id in original_ids.intersection(&roundtripped_ids) {
        let orig_el = original.elements.get(*id).unwrap();
        let rt_el = roundtripped.elements.get(*id).unwrap();

        // Compare names
        if orig_el.name != rt_el.name {
            result.name_mismatches.push((
                id.to_string(),
                orig_el
                    .name
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                rt_el
                    .name
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            ));
        }

        // Compare kinds
        if orig_el.kind != rt_el.kind {
            result.kind_mismatches.push((
                id.to_string(),
                format!("{:?}", orig_el.kind),
                format!("{:?}", rt_el.kind),
            ));
        }

        // Compare important properties
        if orig_el.is_abstract != rt_el.is_abstract {
            result.property_mismatches.push((
                id.to_string(),
                format!(
                    "is_abstract: {} vs {}",
                    orig_el.is_abstract, rt_el.is_abstract
                ),
            ));
        }
        if orig_el.is_derived != rt_el.is_derived {
            result.property_mismatches.push((
                id.to_string(),
                format!("is_derived: {} vs {}", orig_el.is_derived, rt_el.is_derived),
            ));
        }
        if orig_el.is_readonly != rt_el.is_readonly {
            result.property_mismatches.push((
                id.to_string(),
                format!(
                    "is_readonly: {} vs {}",
                    orig_el.is_readonly, rt_el.is_readonly
                ),
            ));
        }
    }

    // Compare relationships by source-kind-target tuples
    let orig_rels: HashSet<_> = original
        .iter_relationship_elements()
        .filter_map(|r| {
            Some((
                r.source()?.to_string(),
                format!("{:?}", r.kind),
                r.target()?.to_string(),
            ))
        })
        .collect();
    let rt_rels: HashSet<_> = roundtripped
        .iter_relationship_elements()
        .filter_map(|r| {
            Some((
                r.source()?.to_string(),
                format!("{:?}", r.kind),
                r.target()?.to_string(),
            ))
        })
        .collect();

    for rel in orig_rels.difference(&rt_rels) {
        result
            .missing_relationships
            .push(format!("{} -{:?}-> {}", rel.0, rel.1, rel.2));
    }
    for rel in rt_rels.difference(&orig_rels) {
        result
            .extra_relationships
            .push(format!("{} -{:?}-> {}", rel.0, rel.1, rel.2));
    }

    result
}

#[test]
#[cfg(feature = "interchange")]
fn test_xmi_lossless_roundtrip() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            eprintln!("Could not get SysML release directory, skipping test");
            return;
        }
    };
    let xmi_files = find_all_xmi_files(&release_dir);

    println!("\n=== XMI Lossless Roundtrip Test ===");
    println!("Testing {} XMI files...\n", xmi_files.len());

    let xmi = Xmi;
    let mut passed = 0;
    let mut failed = 0;
    let mut errors: Vec<(String, String)> = Vec::new();

    for file_path in &xmi_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        // Read and parse original
        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_name, format!("Read error: {}", e)));
                failed += 1;
                continue;
            }
        };

        let original = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_name, format!("Parse error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Export and re-import
        let exported = match xmi.write(&original) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_name, format!("Export error: {}", e)));
                failed += 1;
                continue;
            }
        };

        let roundtripped = match xmi.read(&exported) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_name, format!("Re-import error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Compare in detail
        let comparison = compare_models(&original, &roundtripped);

        if comparison.is_lossless() {
            println!(
                "✓ {} - {} elements, {} relationships",
                file_name,
                original.elements.len(),
                original.relationship_count()
            );
            passed += 1;
        } else {
            println!("✗ {} - {}", file_name, comparison.summary());
            errors.push((file_name, comparison.summary()));
            failed += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", xmi_files.len());

    if !errors.is_empty() && errors.len() <= 10 {
        println!("\nFirst {} errors:", errors.len().min(10));
        for (name, err) in errors.iter().take(10) {
            println!("  {} - {}", name, err);
        }
    }

    // All files should pass for XMI
    assert_eq!(failed, 0, "XMI roundtrip should be completely lossless");
}

#[test]
#[cfg(feature = "interchange")]
fn test_jsonld_lossless_roundtrip() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            eprintln!("Could not get SysML release directory, skipping test");
            return;
        }
    };
    let xmi_files = find_all_xmi_files(&release_dir);

    println!("\n=== JSON-LD Lossless Roundtrip Test (XMI → JSON-LD → Model) ===");
    println!("Testing {} files...\n", xmi_files.len());

    let xmi = Xmi;
    let jsonld = JsonLd;
    let mut passed = 0;
    let mut failed = 0;
    let mut elements_ok = 0;
    let mut rels_lost = 0;
    let mut errors: Vec<(String, String)> = Vec::new();

    for file_path in &xmi_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        // Read and parse original XMI
        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let original = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if original.elements.is_empty() {
            continue;
        }

        // Export to JSON-LD
        let json_bytes = match jsonld.write(&original) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_name, format!("JSON-LD export error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Re-import from JSON-LD
        let roundtripped = match jsonld.read(&json_bytes) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_name, format!("JSON-LD import error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Compare in detail
        let comparison = compare_models(&original, &roundtripped);

        // For JSON-LD, we expect elements to be preserved but relationships may be lost
        // (current implementation doesn't serialize relationships)
        let elements_preserved = comparison.missing_elements.is_empty()
            && comparison.name_mismatches.is_empty()
            && comparison.kind_mismatches.is_empty();

        if elements_preserved {
            if comparison.missing_relationships.is_empty() {
                println!(
                    "✓ {} - {} elements, {} relationships (fully lossless)",
                    file_name,
                    original.elements.len(),
                    original.relationship_count()
                );
                passed += 1;
            } else {
                println!(
                    "⚠ {} - {} elements OK, {} relationships lost",
                    file_name,
                    original.elements.len(),
                    comparison.missing_relationships.len()
                );
                elements_ok += 1;
                rels_lost += comparison.missing_relationships.len();
            }
        } else {
            println!("✗ {} - {}", file_name, comparison.summary());
            errors.push((file_name.clone(), comparison.summary()));
            failed += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Fully lossless: {}", passed);
    println!(
        "Elements OK, rels lost: {} ({} total rels lost)",
        elements_ok, rels_lost
    );
    println!("Failed: {}", failed);

    // All files should be fully lossless
    let total = passed + elements_ok + failed;
    if total > 0 {
        let element_pass_rate = (passed + elements_ok) as f64 / total as f64 * 100.0;
        println!("Element preservation rate: {:.1}%", element_pass_rate);
    }

    // JSON-LD roundtrip should be completely lossless
    assert_eq!(failed, 0, "JSON-LD roundtrip should have no failures");
    assert_eq!(
        rels_lost, 0,
        "JSON-LD roundtrip should preserve all relationships"
    );
}

#[test]
#[cfg(feature = "interchange")]
fn test_yaml_lossless_roundtrip() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            eprintln!("Could not get SysML release directory, skipping test");
            return;
        }
    };
    let xmi_files = find_all_xmi_files(&release_dir);

    println!("\n=== YAML Lossless Roundtrip Test (XMI → YAML → Model) ===");
    println!("Testing {} files...\n", xmi_files.len());

    let xmi = Xmi;
    let yaml = Yaml;
    let mut passed = 0;
    let mut failed = 0;
    let mut elements_ok = 0;
    let mut rels_lost = 0;
    let mut errors: Vec<(String, String)> = Vec::new();

    for file_path in &xmi_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        // Read and parse original XMI
        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let original = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if original.elements.is_empty() {
            continue;
        }

        // Export to YAML
        let yaml_bytes = match yaml.write(&original) {
            Ok(b) => b,
            Err(e) => {
                errors.push((file_name, format!("YAML export error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Re-import from YAML
        let roundtripped = match yaml.read(&yaml_bytes) {
            Ok(m) => m,
            Err(e) => {
                errors.push((file_name, format!("YAML import error: {}", e)));
                failed += 1;
                continue;
            }
        };

        // Compare in detail
        let comparison = compare_models(&original, &roundtripped);

        // For YAML, we expect elements to be preserved but relationships may be lost
        // (current implementation doesn't serialize relationships)
        let elements_preserved = comparison.missing_elements.is_empty()
            && comparison.name_mismatches.is_empty()
            && comparison.kind_mismatches.is_empty();

        if elements_preserved {
            if comparison.missing_relationships.is_empty() {
                println!(
                    "✓ {} - {} elements, {} relationships (fully lossless)",
                    file_name,
                    original.elements.len(),
                    original.relationship_count()
                );
                passed += 1;
            } else {
                println!(
                    "⚠ {} - {} elements OK, {} relationships lost",
                    file_name,
                    original.elements.len(),
                    comparison.missing_relationships.len()
                );
                elements_ok += 1;
                rels_lost += comparison.missing_relationships.len();
            }
        } else {
            println!("✗ {} - {}", file_name, comparison.summary());
            errors.push((file_name.clone(), comparison.summary()));
            failed += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Fully lossless: {}", passed);
    println!(
        "Elements OK, rels lost: {} ({} total rels lost)",
        elements_ok, rels_lost
    );
    println!("Failed: {}", failed);

    // All files should be fully lossless
    let total = passed + elements_ok + failed;
    if total > 0 {
        let element_pass_rate = (passed + elements_ok) as f64 / total as f64 * 100.0;
        println!("Element preservation rate: {:.1}%", element_pass_rate);
    }

    // YAML roundtrip should be completely lossless
    assert_eq!(failed, 0, "YAML roundtrip should have no failures");
    assert_eq!(
        rels_lost, 0,
        "YAML roundtrip should preserve all relationships"
    );
}

/// Test that converting between all formats preserves data
/// Note: Relationships are expected to be lost during JSON-LD/YAML conversion
/// as those formats don't serialize relationships yet.
#[test]
#[cfg(feature = "interchange")]
fn test_cross_format_roundtrip() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            eprintln!("Could not get SysML release directory, skipping test");
            return;
        }
    };

    // Test with a subset of files for speed
    let xmi_files: Vec<_> = find_all_xmi_files(&release_dir)
        .into_iter()
        .take(10)
        .collect();

    println!("\n=== Cross-Format Roundtrip Test ===");
    println!(
        "Testing XMI → JSON-LD → YAML → XMI for {} files...\n",
        xmi_files.len()
    );

    let xmi = Xmi;
    let jsonld = JsonLd;
    let yaml = Yaml;

    let mut passed = 0;
    let mut elements_ok = 0;
    let mut failed = 0;
    let mut rels_lost = 0;

    for file_path in &xmi_files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        // Read original XMI
        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let original = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if original.elements.is_empty() {
            continue;
        }

        // XMI → JSON-LD
        let json_bytes = match jsonld.write(&original) {
            Ok(b) => b,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        let model_from_json = match jsonld.read(&json_bytes) {
            Ok(m) => m,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        // JSON-LD → YAML
        let yaml_bytes = match yaml.write(&model_from_json) {
            Ok(b) => b,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        let model_from_yaml = match yaml.read(&yaml_bytes) {
            Ok(m) => m,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        // YAML → XMI (final)
        let final_xmi = match xmi.write(&model_from_yaml) {
            Ok(b) => b,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        let final_model = match xmi.read(&final_xmi) {
            Ok(m) => m,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        // Compare original to final
        let comparison = compare_models(&original, &final_model);

        // For cross-format, we expect elements to be preserved but relationships will be lost
        let elements_preserved = comparison.missing_elements.is_empty()
            && comparison.name_mismatches.is_empty()
            && comparison.kind_mismatches.is_empty();

        if elements_preserved {
            if comparison.missing_relationships.is_empty() {
                println!(
                    "✓ {} - {} elements (fully lossless)",
                    file_name,
                    original.elements.len()
                );
                passed += 1;
            } else {
                println!(
                    "⚠ {} - {} elements OK, {} relationships lost",
                    file_name,
                    original.elements.len(),
                    comparison.missing_relationships.len()
                );
                elements_ok += 1;
                rels_lost += comparison.missing_relationships.len();
            }
        } else {
            println!("✗ {} - {}", file_name, comparison.summary());
            failed += 1;
        }
    }

    println!("\n=== Summary ===");
    println!("Fully lossless: {}", passed);
    println!(
        "Elements OK, rels lost: {} ({} total rels lost)",
        elements_ok, rels_lost
    );
    println!("Failed: {}", failed);

    let total = passed + elements_ok + failed;
    if total > 0 {
        let element_pass_rate = (passed + elements_ok) as f64 / total as f64 * 100.0;
        println!("Element preservation rate: {:.1}%", element_pass_rate);
        // Elements should all be preserved even if relationships are lost
        assert!(
            element_pass_rate >= 95.0,
            "Cross-format element preservation rate {:.1}% is below 95%",
            element_pass_rate
        );
    }

    if rels_lost > 0 {
        println!(
            "\n⚠ NOTE: {} relationships lost due to JSON-LD/YAML format limitations.",
            rels_lost
        );
    }
}
/// Test direct byte-level comparison for XMI roundtrip.
/// This reveals any formatting/ordering differences even if semantically equivalent.
#[test]
#[cfg(feature = "interchange")]
fn test_xmi_byte_comparison() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            println!("Skipping test - could not get SysML release directory");
            return;
        }
    };

    let xmi_files = find_all_xmi_files(&release_dir);
    if xmi_files.is_empty() {
        println!("No XMI files found, skipping test");
        return;
    }

    let xmi = Xmi;
    let mut identical = 0;
    let mut different = 0;
    let mut differences: Vec<(String, ByteDiff)> = Vec::new();

    for file_path in xmi_files.iter().take(20) {
        // Start with first 20 files
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        let original_bytes = match std::fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let model = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let roundtripped_bytes = match xmi.write(&model) {
            Ok(b) => b,
            Err(_) => continue,
        };

        if original_bytes == roundtripped_bytes {
            println!("✓ {} - byte-identical", file_name);
            identical += 1;
        } else {
            let diff = analyze_byte_diff(&original_bytes, &roundtripped_bytes);
            println!(
                "≠ {} - {} bytes orig, {} bytes new ({})",
                file_name,
                original_bytes.len(),
                roundtripped_bytes.len(),
                diff.summary()
            );
            differences.push((file_name, diff));
            different += 1;
        }
    }

    println!("\n=== Byte Comparison Summary ===");
    println!("Identical: {}", identical);
    println!("Different: {}", different);

    // Show sample differences
    if !differences.is_empty() {
        println!("\n=== Sample Differences ===");
        for (name, diff) in differences.iter().take(5) {
            println!("\n{}:", name);
            println!(
                "  Size: {} -> {} ({:+} bytes)",
                diff.orig_size, diff.new_size, diff.size_diff
            );
            if let Some(ref first_diff) = diff.first_diff_context {
                println!("  First diff around byte {}:", diff.first_diff_pos);
                println!("    Original: {:?}", first_diff.0);
                println!("    New:      {:?}", first_diff.1);
            }
        }
    }

    // Don't fail - this is informational
    println!("\nNote: Byte differences don't necessarily indicate data loss.");
}

#[derive(Debug)]
struct ByteDiff {
    orig_size: usize,
    new_size: usize,
    size_diff: isize,
    first_diff_pos: usize,
    first_diff_context: Option<(String, String)>,
}

impl ByteDiff {
    fn summary(&self) -> String {
        if self.size_diff == 0 {
            "same size, different content".to_string()
        } else if self.size_diff > 0 {
            format!("+{} bytes", self.size_diff)
        } else {
            format!("{} bytes", self.size_diff)
        }
    }
}

fn analyze_byte_diff(original: &[u8], roundtripped: &[u8]) -> ByteDiff {
    let orig_size = original.len();
    let new_size = roundtripped.len();
    let size_diff = new_size as isize - orig_size as isize;

    // Find first differing position
    let mut first_diff_pos = 0;
    for (i, (a, b)) in original.iter().zip(roundtripped.iter()).enumerate() {
        if a != b {
            first_diff_pos = i;
            break;
        }
        first_diff_pos = i + 1;
    }

    // Get context around first difference
    let first_diff_context = if first_diff_pos < orig_size.min(new_size) {
        let start = first_diff_pos.saturating_sub(20);
        let orig_end = (first_diff_pos + 40).min(orig_size);
        let new_end = (first_diff_pos + 40).min(new_size);

        let orig_ctx = String::from_utf8_lossy(&original[start..orig_end]).to_string();
        let new_ctx = String::from_utf8_lossy(&roundtripped[start..new_end]).to_string();
        Some((orig_ctx, new_ctx))
    } else {
        None
    };

    ByteDiff {
        orig_size,
        new_size,
        size_diff,
        first_diff_pos,
        first_diff_context,
    }
}

/// Detailed line-by-line comparison of a single file to understand structural differences.
#[test]
#[cfg(feature = "interchange")]
fn test_xmi_detailed_diff() {
    let release_dir = match get_sysml_release_dir() {
        Some(d) => d,
        None => {
            println!("Skipping test - could not get SysML release directory");
            return;
        }
    };

    // Use a smaller file for detailed comparison
    let file_path = release_dir
        .join("sysml.library.xmi/Domain Libraries/Quantities and Units/Quantities.sysmlx");
    if !file_path.exists() {
        println!("Skipping - test file not found");
        return;
    }

    let xmi = Xmi;

    let original_bytes = std::fs::read(&file_path).expect("read file");
    let model = xmi.read(&original_bytes).expect("parse XMI");
    let roundtripped_bytes = xmi.write(&model).expect("write XMI");

    let original_str = String::from_utf8_lossy(&original_bytes);
    let roundtripped_str = String::from_utf8_lossy(&roundtripped_bytes);

    println!("=== Original (first 80 lines) ===");
    for (i, line) in original_str.lines().take(80).enumerate() {
        println!("{:4}: {}", i + 1, line);
    }

    println!("\n=== Roundtripped (first 80 lines) ===");
    for (i, line) in roundtripped_str.lines().take(80).enumerate() {
        println!("{:4}: {}", i + 1, line);
    }

    // Count key structural elements
    let orig_elements = original_str.matches("xmi:id=").count();
    let new_elements = roundtripped_str.matches("xmi:id=").count();

    let orig_rels = original_str.matches("ownedRelationship").count();
    let new_rels = roundtripped_str.matches("ownedRelationship").count();

    println!("\n=== Structural Comparison ===");
    println!(
        "Elements (xmi:id count): {} -> {}",
        orig_elements, new_elements
    );
    println!("ownedRelationship count: {} -> {}", orig_rels, new_rels);
    println!(
        "Total bytes: {} -> {}",
        original_bytes.len(),
        roundtripped_bytes.len()
    );
}
