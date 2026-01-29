//! Diagnostics — Semantic error reporting.
//!
//! This module provides diagnostic types for semantic analysis errors
//! and warnings. It integrates with the symbol index and resolver.

use std::sync::Arc;

use super::resolve::{ResolveResult, Resolver, SymbolIndex};
use super::symbols::{HirSymbol, SymbolKind};
use crate::base::FileId;

// ============================================================================
// DIAGNOSTIC TYPES
// ============================================================================

/// Severity level of a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Severity {
    /// Convert to LSP severity number.
    pub fn to_lsp(&self) -> u32 {
        match self {
            Severity::Error => 1,
            Severity::Warning => 2,
            Severity::Info => 3,
            Severity::Hint => 4,
        }
    }
}

/// A diagnostic message with location.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// The file containing this diagnostic.
    pub file: FileId,
    /// Start line (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed).
    pub end_col: u32,
    /// Severity level.
    pub severity: Severity,
    /// Error/warning code (e.g., "E0001").
    pub code: Option<Arc<str>>,
    /// The diagnostic message.
    pub message: Arc<str>,
    /// Optional related information.
    pub related: Vec<RelatedInfo>,
}

/// Related information for a diagnostic.
#[derive(Clone, Debug)]
pub struct RelatedInfo {
    /// The file containing this info.
    pub file: FileId,
    /// Line number.
    pub line: u32,
    /// Column number.
    pub col: u32,
    /// The message.
    pub message: Arc<str>,
}

impl Diagnostic {
    /// Create a new error diagnostic.
    pub fn error(file: FileId, line: u32, col: u32, message: impl Into<Arc<str>>) -> Self {
        Self {
            file,
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
            severity: Severity::Error,
            code: None,
            message: message.into(),
            related: Vec::new(),
        }
    }

    /// Create a new warning diagnostic.
    pub fn warning(file: FileId, line: u32, col: u32, message: impl Into<Arc<str>>) -> Self {
        Self {
            file,
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
            severity: Severity::Warning,
            code: None,
            message: message.into(),
            related: Vec::new(),
        }
    }

    /// Set the span (range) for this diagnostic.
    pub fn with_span(mut self, end_line: u32, end_col: u32) -> Self {
        self.end_line = end_line;
        self.end_col = end_col;
        self
    }

    /// Set the error code.
    pub fn with_code(mut self, code: impl Into<Arc<str>>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Add related information.
    pub fn with_related(mut self, info: RelatedInfo) -> Self {
        self.related.push(info);
        self
    }
}

// ============================================================================
// DIAGNOSTIC CODES
// ============================================================================

/// Standard diagnostic codes for semantic errors.
///
/// ## Error Code Ranges
///
/// - **E0001-E0099**: Semantic analysis errors (symbol resolution, type checking, validation)
/// - **W0001-W0099**: Warnings (unused, deprecated, conventions)
#[allow(dead_code)]
pub mod codes {
    // ========================================================================
    // SEMANTIC ERRORS (E0001-E0099)
    // ========================================================================

    /// Undefined reference (name not found).
    pub const UNDEFINED_REFERENCE: &str = "E0001";
    /// Ambiguous reference (multiple candidates).
    pub const AMBIGUOUS_REFERENCE: &str = "E0002";
    /// Type mismatch.
    pub const TYPE_MISMATCH: &str = "E0003";
    /// Duplicate definition.
    pub const DUPLICATE_DEFINITION: &str = "E0004";
    /// Missing required element.
    pub const MISSING_REQUIRED: &str = "E0005";
    /// Invalid specialization relationship.
    pub const INVALID_SPECIALIZATION: &str = "E0006";
    /// Circular dependency detected.
    pub const CIRCULAR_DEPENDENCY: &str = "E0007";
    /// Invalid type.
    pub const INVALID_TYPE: &str = "E0008";
    /// Invalid redefinition.
    pub const INVALID_REDEFINITION: &str = "E0009";
    /// Invalid subsetting relationship.
    pub const INVALID_SUBSETTING: &str = "E0010";
    /// Constraint violation.
    pub const CONSTRAINT_VIOLATION: &str = "E0011";
    /// Feature used in invalid context.
    pub const INVALID_FEATURE_CONTEXT: &str = "E0012";
    /// Cannot instantiate abstract element.
    pub const ABSTRACT_INSTANTIATION: &str = "E0013";
    /// Invalid import statement.
    pub const INVALID_IMPORT: &str = "E0014";

