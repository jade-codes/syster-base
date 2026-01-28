# Interchange Integration Plan

This document outlines the implementation plan for adding import/export commands to both the LSP (syster-lsp) and CLI (syster-cli) tools.

## Overview

The `syster-base` crate has a new `interchange` module (feature-gated) that supports:
- **XMI** - XML Model Interchange (OMG standard)
- **KPAR** - Kernel Package Archive (ZIP-based)
- **JSON-LD** - JSON for Linked Data (OMG API compatible)

The module provides a standalone `Model` type with `Element`, `Relationship`, etc., and a `ModelFormat` trait with `read_model()` and `write_model()` methods.

**Current Status:** 30 tests passing for the interchange module.

---

## Phase 1: Integration Layer (Foundation)

**File:** `base/src/interchange/integrate.rs` (new)

### Functions

| Function | Signature | Purpose |
|----------|-----------|---------|
| `model_from_database` | `(&RootDatabase) -> Model` | Extract all elements/relationships from HIR into standalone Model |
| `model_to_files` | `(&Model) -> Vec<(PathBuf, String)>` | Generate SysML source files from Model (lossy - no comments/formatting) |

### Mapping Strategy

| HIR Type | Interchange Type |
|----------|------------------|
| `hir::Element` | `interchange::Element` |
| `hir::Package` | `ElementKind::Package` |
| `hir::PartDef` | `ElementKind::PartDefinition` |
| `hir::PortDef` | `ElementKind::PortDefinition` |
| Relationship graphs | `Relationship` entries |

### Dependencies
- Requires access to `RootDatabase` queries
- Maps HIR element IDs to `ElementId` (UUID generation)

---

## Phase 2: CLI Commands

**Files:**
- `cli/src/main.rs` - Add subcommand registration
- `cli/src/export.rs` (new) - Export implementation
- `cli/src/import.rs` (new) - Import implementation

### Commands

```bash
# Export SysML model to interchange format
syster export <input.sysml> --format xmi|kpar|jsonld -o output.xmi

# Import and validate interchange file
syster import <input.xmi> --validate
```

### Export Flow

```
Parse SysML files
    ↓
Build RootDatabase
    ↓
model_from_database()
    ↓
Xmi/Kpar/JsonLd::write_model()
    ↓
Write output file
```

### Import Flow

```
Read input file
    ↓
Xmi/Kpar/JsonLd::read_model()
    ↓
Validate Model structure
    ↓
Print diagnostics / statistics
```

---

## Phase 3: LSP Commands

**Files:**
- `language-server/crates/syster-lsp/src/server/interchange.rs` (new)
- `language-server/crates/syster-lsp/src/server.rs` - Register handlers

### Custom Requests

| Request | Parameters | Response |
|---------|------------|----------|
| `syster/exportModel` | `{ format: "xmi" \| "kpar" \| "jsonld" }` | `{ data: bytes, filename: string }` |
| `syster/importModel` | `{ uri: string, format?: string }` | `{ success: bool, diagnostics: [] }` |

### Implementation Pattern

Uses existing `async-lsp` custom request pattern:

```rust
#[derive(Serialize, Deserialize)]
struct ExportModelParams {
    format: String,
}

#[derive(Serialize, Deserialize)]
struct ExportModelResult {
    data: Vec<u8>,
    filename: String,
}
```

---

## Phase 4: Feature Flags & Dependencies

### Cargo.toml Updates

**cli/Cargo.toml:**
```toml
[dependencies]
syster-base = { path = "../base", features = ["interchange"] }
```

**language-server/crates/syster-lsp/Cargo.toml:**
```toml
[dependencies]
syster-base = { path = "../../../base", features = ["interchange"] }
```

---

## Phase 5: Integration Tests

### Test Categories

| Category | Location | Description |
|----------|----------|-------------|
| Integration layer | `base/src/interchange/integrate.rs` | Roundtrip: Database → Model → Database |
| CLI export | `cli/tests/` | Export SysML to all formats |
| CLI import | `cli/tests/` | Import and validate all formats |
| LSP export | `language-server/crates/syster-lsp/tests/` | Custom request handling |
| LSP import | `language-server/crates/syster-lsp/tests/` | Custom request handling |

