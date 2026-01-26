//! Diagnostic assertion helpers for HIR tests.

use syster::hir::{Diagnostic, Severity, check_file};

use crate::helpers::hir_helpers::analysis_from_sysml;

/// Get all diagnostics for a SysML source string.
pub fn diagnostics_from_sysml(source: &str) -> Vec<Diagnostic> {
    let (mut host, file_id) = analysis_from_sysml(source);
    let analysis = host.analysis();
    check_file(analysis.symbol_index(), file_id)
}

/// Get only error-level diagnostics.
pub fn errors_from_sysml(source: &str) -> Vec<Diagnostic> {
    diagnostics_from_sysml(source)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

/// Assert a source has no errors.
pub fn assert_no_errors(source: &str) {
    let errors = errors_from_sysml(source);
    assert!(
        errors.is_empty(),
        "Expected no errors, got {} error(s):\n{}",
        errors.len(),
        errors
            .iter()
            .map(|e| format!("  Line {}: {}", e.start_line + 1, e.message))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_no_errors_passes_for_valid_source() {
        assert_no_errors("part def Vehicle;");
    }

    #[test]
    fn test_diagnostics_from_sysml_returns_list() {
        let diagnostics = diagnostics_from_sysml("part def Car;");
        let _ = diagnostics;
    }
}
