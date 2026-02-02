use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syster::hir::{RefKind, TypeRefKind};
use syster::ide::AnalysisHost;
use syster::project::StdLibLoader;

fn main() {
    let mut host = AnalysisHost::new();

    // Load stdlib
    let stdlib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml.library");
    let mut stdlib_loader = StdLibLoader::with_path(stdlib_path);
    stdlib_loader
        .ensure_loaded_into_host(&mut host)
        .expect("Failed to load stdlib");

    // Scan all .sysml files in the examples folder
    let examples_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sysml-examples");
    let mut sysml_files: Vec<PathBuf> = Vec::new();
    collect_sysml_files(&examples_path, &mut sysml_files);

    println!("Found {} SysML example files", sysml_files.len());

    // Load all files
    for path in &sysml_files {
        let relative = path.strip_prefix(&examples_path).unwrap();
        let file_key = format!("examples/{}", relative.display());
        let content = fs::read_to_string(path).expect("Failed to read file");
        host.set_file_content(&file_key, &content);
    }

    let analysis = host.analysis();
    let index = analysis.symbol_index();

    // Collect stats per file
    let mut total_refs = 0usize;
    let mut total_hover = 0usize;
    let mut total_no_hover = 0usize;

    // Track no-hover by kind
    let mut no_hover_by_kind: HashMap<RefKind, usize> = HashMap::new();

    // Track which files have issues
    let mut files_with_issues: Vec<(String, usize, usize)> = Vec::new();

    for path in &sysml_files {
        let relative = path.strip_prefix(&examples_path).unwrap();
        let file_key = format!("examples/{}", relative.display());
        let file_id = match analysis.get_file_id(&file_key) {
            Some(id) => id,
            None => continue,
        };

        let _content = fs::read_to_string(path).unwrap();

        let mut file_refs = 0usize;
        let mut file_hover = 0usize;
        let mut file_no_hover = 0usize;

        // Collect all type refs
        for sym in index.symbols_in_file(file_id) {
            for trk in &sym.type_refs {
                let refs_to_check: Vec<(u32, u32, u32, RefKind)> = match trk {
                    TypeRefKind::Simple(tr) => {
                        vec![(tr.start_line, tr.start_col, tr.end_col, tr.kind)]
                    }
                    TypeRefKind::Chain(chain) => chain
                        .parts
                        .iter()
                        .map(|p| (p.start_line, p.start_col, p.end_col, p.kind))
                        .collect(),
                };

                for (line, start_col, end_col, kind) in refs_to_check {
                    file_refs += 1;
                    let mid_col = (start_col + end_col) / 2;
                    let hover = analysis.hover(file_id, line, mid_col);

                    if hover.is_some() {
                        file_hover += 1;
                    } else {
                        file_no_hover += 1;
                        *no_hover_by_kind.entry(kind).or_insert(0) += 1;
                    }
                }
            }
        }

        total_refs += file_refs;
        total_hover += file_hover;
        total_no_hover += file_no_hover;

        if file_no_hover > 0 {
            files_with_issues.push((relative.display().to_string(), file_refs, file_no_hover));
        }
    }

    println!("\n=== TRIAGE REPORT (ALL EXAMPLES) ===\n");
    println!("Total type refs: {}", total_refs);
    println!(
        "Resolved with hover: {} ({:.1}%)",
        total_hover,
        100.0 * total_hover as f64 / total_refs as f64
    );
    println!(
        "No hover: {} ({:.1}%)",
        total_no_hover,
        100.0 * total_no_hover as f64 / total_refs as f64
    );

    println!("\n=== NO HOVER BY KIND ===");
    let mut kinds: Vec<_> = no_hover_by_kind.iter().collect();
    kinds.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (kind, count) in kinds {
        println!("{:?}: {}", kind, count);
    }

    println!("\n=== FILES WITH ISSUES (sorted by no-hover count) ===");
    files_with_issues.sort_by_key(|(_, _, no_hover)| std::cmp::Reverse(*no_hover));
    for (file, refs, no_hover) in files_with_issues.iter().take(20) {
        let pct = 100.0 * *no_hover as f64 / *refs as f64;
        println!(
            "{}: {} refs, {} no-hover ({:.1}%)",
            file, refs, no_hover, pct
        );
    }
    if files_with_issues.len() > 20 {
        println!("... and {} more files", files_with_issues.len() - 20);
    }
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