    // ========================================================================
    // WARNINGS (W0001-W0099)
    // ========================================================================

    /// Unused symbol.
    pub const UNUSED_SYMBOL: &str = "W0001";
    /// Deprecated usage.
    pub const DEPRECATED: &str = "W0002";
    /// Naming convention violation.
    pub const NAMING_CONVENTION: &str = "W0003";
}

// ============================================================================
// DIAGNOSTIC COLLECTOR
// ============================================================================

/// Collects diagnostics during semantic analysis.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticCollector {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic.
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add an undefined reference error.
    pub fn undefined_reference(&mut self, file: FileId, symbol: &HirSymbol, name: &str) {
        self.add(
            Diagnostic::error(
                file,
                symbol.start_line,
                symbol.start_col,
                format!("undefined reference: '{}'", name),
            )
            .with_span(symbol.end_line, symbol.end_col)
            .with_code(codes::UNDEFINED_REFERENCE),
        );
    }

    /// Add an ambiguous reference error.
    pub fn ambiguous_reference(
        &mut self,
        file: FileId,
        symbol: &HirSymbol,
        name: &str,
        candidates: &[HirSymbol],
    ) {
        let candidate_names: Vec<_> = candidates
            .iter()
            .map(|c| c.qualified_name.as_ref())
            .collect();
        let mut diag = Diagnostic::error(
            file,
            symbol.start_line,
            symbol.start_col,
            format!(
                "ambiguous reference: '{}' could be: {}",
                name,
                candidate_names.join(", ")
            ),
        )
        .with_span(symbol.end_line, symbol.end_col)
        .with_code(codes::AMBIGUOUS_REFERENCE);

        // Add related info for each candidate
        for candidate in candidates {
            diag = diag.with_related(RelatedInfo {
                file: candidate.file,
                line: candidate.start_line,
                col: candidate.start_col,
                message: Arc::from(format!("candidate: {}", candidate.qualified_name)),
            });
        }

        self.add(diag);
    }

    /// Add a duplicate definition error.
    pub fn duplicate_definition(&mut self, file: FileId, symbol: &HirSymbol, existing: &HirSymbol) {
        self.add(
            Diagnostic::error(
                file,
                symbol.start_line,
                symbol.start_col,
                format!("duplicate definition: '{}' is already defined", symbol.name),
            )
            .with_span(symbol.end_line, symbol.end_col)
            .with_code(codes::DUPLICATE_DEFINITION)
            .with_related(RelatedInfo {
                file: existing.file,
                line: existing.start_line,
                col: existing.start_col,
                message: Arc::from(format!("previous definition of '{}'", existing.name)),
            }),
        );
    }

    /// Add a type mismatch error.
    pub fn type_mismatch(&mut self, file: FileId, symbol: &HirSymbol, expected: &str, found: &str) {
        self.add(
            Diagnostic::error(
                file,
                symbol.start_line,
                symbol.start_col,
                format!("type mismatch: expected '{}', found '{}'", expected, found),
            )
            .with_span(symbol.end_line, symbol.end_col)
            .with_code(codes::TYPE_MISMATCH),
        );
    }

    /// Add an unused symbol warning.
    pub fn unused_symbol(&mut self, symbol: &HirSymbol) {
        self.add(
            Diagnostic::warning(
                symbol.file,
                symbol.start_line,
                symbol.start_col,
                format!("unused {}: '{}'", symbol.kind.display(), symbol.name),
            )
            .with_span(symbol.end_line, symbol.end_col)
            .with_code(codes::UNUSED_SYMBOL),
        );
    }

    /// Get all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Get diagnostics for a specific file.
    pub fn diagnostics_for_file(&self, file: FileId) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.file == file).collect()
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Get the number of warnings.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Take all diagnostics, leaving the collector empty.
    pub fn take(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Clear all diagnostics.
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }
}

