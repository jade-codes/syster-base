# Test Migration Plan: Archived Semantic Tests → HIR Layer

This document outlines the migration plan for refactoring archived semantic tests to work with the new HIR-based architecture.

## ✅ Migration Complete

**Status: COMPLETE** (January 2026)

| Metric | Value |
|--------|-------|
| Tests Passing | **2085** |
| Tests Ignored | 0 |
| New Test Files | 17 |
| Lines of Test Code | ~3000+ |
| Weeks Completed | 13/13 |

### New Test Organization

```
tests/
├── helpers/           # Reusable test utilities
│   ├── hir_helpers.rs
│   ├── symbol_assertions.rs
│   ├── diagnostic_helpers.rs
│   └── source_fixtures.rs
├── hir/               # HIR layer tests
│   ├── tests_symbol_extraction.rs
│   ├── tests_kerml_extraction.rs
│   ├── tests_name_resolution.rs
│   ├── tests_import_resolution.rs
│   ├── tests_type_refs.rs
│   ├── tests_stdlib.rs
│   ├── tests_diagnostics.rs
│   ├── tests_spans.rs
│   └── tests_edge_cases.rs
└── ide/               # IDE feature tests
    ├── tests_hover.rs
    ├── tests_goto.rs
    ├── tests_references.rs
    ├── tests_symbols.rs
    ├── tests_completion.rs
    ├── tests_semantic_tokens.rs
    └── tests_folding.rs
```

---

## Executive Summary

We are migrating ~1,200+ lines of archived semantic tests from the old `syster::semantic` layer to the new `syster::hir` + `syster::ide` architecture. The focus is on:

1. **Data validation** - Testing that correct data is returned for given inputs
2. **Entry point focus** - Starting with HIR layer APIs
3. **Test helper abstraction** - Isolating test setup from assertions for maintainability

### Test Coverage Goals

| Category | Estimated Tests | Priority |
|----------|----------------|----------|
| SysML Symbol Extraction | ~25 | HIGH |
| KerML Symbol Extraction | ~12 | HIGH |
| Type References & Relationships | ~10 | HIGH |
| Name Resolution | ~8 | HIGH |
| Import Resolution | ~11 | HIGH |
| Cross-File Resolution | ~5 | HIGH |
| Standard Library | ~10 | MEDIUM |
| Diagnostics | ~8 | MEDIUM |
| Span/Position Tracking | ~5 | MEDIUM |
| IDE Features | ~14 | LOW |
| Edge Cases | ~8 | LOW |
| **TOTAL** | **~116 tests** | |

---

## Current State Analysis

### Archived Test Files (to migrate)

| File | Lines | Focus Area | Priority |
|------|-------|------------|----------|
| `tests_semantic_resolver.rs` | 1283 | Name resolution, scope walking | HIGH |
| `tests_semantic_workspace.rs` | 807 | Workspace state, file management | MEDIUM |
| `tests_semantic_import.rs` | 553 | Import resolution, wildcards | HIGH |
| `tests_semantic_cross_file.rs` | 353 | Cross-file references | HIGH |
| `tests_stdlib_kerml.rs` | 248 | Stdlib symbol extraction | MEDIUM |
| `tests_documentation.rs` | 166 | API existence verification | LOW |
| `semantic/tests_sysml_visitor.rs` | 1320 | Symbol extraction, duplicates | HIGH |
| `semantic/tests_selection.rs` | 130+ | Selection spans | LOW |
| `semantic/tests_kerml_visitor.rs` | ~500 | KerML symbol extraction | MEDIUM |
| `semantic/tests_import.rs` | ~300 | Import parsing | HIGH |
| `tests_kerml_imports.rs` | ~100 | KerML import extraction | MEDIUM |
| `workspace_loader_tests/` | 247 | File loading, error handling | MEDIUM |

### Old vs New Architecture

**Old Architecture (semantic layer):**
```rust
// Mutable state, visitor pattern
let mut workspace = Workspace::<SyntaxFile>::new();
let mut symbol_table = SymbolTable::new();
let mut populator = SysmlAdapter::with_index(&mut symbol_table, &mut reference_index);
populator.populate(&file).unwrap();
let resolver = Resolver::new(&symbol_table);
let symbol = resolver.resolve("Name");
```

