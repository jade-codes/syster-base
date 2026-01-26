//! HIR test helpers for setting up analysis hosts and symbol indexes.

use syster::base::FileId;
use syster::hir::HirSymbol;
use syster::ide::AnalysisHost;

/// Creates an AnalysisHost with a single SysML file.
pub fn analysis_from_sysml(source: &str) -> (AnalysisHost, FileId) {
    analysis_from_source(source, "test.sysml")
}

/// Creates an AnalysisHost with a single KerML file.
pub fn analysis_from_kerml(source: &str) -> (AnalysisHost, FileId) {
    analysis_from_source(source, "test.kerml")
}

/// Creates an AnalysisHost with a single file.
pub fn analysis_from_source(source: &str, filename: &str) -> (AnalysisHost, FileId) {
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content(filename, source);
    assert!(
        errors.is_empty(),
        "Parse errors in '{}': {:?}",
        filename,
        errors
    );

    let file_id = {
        let analysis = host.analysis();
        analysis
            .get_file_id(filename)
            .expect("File should be in index after set_file_content")
    };

    (host, file_id)
}

/// Creates an AnalysisHost with multiple files.
pub fn analysis_from_sources(files: &[(&str, &str)]) -> AnalysisHost {
    let mut host = AnalysisHost::new();
    for (path, content) in files {
        let errors = host.set_file_content(path, content);
        assert!(
            errors.is_empty(),
            "Parse errors in '{}': {:?}",
            path,
            errors
        );
    }
    let _ = host.analysis();
    host
}

/// Get all symbols extracted from a single source file.
pub fn symbols_from_sysml(source: &str) -> Vec<HirSymbol> {
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    analysis
        .symbol_index()
        .symbols_in_file(file_id)
        .into_iter()
        .cloned()
        .collect()
}

/// Get all symbols with a specific simple name.
pub fn symbols_named<'a>(
    index: &'a syster::hir::SymbolIndex,
    name: &str,
) -> Vec<&'a syster::hir::HirSymbol> {
    index
        .all_symbols()
        .filter(|s| s.name.as_ref() == name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_from_sysml_works() {
        let (mut host, file_id) = analysis_from_sysml("part def Vehicle;");
        let analysis = host.analysis();
        assert!(!analysis.symbol_index().symbols_in_file(file_id).is_empty());
    }

    #[test]
    fn test_symbols_from_sysml_extracts_symbols() {
        let symbols = symbols_from_sysml("part def Car;");
        assert!(!symbols.is_empty());
        assert!(symbols.iter().any(|s| s.name.as_ref() == "Car"));
    }

    #[test]
    fn test_analysis_from_sources_multiple_files() {
        let mut host =
            analysis_from_sources(&[("a.sysml", "part def A;"), ("b.sysml", "part def B;")]);
        let analysis = host.analysis();
        assert!(analysis.symbol_index().lookup_qualified("A").is_some());
        assert!(analysis.symbol_index().lookup_qualified("B").is_some());
    }
}
