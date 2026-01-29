# TODO: Filter Import Evaluation (SysML v2 §7.5.4)

## Overview

SysML v2 supports filtered imports that selectively import elements based on metadata conditions:

```sysml
// Bracket syntax on import
import Source::*[@Safety];
import DesignModel::**[@Approval and approved and level > 1];

// Package-level filter statement
package SafetyGroup {
    import Source::*;
    filter @Safety;
}
```

**Current Status:** Package-level `filter @Metadata;` statements are **fully working**. Bracket syntax `[@filter]` on imports not yet implemented.

---

## Implementation Progress

### ✅ Completed

1. **Metadata annotations extraction** (2024-01)
   - Added `metadata_annotations: Vec<Arc<str>>` field to `HirSymbol` struct
   - Implemented `extract_metadata_annotations()` function in `src/hir/symbols.rs`
   - Fixed parser to capture metadata typing from `@MetadataType` syntax
   - Added handler for `Rule::metadata_usage_declaration` in parsers.rs
   - Test `test_metadata_annotations_extracted` passes

2. **Package-level filter statement extraction** (2024-01)
   - Added `NormalizedFilter` struct to normalized.rs
   - Added `NormalizedElement::Filter` variant
   - Created `ExtractionResult` struct with symbols + filters
   - Added `extract_with_filters()` function
   - Added `scope_filters` map to `SymbolIndex`
   - Added `add_scope_filter()` and `add_extraction_result()` methods

3. **Filter evaluation in import resolution** (2024-01)
   - Added `symbol_passes_filter()` helper method
   - Modified `process_imports_recursive()` to apply filters
   - Tests `test_filtered_import_excludes_non_matching_elements` and `test_package_level_filter_statement_parses` pass

### 📋 Pending

4. **Bracket syntax filters on imports**
   - `import X::*[@Filter]` requires storing filter on import symbol
   - Need to modify `NormalizedImport` to include filter expressions
   - Need to extract filter from `filter_package` rule

5. **Boolean logic in filters**
   - `and`, `or`, `not` operators
   - Requires expression evaluation

---

## Development Approach: Test-Driven Development (TDD)

This feature MUST be implemented using strict TDD:

1. **Write failing tests FIRST** - All tests start with `#[ignore]`
2. **Implement incrementally** - Remove ONE `#[ignore]` at a time
3. **Each test = one capability** - Small, focused tests for each edge case
4. **Never remove multiple ignores at once** - Ensures each change is validated

### TDD Workflow

```
1. Write test with #[ignore] → commit
2. Remove #[ignore] → test fails (RED)
3. Implement minimal code to pass → test passes (GREEN)  
4. Refactor if needed → tests still pass (REFACTOR)
5. Repeat for next test
```

---

## Test Coverage Plan

### Test Location
All tests in `tests/hir/tests_import_resolution.rs` under the `FILTER IMPORTS` section.

### Milestone 1: Simple Metadata Presence Filters

Tests for `[@MetadataType]` - does element have this metadata applied?

| Test Name | Status | Description |
|-----------|--------|-------------|
| `test_metadata_annotations_extracted` | ✅ PASS | Foundation: metadata extracted to HirSymbol |
| `test_filtered_import_excludes_non_matching_elements` | ✅ PASS | Basic filter with `filter @Safety;` statement |
| `test_package_level_filter_statement_parses` | ✅ PASS | Package-level `filter @X;` works |
| `test_filtered_import_with_bracket_syntax` | `#[ignore]` | Filter in bracket syntax `import X::*[@Approved]` - NOT YET IMPLEMENTED |
| `test_filter_with_no_matching_elements` | `#[ignore]` | Uses bracket syntax - NOT YET IMPLEMENTED |
| `test_filter_with_all_matching_elements` | `#[ignore]` | Uses bracket syntax - NOT YET IMPLEMENTED |
| `test_filter_metadata_short_name_matches` | TODO | `@Safety` matches `MySafety::Safety` |
| `test_filter_metadata_qualified_name_matches` | TODO | `@Pkg::Safety` matches exactly |
| `test_filter_nonexistent_metadata_imports_nothing` | TODO | `@NonExistent` filter imports nothing |
| `test_filter_on_recursive_import` | TODO | `import X::**[@M]` filters recursively - bracket syntax |
| `test_multiple_filters_on_same_import` | TODO | `import X::*[@A][@B]` requires both - bracket syntax |
| `test_filter_preserves_public_visibility` | TODO | `public import X::*[@M]` re-exports filtered |

**Note:** Bracket syntax `[@filter]` on imports requires additional parser/extraction work. The `filter @X;` statement syntax is now working.

### Milestone 2: Boolean Logic in Filters

Tests for `and`, `or`, `not` operators.