**New Architecture (HIR layer):**
```rust
// Pure functions, immutable data, Salsa-backed
let mut host = AnalysisHost::new();
host.set_file_content(&path, &content);
let analysis = host.analysis();
let resolver = Resolver::new(analysis.symbol_index())
    .with_scope("Package::Scope");
let result = resolver.resolve("Name");  // Returns ResolveResult enum
```

---

## Phase 1: Test Infrastructure

### 1.1 Create Test Helper Module

Create a new test helper module at `tests/helpers/mod.rs`:

```rust
//! Test helpers for HIR-based tests
//!
//! Provides utilities for:
//! - Setting up analysis hosts with source code
//! - Building symbol indexes from inline source
//! - Common assertions for symbol resolution

pub mod hir_helpers;
pub mod symbol_assertions;
pub mod source_fixtures;
```

### 1.2 Core Helper Functions

**`tests/helpers/hir_helpers.rs`:**

```rust
use syster::ide::AnalysisHost;
use syster::hir::{HirSymbol, SymbolIndex, Resolver, ResolveResult, SymbolKind};
use syster::base::FileId;

/// Creates an AnalysisHost with a single file and returns analysis snapshot
pub fn analysis_from_source(source: &str) -> (AnalysisHost, FileId) {
    let mut host = AnalysisHost::new();
    let errors = host.set_file_content("test.sysml", source);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);
    
    let analysis = host.analysis();
    let file_id = analysis.get_file_id("test.sysml").unwrap();
    // Return host since analysis borrows it
    drop(analysis);
    (host, file_id)
}

/// Creates an AnalysisHost with multiple files
pub fn analysis_from_sources(files: &[(&str, &str)]) -> AnalysisHost {
    let mut host = AnalysisHost::new();
    for (path, content) in files {
        let errors = host.set_file_content(path, content);
        assert!(errors.is_empty(), "Parse errors in {}: {:?}", path, errors);
    }
    host
}

/// Get all symbols of a specific kind from a source
pub fn symbols_of_kind(source: &str, kind: SymbolKind) -> Vec<HirSymbol> {
    let (mut host, file_id) = analysis_from_source(source);
    let analysis = host.analysis();
    analysis.symbol_index()
        .symbols_in_file(file_id)
        .filter(|s| s.kind == kind)
        .cloned()
        .collect()
}

/// Resolve a name from a specific scope and assert it's found
pub fn assert_resolves(index: &SymbolIndex, scope: &str, name: &str) -> HirSymbol {
    let resolver = Resolver::new(index).with_scope(scope);
    match resolver.resolve(name) {
        ResolveResult::Found(sym) => sym.clone(),
        ResolveResult::NotFound => panic!("'{}' did not resolve from scope '{}'", name, scope),
        ResolveResult::Ambiguous(candidates) => {
            panic!("'{}' is ambiguous from scope '{}': {:?}", name, scope, 
                   candidates.iter().map(|s| &s.qualified_name).collect::<Vec<_>>())
        }
    }
}

/// Resolve a name and assert it's NOT found
pub fn assert_not_found(index: &SymbolIndex, scope: &str, name: &str) {
    let resolver = Resolver::new(index).with_scope(scope);
    match resolver.resolve(name) {
        ResolveResult::NotFound => (),
        ResolveResult::Found(sym) => {
            panic!("'{}' should not resolve from '{}', but found: {}", 
                   name, scope, sym.qualified_name)
        }
        ResolveResult::Ambiguous(_) => {
            panic!("'{}' should not resolve from '{}', but was ambiguous", name, scope)
        }
    }
}
```

### 1.3 Symbol Assertion Helpers

**`tests/helpers/symbol_assertions.rs`:**

