//! Quick hover triage for SimpleVehicleModel.sysml
//!
//! Run with: cargo test --test test_quick_hover_triage -- --nocapture

use std::collections::HashMap;
use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::parser::{SyntaxKind, SyntaxNode, parse_sysml};
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        if let Err(e) = stdlib_loader.ensure_loaded_into_host(&mut host) {
            eprintln!("Warning: Failed to load stdlib: {}", e);
        }
    } else {
        eprintln!("Warning: stdlib not found at {:?}", stdlib);
    }
    host
}

fn main() {
    run_hover_triage();
}

#[test]
fn test_run_hover_triage() {
    run_hover_triage();
}

fn run_hover_triage() {
    let file_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "tests/sysml-examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml",
    );

    if !file_path.exists() {
        eprintln!("File not found: {:?}", file_path);
        return;
    }

    let content = std::fs::read_to_string(&file_path).expect("Failed to read file");
    let path_str = file_path.to_string_lossy().to_string();

    // Create host WITH standard library
    let mut host = create_host_with_stdlib();
    let _parse_errors = host.set_file_content(&path_str, &content);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id(&path_str).expect("File not in index");

    // Count hover successes and failures by line context
    let mut total_refs = 0;
    let mut hover_success = 0;
    let mut hover_failures: HashMap<String, Vec<(u32, u32, String)>> = HashMap::new();

    // Parse to get all identifiers that should be hoverable
    let parsed = parse_sysml(&content);
    let root = parsed.syntax();

    // Walk through all tokens looking for IDENTs
    fn collect_idents(node: &SyntaxNode, idents: &mut Vec<(u32, u32, String)>, content: &str) {
        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Token(token) => {
                    if token.kind() == SyntaxKind::IDENT {
                        let range = token.text_range();
                        let start = range.start().into();
                        // Convert byte offset to line/col
                        let (line, col) = offset_to_line_col(content, start);
                        idents.push((line, col, token.text().to_string()));
                    }
                }
                rowan::NodeOrToken::Node(n) => {
                    collect_idents(&n, idents, content);
                }
            }
        }
    }

    let mut idents = Vec::new();
    collect_idents(&root, &mut idents, &content);

    println!("=== Hover Triage for SimpleVehicleModel.sysml ===\n");
    println!("Total identifiers found: {}", idents.len());

    // Test hover on each identifier
    for (line, col, text) in &idents {
        total_refs += 1;
        let hover = analysis.hover(file_id, *line, *col);

        if hover.is_some() {
            hover_success += 1;
        } else {
            // Get surrounding context
            let lines: Vec<&str> = content.lines().collect();
            let context = if (*line as usize) < lines.len() {
                lines[*line as usize].trim().to_string()
            } else {
                "".to_string()
            };

            // Categorize the failure
            let category = categorize_failure(&context, text);
            hover_failures
                .entry(category)
                .or_default()
                .push((*line, *col, text.clone()));
        }
    }

    println!(
        "\nHover success: {}/{} ({:.1}%)",
        hover_success,
        total_refs,
        (hover_success as f64 / total_refs as f64) * 100.0
    );
    println!(
        "Hover failures: {} ({:.1}%)",
        total_refs - hover_success,
        ((total_refs - hover_success) as f64 / total_refs as f64) * 100.0
    );

    println!("\n=== Failure Categories ===");
    let mut categories: Vec<_> = hover_failures.iter().collect();
    categories.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (category, failures) in categories {
        println!("\n{} ({} failures):", category, failures.len());
        // Show first 3 examples
        for (line, _col, text) in failures.iter().take(3) {
            let lines: Vec<&str> = content.lines().collect();
            let context = if (*line as usize) < lines.len() {
                lines[*line as usize].trim()
            } else {
                ""
            };
            println!(
                "  Line {}: '{}' in: {}",
                line + 1,
                text,
                truncate(context, 70)
            );
        }
        if failures.len() > 3 {
            println!("  ... and {} more", failures.len() - 3);
        }
    }
}

fn offset_to_line_col(content: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut line_start = 0usize;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1; // Next byte after newline
        }
    }

    // Column is byte offset from line start
    let col = (offset - line_start) as u32;
    (line, col)
}

fn categorize_failure(context: &str, ident: &str) -> String {
    // Check for common patterns
    if context.contains("forAll") || context.contains("filter") || context.contains("collect") {
        return "EXPRESSION_LAMBDA".to_string();
    }
    if context.contains(".") && context.contains(ident) {
        // Check if it's a chain member
        let parts: Vec<&str> = context.split('.').collect();
        if parts.len() > 1 {
            // Is this ident the first or later part?
            if parts[0].contains(ident) {
                return "CHAIN_FIRST".to_string();
            } else {
                return "CHAIN_MEMBER".to_string();
            }
        }
    }
    if context.contains("calc") || context.contains("constraint") {
        return "CALC_EXPRESSION".to_string();
    }
    if context.contains(":>>") || context.contains("redefines") {
        return "REDEFINES".to_string();
    }
    if context.contains(":>") || context.contains("specializes") || context.contains("subsets") {
        return "SPECIALIZATION".to_string();
    }
    if context.contains(":") && !context.contains("::") {
        return "TYPING".to_string();
    }
    if context.contains("then") || context.contains("first") {
        return "TRANSITION".to_string();
    }
    if context.contains("accept") || context.contains("send") {
        return "MESSAGE".to_string();
    }
    if context.contains("bind") || context.contains("connect") {
        return "BINDING".to_string();
    }
    if context.starts_with("part ")
        || context.starts_with("port ")
        || context.starts_with("attribute ")
        || context.starts_with("action ")
    {
        return "DEFINITION".to_string();
    }

    "OTHER".to_string()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
