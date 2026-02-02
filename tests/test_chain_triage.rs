//! Detailed triage of chain-related hover failures

use std::collections::HashMap;
use std::path::PathBuf;
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn stdlib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library")
}

fn create_host_with_stdlib() -> AnalysisHost {
    let mut host = AnalysisHost::new();
    let stdlib = stdlib_path();
    if stdlib.exists() {
        let mut stdlib_loader = StdLibLoader::with_path(stdlib);
        let _ = stdlib_loader.ensure_loaded_into_host(&mut host);
    }
    host
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Extract chain expressions like "a.b.c" from a line
fn find_chains(line: &str) -> Vec<(usize, Vec<(usize, String)>)> {
    let chars: Vec<char> = line.chars().collect();
    let mut chains = Vec::new();
    let mut i = 0;
    
    while i < chars.len() {
        // Find start of identifier
        if is_ident_char(chars[i]) && (i == 0 || !is_ident_char(chars[i-1])) {
            let start = i;
            let mut parts = Vec::new();
            
            // Collect first identifier
            let mut ident_start = i;
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            let first_ident: String = chars[ident_start..i].iter().collect();
            parts.push((ident_start, first_ident));
            
            // Check for chain continuation
            while i < chars.len() && chars[i] == '.' {
                i += 1; // skip dot
                if i < chars.len() && is_ident_char(chars[i]) {
                    ident_start = i;
                    while i < chars.len() && is_ident_char(chars[i]) {
                        i += 1;
                    }
                    let ident: String = chars[ident_start..i].iter().collect();
                    parts.push((ident_start, ident));
                } else {
                    break;
                }
            }
            
            if parts.len() > 1 {
                chains.push((start, parts));
            }
        } else {
            i += 1;
        }
    }
    chains
}

#[test]
fn triage_chain_failures() {
    let example_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../language-server/crates/syster-lsp/tests/sysml-examples/SimpleVehicleModel.sysml");
    
    let source = std::fs::read_to_string(&example_path).expect("Failed to read example file");
    
    let mut host = create_host_with_stdlib();
    host.set_file_content("test.sysml", &source);
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    
    let lines: Vec<&str> = source.lines().collect();
    
    println!("\n=== CHAIN FAILURE ANALYSIS ===\n");
    
    let mut chain_first_failures: Vec<(u32, String, String)> = Vec::new();
    let mut chain_member_failures: Vec<(u32, String, usize, String)> = Vec::new(); // line, ident, depth, context
    
    for (line_idx, line_text) in lines.iter().enumerate() {
        let line = line_idx as u32;
        
        for (_chain_start, parts) in find_chains(line_text) {
            for (idx, (col, ident)) in parts.iter().enumerate() {
                let hover_result = analysis.hover(file_id, line, *col as u32);
                
                if hover_result.is_none() {
                    let context = if line_text.len() > 90 {
                        format!("{}...", &line_text.trim()[..90.min(line_text.trim().len())])
                    } else {
                        line_text.trim().to_string()
                    };
                    
                    if idx == 0 {
                        chain_first_failures.push((line + 1, ident.clone(), context));
                    } else {
                        chain_member_failures.push((line + 1, ident.clone(), idx + 1, context));
                    }
                }
            }
        }
    }
    
    // Categorize CHAIN_FIRST failures
    println!("=== CHAIN_FIRST FAILURES ({}) ===\n", chain_first_failures.len());
    
    let mut first_by_context: HashMap<&str, Vec<(u32, String)>> = HashMap::new();
    for (line, ident, ctx) in &chain_first_failures {
        let pattern = if ctx.contains("redefines") && ctx.contains("subject") {
            "subject redefines"
        } else if ctx.contains("redefines") {
            "redefines expr"
        } else if ctx.contains("#") {
            "metadata"
        } else if ctx.contains("bind") {
            "bind statement"
        } else if ctx.contains("flow") {
            "flow endpoint"
        } else if ctx.contains("connect") {
            "connect endpoint"
        } else {
            "other"
        };
        first_by_context.entry(pattern).or_default().push((*line, ident.clone()));
    }
    
    for (pattern, failures) in first_by_context.iter() {
        println!("{} ({}):", pattern, failures.len());
        for (line, ident) in failures.iter().take(5) {
            println!("  Line {}: '{}'", line, ident);
        }
        if failures.len() > 5 {
            println!("  ... and {} more", failures.len() - 5);
        }
        println!();
    }
    
    // Categorize CHAIN_MEMBER failures by depth
    println!("\n=== CHAIN_MEMBER FAILURES ({}) ===\n", chain_member_failures.len());
    
    let mut member_by_depth: HashMap<usize, Vec<(u32, String, String)>> = HashMap::new();
    for (line, ident, depth, ctx) in &chain_member_failures {
        member_by_depth.entry(*depth).or_default().push((*line, ident.clone(), ctx.clone()));
    }
    
    for depth in 2..=4 {
        if let Some(failures) = member_by_depth.get(&depth) {
            println!("Depth {} - part #{} of chain ({}):", depth, depth, failures.len());
            for (line, ident, ctx) in failures.iter().take(5) {
                println!("  Line {}: '{}' in: {}", line, ident, 
                    if ctx.len() > 65 { format!("{}...", &ctx[..65]) } else { ctx.clone() });
            }
            if failures.len() > 5 {
                println!("  ... and {} more", failures.len() - 5);
            }
            println!();
        }
    }
    
    // Also categorize by statement type
    println!("\n=== CHAIN_MEMBER by Statement Type ===\n");
    
    let mut member_by_stmt: HashMap<&str, Vec<(u32, String, usize)>> = HashMap::new();
    for (line, ident, depth, ctx) in &chain_member_failures {
        let stmt = if ctx.contains("bind") {
            "bind"
        } else if ctx.contains("flow") {
            "flow"
        } else if ctx.contains("connect") {
            "connect"
        } else if ctx.contains("message") {
            "message"
        } else if ctx.contains("first") || ctx.contains("then") {
            "succession"
        } else if ctx.contains("redefines") && ctx.contains("=") {
            "redefines with expr"
        } else if ctx.contains("redefines") {
            "redefines"
        } else {
            "other"
        };
        member_by_stmt.entry(stmt).or_default().push((*line, ident.clone(), *depth));
    }
    
    for (stmt, failures) in member_by_stmt.iter() {
        println!("{} ({}):", stmt, failures.len());
        for (line, ident, depth) in failures.iter().take(3) {
            println!("  Line {}: '{}' (depth {})", line, ident, depth);
        }
        if failures.len() > 3 {
            println!("  ... and {} more", failures.len() - 3);
        }
        println!();
    }
    
    // Debug specific examples
    println!("\n=== SPECIFIC EXAMPLES ===\n");
    
    // Find a 3-level chain failure
    for (line, ident, depth, ctx) in chain_member_failures.iter() {
        if *depth == 3 {
            println!("3-level chain failure - Line {}: '{}' (depth {})", line, ident, depth);
            println!("  Context: {}", ctx);
            
            let line_idx = (*line - 1) as u32;
            let line_text = lines[line_idx as usize];
            println!("  Hover scan:");
            let mut last_qn: Option<String> = None;
            for col in 0..line_text.len().min(100) {
                if let Some(hover) = analysis.hover(file_id, line_idx, col as u32) {
                    let qn = hover.qualified_name.as_ref().map(|s| s.to_string());
                    if qn != last_qn {
                        println!("    col {}: {}", col, qn.as_deref().unwrap_or("(none)"));
                        last_qn = qn;
                    }
                }
            }
            break;
        }
    }
}
