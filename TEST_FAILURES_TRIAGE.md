# Test Failures Triage

Generated: 2026-01-30

**Total failures: 145**

---

## Category 1: Missing Keywords in Parser (43 failures)

**Location:** `src/parser/lexer.rs` + `src/parser/syntax_kind.rs` + `src/parser/parser.rs`

These tests fail because certain keywords aren't recognized by the lexer/parser.

### Missing Keywords Identified:
- `allocate` - used in allocation statements (`allocate x to y;`)
- Possibly others in the stdlib/example files

### Files Affected:
- All 43 `example_*` tests in `tests_sysml_examples.rs`

### Fix:
1. Add keyword to lexer (`src/parser/lexer.rs`)
2. Add to SyntaxKind enum (`src/parser/syntax_kind.rs`)
3. Add to parser namespace member match (`src/parser/parser.rs`)

---

## Category 2: KerML Extraction Issues (16 failures)

**Location:** `src/hir/symbols.rs` (extraction) + `src/syntax/normalized.rs` (normalization)

### Tests:
- `hir::tests_kerml_extraction::test_kerml_association_extraction`
- `hir::tests_kerml_extraction::test_kerml_behavior_extraction`
- `hir::tests_kerml_extraction::test_kerml_class_extraction`
- `hir::tests_kerml_extraction::test_kerml_class_specialization`
- `hir::tests_kerml_extraction::test_kerml_connector_extraction`
- `hir::tests_kerml_extraction::test_kerml_datatype_extraction`
- `hir::tests_kerml_extraction::test_kerml_feature_extraction`
- `hir::tests_kerml_extraction::test_kerml_function_extraction`
- `hir::tests_kerml_extraction::test_kerml_import_resolution`
- `hir::tests_kerml_extraction::test_kerml_interaction_extraction`
- `hir::tests_kerml_extraction::test_kerml_metaclass_extraction`
- `hir::tests_kerml_extraction::test_kerml_nested_packages`
- `hir::tests_kerml_extraction::test_kerml_public_import_reexport`
- `hir::tests_kerml_extraction::test_kerml_step_extraction`
- `hir::tests_kerml_extraction::test_kerml_struct_extraction`
- `hir::tests_kerml_extraction::test_kerml_succession_extraction`

### Issue:
The normalized layer and symbol extraction doesn't properly handle KerML-specific constructs (class, struct, behavior, function, etc.). The parser now recognizes these keywords, but the extraction code needs to:
1. Map KerML keywords to appropriate `NormalizedDefKind` / `NormalizedUsageKind`
2. Extract specializations and type refs from KerML syntax

### Fix Location:
- `src/syntax/normalized.rs` - Add KerML def kinds, handle KerML AST nodes
- `src/hir/symbols.rs` - Map new normalized kinds to SymbolKind

---

## Category 3: Import/Filter Resolution (17 failures)

**Location:** `src/hir/resolve.rs` (visibility maps) + `src/hir/symbols.rs` (filter extraction)

### Tests:
- `hir::tests_import_resolution::test_filtered_import_*` (12 tests)
- `hir::tests_import_resolution::test_filter_*` (5 tests)

### Issue:
The filter import evaluation (`import X::*[@Filter]`) isn't working properly. The parser may be parsing the syntax, but the HIR isn't:
1. Extracting the filter metadata from imports
2. Evaluating which symbols pass the filter
3. Building visibility maps correctly with filtered imports

### Fix Location:
- `src/syntax/normalized.rs` - Extract filter annotations from imports
- `src/hir/symbols.rs` - Pass filter info to ExtractionResult
- `src/hir/resolve.rs` - Apply filters when building visibility maps

---

## Category 4: Symbol Extraction Issues (10 failures)

**Location:** `src/hir/symbols.rs` + `src/syntax/normalized.rs`

### Tests:
- `hir::tests_symbol_extraction::test_action_usage_extraction`
- `hir::tests_symbol_extraction::test_anonymous_usage_no_name`
- `hir::tests_symbol_extraction::test_attribute_usage_extraction`
- `hir::tests_symbol_extraction::test_item_usage_extraction`
- `hir::tests_symbol_extraction::test_nested_usages_have_qualified_names`
- `hir::tests_symbol_extraction::test_port_usage_extraction`
- `hir::tests_symbol_extraction::test_ref_usage_extraction`
- `hir::tests_symbol_extraction::test_use_case_def_extraction`
- `hir::tests_type_refs::*` (various)

