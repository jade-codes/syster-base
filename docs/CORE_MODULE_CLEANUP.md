# Core Module Cleanup Plan

## Status: ✅ COMPLETED (January 23, 2026)

The `core/` module has been fully eliminated. All useful code has been relocated to the new architecture.

---

## Final Module Structure

```
base/           (foundation - no dependencies on other syster modules)
  ├── constants.rs    - File extensions, relationship types, roles
  ├── file_id.rs      - FileId interning
  ├── intern.rs       - Name/Interner for string interning
  ├── position.rs     - Position, Span (line/column based)
  └── span.rs         - TextRange, LineCol, LineIndex (byte-offset based)

parser/         (depends on: base)
  ├── file_io.rs      - File loading utilities
  ├── result.rs       - ParseResult, ParseError
  ├── keywords.rs     - Language keywords
  ├── kerml.rs        - KerML pest parser
  └── sysml.rs        - SysML pest parser

syntax/         (depends on: base, parser)
  ├── traits.rs       - AstNode, Named, ToSource
  ├── span.rs         - Re-exports Position, Span from base
  ├── kerml/          - KerML AST types
  └── sysml/          - SysML AST types

hir/            (depends on: base, parser, syntax)
  └── diagnostics.rs  - Diagnostic codes (merged from error_codes.rs)

ide/            (depends on: base, parser, syntax, hir)
  └── text_utils.rs   - Text manipulation utilities

project/        (depends on: base, parser, syntax, hir, ide)
  └── ...             - Workspace loading, stdlib
```

---

## Migration Summary

### Deleted (Unused)
- `core/interner.rs` - Replaced by `base/intern.rs`
- `core/events.rs` - No longer needed (Salsa handles invalidation)
- `core/operation.rs` - No longer needed (Salsa pattern)

### Relocated
| From | To |
|------|-----|
| `core/constants.rs` | `base/constants.rs` |
| `core/span.rs` (Position, Span) | `base/position.rs` |
| `core/file_io.rs` | `parser/file_io.rs` |
| `core/parse_result.rs` | `parser/result.rs` |
| `core/traits.rs` | `syntax/traits.rs` |
| `core/text_utils.rs` | `ide/text_utils.rs` |

### Merged
| From | Into |
|------|------|
| `core/error_codes.rs` | `hir/diagnostics.rs` (codes module) |

---

## Architecture Tests

All architecture layer tests now pass with the new structure:

- `base` → no dependencies (only std + external crates)
- `parser` → only base
- `syntax` → base, parser
- `hir` → base, parser, syntax  
- `ide` → base, parser, syntax, hir
- `project` → base, parser, syntax, hir, ide

---

## Phase 1: Immediate Removals

These files have **no active usage** in `syster-base/src/` (only in archived code or the separate monorepo):

### 1.1 Delete `core/interner.rs`

**Reason**: Fully replaced by `base/intern.rs`

| Old (`core/interner.rs`) | New (`base/intern.rs`) |
|--------------------------|------------------------|
| `IStr` = `Rc<str>` | `Name` = `u32` handle |
| `Interner` with `HashSet<Rc<str>>` | `Interner` with `FxHashMap` + `Vec<SmolStr>` |
| Not thread-safe | Thread-safe (`RwLock`) |
| Variable-size clones | O(1) comparison, 4-byte copies |

### 1.2 Delete `core/events.rs`

**Reason**: The event system was designed for eager/mutable population. With Salsa queries, we get automatic invalidation and don't need manual event emission.

**Old pattern**:
```rust
// Emit event when symbol added
self.events.emit(SymbolAdded { id }, self);
```

**New pattern** (Salsa):
```rust
// Just set input - Salsa handles invalidation
db.set_file_text(file, text);
// Queries automatically recompute
```

### 1.3 Delete `core/operation.rs`

**Reason**: The `OperationResult<T, E, Ev>` pattern was middleware for combining results with events. No longer needed with Salsa's declarative model.

