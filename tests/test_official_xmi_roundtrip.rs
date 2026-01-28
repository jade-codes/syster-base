//! Integration tests for official SysML v2 XMI files.
//!
//! This test uses the official SysML v2 library XMI files from the GitHub repository.
//! 
//! To run these tests, first clone the official repository:
//!   git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release
//!
//! Then run:
//!   cargo test --features interchange --test test_official_xmi_roundtrip -- --nocapture
//!
//! If the repository is not cloned, the tests will be skipped.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(feature = "interchange")]
use syster::interchange::{ModelFormat, Xmi};

/// Possible locations for the official SysML v2 repository
const REPO_LOCATIONS: &[&str] = &[
    "/tmp/sysml-v2-release",
    "target/test-data/sysml-v2-release",
    "../sysml-v2-release",
    "../../sysml-v2-release",
];

/// XMI library subdirectories within the repo
const XMI_SUBDIRS: &[&str] = &[
    "sysml.library.xmi",
    "sysml.library.xmi.implied",
];

/// Find the official repository or clone it
fn find_or_clone_repo() -> Option<PathBuf> {
    // Check existing locations
    for loc in REPO_LOCATIONS {
        let path = PathBuf::from(loc);
        // Check if at least the first XMI subdir exists
        let xmi_path = path.join(XMI_SUBDIRS[0]);
        if xmi_path.exists() && xmi_path.is_dir() {
            eprintln!("Found SysML v2 repository at: {}", path.display());
            return Some(path);
        }
    }

    // Try to clone to target directory
    let clone_path = PathBuf::from("target/test-data/sysml-v2-release");
    if !clone_path.exists() {
        eprintln!("Cloning SysML v2 repository (this may take a moment)...");
        let result = Command::new("git")
            .args([
                "clone",
                "--depth", "1",
                "--filter=blob:none",
                "--sparse",
                "https://github.com/Systems-Modeling/SysML-v2-Release.git",
                clone_path.to_str().unwrap(),
            ])
            .output();

        if let Ok(output) = result {
            if output.status.success() {
                // Enable sparse checkout for all XMI libraries
                let mut args = vec!["sparse-checkout", "set"];
                args.extend(XMI_SUBDIRS.iter().copied());
                let _ = Command::new("git")
                    .args(&args)
                    .current_dir(&clone_path)
                    .output();
                eprintln!("Repository cloned successfully.");
            } else {
                eprintln!(
                    "Failed to clone: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return None;
            }
        } else {
            eprintln!("Git not available for cloning.");
            return None;
        }
    }

    // Check if at least the first XMI subdir exists
    let xmi_path = clone_path.join(XMI_SUBDIRS[0]);
    if xmi_path.exists() {
        Some(clone_path)
    } else {
        None
    }
}

/// Count XMI files in a directory recursively
fn count_xmi_files(path: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_xmi_files(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "sysmlx" || ext == "kermlx" {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Collect all XMI files from a directory
fn collect_xmi_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_xmi_files(&path));
            } else if let Some(ext) = path.extension() {
                if ext == "sysmlx" || ext == "kermlx" {
                    files.push(path);
                }
            }
        }
    }
    files
}