| Test Name | Status | Description |
|-----------|--------|-------------|
| `test_filter_with_not_operator` | TODO | `[not @Draft]` excludes drafts |
| `test_filter_with_and_operator` | TODO | `[@A and @B]` requires both metadata |
| `test_filter_with_or_operator` | TODO | `[@A or @B]` requires either metadata |
| `test_filter_complex_boolean` | TODO | `[@A and (@B or @C)]` nested logic |
| `test_filter_not_precedence` | TODO | `[not @A and @B]` = `[(not @A) and @B]` |
| `test_filter_double_negation` | TODO | `[not not @A]` same as `[@A]` |

### Milestone 3: Metadata Attribute Access

Tests for accessing attributes of applied metadata.

| Test Name | Status | Description |
|-----------|--------|-------------|
| `test_filter_boolean_attribute_true` | TODO | `[@M and approved]` where approved=true |
| `test_filter_boolean_attribute_false` | TODO | `[@M and approved]` where approved=false |
| `test_filter_attribute_without_metadata_test` | TODO | `[approved]` implicit metadata context |
| `test_filter_attribute_comparison_greater` | TODO | `[level > 1]` numeric comparison |
| `test_filter_attribute_comparison_equal` | TODO | `[level == 2]` equality check |
| `test_filter_attribute_comparison_less_equal` | TODO | `[level <= 3]` |
| `test_filter_attribute_not_set` | TODO | Attribute not assigned in usage |
| `test_filter_attribute_null_handling` | TODO | `[attr != null]` null checks |

### Milestone 4: Complex Expressions (Full Spec)

Tests for complex filter expressions from SysML spec.

| Test Name | Status | Description |
|-----------|--------|-------------|
| `test_spec_example_approval_filter` | TODO | `[@Approval and approved and level > 1]` |
| `test_filter_string_attribute` | TODO | `[status == "released"]` |
| `test_filter_arithmetic_in_comparison` | TODO | `[level + 1 > 2]` |
| `test_filter_chained_attribute_access` | TODO | `[meta.nested.value]` |
| `test_filter_type_coercion` | TODO | Integer vs Real comparison |

### Milestone 5: Edge Cases & Error Handling

| Test Name | Status | Description |
|-----------|--------|-------------|
| `test_filter_circular_import_with_filter` | TODO | A imports B, B imports A with filters |
| `test_filter_evaluation_error_permissive` | TODO | Bad expression includes element (permissive) |
| `test_filter_on_empty_package` | TODO | Filtering empty package |
| `test_filter_preserves_import_alias` | TODO | `import X::Y as Z [@M]` |
| `test_filter_on_member_import` | TODO | `import X::specific[@M]` |
| `test_multiple_filter_statements` | TODO | `filter @A; filter @B;` stacks |
| `test_filter_statement_vs_bracket_combined` | TODO | Both bracket and statement filter |

---

## Test Template

```rust
#[test]
#[ignore = "filter evaluation not yet implemented - Milestone 1"]
fn test_filter_DESCRIPTION() {
    let source = r#"
        metadata def METADATA_NAME;
        
        package Source {
            part matching { @METADATA_NAME; }
            part nonMatching;
        }
        package Consumer {
            import Source::*[@METADATA_NAME];
        }
    "#;
    let (mut host, _) = analysis_from_sysml(source);
    let analysis = host.analysis();

    // Should be visible (has metadata)
    assert_resolves(analysis.symbol_index(), "Consumer", "matching");
    
    // Should NOT be visible (filtered out)
    assert_not_found(analysis.symbol_index(), "Consumer", "nonMatching");
}
```

---

## Implementation Phases (TDD Order)

### Phase 1: Metadata Index + Simple Presence (~1 week)

**Goal:** Pass Milestone 1 tests

**Remove ignores in order:**
1. `test_filtered_import_excludes_non_matching_elements`
2. `test_filtered_import_with_bracket_syntax`
3. `test_filter_with_no_matching_elements`
4. ... etc

**Implementation:**
1. Add `metadata_annotations: Vec<Arc<str>>` to `HirSymbol`
2. Populate during HIR build from `Meta` relationships
3. Add `filters` field to `Import` AST
4. Parse `filter_package_member` into simple metadata name
5. Check `symbol.metadata_annotations.contains(&filter_name)` in resolution

### Phase 2: Boolean Operators (~3-4 days)

**Goal:** Pass Milestone 2 tests

**Remove ignores in order:**
1. `test_filter_with_not_operator`
2. `test_filter_with_and_operator`
3. `test_filter_with_or_operator`
4. ... etc

**Implementation:**
1. Create `FilterExpr` enum with `And`, `Or`, `Not` variants
2. Parse boolean operators from expression tree
3. Implement `evaluate()` for boolean logic

### Phase 3: Attribute Access (~1 week)

**Goal:** Pass Milestone 3 tests

**Implementation:**
1. Create `AppliedMetadata` with attribute values
2. Populate attribute values from metadata usage body
3. Implement `FeatureAccess` evaluation
4. Implement comparison operators