// ============================================================================
// SEMANTIC CHECKER
// ============================================================================

/// Performs semantic checks on symbols using the resolver.
pub struct SemanticChecker<'a> {
    index: &'a SymbolIndex,
    collector: DiagnosticCollector,
    /// Track which symbols are referenced (for unused detection).
    referenced: std::collections::HashSet<Arc<str>>,
}

impl<'a> SemanticChecker<'a> {
    /// Create a new semantic checker.
    pub fn new(index: &'a SymbolIndex) -> Self {
        Self {
            index,
            collector: DiagnosticCollector::new(),
            referenced: std::collections::HashSet::new(),
        }
    }

    /// Check all symbols in a file.
    pub fn check_file(&mut self, file: FileId) {
        let symbols = self.index.symbols_in_file(file);

        // Pass 1: Check references and collect what's referenced
        for symbol in &symbols {
            self.check_symbol(symbol);
        }

        // Pass 2: Check for duplicates within this file
        self.check_duplicates(file, &symbols);
    }

    /// Run all checks across the entire index (for workspace-wide diagnostics).
    pub fn check_all(&mut self) {
        // Collect all symbols first
        let all_symbols: Vec<_> = self.index.all_symbols().cloned().collect();

        // Check each symbol
        for symbol in &all_symbols {
            self.check_symbol(symbol);
        }

        // Check for unused definitions (only meaningful after checking all references)
        // Disabled by default as it can be noisy - uncomment to enable
        // self.check_unused(&all_symbols);
    }

    /// Check a single symbol.
    fn check_symbol(&mut self, symbol: &HirSymbol) {
        // Check supertype references (e.g., `part wheel : Wheel`)
        // These are always type references that should resolve via scope
        for supertype in &symbol.supertypes {
            self.check_type_reference(symbol, supertype);
        }

        // Check type_refs based on their RefKind
        self.check_type_refs(symbol);
    }

    /// Check type references in a symbol's body, filtering by RefKind.
    fn check_type_refs(&mut self, symbol: &HirSymbol) {
        use crate::hir::symbols::TypeRefKind;

        for type_ref in &symbol.type_refs {
            match type_ref {
                TypeRefKind::Simple(tr) => {
                    // If already resolved, track as referenced
                    if let Some(ref resolved) = tr.resolved_target {
                        self.referenced.insert(resolved.clone());
                        continue;
                    }

                    // Check based on reference kind
                    if tr.kind.is_type_reference() {
                        // Type references resolve via scope walking
                        self.check_type_reference(symbol, &tr.target);
                    } else if tr.kind.is_feature_reference() {
                        // Feature references (Redefines, Subsets) resolve via inheritance
                        self.check_feature_reference(symbol, &tr.target);
                    }
                    // Expression and Other refs are not checked
                }
                TypeRefKind::Chain(chain) => {
                    // Track resolved parts
                    for part in &chain.parts {
                        if let Some(ref resolved) = part.resolved_target {
                            self.referenced.insert(resolved.clone());
                        }
                    }
                    // Chains are typically feature access paths, don't check as types
                }
            }
        }
    }