#[cfg(feature = "interchange")]
#[test]
fn test_official_xmi_roundtrip() {
    let repo_path = match find_or_clone_repo() {
        Some(p) => p,
        None => {
            eprintln!("\n=== SKIPPED ===");
            eprintln!("SysML v2 repository not found.");
            eprintln!("To run this test, clone the official repository:");
            eprintln!("  git clone --depth 1 https://github.com/Systems-Modeling/SysML-v2-Release.git /tmp/sysml-v2-release");
            return;
        }
    };

    // Collect files from all XMI subdirectories
    let mut files = Vec::new();
    for subdir in XMI_SUBDIRS {
        let xmi_path = repo_path.join(subdir);
        if xmi_path.exists() {
            files.extend(collect_xmi_files(&xmi_path));
        }
    }

    if files.is_empty() {
        eprintln!("No XMI files found. Test skipped.");
        return;
    }

    eprintln!("\nTesting {} official XMI files...\n", files.len());

    let xmi = Xmi::default();
    let mut results = TestResults::default();

    for file_path in &files {
        let file_name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Read original file
        let original_bytes = match fs::read(file_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                results.add_error(&file_name, format!("Read error: {}", e));
                continue;
            }
        };

        // Parse original XMI
        let model1 = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(e) => {
                results.add_error(&file_name, format!("Parse error: {}", e));
                continue;
            }
        };

        let element_count1 = model1.element_count();
        if element_count1 == 0 {
            results.add_error(&file_name, "No elements parsed".to_string());
            continue;
        }

        // Write to XMI
        let exported_bytes = match xmi.write(&model1) {
            Ok(bytes) => bytes,
            Err(e) => {
                results.add_error(&file_name, format!("Export error: {}", e));
                continue;
            }
        };

        // Re-parse exported XMI
        let model2 = match xmi.read(&exported_bytes) {
            Ok(m) => m,
            Err(e) => {
                results.add_error(&file_name, format!("Re-parse error: {}", e));
                continue;
            }
        };

        let element_count2 = model2.element_count();

        // Compare element counts
        if element_count1 != element_count2 {
            results.add_warning(
                &file_name,
                format!(
                    "Element count mismatch: {} → {}",
                    element_count1, element_count2
                ),
            );
        }

        // Compare element names
        let names1: Vec<_> = model1
            .iter_elements()
            .filter_map(|e| e.name.as_ref().map(|n| n.to_string()))
            .collect();
        let names2: Vec<_> = model2
            .iter_elements()
            .filter_map(|e| e.name.as_ref().map(|n| n.to_string()))
            .collect();

        let names_match = names1.len() == names2.len()
            && names1.iter().all(|n| names2.contains(n));

        if !names_match {
            results.add_warning(
                &file_name,
                format!(
                    "Named element count: {} → {}",
                    names1.len(),
                    names2.len()
                ),
            );
        }

        // Compare element kinds
        let mut kinds1: HashMap<String, usize> = HashMap::new();
        for e in model1.iter_elements() {
            *kinds1.entry(format!("{:?}", e.kind)).or_default() += 1;
        }
        let mut kinds2: HashMap<String, usize> = HashMap::new();
        for e in model2.iter_elements() {
            *kinds2.entry(format!("{:?}", e.kind)).or_default() += 1;
        }

        let kinds_match = kinds1 == kinds2;
        if !kinds_match {
            results.add_warning(&file_name, "Element kind distribution changed".to_string());
        }

        // Compare isAbstract flags
        let abstract_count1 = model1.iter_elements().filter(|e| e.is_abstract).count();
        let abstract_count2 = model2.iter_elements().filter(|e| e.is_abstract).count();
        if abstract_count1 != abstract_count2 {
            results.add_warning(
                &file_name,
                format!(
                    "Abstract count: {} → {}",
                    abstract_count1, abstract_count2
                ),
            );
        }

        // Deep comparison: verify each element's content matches
        let mut content_match = true;
        for elem1 in model1.iter_elements() {
            if let Some(elem2) = model2.get(&elem1.id) {
                // Check name
                if elem1.name != elem2.name {
                    results.add_warning(
                        &file_name,
                        format!("Name mismatch for {}: {:?} → {:?}", elem1.id, elem1.name, elem2.name),
                    );
                    content_match = false;
                }
                // Check kind
                if elem1.kind != elem2.kind {
                    results.add_warning(
                        &file_name,
                        format!("Kind mismatch for {}: {:?} → {:?}", elem1.id, elem1.kind, elem2.kind),
                    );
                    content_match = false;
                }
                // Check isAbstract
                if elem1.is_abstract != elem2.is_abstract {
                    results.add_warning(
                        &file_name,
                        format!("isAbstract mismatch for {}: {} → {}", elem1.id, elem1.is_abstract, elem2.is_abstract),
                    );
                    content_match = false;
                }
                // Check documentation
                if elem1.documentation != elem2.documentation {
                    results.add_warning(
                        &file_name,
                        format!("documentation mismatch for {}", elem1.id),
                    );
                    content_match = false;
                }
                // Check owner
                if elem1.owner != elem2.owner {
                    results.add_warning(
                        &file_name,
                        format!("owner mismatch for {}: {:?} → {:?}", elem1.id, elem1.owner, elem2.owner),
                    );
                    content_match = false;
                }
                // Check properties count
                if elem1.properties.len() != elem2.properties.len() {
                    results.add_warning(
                        &file_name,
                        format!("properties count mismatch for {}: {} → {}", elem1.id, elem1.properties.len(), elem2.properties.len()),
                    );
                    content_match = false;
                }
            } else {
                results.add_warning(
                    &file_name,
                    format!("Element {} missing after roundtrip", elem1.id),
                );
                content_match = false;
            }
        }

        // Success if we got here with matching counts AND content
        if element_count1 == element_count2 && names_match && kinds_match && content_match {
            results.add_success(&file_name, element_count1);
        } else {
            results.add_partial(&file_name, element_count1, element_count2);
        }
    }

    // Print results
    results.print_summary();

    // Assert at least 80% success rate
    let success_rate = results.success_count as f64 / files.len() as f64;
    assert!(
        success_rate >= 0.8,
        "Success rate {:.1}% is below 80% threshold",
        success_rate * 100.0
    );
}