---

## Phase 2: Relocate Active Files

### 2.1 `core/constants.rs` → `parser/constants.rs`

**Current users**:
- `syntax/parser.rs` - `KERML_EXT`, `SYSML_EXT`
- `project/stdlib_loader.rs` - `STDLIB_DIR`
- `project/file_loader/collection.rs` - `SUPPORTED_EXTENSIONS`

**Action**: Move to `parser/` since file extensions are primarily parser concerns.

**Migration**:
```rust
// Before
use crate::core::constants::{KERML_EXT, SYSML_EXT};

// After  
use crate::parser::constants::{KERML_EXT, SYSML_EXT};
```

### 2.2 `core/file_io.rs` → `project/file_io.rs`

**Current users**:
- `syntax/parser.rs`, `syntax/sysml/parser.rs`, `syntax/kerml/parser.rs`
- `project/file_loader/parsing.rs`

**Action**: Move to `project/` - file I/O is a project-level concern.

**Migration**:
```rust
// Before
use crate::core::{load_file, validate_extension, get_extension};

// After
use crate::project::{load_file, validate_extension, get_extension};
```

### 2.3 `core/parse_result.rs` → `parser/result.rs`

**Current users**:
- `syntax/*.rs` parsers
- `project/mod.rs` (re-exports)

**Action**: Move to `parser/` - parse results belong with the parser.

**Note**: `hir/db.rs` has its own `ParseResult` for Salsa queries. These serve different purposes:
- `parser::ParseResult<T>` - Raw parser output with legacy `Position`
- `hir::ParseResult` - Salsa-tracked parse result

Consider eventually unifying these.

### 2.4 `core/traits.rs` → `syntax/traits.rs`

**Current users**:
- `syntax/sysml/ast/tests/tests_ast.rs`

**Action**: Move to `syntax/` - AST traits belong with syntax definitions.

**Contents**:
```rust
pub trait AstNode: fmt::Debug + Clone {
    fn node_type(&self) -> &'static str;
    fn has_children(&self) -> bool { false }
}

pub trait Named {
    fn name(&self) -> Option<&str>;
}

pub trait ToSource {
    fn to_source(&self) -> String;
}
```

### 2.5 `core/text_utils.rs` → `ide/text_utils.rs`

**Current users**:
- LSP integration tests
- Archived stdlib tests

**Action**: Move to `ide/` - text manipulation utilities are for IDE features.

**Key functions**:
- `extract_word_at_cursor(line, position)` - Get identifier at position
- `extract_qualified_name_at_cursor(line, position)` - Get qualified name (e.g., `Pkg::Type`)
- `find_word_boundaries(chars, position)` - Find word start/end

### 2.6 `core/error_codes.rs` → Merge into `hir/diagnostics.rs`

**Current state**:
- `core/error_codes.rs` defines constants like `SEMANTIC_UNDEFINED_REFERENCE = "E002"`
- `hir/diagnostics.rs` has `Diagnostic` struct with `code: Option<Arc<str>>`

**Action**: Merge error code constants into `hir/diagnostics.rs` to have a single source of truth for diagnostic codes.

---

## Phase 3: Span Type Migration

### Problem

Two span systems exist:

**Old** (`core/span.rs`):
```rust
pub struct Span {
    pub start: Position,
    pub end: Position,
}

pub struct Position {
    pub line: usize,    // 0-indexed
    pub column: usize,  // 0-indexed
}
```

**New** (`base/span.rs`):
```rust
pub use text_size::TextRange;  // Byte offsets
pub use text_size::TextSize;

pub struct LineCol {
    pub line: u32,  // 0-indexed
    pub col: u32,   // 0-indexed (bytes)
}

pub struct LineIndex { /* converts between TextSize and LineCol */ }
```

### Migration Strategy

