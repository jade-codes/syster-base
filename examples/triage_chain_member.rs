//! Detailed chain member resolution triage
//!
//! Run with: cargo run --example triage_chain_member

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

    // Collect chain member failures with detailed info
    let mut failures: Vec<ChainFailure> = Vec::new();

    for path in &files {
        let file_key = path.to_string_lossy().to_string();
        let relative = path.strip_prefix(&examples_dir).unwrap_or(path);

        let file_id = match analysis.get_file_id(&file_key) {
            Some(id) => id,
            None => continue,
        };

        let content = fs::read_to_string(path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();

        for sym in index.symbols_in_file(file_id) {
            for trk in &sym.type_refs {
                if let TypeRefKind::Chain(chain) = trk {
                    // Check each part of the chain
                    for (i, part) in chain.parts.iter().enumerate() {
                        if i == 0 {
                            continue; // Skip first part, focus on chain members
                        }

                        let mid_col = (part.start_col + part.end_col) / 2;
                        let hover = analysis.hover(file_id, part.start_line, mid_col);

                        if hover.is_none() && part.kind == RefKind::Expression {
                            let prev_part = &chain.parts[i - 1];
                            
                            // Get the first part's symbol info
                            let first_part = &chain.parts[0];
                            let first_resolved = first_part.resolved_target.as_ref().map(|s| s.to_string());
                            
                            // Try to find the first part's type
                            let first_type = if let Some(ref resolved) = first_resolved {
                                if let Some(first_sym) = index.lookup_qualified(resolved) {
                                    // Check supertypes
                                    if let Some(st) = first_sym.supertypes.first() {
                                        Some(format!("supertype: {}", st))
                                    } else {
                                        // Check type_refs for TypedBy
                                        let typed_by = first_sym.type_refs.iter()
                                            .filter_map(|tr| tr.as_refs().into_iter().next())
                                            .find(|tr| matches!(tr.kind, RefKind::TypedBy))
                                            .and_then(|tr| tr.resolved_target.as_ref().or(Some(&tr.target)))
                                            .map(|s| format!("typed_by: {}", s));
                                        typed_by.or_else(|| Some("no type info".to_string()))
                                    }
                                } else {
                                    Some("symbol not found".to_string())
                                }
                            } else {
                                Some("first not resolved".to_string())
                            };

                            let source_line = lines
                                .get(part.start_line as usize)
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            failures.push(ChainFailure {
                                file: relative.display().to_string(),
                                line: part.start_line,
                                source_line: source_line.trim().to_string(),
                                chain_length: chain.parts.len(),
                                position_in_chain: i,
                                target: part.target.to_string(),
                                prev_part: prev_part.target.to_string(),
                                prev_resolved: prev_part.resolved_target.as_ref().map(|s| s.to_string()),
                                first_part: first_part.target.to_string(),
                                first_resolved,
                                first_type,
                                containing_symbol: sym.qualified_name.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    println!("=== CHAIN MEMBER RESOLUTION FAILURES ===\n");
    println!("Total failures: {}\n", failures.len());

    // Group by first_type (the reason resolution fails)
    let mut by_type_issue: HashMap<String, Vec<&ChainFailure>> = HashMap::new();
    for f in &failures {
        let key = f.first_type.clone().unwrap_or_else(|| "unknown".to_string());
        by_type_issue.entry(key).or_default().push(f);
    }

    let mut type_issues: Vec<_> = by_type_issue.iter().collect();
    type_issues.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    println!("=== GROUPED BY TYPE LOOKUP ISSUE ===\n");
    for (issue, failures) in &type_issues {
        println!("\n### {} ({} failures) ###\n", issue, failures.len());
        
        for (i, f) in failures.iter().take(3).enumerate() {
            println!("  Example {}:", i + 1);
            println!("    File: {}", f.file);
            println!("    Line {}: {}", f.line + 1, f.source_line);
            println!("    Chain: {} parts, failing at position {}", f.chain_length, f.position_in_chain);
            println!("    First part: '{}' -> {:?}", f.first_part, f.first_resolved);
            println!("    Prev part: '{}' -> {:?}", f.prev_part, f.prev_resolved);
            println!("    Target: '{}'", f.target);
            println!();
        }
        if failures.len() > 3 {
            println!("  ... and {} more\n", failures.len() - 3);
        }
    }

    // Detailed look at specific patterns
    println!("\n=== DETAILED CHAIN ANALYSIS ===\n");
    
    // Find unique chain patterns (first.second)
    let mut patterns: HashMap<String, Vec<&ChainFailure>> = HashMap::new();
    for f in &failures {
        let pattern = format!("{}.{}", f.prev_part, f.target);
        patterns.entry(pattern).or_default().push(f);
    }

    let mut patterns_vec: Vec<_> = patterns.iter().collect();
    patterns_vec.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    println!("Top 20 failing chain patterns (prev.target):\n");
    for (pattern, failures) in patterns_vec.iter().take(20) {
        let f = failures[0];
        println!("  {} ({} occurrences)", pattern, failures.len());
        println!("    prev_resolved: {:?}", f.prev_resolved);
        println!("    first_type: {:?}", f.first_type);
        println!();
    }

    // Show some specific debugging for the most common case
    println!("\n=== DEBUGGING MOST COMMON PATTERN ===\n");
    if let Some((pattern, failures)) = patterns_vec.first() {
        let f = failures[0];
        println!("Pattern: {}", pattern);
        println!("File: {}", f.file);
        println!("Line: {}", f.line + 1);
        println!("Source: {}", f.source_line);
        println!();
        
        // Try to manually trace the chain
        println!("Chain trace:");
        println!("  1. First part: '{}'", f.first_part);
        if let Some(ref resolved) = f.first_resolved {
            println!("     Resolved to: {}", resolved);
            if let Some(sym) = index.lookup_qualified(resolved) {
                println!("     Symbol found!");
                println!("     Kind: {:?}", sym.kind);
                println!("     Supertypes: {:?}", sym.supertypes);
                println!("     Type refs:");
                for tr in &sym.type_refs {
                    println!("       {:?}", tr);
                }
            } else {
                println!("     Symbol NOT found in index!");
            }
        } else {
            println!("     NOT resolved");
        }
        
        println!("\n  2. Prev part (position {}): '{}'", f.position_in_chain - 1, f.prev_part);
        if let Some(ref resolved) = f.prev_resolved {
            println!("     Resolved to: {}", resolved);
            if let Some(sym) = index.lookup_qualified(resolved) {
                println!("     Symbol found!");
                println!("     Kind: {:?}", sym.kind);
                println!("     Supertypes: {:?}", sym.supertypes);
            }
        } else {
            println!("     NOT resolved");
        }
        
        println!("\n  3. Target (position {}): '{}'", f.position_in_chain, f.target);
        println!("     This is what fails to hover");
    }
}

struct ChainFailure {
    file: String,
    line: u32,
    source_line: String,
    chain_length: usize,
    position_in_chain: usize,
    target: String,
    prev_part: String,
    prev_resolved: Option<String>,
    first_part: String,
    first_resolved: Option<String>,
    first_type: Option<String>,
    containing_symbol: String,
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