    /// Check a feature reference resolves via inheritance.
    ///
    /// For `attribute mass redefines Vehicle::mass`, we:
    /// 1. Parse the target - could be qualified (Vehicle::mass) or simple (mass)
    /// 2. For qualified: resolve the prefix, then find member in that scope
    /// 3. For simple: look in containing symbol's supertypes
    fn check_feature_reference(&mut self, symbol: &HirSymbol, target: &str) {
        // Handle qualified references like "Vehicle::mass"
        if let Some(idx) = target.rfind("::") {
            let prefix = &target[..idx];
            let member = &target[idx + 2..];

            // Resolve the prefix (e.g., "Vehicle")
            let scope = Self::extract_scope(&symbol.qualified_name);
            let resolver = Resolver::new(self.index).with_scope(scope);

            match resolver.resolve(prefix) {
                ResolveResult::Found(prefix_sym) => {
                    // Now look for the member in that symbol's scope
                    if let Some(found) = self
                        .index
                        .find_member_in_scope(&prefix_sym.qualified_name, member)
                    {
                        self.referenced.insert(found.qualified_name.clone());
                    } else if !Self::is_builtin_type(target) {
                        // Member not found in the specified scope
                        self.collector
                            .undefined_reference(symbol.file, symbol, target);
                    }
                }
                ResolveResult::NotFound => {
                    // Prefix itself couldn't be resolved
                    if !Self::is_builtin_type(prefix) {
                        self.collector
                            .undefined_reference(symbol.file, symbol, target);
                    }
                }
                ResolveResult::Ambiguous(_) => {
                    // Ambiguous prefix - could report but likely not the user's error
                }
            }
            return;
        }

        // Simple reference like "mass" - look in containing symbol's inheritance chain
        // The containing symbol should have supertypes that define this feature

        // First, check if it's defined in the current symbol itself (local redefinition)
        let member_qname = format!("{}::{}", symbol.qualified_name, target);
        if self.index.lookup_qualified(&member_qname).is_some() {
            self.referenced.insert(Arc::from(member_qname));
            return;
        }

        // Look in the symbol's supertypes for the feature
        for supertype in &symbol.supertypes {
            // Resolve the supertype
            let scope = Self::extract_scope(&symbol.qualified_name);
            let resolver = Resolver::new(self.index).with_scope(scope);

            if let ResolveResult::Found(super_sym) = resolver.resolve(supertype) {
                // Look for the member in the supertype (recursive search)
                if let Some(found) = self
                    .index
                    .find_member_in_scope(&super_sym.qualified_name, target)
                {
                    self.referenced.insert(found.qualified_name.clone());
                    return;
                }
            }
        }

        // Still not found - might be in a supertype's supertype, or genuinely undefined
        // Don't report error if it could be from stdlib or unloaded types
        if !Self::is_builtin_type(target) && !symbol.supertypes.is_empty() {
            // Only report if the symbol has supertypes (so we expected to find it)
            // But actually, this is often a false positive since we haven't loaded stdlib
            // For now, we'll be lenient and not report these
            // self.collector.undefined_reference(symbol.file, symbol, target);
        }
    }

    /// Check a type reference resolves correctly.
    fn check_type_reference(&mut self, symbol: &HirSymbol, name: &str) {
        let scope = Self::extract_scope(&symbol.qualified_name);
        let resolver = Resolver::new(self.index).with_scope(scope);

        match resolver.resolve(name) {
            ResolveResult::Found(resolved) => {
                // Track that this symbol is referenced
                self.referenced.insert(resolved.qualified_name.clone());
            }
            ResolveResult::Ambiguous(candidates) => {
                self.collector
                    .ambiguous_reference(symbol.file, symbol, name, &candidates);
            }
            ResolveResult::NotFound => {
                // Only report undefined if it's not a built-in/primitive type
                // Also skip expression paths that contain dots (like "foo.bar.baz")
                // These are member access expressions, not type references
                if !Self::is_builtin_type(name) && !name.contains('.') {
                    self.collector
                        .undefined_reference(symbol.file, symbol, name);
                }
            }
        }
    }

    /// Check for duplicate definitions within a file.
    fn check_duplicates(&mut self, file: FileId, symbols: &[&HirSymbol]) {
        use std::collections::HashMap;

        // Group by qualified name
        let mut by_qname: HashMap<&str, Vec<&HirSymbol>> = HashMap::new();
        for symbol in symbols {
            // Skip imports and aliases - they don't count as definitions
            if symbol.kind == SymbolKind::Import || symbol.kind == SymbolKind::Alias {
                continue;
            }
            by_qname
                .entry(symbol.qualified_name.as_ref())
                .or_default()
                .push(symbol);
        }

        // Report duplicates
        for (_qname, defs) in by_qname {
            if defs.len() > 1 {
                // Report error on all but the first definition
                let first = defs[0];
                for dup in &defs[1..] {
                    self.collector.duplicate_definition(file, dup, first);
                }
            }
        }
    }