```rust
use syster::hir::{HirSymbol, SymbolKind, SymbolIndex};

/// Assert a symbol exists with the given qualified name
pub fn assert_symbol_exists(index: &SymbolIndex, qname: &str) {
    assert!(
        index.lookup_qualified(qname).is_some(),
        "Expected symbol '{}' to exist", qname
    );
}

/// Assert a symbol has the expected kind
pub fn assert_symbol_kind(symbol: &HirSymbol, expected: SymbolKind) {
    assert_eq!(
        symbol.kind, expected,
        "Expected symbol '{}' to have kind {:?}, got {:?}",
        symbol.qualified_name, expected, symbol.kind
    );
}

/// Assert symbol count of a specific kind
pub fn assert_symbol_count(symbols: &[HirSymbol], kind: SymbolKind, expected: usize) {
    let count = symbols.iter().filter(|s| s.kind == kind).count();
    assert_eq!(
        count, expected,
        "Expected {} symbols of kind {:?}, found {}", 
        expected, kind, count
    );
}

/// Assert no duplicate symbols by qualified name
pub fn assert_no_duplicate_symbols(symbols: &[HirSymbol]) {
    let mut seen = std::collections::HashSet::new();
    for sym in symbols {
        if !seen.insert(&sym.qualified_name) {
            panic!("Duplicate symbol: {}", sym.qualified_name);
        }
    }
}

/// Assert a symbol has a type reference to a specific target
pub fn assert_typed_by(symbol: &HirSymbol, type_name: &str) {
    let has_type = symbol.type_refs.iter()
        .any(|tr| tr.target.as_ref() == type_name || 
                  tr.resolved_target.as_deref() == Some(type_name));
    assert!(
        has_type,
        "Expected '{}' to be typed by '{}', but type_refs: {:?}",
        symbol.name, type_name, symbol.type_refs
    );
}

/// Assert a symbol specializes another
pub fn assert_specializes(symbol: &HirSymbol, parent_name: &str) {
    let specializes = symbol.relationships.iter()
        .any(|r| r.kind == syster::hir::RelationshipKind::Specializes &&
                 (r.target.as_ref() == parent_name || 
                  r.resolved_target.as_deref() == Some(parent_name)));
    assert!(
        specializes,
        "Expected '{}' to specialize '{}', but relationships: {:?}",
        symbol.name, parent_name, symbol.relationships
    );
}
```

---

## Phase 2: HIR Symbol Extraction Tests

**Priority: HIGH** - These form the foundation for resolution tests.

### 2.1 Test Categories

| Category | Old Tests | New Test File |
|----------|-----------|---------------|
| Package extraction | `test_visitor_creates_package_symbol` | `tests_hir_packages.rs` |
| Definition extraction | `test_visitor_creates_definition_symbol` | `tests_hir_definitions.rs` |
| Usage extraction | `test_file_symbols_nested_usages` | `tests_hir_usages.rs` |
| Duplicate detection | `test_qualified_redefinition_does_not_create_duplicate_symbols` | `tests_hir_duplicates.rs` |
| Namespace scoping | `test_same_name_in_different_namespaces_creates_two_symbols` | `tests_hir_namespaces.rs` |

### 2.2 Example Migration: Symbol Extraction

**Old test:**
```rust
#[test]
fn test_visitor_creates_package_symbol() {
    let source = "package MyPackage;";
    let mut pairs = SysMLParser::parse(Rule::file, source).unwrap();
    let file = parse_file(&mut pairs).unwrap();

    let mut symbol_table = SymbolTable::new();
    let mut graph = ReferenceIndex::new();
    let mut adapter = SysmlAdapter::with_index(&mut symbol_table, &mut graph);
    adapter.populate(&file).unwrap();

    assert!(Resolver::new(&symbol_table).resolve("MyPackage").is_some());
}
```

**New test:**
```rust
use crate::helpers::{analysis_from_source, assert_symbol_exists};
use syster::hir::SymbolKind;

#[test]
fn test_package_symbol_extraction() {
    // Given: source with a package declaration
    let source = "package MyPackage;";
    
    // When: we extract symbols
    let (mut host, _) = analysis_from_source(source);
    let analysis = host.analysis();
    
    // Then: the package symbol exists
    assert_symbol_exists(analysis.symbol_index(), "MyPackage");
    
    // And: it has the correct kind
    let sym = analysis.symbol_index().lookup_qualified("MyPackage").unwrap();
    assert_eq!(sym.kind, SymbolKind::Package);
}
```