### Issue:
Basic usage/definition extraction isn't working correctly. The rowan AST layer may not be exposing the right information, or the normalized layer isn't capturing it.

### Fix Location:
- `src/parser/ast.rs` - Check Usage/Definition AST methods
- `src/syntax/normalized.rs` - Verify NormalizedUsage/NormalizedDefinition capture all data

---

## Category 5: Stdlib Tests (11 failures)

**Location:** Multiple - likely parser + extraction

### Tests:
- `hir::tests_stdlib::test_*` (11 tests)

### Issue:
These test that stdlib files parse and extract correctly. Likely cascading failures from Categories 1-4.

### Fix:
Will likely be resolved when Categories 1-4 are fixed.

---

## Category 6: Feature Chain & Expression Extraction (20+ failures)

**Location:** `src/hir/symbols.rs` + `src/syntax/normalized.rs` + `src/ide/type_info.rs`

### Tests:
- `test_hover_on_feature_chain_*`
- `test_hover_on_bind_*`
- `test_hover_on_expression_*`
- `test_expression_*`
- `test_bind_*`
- `test_connect_*`

### Issue:
Feature chains (like `x.y.z`) and expressions aren't being extracted with proper position/range info for hover support.

### Fix Location:
- `src/syntax/normalized.rs` - Extract expression chains with TextRange
- `src/hir/symbols.rs` - Create TypeRef entries for each chain part
- `src/ide/type_info.rs` - `find_type_ref_at_position` lookup logic

---

## Category 7: Transition/State Extraction (15+ failures)

**Location:** `src/hir/symbols.rs` + `src/syntax/normalized.rs`

### Tests:
- `test_transition_*`
- `test_then_*`
- `test_first_*`
- `debug_hover_on_transition_*`

### Issue:
Transition syntax (`transition initial then off;`, `first x then y;`) isn't being extracted as type refs for the source/target states.

### Fix Location:
- `src/syntax/normalized.rs` - Parse transition source/target as relationships
- `src/hir/symbols.rs` - Create TypeRef entries for transition endpoints

---

## Category 8: Semantic Resolution (5+ failures)

**Location:** `src/hir/resolve.rs`

### Tests:
- `test_sibling_resolution`
- `test_nested_scope_resolution`
- `test_simple_vehicle_model_resolution`
- `test_specializes_nested_attribute`
- `test_subsets_action`

### Issue:
Resolution of names in nested/sibling scopes isn't working correctly. The visibility maps may not be built right, or the resolution algorithm has edge cases.

### Fix Location:
- `src/hir/resolve.rs` - `Resolver::resolve()` and `ensure_visibility_maps()`

---

## Recommended Fix Order

1. **Category 1 (Keywords)** - Quick win, unblocks Category 5
2. **Category 4 (Basic Extraction)** - Foundation for everything else
3. **Category 2 (KerML)** - Needed for many tests
4. **Category 6 (Feature Chains)** - Important for IDE hover
5. **Category 7 (Transitions)** - SysML-specific
6. **Category 3 (Filters)** - Advanced feature
7. **Category 8 (Resolution)** - Edge cases

---

## Chain of Responsibility

```
Source Text
    ↓
[Lexer] src/parser/lexer.rs
    - Tokenizes keywords → needs all SysML/KerML keywords
    ↓
[Parser] src/parser/parser.rs
    - Builds CST → needs to handle all grammar constructs
    ↓
[AST Layer] src/parser/ast.rs
    - Typed wrappers → needs methods for all node types
    ↓
[Normalized Layer] src/syntax/normalized.rs
    - Unified extraction → needs to handle SysML & KerML variants
    ↓
[HIR Symbols] src/hir/symbols.rs
    - Symbol extraction → creates HirSymbol with type_refs, relationships
    ↓
[Resolution] src/hir/resolve.rs
    - Visibility maps → builds scope-aware name lookup
    - Type resolution → resolves type_ref targets
    ↓
[IDE] src/ide/type_info.rs
    - Position lookup → find_type_ref_at_position for hover
```