/// Test that writing XMI produces stable output (write twice, get same bytes)
#[cfg(feature = "interchange")]
#[test]
fn test_xmi_write_stability() {
    let repo_path = match find_or_clone_repo() {
        Some(p) => p,
        None => {
            eprintln!("SysML v2 repository not found. Skipping stability test.");
            return;
        }
    };

    // Collect files from all XMI subdirectories
    let mut files = Vec::new();
    for subdir in XMI_SUBDIRS {
        let xmi_path = repo_path.join(subdir);
        if xmi_path.exists() {
            files.extend(collect_xmi_files(&xmi_path));
        }
    }

    eprintln!("\nTesting XMI write stability for {} files...\n", files.len());

    let xmi = Xmi::default();
    let mut stable = 0;
    let mut unstable = 0;

    for file_path in &files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();

        let original_bytes = match fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let model = match xmi.read(&original_bytes) {
            Ok(m) if m.element_count() > 0 => m,
            _ => continue,
        };

        // Write twice
        let write1 = xmi.write(&model).unwrap();
        let write2 = xmi.write(&model).unwrap();

        if write1 == write2 {
            stable += 1;
        } else {
            eprintln!("  ✗ {}: Write output differs ({} vs {} bytes)", file_name, write1.len(), write2.len());
            unstable += 1;
        }
    }

    eprintln!("\nStable: {}, Unstable: {}", stable, unstable);
    assert_eq!(unstable, 0, "XMI write should produce stable output");
}

/// Test that XMI roundtrip converges (write→read→write produces same bytes)
#[cfg(feature = "interchange")]
#[test]
fn test_xmi_roundtrip_convergence() {
    let repo_path = match find_or_clone_repo() {
        Some(p) => p,
        None => {
            eprintln!("SysML v2 repository not found. Skipping convergence test.");
            return;
        }
    };

    // Collect files from all XMI subdirectories
    let mut files = Vec::new();
    for subdir in XMI_SUBDIRS {
        let xmi_path = repo_path.join(subdir);
        if xmi_path.exists() {
            files.extend(collect_xmi_files(&xmi_path));
        }
    }

    eprintln!("\nTesting XMI roundtrip convergence for {} files...\n", files.len());

    let xmi = Xmi::default();
    let mut converged = 0;
    let mut not_converged = 0;
    let mut size_diffs: Vec<(String, usize, usize)> = Vec::new();

    for file_path in &files {
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

        let original_bytes = match fs::read(file_path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // First pass: read original → write
        let model1 = match xmi.read(&original_bytes) {
            Ok(m) if m.element_count() > 0 => m,
            _ => continue,
        };
        let write1 = xmi.write(&model1).unwrap();

        // Second pass: read written → write again
        let model2 = match xmi.read(&write1) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  ✗ {}: Failed to re-read: {}", file_name, e);
                not_converged += 1;
                continue;
            }
        };
        let write2 = xmi.write(&model2).unwrap();

        // Check if output converged
        if write1 == write2 {
            converged += 1;
            eprintln!("  ✓ {} (converged at {} bytes, original {} bytes)", 
                file_name, write1.len(), original_bytes.len());
        } else {
            not_converged += 1;
            size_diffs.push((file_name.clone(), write1.len(), write2.len()));
            eprintln!("  ✗ {}: Did not converge ({} → {} bytes)", 
                file_name, write1.len(), write2.len());
        }
    }

    eprintln!("\n=== Convergence Results ===");
    eprintln!("Converged: {}", converged);
    eprintln!("Not converged: {}", not_converged);

    if !size_diffs.is_empty() {
        eprintln!("\nSize differences:");
        for (name, s1, s2) in &size_diffs {
            eprintln!("  {}: {} → {}", name, s1, s2);
        }
    }

    let rate = (converged as f64 / (converged + not_converged) as f64) * 100.0;
    eprintln!("\nConvergence rate: {:.1}%", rate);

    assert!(rate >= 95.0, "Convergence rate {:.1}% is below 95%", rate);
}