---

## Phase 3: Name Resolution Tests

**Priority: HIGH** - Core semantic functionality.

### 3.1 Test Categories

| Category | Old File | Focus |
|----------|----------|-------|
| Simple name resolution | `tests_semantic_resolver.rs` | `resolve("Name")` |
| Qualified name resolution | `tests_semantic_resolver.rs` | `resolve("A::B::C")` |
| Scope-aware resolution | `tests_semantic_resolver.rs` | Parent scope walking |
| Import resolution | `tests_semantic_import.rs` | Wildcard, member imports |
| Cross-file resolution | `tests_semantic_cross_file.rs` | Multi-file workspaces |

### 3.2 Example Migration: Scope Resolution

**Old test:**
```rust
#[test]
fn test_resolve_qualified_name() {
    let mut table = SymbolTable::new();
    table.insert("Root".to_string(), Symbol::Package { ... }).unwrap();
    table.enter_scope();
    table.insert("Child".to_string(), Symbol::Package { 
        qualified_name: "Root::Child".to_string(), ...
    }).unwrap();

    let resolver = Resolver::new(&table);
    let result = resolver.resolve("Root::Child");
    // assertions...
}
```

**New test:**
```rust
use crate::helpers::{analysis_from_source, assert_resolves};

#[test]
fn test_resolve_qualified_name() {
    // Given: nested package structure
    let source = r#"
        package Root {
            package Child;
        }
    "#;
    
    let (mut host, _) = analysis_from_source(source);
    let analysis = host.analysis();
    
    // When/Then: qualified name resolves correctly
    let sym = assert_resolves(analysis.symbol_index(), "", "Root::Child");
    assert_eq!(sym.qualified_name.as_ref(), "Root::Child");
}
```

### 3.3 Import Resolution Tests

**Old pattern:**
```rust
let mut workspace = Workspace::<SyntaxFile>::new();
workspace.add_file(path, syntax_file);
workspace.populate_all();
let resolver = Resolver::new(workspace.symbol_table());
```

**New pattern:**
```rust
let (mut host, _) = analysis_from_source(source);
let analysis = host.analysis();
let resolver = Resolver::new(analysis.symbol_index())
    .with_scope("PackageName");  // Important: set resolution scope
let result = resolver.resolve("ImportedName");
```

---

## Phase 4: Cross-File Resolution Tests

**Priority: HIGH** - Tests multi-file semantic correctness.

### 4.1 Helper for Multi-File Tests

```rust
/// Setup helper for cross-file resolution tests
pub fn setup_cross_file_test() -> AnalysisHost {
    analysis_from_sources(&[
        ("base.sysml", "part def Vehicle;"),
        ("derived.sysml", "part def Car :> Vehicle;"),
    ])
}

#[test]
fn test_cross_file_specialization_resolves() {
    let mut host = setup_cross_file_test();
    let analysis = host.analysis();
    
    // Car should resolve its specialization to Vehicle
    let car = analysis.symbol_index()
        .lookup_qualified("Car")
        .expect("Car not found");
    
    assert_specializes(&car, "Vehicle");
}
```

---

## Phase 5: Diagnostic Tests

**Priority: MEDIUM** - Test semantic error detection.

### 5.1 Diagnostic Test Helpers

```rust
use syster::hir::{check_file, Severity};

pub fn get_errors_for_source(source: &str) -> Vec<String> {
    let (mut host, file_id) = analysis_from_source(source);
    let analysis = host.analysis();
    
    check_file(analysis.symbol_index(), file_id)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message)
        .collect()
}

pub fn assert_no_errors(source: &str) {
    let errors = get_errors_for_source(source);
    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}

pub fn assert_has_error_containing(source: &str, substring: &str) {
    let errors = get_errors_for_source(source);
    assert!(
        errors.iter().any(|e| e.contains(substring)),
        "Expected error containing '{}', got: {:?}", substring, errors
    );
}
```