### Roundtrip Test Strategy

```
SysML source
    ↓
Parse → Database
    ↓
model_from_database()
    ↓
Xmi::write_model()
    ↓
Xmi::read_model()
    ↓
Compare Model elements/relationships
```

---

## Design Decisions

### Import Fidelity

Converting `Model` back to textual SysML is **lossy** (no original formatting, comments, or whitespace preserved).

**Options:**
- **A) Export only** - No file regeneration, simplest approach
- **B) Generate minimal SysML** - Create valid but minimal textual representation
- **C) XMI round-trip only** - No textual SysML output, XMI ↔ XMI only

**Decision:** Start with **Option A** (export only) + validation for imports. Add file generation in a later phase if needed.

### Scope: File vs Workspace

- **CLI:** Operates on specified files/directories
- **LSP:** Exports entire workspace by default, with optional file filter parameter

### Error Handling

- Invalid format → `InterchangeError::InvalidFormat`
- Parse errors → `InterchangeError::ParseError` with location info
- Missing elements → `InterchangeError::MissingElement` with ID

---

## Implementation Order (TDD)

Following **Test-Driven Development** - write failing tests first, then implement.

### Phase 1: Integration Layer

1. **Write test:** `test_model_from_database_empty` - empty database → empty model
2. **Implement:** Minimal `model_from_database()` to pass
3. **Write test:** `test_model_from_database_single_package` - one package
4. **Implement:** Package extraction logic
5. **Write test:** `test_model_from_database_with_parts` - package with part definitions
6. **Implement:** Part definition extraction
7. **Write test:** `test_model_from_database_relationships` - specialization/typing
8. **Implement:** Relationship extraction
9. **Refactor** while keeping tests green

### Phase 2: CLI Export

1. **Write test:** `test_cli_export_xmi` - export to XMI format
2. **Implement:** Export subcommand with XMI support
3. **Write test:** `test_cli_export_kpar` - export to KPAR format
4. **Implement:** KPAR support
5. **Write test:** `test_cli_export_jsonld` - export to JSON-LD format
6. **Implement:** JSON-LD support
7. **Refactor**

### Phase 3: CLI Import

1. **Write test:** `test_cli_import_validates_xmi` - import and validate XMI
2. **Implement:** Import subcommand with validation
3. **Write test:** `test_cli_import_invalid_file` - error handling
4. **Implement:** Error reporting
5. **Refactor**

### Phase 4: LSP Commands

1. **Write test:** `test_lsp_export_model_request` - export request handler
2. **Implement:** `syster/exportModel` handler
3. **Write test:** `test_lsp_import_model_request` - import request handler
4. **Implement:** `syster/importModel` handler
5. **Refactor**

### TDD Cycle Reminder

```
┌─────────────────────────────────────┐
│  1. Write a failing test            │
│           ↓                         │
│  2. Run test - confirm it fails     │
│           ↓                         │
│  3. Write minimal code to pass      │
│           ↓                         │
│  4. Run test - confirm it passes    │
│           ↓                         │
│  5. Refactor (keep tests green)     │
│           ↓                         │
│  6. Repeat                          │
└─────────────────────────────────────┘
```

**Rules:**
- One function at a time
- Small changes (< 15 lines when possible)
- Stop if modifying multiple files simultaneously
- Break down large tasks into smaller steps

---

## Todo Checklist

- [ ] Create integration layer (`base/src/interchange/integrate.rs`)
- [ ] Add CLI export subcommand
- [ ] Add CLI import subcommand
- [ ] Add LSP export command (`syster/exportModel`)
- [ ] Add LSP import command (`syster/importModel`)
- [ ] Update feature flags in Cargo.toml files
- [ ] Write integration tests

---

## References

- [Interchange Module](../src/interchange/mod.rs) - Public API
- [Model Type](../src/interchange/model.rs) - Standalone model representation
- [XMI Format](../src/interchange/xmi.rs) - XML Model Interchange
- [KPAR Format](../src/interchange/kpar.rs) - Kernel Package Archive
- [JSON-LD Format](../src/interchange/jsonld.rs) - JSON for Linked Data
