//! Detailed hover triage - shows WHAT is failing and WHY
//!
//! Run with: cargo run --example triage_hover_detailed

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syster::hir::{RefKind, TypeRefKind};
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn main() {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sysml-examples");

    let mut files: Vec<PathBuf> = Vec::new();
    collect_sysml_files(&examples_dir, &mut files);
    files.sort();

    println!("Found {} SysML example files\n", files.len());

    // Create analysis host with stdlib
    let mut host = AnalysisHost::new();
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library");
    let mut stdlib_loader = StdLibLoader::with_path(stdlib_path);
    stdlib_loader
        .ensure_loaded_into_host(&mut host)
        .expect("Failed to load stdlib");

    // Add all example files
    for path in &files {
        let content = fs::read_to_string(path).unwrap_or_default();
        let file_key = path.to_string_lossy().to_string();
        let _errors = host.set_file_content(&file_key, &content);
    }

    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Collect failures grouped by pattern
    let mut failures_by_pattern: HashMap<String, Vec<FailureInfo>> = HashMap::new();

    for path in &files {
        let file_key = path.to_string_lossy().to_string();
        let relative = path.strip_prefix(&examples_dir).unwrap_or(path);

        let file_id = match analysis.get_file_id(&file_key) {
            Some(id) => id,
            None => continue,
        };

        let content = fs::read_to_string(path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();

        // Collect all type refs
        for sym in index.symbols_in_file(file_id) {
            for trk in &sym.type_refs {
                let refs_to_check: Vec<FailureCandidate> = match trk {
                    TypeRefKind::Simple(tr) => {
                        vec![FailureCandidate {
                            target: tr.target.to_string(),
                            resolved: tr.resolved_target.as_ref().map(|s| s.to_string()),
                            kind: tr.kind,
                            line: tr.start_line,
                            start_col: tr.start_col,
                            end_col: tr.end_col,
                            chain_context: None,
                        }]
                    }
                    TypeRefKind::Chain(chain) => chain
                        .parts
                        .iter()
                        .enumerate()
                        .map(|(i, p)| FailureCandidate {
                            target: p.target.to_string(),
                            resolved: p.resolved_target.as_ref().map(|s| s.to_string()),
                            kind: p.kind,
                            line: p.start_line,
                            start_col: p.start_col,
                            end_col: p.end_col,
                            chain_context: if i > 0 {
                                Some(chain.parts[i - 1].target.to_string())
                            } else {
                                None
                            },
                        })
                        .collect(),
                };

                for candidate in refs_to_check {
                    let mid_col = (candidate.start_col + candidate.end_col) / 2;
                    let hover = analysis.hover(file_id, candidate.line, mid_col);

                    if hover.is_none() {
                        // Get source line for context
                        let source_line = lines
                            .get(candidate.line as usize)
                            .map(|s| s.to_string())
                            .unwrap_or_default();

                        let pattern = categorize_failure(&candidate, &source_line);

                        failures_by_pattern
                            .entry(pattern)
                            .or_default()
                            .push(FailureInfo {
                                file: relative.display().to_string(),
                                line: candidate.line,
                                target: candidate.target.clone(),
                                resolved: candidate.resolved.clone(),
                                kind: candidate.kind,
                                source_line: source_line.trim().to_string(),
                                chain_context: candidate.chain_context,
                                containing_symbol: sym.qualified_name.to_string(),
                            });
                    }
                }
            }
        }
    }

    // Print report
    println!("=== DETAILED FAILURE ANALYSIS ===\n");

    let mut patterns: Vec<_> = failures_by_pattern.iter().collect();
    patterns.sort_by_key(|(_, failures)| std::cmp::Reverse(failures.len()));

    for (pattern, failures) in &patterns {
        println!(
            "\n### {} ({} failures) ###\n",
            pattern,
            failures.len()
        );

        // Show first 5 examples
        for (i, failure) in failures.iter().take(5).enumerate() {
            println!("  Example {}:", i + 1);
            println!("    File: {}", failure.file);
            println!("    Line {}: {}", failure.line + 1, failure.source_line);
            println!(
                "    Target: '{}' (kind: {:?})",
                failure.target, failure.kind
            );
            if let Some(resolved) = &failure.resolved {
                println!("    Pre-resolved to: {}", resolved);
            } else {
                println!("    Pre-resolved: None");
            }
            if let Some(ctx) = &failure.chain_context {
                println!("    Chain after: '{}'", ctx);
            }
            println!("    Containing symbol: {}", failure.containing_symbol);
            println!();
        }

        if failures.len() > 5 {
            println!("  ... and {} more\n", failures.len() - 5);
        }
    }

    // Summary
    let total_failures: usize = failures_by_pattern.values().map(|v| v.len()).sum();
    println!("\n=== SUMMARY ===");
    println!("Total failures: {}", total_failures);
    println!("\nFailure counts by pattern:");
    for (pattern, failures) in patterns.iter().take(15) {
        let pct = 100.0 * failures.len() as f64 / total_failures as f64;
        println!("  {}: {} ({:.1}%)", pattern, failures.len(), pct);
    }
}

struct FailureCandidate {
    target: String,
    resolved: Option<String>,
    kind: RefKind,
    line: u32,
    start_col: u32,
    end_col: u32,
    chain_context: Option<String>,
}

struct FailureInfo {
    file: String,
    line: u32,
    target: String,
    resolved: Option<String>,
    kind: RefKind,
    source_line: String,
    chain_context: Option<String>,
    containing_symbol: String,
}

fn categorize_failure(candidate: &FailureCandidate, source_line: &str) -> String {
    let line_lower = source_line.to_lowercase();

    // Check if it's a chain member (has chain_context)
    if candidate.chain_context.is_some() {
        return format!("CHAIN_MEMBER ({:?})", candidate.kind);
    }

    // Check for specific patterns in source
    if line_lower.contains("calc ") || line_lower.contains("= ") && candidate.kind == RefKind::Expression {
        return "CALC_EXPRESSION".to_string();
    }

    if line_lower.contains("analysis ") || line_lower.contains("verify ") {
        return format!("ANALYSIS_CONTEXT ({:?})", candidate.kind);
    }

    if line_lower.contains("timeslice") || line_lower.contains("snapshot") {
        return format!("TIMESLICE_SNAPSHOT ({:?})", candidate.kind);
    }

    if line_lower.contains("constraint") || line_lower.contains("require ") {
        return format!("CONSTRAINT_CONTEXT ({:?})", candidate.kind);
    }

    if line_lower.contains(">>") || line_lower.contains(":>") {
        return format!("SHORTHAND_REDEFINE ({:?})", candidate.kind);
    }

    if line_lower.contains("::") {
        return format!("QUALIFIED_PATH ({:?})", candidate.kind);
    }

    if candidate.resolved.is_some() {
        return format!("RESOLVED_BUT_NO_HOVER ({:?})", candidate.kind);
    }

    format!("UNRESOLVED ({:?})", candidate.kind)
}

fn collect_sysml_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_sysml_files(&path, files);
            } else if path.extension().is_some_and(|e| e == "sysml") {
                files.push(path);
            }
        }
    }
}