---

## Phase 6: IDE Feature Tests

**Priority: LOW** - After core HIR tests pass.

### 6.1 Hover, Goto, Completion

These tests can largely remain as-is since they already use `AnalysisHost`, but should be refactored to use the new helpers for consistency.

---

## Execution Timeline & Comprehensive TODO List

### Week 1: Infrastructure Setup
- [x] Create `tests/helpers/mod.rs` module
- [x] Implement `hir_helpers.rs` with core functions
- [x] Implement `symbol_assertions.rs`
- [x] Implement `source_fixtures.rs` with common test sources
- [x] Add helper module to `tests/tests_main.rs`
- [x] Create test organization structure (`tests/hir/`, `tests/ide/`)
- [x] Implement `diagnostic_helpers.rs`

### Week 2: Symbol Extraction Tests (SysML)
- [x] **Package symbols** - Extract package with correct qualified name
- [x] **Part definitions** - `part def Vehicle;` → SymbolKind::PartDef
- [x] **Port definitions** - `port def DataPort;` → SymbolKind::PortDef
- [x] **Action definitions** - `action def Move;` → SymbolKind::ActionDef
- [x] **Item definitions** - `item def Payload;` → SymbolKind::ItemDef
- [x] **Attribute definitions** - `attribute def Mass;` → SymbolKind::AttributeDef
- [x] **Connection definitions** - `connection def Link;`
- [x] **Interface definitions** - `interface def I1;`
- [x] **Allocation definitions** - `allocation def Alloc;`
- [x] **Requirement definitions** - `requirement def R1;`
- [x] **Constraint definitions** - `constraint def C1;`
- [x] **State definitions** - `state def S1;`
- [x] **Calc definitions** - `calc def CalcMass;`
- [x] **Case definitions** - `case def UseCase1;`
- [x] **Analysis case definitions** - `analysis def Analysis1;`
- [x] **Verification case definitions** - `verification def V1;` (maps to AnalysisCaseDef)
- [x] **View definitions** - `view def DiagramView;`
- [x] **Viewpoint definitions** - `viewpoint def VP1;`
- [x] **Rendering definitions** - `rendering def R1;`
- [x] **Enumeration definitions** - `enum def E1;`
- [x] **Metadata definitions** - `metadata def MD1;` (maps to SymbolKind::Other)
- [x] **Part usages** - `part engine : Engine;` → SymbolKind::PartUsage
- [x] **Port usages** - `port dataIn : DataPort;`
- [x] **Attribute usages** - `attribute mass : Real;`
- [x] **Action usages** - `action move : Move;`
- [x] **Item usages** - `item i : Item;`
- [x] **Ref usages** - `ref part r : RefTarget;`
- [x] **Nested usages** - Usages inside definitions with correct qualified names
- [x] **Anonymous usages** - `: Type` without name
- [x] Migrate `tests_sysml_visitor.rs` → `tests_hir_symbol_extraction.rs`
- [x] Migrate duplicate detection tests
- [x] Migrate namespace scoping tests

### Week 3: Symbol Extraction Tests (KerML)
- [x] **KerML packages** - `package KerMLPkg;`
- [x] **KerML classifiers** - `class MyClass;`, `datatype DT;`, `struct S;`
- [x] **KerML features** - `feature f;`
- [x] **KerML step** - `step s;`
- [x] **KerML functions** - `function fn;`
- [x] **KerML behaviors** - `behavior B;`
- [x] **KerML interactions** - `interaction I;` (added InteractionDef SymbolKind)
- [x] **KerML metaclasses** - `metaclass MC;` (added MetaclassDef SymbolKind)
- [x] **KerML connectors** - `connector c;`
- [x] **KerML successions** - `succession first a then b;`
- [x] **KerML relationship extraction**
- [x] **KerML imports** - Import resolution in KerML files
- [x] **KerML public import re-export** - Public import chains
- [x] Migrate `semantic/tests_kerml_visitor.rs` → `tests_kerml_extraction.rs`
- [x] Migrate `tests_kerml_imports.rs`