### Phase 4: Full Expression Evaluation (~1 week)

**Goal:** Pass Milestone 4 & 5 tests

**Implementation:**
1. Full expression parser → FilterExpr
2. Type coercion
3. Error handling
4. Edge cases

---

## Architecture Analysis

### What Exists

| Component | Status | Location |
|-----------|--------|----------|
| Expression Grammar | ✅ Complete | `src/parser/kerml_expressions.pest` |
| Filter Grammar Rules | ✅ Complete | `filter_package`, `filter_package_member` in `.pest` files |
| Expression AST Types | ⚠️ Partial | `src/syntax/kerml/model/types.rs` (defined, not populated) |
| Reference Extraction | ✅ Works | `ExtractedRef` captures names/spans |
| Feature Chain Parsing | ✅ Works | `FeatureChain` structure exists |
| Feature Chain Resolution | ⚠️ Partial | Works for types, not values |
| Metadata Def Parsing | ✅ Works | Parsed as Definition |
| Metadata Usage Parsing | ✅ Works | `@Type` applied to elements |
| Filter Expression Parsing | ✅ Works | `[condition]` syntax parsed |

### What's Missing

| Component | Effort | Notes |
|-----------|--------|-------|
| Filter field on Import AST | Low | Add `filters: Vec<FilterExpression>` to Import structs |
| Expression AST Builder | Medium | Build typed AST from parse tree |
| Expression Evaluator | High | Interpret operators, resolve values |
| Value Representation | Medium | Runtime `Value` enum (Bool, Int, Real, etc.) |
| Metadata Attribute Access | Medium | `@Approval.approved` resolution |
| Applied Metadata Index | Medium | Map elements → their applied metadata |
| Type Checking | Medium | Validate `level > 1` (Natural vs literal) |
| Boolean Logic | Low | Implement `and`, `or`, `not` |
| Comparison Operators | Medium | `>`, `<`, `==` with type handling |

---

## Key Technical Challenges

| Challenge | Complexity | Solution |
|-----------|------------|----------|
| **Feature resolution in expressions** | High | `approved` needs context - implicit metadata context binding |
| **Metadata attribute value extraction** | Medium | Parse `@Approval { approved = true; }` body expressions |
| **Cross-reference resolution** | Medium | `@Safety` must match both short and qualified names |
| **Type coercion** | Medium | `level > 1` comparing Integer attribute to literal |
| **Nested metadata access** | Low-Medium | `@A.@B` patterns (metadata on metadata) |

---

## Expression Context Rules (Per SysML Spec)

When evaluating `[@Approval and approved and level > 1]`:

1. **`@Approval`** - Classification test: does element have `Approval` metadata applied?
2. **`approved`** - Feature access: resolve from the *applied metadata instance*, not the element
3. **`level > 1`** - Comparison: `level` from metadata instance, `1` is literal

The expression context implicitly binds to "the metadata being tested." `approved` isn't a feature of the filtered element—it's a feature of the `@Approval` metadata *applied to* that element.

---

## Files to Modify

### AST Layer
- `src/syntax/sysml/ast/types.rs` - Add `filters` field to `Import`
- `src/syntax/kerml/ast/types.rs` - Add `filters` field to `Import`
- `src/syntax/sysml/ast/parsers.rs` - Parse `filter_package_member` into Import
- `src/syntax/kerml/ast/parsers.rs` - Same for KerML

### New Modules
- `src/eval/mod.rs` - Value, FilterExpr, evaluate()
- `src/eval/expr.rs` - Expression parsing from pest pairs
- `src/hir/metadata.rs` - MetadataIndex, AppliedMetadata

### HIR Layer
- `src/hir/symbols.rs` - Add `metadata_annotations` to `HirSymbol`
- `src/hir/build.rs` - Populate MetadataIndex during HIR construction
- `src/hir/resolve.rs` - Filter evaluation in `process_imports_recursive()`

---

## Progress Tracking

### Current Status
- [ ] Milestone 1: Simple Metadata Presence (0/10 tests passing)
- [ ] Milestone 2: Boolean Logic (0/6 tests passing)
- [ ] Milestone 3: Attribute Access (0/8 tests passing)
- [ ] Milestone 4: Complex Expressions (0/5 tests passing)
- [ ] Milestone 5: Edge Cases (0/7 tests passing)

**Total: 0/36 tests passing**

### Next Action
1. Write all Milestone 1 tests with `#[ignore]`
2. Remove ignore from `test_filtered_import_excludes_non_matching_elements`
3. Implement until that test passes
4. Commit
5. Repeat

---

## References

- SysML v2 Specification §7.5.4 - Filter Package Import
- Grammar: `src/parser/sysml.pest` lines 2544-2554
- Grammar: `src/parser/kerml.pest` lines 1117-1131