1. **Keep `core/span.rs` temporarily** with deprecation warning
2. **Add `Position` compatibility** in `base/span.rs`:
   ```rust
   /// Compatibility alias for legacy code
   #[deprecated(note = "Use LineCol instead")]
   pub type Position = LineCol;
   ```
3. **Gradually migrate** `syntax/` AST nodes from `Span` to `TextRange`
4. **Delete `core/span.rs`** once all usages are migrated

### Files to Update

- `syntax/sysml/ast/types.rs`
- `syntax/sysml/ast/parsers.rs`
- `syntax/kerml/ast/types.rs`
- `syntax/kerml/ast/parsers.rs`
- `syntax/kerml/ast/utils.rs`
- `syntax/kerml/model/types.rs`
- `syntax/normalized.rs`

---

## Phase 4: Final Cleanup

### 4.1 Update `core/mod.rs`

After each phase, update to remove deleted/moved modules.

### 4.2 Update `lib.rs`

Remove `pub mod core;` and update re-exports:

```rust
// Before
pub mod core;
pub use core::{ParseError, ParseErrorKind, ParseResult};

// After
pub use parser::{ParseError, ParseErrorKind, ParseResult};
```

### 4.3 Delete `core/` Directory

Once all files are migrated and all imports updated.

### 4.4 Update Tests

- Move `core/tests/` contents to appropriate module test directories
- Update test imports

---

## Execution Checklist

### Phase 1: Deletions
- [ ] Delete `src/core/interner.rs`
- [ ] Delete `src/core/events.rs`
- [ ] Delete `src/core/operation.rs`
- [ ] Update `src/core/mod.rs` to remove deleted modules
- [ ] Run `cargo check` to verify no breakage

### Phase 2: Relocations
- [ ] Move `constants.rs` → `parser/constants.rs`
- [ ] Move `file_io.rs` → `project/file_io.rs`
- [ ] Move `parse_result.rs` → `parser/result.rs`
- [ ] Move `traits.rs` → `syntax/traits.rs`
- [ ] Move `text_utils.rs` → `ide/text_utils.rs`
- [ ] Merge `error_codes.rs` → `hir/diagnostics.rs`
- [ ] Update all import paths
- [ ] Run `cargo check` and `cargo test`

### Phase 3: Span Migration
- [ ] Add `Position` compatibility to `base/span.rs`
- [ ] Migrate `syntax/sysml/ast/` to use `TextRange`
- [ ] Migrate `syntax/kerml/ast/` to use `TextRange`
- [ ] Delete `core/span.rs`
- [ ] Run full test suite

### Phase 4: Cleanup
- [ ] Delete `src/core/` directory
- [ ] Update `lib.rs` exports
- [ ] Update documentation
- [ ] Run `cargo doc` to verify docs build

---

## Dependency Graph (After Migration)

```
┌─────────────────────────────────────────────────────────────────┐
│                           lib.rs                                 │
└─────────────────────────────────────────────────────────────────┘
        │
        ├── base/           (foundation: FileId, Name, TextRange, LineCol)
        │
        ├── parser/         (pest grammars, ParseResult, constants)
        │     └── uses: base
        │
        ├── syntax/         (AST types, traits)
        │     └── uses: base, parser
        │
        ├── hir/            (Salsa queries, diagnostics, symbols)
        │     └── uses: base, parser, syntax
        │
        ├── ide/            (completion, hover, text_utils)
        │     └── uses: base, hir
        │
        └── project/        (file loading, stdlib)
              └── uses: base, parser
```

---

## Timeline Estimate

| Phase | Effort | Risk |
|-------|--------|------|
| Phase 1: Deletions | 30 min | Low (no dependencies) |
| Phase 2: Relocations | 2-3 hours | Medium (import updates) |
| Phase 3: Span Migration | 4-6 hours | High (AST changes) |
| Phase 4: Cleanup | 30 min | Low |

**Total**: ~1 day of focused work

---

*Document Version: 1.0*  
*Created: January 23, 2026*