#### KerML Bug Fixes Applied:
- [x] Fixed empty package bug (same as SysML) - `KerMLPkg` no longer appears as `KerMLPkg::KerMLPkg`
- [x] Fixed wildcard import path - `import Source::*` now correctly includes `::*` suffix
- [x] Fixed public import visibility - `public import` now correctly sets `is_public=true`
- [x] Added `Interaction` to ClassifierKind and parser pipeline
- [x] Added `MetaclassDef` and `InteractionDef` to SymbolKind enum
- [x] Added step, connector, succession to classifier member extraction

### Week 4: Type References & Relationships
- [x] **TypedBy (`:`)** - `part car : Vehicle;` extracts TypeRef to "Vehicle"
- [x] **Specializes (`:>`)** - `part def Car :> Vehicle;` extracts relationship
- [x] **Redefines (`:>>`)** - `part :>> existingPart;` extracts redefinition
- [x] **Subsets** - `part subsets otherPart;`
- [x] **References (`::>`)** - `ref part ::> refTarget;`
- [x] **Chained references** - `action.subaction.step` chain detection
- [x] **Multiple type refs** - `part x : A, B, C;` extracts all
- [x] **Conjugated ports** - `port ~p : ~PortType;`
- [x] **Relationship span tracking** - Correct line/column for type refs
- [x] **Resolved vs unresolved targets** - Pre-resolution vs post-resolution

### Week 5: Name Resolution Tests
- [x] **Simple name resolution** - `resolve("Vehicle")` in global scope
- [x] **Qualified name resolution** - `resolve("A::B::C")`
- [x] **Non-existent name** - Returns `NotFound`
- [x] **Deeply nested resolution** - 5+ levels of nesting
- [x] **Scope walking** - Child scope finds parent's symbols
- [x] **Shadowing** - Local definition shadows outer
- [x] **Multiple imports same name** - Documents current behavior (last import wins)
- [x] Migrate `tests_semantic_resolver.rs` basic tests

### Week 6: Import Resolution Tests
- [x] **Wildcard import** - `import Pkg::*;` makes members visible
- [x] **Member import** - `import Pkg::Member;` imports single member
- [x] **Public import** - `public import` re-exports to importers
- [x] **Private import** - Non-public import stays local *(bug fixed!)*
- [x] **Recursive import** - `import Pkg::**;` (test added, documents current behavior)
- [x] **Chained imports** - A imports B imports C
- [x] **Import from parent scope** - Child imports from sibling via parent
- [x] **Transitive public re-export** - `public import` chains *(bug fixed!)*
- [x] **Alias** - `alias Y for X;` creates alias symbol with target in supertypes
- [x] **Filter import syntax** - Documents that SysML uses `[condition]` not `except`
- [x] Migrate `tests_semantic_import.rs` import tests
- [x] Add scope-aware resolution tests

### Week 7: Cross-File Resolution Tests
- [x] **Cross-file specialization** - `Car :> Vehicle` where Vehicle is in another file
- [x] **Cross-file typing** - `part x : ExternalType;`
- [x] **Transitive cross-file** - A→B→C across 3 files
- [x] **File order independence** - Resolution works regardless of load order
- [x] **Incremental update** - Change one file, resolution updates
- [x] Migrate `tests_semantic_cross_file.rs`

### Week 8: Standard Library Tests
- [x] **Stdlib loading** - All stdlib files parse without error
- [x] **Key symbol existence** - ScalarValues::Real, Integer, Boolean, String
- [x] **Symbol count validation** - Minimum expected symbols per file
- [x] **No duplicate symbols in stdlib** - Verify no duplicates
- [x] **Performances.kerml** - `thisPerformance` not duplicated
- [x] **Observation.kerml** - `observations` not duplicated
- [x] **Objects.kerml** - `StructuredSpaceObject` not duplicated
- [x] **MeasurementReferences.sysml** - Package not duplicated
- [x] Migrate `tests_stdlib_kerml.rs` → `tests/hir/tests_stdlib.rs`
- [x] Full stdlib tests (ignored - slow) for SI, ISQ packages