    /// Check for unused definitions (optional, can be noisy).
    #[allow(dead_code)]
    fn check_unused(&mut self, symbols: &[HirSymbol]) {
        for symbol in symbols {
            // Only check definitions, not usages
            if !symbol.kind.is_definition() {
                continue;
            }

            // Skip packages - they're organizational
            if symbol.kind == SymbolKind::Package {
                continue;
            }

            // Skip if referenced
            if self.referenced.contains(&symbol.qualified_name) {
                continue;
            }

            // Skip if it has supertypes (might be used via specialization)
            if !symbol.supertypes.is_empty() {
                continue;
            }

            self.collector.unused_symbol(symbol);
        }
    }

    /// Check if a type name is a built-in/primitive that doesn't need resolution.
    ///
    /// This includes:
    /// - KerML primitives (Boolean, Integer, etc.)
    /// - References to standard library packages (ISQ::*, SI::*, etc.)
    fn is_builtin_type(name: &str) -> bool {
        // Primitive types from KerML
        if matches!(
            name,
            "Boolean"
                | "String"
                | "Integer"
                | "Real"
                | "Natural"
                | "Positive"
                | "UnlimitedNatural"
                | "Complex"
                | "ScalarValues"
                | "Base"
                | "Anything"
        ) {
            return true;
        }

        // Standard library package prefixes
        // These are defined in the SysML Standard Library
        let stdlib_prefixes = [
            "ISQ::", // International System of Quantities
            "SI::",  // International System of Units
            "USCustomaryUnits::",
            "Quantities::",
            "MeasurementReferences::",
            "QuantityCalculations::",
            "TensorMeasurements::",
            "TrigFunctions::",
            "BaseFunctions::",
            "DataFunctions::",
            "ControlFunctions::",
            "NumericalFunctions::",
            "VectorFunctions::",
            "SequenceFunctions::",
            "CollectionFunctions::",
            "Performances::",
            "ScalarValues::",
            "RealFunctions::",
            "Time::",
            "Collections::",
            "Links::",
            "Occurrences::",
            "Objects::",
            "Items::",
            "Parts::",
            "Ports::",
            "Connections::",
            "Interfaces::",
            "Allocations::",
            "Actions::",
            "Calculations::",
            "Constraints::",
            "Requirements::",
            "Cases::",
            "AnalysisCases::",
            "Metadata::",
            "KerML::",
            "SysML::",
        ];

        for prefix in stdlib_prefixes {
            if name.starts_with(prefix) {
                return true;
            }
        }

        // Also handle simple names that are common stdlib types/packages
        // These might be imported with wildcard imports or referenced directly
        let stdlib_types = [
            // Packages (used as namespace references)
            "ISQ",
            "SI",
            "USCustomaryUnits",
            "Quantities",
            // Quantities
            "MassValue",
            "LengthValue",
            "TimeValue",
            "VelocityValue",
            "AccelerationValue",
            "ForceValue",
            "EnergyValue",
            "PowerValue",
            "PressureValue",
            "TemperatureValue",
            "ElectricCurrentValue",
            "TorqueValue",
            "AreaValue",
            "VolumeValue",
            "DensityValue",
            "AngleValue",
            "AngularVelocityValue",
            "AngularAccelerationValue",
            // Units
            "kg",
            "m",
            "s",
            "A",
            "K",
            "mol",
            "cd",
            "N",
            "J",
            "W",
            "Pa",
            // Common types
            "distancePerVolume",
            "length",
            "time",
            "mass",
            "power",
            // Functions and calculations
            "SampledFunction",
            "SamplePair",
            // Trade studies
            "TradeStudy",
            "evaluationFunction",
            // Modeling metadata
            "mop",
            "status",
        ];

        stdlib_types.contains(&name)
    }

    /// Extract scope from a qualified name.
    fn extract_scope(qualified_name: &str) -> String {
        if let Some(pos) = qualified_name.rfind("::") {
            qualified_name[..pos].to_string()
        } else {
            String::new()
        }
    }

    /// Get the collected diagnostics, deduplicated.
    pub fn finish(self) -> Vec<Diagnostic> {
        let mut seen = std::collections::HashSet::new();
        self.collector
            .diagnostics
            .into_iter()
            .filter(|d| {
                // Deduplicate by (file, line, col, message)
                let key = (d.file, d.start_line, d.start_col, d.message.clone());
                seen.insert(key)
            })
            .collect()
    }
}

