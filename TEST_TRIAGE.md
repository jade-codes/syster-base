# Test Triage Report

Generated: 2026-01-30

## Summary

- **Active test files:** 32
- **Archived test files:** 14  
- **Currently failing:** 8 test files
- **Library tests:** 253 passing

---

## FAILING TEST FILES (8)

### Category A: Wrong argument count to `extract_symbols_unified` (5 files)

These tests call `extract_symbols_unified(&syntax_file)` but it now requires `extract_symbols_unified(FileId, &syntax_file)`.

| File | Error |
|------|-------|
| `test_debug_hover_transition.rs:57` | mismatched types - missing FileId argument |
| `test_debug_vehicle_transition.rs:57` | mismatched types - missing FileId argument |
| `test_debug_position_479.rs:64` | mismatched types - missing FileId argument |
| `test_debug_position_524.rs:122` | mismatched types - missing FileId argument |
| `test_debug_visibility_maps.rs` | Also calls private method `resolve_feature_chain_member` |

**Fix:** Add `FileId::new(0)` as first argument.

---

### Category B: Uses private `resolve` module methods (2 files)

| File | Error |
|------|-------|
| `test_debug_vehicle_structure.rs:105` | `resolve_feature_chain_member` is private |
| `test_debug_visibility_maps.rs:175` | `resolve_feature_chain_member` is private |

**Options:**
1. Make method public
2. Archive tests
3. Rewrite tests to use public API

---

### Category C: Completely broken API usage (2 files)

#### `test_debug_vehicle_chain.rs` (9 errors)
- `SyntaxFile::from_parse()` doesn't exist (use `parse_content()` which returns SyntaxFile directly)
- `extract_symbols_unified()` wrong arg count
- `SymbolIndex::insert()` doesn't exist
- `SymbolIndex::visibility_maps()` doesn't exist (use `ensure_visibility_maps()`)
- Uses `resolve::find_type_ref_at_position` - private module

#### `test_hover_resolution.rs` (1 error)
- `Definition::members()` doesn't exist on rowan AST

**Options:**
1. Rewrite completely
2. Archive

---

## ARCHIVED TEST FILES (14)

These use the old **pest parser** which has been replaced by **rowan**.

| File | Reason |
|------|--------|
| `tests_parser_kerml_pest.rs` | Uses `pest::Parser`, `KerMLParser::parse(Rule::...)` |
| `tests_parser_sysml_pest.rs` | Uses `pest::Parser`, `SysMLParser::parse(Rule::...)` |
| `tests_parser_kerml_ast.rs` | Uses old AST types like `DefinitionKind::Classifier` |
| `tests_parser_sysml_ast.rs` | Uses old AST types |
| `tests_parser_expression_rowan.rs` | Uses `rule_parser` module that doesn't exist |
| `tests_parser_kerml_span.rs` | Uses `syster::syntax::kerml::ast::KerMLFile` |
| `tests_kerml_examples.rs` | Uses `KerMLParser`, `kerml::Rule` |
| `tests_kerml_import_detection.rs` | Uses `kerml::KerMLParser`, `kerml::ast::enums` |
| `tests_multiple_packages.rs` | Uses `syster::syntax::sysml::parser::parse_content` |
| `tests_core_parse_result.rs` | Uses `ParseErrorKind` that doesn't exist |
| `test_debug_extended_usage.rs` | Uses pest `SysMLParser`, `sysml::Rule` |
| `test_debug_typed_by_span.rs` | Uses pest `SysMLParser`, `sysml::ast` |
| `test_debug_typed_by_span2.rs` | Uses pest `SysMLParser`, `sysml::ast` |

**Decision needed:** Delete permanently or keep archived for reference?

---

## ALSO ARCHIVED (in subdirectories)

### `tests/parser/mod.rs` - commented out:
- `tests_constraint.rs` - pest parser
- `tests_expression.rs` - pest parser  
- `tests_kerml.rs` - pest parser
- `tests_kerml_stdlib.rs` - pest parser
- `tests_sysml.rs` - pest parser
- `tests_usage_body.rs` - pest parser

### `tests/syntax/mod.rs` - commented out:
- `tests_kerml_ast.rs` - pest AST types
- `tests_sysml_ast.rs` - pest AST types

---

## RECOMMENDED ACTIONS

### Quick wins (5 files, ~5 line changes each):
Fix `extract_symbols_unified` call to include `FileId::new(0)`:
- `test_debug_hover_transition.rs`
- `test_debug_vehicle_transition.rs`
- `test_debug_position_479.rs`
- `test_debug_position_524.rs`

### Decisions needed:

1. **Private method access** (`resolve_feature_chain_member`):
   - Make public? Or archive `test_debug_vehicle_structure.rs` and `test_debug_visibility_maps.rs`?

2. **Completely broken tests**:
   - Archive `test_debug_vehicle_chain.rs` and `test_hover_resolution.rs`?
   - Or rewrite them?

3. **Archived pest tests**:
   - Delete permanently?
   - Move to `tests_archived/` folder?
   - Keep as `.archived` suffix?