### Week 9: Diagnostic Tests
- [x] **Undefined reference error** - Reference to non-existent symbol
- [x] **Undefined specialization** - Specializes non-existent type
- [x] **Valid reference no error** - Correct code produces no errors
- [x] **Severity levels** - Error severity for undefined refs
- [x] **Diagnostic spans** - Correct file and position info
- [x] **Error codes** - Diagnostics have error codes
- [x] **Cross-file resolution** - No false positives with imports
- [x] **Self-referential types** - No crash/infinite loop
- [x] **Qualified references** - No false positives
- [x] Add `tests/hir/tests_diagnostics.rs`

### Week 10: Span & Position Tests
- [x] **Symbol line info** - Symbols have valid line numbers
- [x] **Symbol column info** - Symbols have valid column numbers
- [x] **Symbol end after start** - End position >= start position
- [x] **Multiline definition name span** - Documents HIR tracks name position
- [x] **Nested symbol positions** - Valid positions for nested symbols
- [x] **Type ref spans** - Supertypes captured correctly
- [x] **Specialization ref spans** - Specialization refs captured
- [x] **Symbol ordering by position** - A before B before C
- [x] **Deeply nested spans** - L1 → L2 → L3 → Deep
- [x] **Tab handling** - No crash with tabs
- [x] Add `tests/hir/tests_spans.rs`

### Week 11: IDE Feature Tests
- [x] **Hover on definition** - Shows definition info
- [x] **Hover on package** - Shows package info
- [x] **Hover result qualified name** - Has qualified name
- [x] **Hover is_definition flag** - Correct for definitions
- [x] **Hover on usage** - Shows type info
- [x] **Hover shows specialization** - Relationships in hover
- [x] **Goto definition** - From type reference to definition
- [x] **Goto definition** - From specialization to base
- [x] **Goto definition cross-file** - Across files
- [x] **Find references** - All usages of a definition
- [x] **Find references include declaration** - Definition in results
- [x] **Find references from usage** - Works from usage site
- [x] **Find references cross-file** - Across multiple files
- [x] **Document symbols** - Returns all symbols in file
- [x] **Document symbols has names** - Correct names
- [x] **Document symbols has kind** - Correct kinds
- [x] **Document symbols has position** - Valid positions
- [x] **Workspace symbols search** - Search by name
- [x] **Workspace symbols no match** - Empty for non-matching
- [x] **Completions with incomplete syntax** - Works with parse errors
- [x] **Completions with partial word** - "Veh" suggests "Vehicle"
- [x] **Completions with no word** - Suggests all types
- [x] **Completions has kind** - Correct CompletionKind
- [x] **Semantic tokens** - Returns tokens for symbols
- [x] **Semantic tokens has position** - Valid line/col/length
- [x] **Semantic tokens namespace** - Package → Namespace
- [x] **Semantic tokens type** - Part def → Type
- [x] **Folding ranges** - Valid ranges for packages
- [x] Add `tests/ide/` module with tests_hover, tests_goto, tests_references, tests_symbols, tests_completion, tests_semantic_tokens, tests_folding

### Week 12: Edge Cases & Error Recovery
- [x] **Empty file** - No crash, empty symbols
- [x] **Whitespace only file** - No crash, empty symbols
- [x] **Comment only file** - No crash
- [x] **Partial parse** - Recovers valid symbols before error
- [x] **Syntax error no crash** - Invalid syntax doesn't crash
- [x] **Unclosed brace** - No crash
- [x] **Deeply nested (20 levels)** - No stack overflow
- [x] **Wide file (100 siblings)** - Performance acceptable
- [x] **Underscore in names** - `my_vehicle`, `vehicle_v2`
- [x] **Numbers in names** - `Item1`, `V8Engine`
- [x] **Long names (200 chars)** - No crash
- [x] **Many type references** - Multiple supertypes captured
- [x] **Duplicate names same scope** - No crash
- [x] **Same name different scopes** - Both extracted with qualified names
- [x] **Self-referential types** - No infinite loop
- [x] **Mutual references** - A→B→A no crash
- [x] Add `tests/hir/tests_edge_cases.rs`