/// Check a file and return diagnostics.
pub fn check_file(index: &SymbolIndex, file: FileId) -> Vec<Diagnostic> {
    let mut checker = SemanticChecker::new(index);
    checker.check_file(file);
    checker.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{SymbolKind, new_element_id};

    fn make_symbol(name: &str, qualified: &str, kind: SymbolKind, file: u32) -> HirSymbol {
        HirSymbol {
            name: Arc::from(name),
            short_name: None,
            qualified_name: Arc::from(qualified),
            element_id: new_element_id(),
            kind,
            file: FileId::new(file),
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
            short_name_start_line: None,
            short_name_start_col: None,
            short_name_end_line: None,
            short_name_end_col: None,
            doc: None,
            supertypes: Vec::new(),
            relationships: Vec::new(),
            type_refs: Vec::new(),
            is_public: false,
            view_data: None,
        }
    }

    #[test]
    fn test_diagnostic_error() {
        let diag = Diagnostic::error(FileId::new(0), 10, 5, "test error");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.start_line, 10);
        assert_eq!(diag.start_col, 5);
    }

    #[test]
    fn test_diagnostic_with_code() {
        let diag =
            Diagnostic::error(FileId::new(0), 0, 0, "test").with_code(codes::UNDEFINED_REFERENCE);
        assert_eq!(diag.code.as_deref(), Some("E0001"));
    }

    #[test]
    fn test_collector_counts() {
        let mut collector = DiagnosticCollector::new();
        collector.add(Diagnostic::error(FileId::new(0), 0, 0, "error 1"));
        collector.add(Diagnostic::error(FileId::new(0), 0, 0, "error 2"));
        collector.add(Diagnostic::warning(FileId::new(0), 0, 0, "warning 1"));

        assert_eq!(collector.error_count(), 2);
        assert_eq!(collector.warning_count(), 1);
        assert!(collector.has_errors());
    }

    #[test]
    fn test_collector_by_file() {
        let mut collector = DiagnosticCollector::new();
        collector.add(Diagnostic::error(FileId::new(0), 0, 0, "file 0"));
        collector.add(Diagnostic::error(FileId::new(1), 0, 0, "file 1"));
        collector.add(Diagnostic::error(FileId::new(0), 0, 0, "file 0 again"));

        let file0_diags = collector.diagnostics_for_file(FileId::new(0));
        assert_eq!(file0_diags.len(), 2);

        let file1_diags = collector.diagnostics_for_file(FileId::new(1));
        assert_eq!(file1_diags.len(), 1);
    }

    #[test]
    fn test_severity_to_lsp() {
        assert_eq!(Severity::Error.to_lsp(), 1);
        assert_eq!(Severity::Warning.to_lsp(), 2);
        assert_eq!(Severity::Info.to_lsp(), 3);
        assert_eq!(Severity::Hint.to_lsp(), 4);
    }

    #[test]
    fn test_semantic_checker_undefined_reference() {
        let mut index = SymbolIndex::new();

        // Add a symbol that references a non-existent type
        let mut symbol = make_symbol("wheel", "Vehicle::wheel", SymbolKind::PartUsage, 0);
        symbol.supertypes = vec![Arc::from("NonExistent")];

        index.add_file(FileId::new(0), vec![symbol]);

        let diagnostics = check_file(&index, FileId::new(0));

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("undefined reference"));
    }

    #[test]
    fn test_semantic_checker_valid_reference() {
        let mut index = SymbolIndex::new();

        // Add the type definition
        let wheel_def = make_symbol("Wheel", "Wheel", SymbolKind::PartDef, 0);

        // Add a symbol that references the type
        let mut wheel_usage = make_symbol("wheel", "Vehicle::wheel", SymbolKind::PartUsage, 0);
        wheel_usage.supertypes = vec![Arc::from("Wheel")];

        index.add_file(FileId::new(0), vec![wheel_def, wheel_usage]);

        let diagnostics = check_file(&index, FileId::new(0));

        // Should have no errors - reference resolves
        assert_eq!(diagnostics.len(), 0);
    }
}