#[cfg(feature = "interchange")]
#[test]
fn test_official_xmi_to_jsonld_roundtrip() {
    use syster::interchange::JsonLd;

    let repo_path = match find_or_clone_repo() {
        Some(p) => p,
        None => {
            eprintln!("SysML v2 repository not found. Skipping JSON-LD roundtrip test.");
            return;
        }
    };

    // Collect files from all XMI subdirectories
    let mut files = Vec::new();
    for subdir in XMI_SUBDIRS {
        let xmi_path = repo_path.join(subdir);
        if xmi_path.exists() {
            files.extend(collect_xmi_files(&xmi_path));
        }
    }

    if files.is_empty() {
        eprintln!("No XMI files found. Skipping JSON-LD roundtrip test.");
        return;
    }

    eprintln!("\nTesting XMI → JSON-LD → XMI roundtrip for {} files...\n", files.len());

    let xmi = Xmi::default();
    let jsonld = JsonLd::default();
    let mut results = TestResults::default();

    for file_path in &files {
        let file_name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Read original XMI
        let original_bytes = match fs::read(file_path) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        // Parse XMI
        let model1 = match xmi.read(&original_bytes) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let element_count1 = model1.element_count();
        if element_count1 == 0 {
            continue;
        }

        // Convert to JSON-LD
        let json_bytes = match jsonld.write(&model1) {
            Ok(bytes) => bytes,
            Err(e) => {
                results.add_error(&file_name, format!("JSON-LD export error: {}", e));
                continue;
            }
        };

        // Parse JSON-LD back
        let model2 = match jsonld.read(&json_bytes) {
            Ok(m) => m,
            Err(e) => {
                results.add_error(&file_name, format!("JSON-LD parse error: {}", e));
                continue;
            }
        };

        let element_count2 = model2.element_count();

        // Compare counts
        if element_count1 == element_count2 {
            results.add_success(&file_name, element_count1);
        } else {
            results.add_partial(&file_name, element_count1, element_count2);
        }
    }

    results.print_summary();

    // JSON-LD roundtrip may lose some elements due to format differences
    // We expect at least 70% success
    let total = results.success_count + results.partial_count + results.error_count;
    if total > 0 {
        let success_rate = results.success_count as f64 / total as f64;
        assert!(
            success_rate >= 0.7,
            "JSON-LD success rate {:.1}% is below 70% threshold",
            success_rate * 100.0
        );
    }
}

#[derive(Default)]
struct TestResults {
    success_count: usize,
    partial_count: usize,
    warning_count: usize,
    error_count: usize,
    details: Vec<String>,
}

impl TestResults {
    fn add_success(&mut self, file: &str, count: usize) {
        self.success_count += 1;
        self.details
            .push(format!("✓ {} ({} elements)", file, count));
    }

    fn add_partial(&mut self, file: &str, before: usize, after: usize) {
        self.partial_count += 1;
        self.details
            .push(format!("◐ {} ({} → {} elements)", file, before, after));
    }

    fn add_warning(&mut self, file: &str, msg: String) {
        self.warning_count += 1;
        self.details.push(format!("⚠ {}: {}", file, msg));
    }

    fn add_error(&mut self, file: &str, msg: String) {
        self.error_count += 1;
        self.details.push(format!("✗ {}: {}", file, msg));
    }

    fn print_summary(&self) {
        eprintln!("\n=== Test Results ===");
        for detail in &self.details {
            eprintln!("{}", detail);
        }
        eprintln!("\n--- Summary ---");
        eprintln!("Success: {}", self.success_count);
        eprintln!("Partial: {}", self.partial_count);
        eprintln!("Warnings: {}", self.warning_count);
        eprintln!("Errors: {}", self.error_count);

        let total = self.success_count + self.partial_count + self.error_count;
        if total > 0 {
            let rate = (self.success_count as f64 / total as f64) * 100.0;
            eprintln!("Success Rate: {:.1}%", rate);
        }
    }
}

#[cfg(not(feature = "interchange"))]
#[test]
fn test_official_xmi_roundtrip() {
    eprintln!("Test skipped: requires 'interchange' feature");
}

#[cfg(not(feature = "interchange"))]
#[test]
fn test_official_xmi_to_jsonld_roundtrip() {
    eprintln!("Test skipped: requires 'interchange' feature");
}