### Week 13: Cleanup & Documentation
- [x] Update test helper documentation (`tests/helpers/mod.rs`)
- [x] Add migration summary to plan document
- [x] Document new test organization structure
- [x] Verify all tests pass (2085 passing, 0 ignored)
- [x] Remove archived tests (`tests_archived/` deleted)
- [x] Remove dead code from helper modules
- [N/A] *(Optional)* Add test coverage tooling
- [N/A] *(Optional)* Performance benchmarks for resolution

---

## Test File Organization

```
tests/
├── helpers/
│   ├── mod.rs                         # Re-exports all helpers
│   ├── hir_helpers.rs                 # analysis_from_source, analysis_from_sources
│   ├── symbol_assertions.rs           # assert_symbol_exists, assert_resolves, etc.
│   ├── diagnostic_helpers.rs          # get_errors_for_source, assert_no_errors
│   └── source_fixtures.rs             # Common test source snippets
├── hir/
│   ├── mod.rs
│   ├── tests_symbol_extraction.rs     # SysML symbol extraction
│   ├── tests_kerml_extraction.rs      # KerML symbol extraction  
│   ├── tests_type_refs.rs             # TypeRef and relationship extraction
│   ├── tests_name_resolution.rs       # Basic name resolution
│   ├── tests_import_resolution.rs     # Import visibility and resolution
│   ├── tests_cross_file.rs            # Multi-file resolution
│   ├── tests_diagnostics.rs           # Semantic error detection
│   ├── tests_stdlib.rs                # Standard library loading
│   └── tests_edge_cases.rs            # Empty files, large files, errors
├── ide/
│   ├── mod.rs
│   ├── tests_hover.rs                 # Hover information
│   ├── tests_goto.rs                  # Goto definition/type definition
│   ├── tests_references.rs            # Find references
│   ├── tests_completion.rs            # Code completion
│   ├── tests_symbols.rs               # Document/workspace symbols
│   ├── tests_semantic_tokens.rs       # Syntax highlighting tokens
│   ├── tests_folding.rs               # Folding ranges
│   └── tests_inlay_hints.rs           # Inlay hints
└── spans/
    ├── mod.rs
    ├── tests_symbol_spans.rs          # Symbol position tracking
    ├── tests_type_ref_spans.rs        # Type reference positions
    └── tests_selection_ranges.rs      # Selection expansion
```

---

## Migration Checklist Template

For each test being migrated:

- [ ] Identify the semantic concept being tested
- [ ] Determine if test is still relevant with new architecture
- [ ] Find equivalent API in HIR layer
- [ ] Write test using new helpers
- [ ] Verify test passes
- [ ] Mark old test as migrated in archive
- [ ] Document any behavior differences

---

## Key API Mappings

| Old API | New API |
|---------|---------|
| `Workspace::new()` | `AnalysisHost::new()` |
| `workspace.add_file(path, file)` | `host.set_file_content(path, content)` |
| `workspace.populate_all()` | `host.analysis()` (triggers rebuild) |
| `Resolver::new(&symbol_table)` | `Resolver::new(analysis.symbol_index())` |
| `resolver.resolve("name")` | `resolver.with_scope("...").resolve("name")` |
| `Symbol::Package { ... }` | `HirSymbol { kind: SymbolKind::Package, ... }` |
| `workspace.symbol_table().iter_symbols()` | `index.symbols_in_file(file_id)` |

---

## Notes

1. **Scope is now explicit**: The new resolver requires setting scope with `.with_scope()` for accurate resolution.

2. **Resolution returns enum**: `ResolveResult` has `Found`, `NotFound`, `Ambiguous` variants instead of `Option<Symbol>`.

3. **Symbols are immutable**: `HirSymbol` is extracted data, not mutable state.

4. **FileId is required**: Most operations now take `FileId` instead of path strings.

5. **Pre-computed visibility**: Import resolution is pre-computed in `SymbolIndex`, not resolved on-demand.
