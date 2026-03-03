# Syster Base

Core library for SysML v2 and KerML parsing, AST, semantic analysis, and model interchange.

## Features

- **Parser**: Lossless incremental parser using logos (lexer) and rowan (CST)
- **AST**: Typed wrappers over a lossless Concrete Syntax Tree
- **Incremental Semantic Analysis**: Salsa-powered query system with automatic memoization
- **Name Resolution**: Scope-aware resolver with import and alias handling
- **IDE Features**: Completion, hover, goto-definition, find-references, semantic tokens, and more
- **Standard Library**: Bundled SysML v2 standard library files
- **Interchange** (feature-gated): Export/import via XMI, YAML, JSON-LD, KPAR; programmatic model editing with ID round-trip

## Architecture

### Module Stack

Layers are listed in dependency order — each depends only on the layers below it.

```
ide           → IDE features (completion, hover, goto-def, references, …)
  ↓
hir           → Semantic model with Salsa queries
  ↓
project       → Workspace loading, stdlib resolution
  ↓
syntax        → AST types, formatter, Span/Position
  ↓
parser        → logos lexer, rowan CST, recursive-descent parser
  ↓
base          → Primitives (FileId, Name interning, TextRange)

interchange   → (feature-gated) Model, views, editing, render, format I/O
```

### Parser Pipeline

```
Source Text
    ↓
Lexer (logos) → Token stream with SyntaxKind
    ↓
Parser → GreenNode tree (immutable, cheap to clone)
    ↓
SyntaxNode (rowan) → Lossless CST with parent pointers
    ↓
AST layer → Typed wrappers over SyntaxNode
    ↓
HIR extraction → HirSymbol[] per file
```

The CST preserves all whitespace and comments, enabling exact formatting and incremental reparsing.

### Query Layers (Salsa)

```
file_text(file)           ← INPUT: raw source text
    │
    ▼
parse(file)               ← Parse into AST (memoized per-file)
    │
    ▼
file_symbols(file)        ← Extract HIR symbols (memoized per-file)
    │
    ▼
symbol_index              ← Workspace-wide name index
    │
    ▼
resolve_name(scope, name) ← Name resolution with imports
    │
    ▼
file_diagnostics(file)    ← Semantic errors
```

### Key Types

| Type | Purpose |
|------|---------|
| `FileId` | Interned file identifier (4 bytes, O(1) comparison) |
| `Name` / `Interner` | String interning for efficient symbol comparison |
| `Span`, `Position`, `LineCol` | Source location tracking |
| `TextRange`, `TextSize` | Byte-level ranges (via rowan) |
| `RootDatabase` | Salsa database holding all inputs and query results |
| `HirSymbol` | Symbol extracted from the AST (name, kind, span, relationships) |
| `SymbolIndex` | Workspace-wide index mapping names → symbols |
| `Resolver` | Name resolution with import and alias handling |
| `DefId` | Globally unique definition ID |
| `Diagnostic` | Semantic error/warning with source location and severity |
| `AnalysisHost` | Mutable owner of the database; call `.analysis()` for read-only snapshot |

## Modules

| Module | Description |
|--------|-------------|
| `base` | Foundation types: `FileId`, `Name`, `Interner`, `TextRange` |
| `parser` | logos lexer, rowan CST, recursive-descent parser, grammar traits |
| `syntax` | AST types, formatter, `Span`, `Position`, `ParseError` |
| `project` | File/workspace/stdlib loading |
| `hir` | Salsa-based semantic model: queries, symbols, resolution, diagnostics |
| `ide` | IDE features: completion, goto, hover, references, semantic tokens, inlay hints, folding, selection |
| `interchange` | (feature-gated) Standalone model, format I/O, views, editing, rendering, metadata |

### Interchange Submodules

Enabled with `cargo build --features interchange`. See `ARCHITECTURE.md` for detailed diagrams.

| Submodule | Purpose |
|-----------|---------|
| `model` | `Model`, `Element`, `ElementId`, `ElementKind`, `Relationship` — standalone model graph |
| `views` | Zero-copy typed views (`ElementView`, `PackageView`, `DefinitionView`, …) |
| `host` | `ModelHost` — parse text or load XMI → typed queries via views |
| `editing` | `ChangeTracker` — mutation API (rename, add, remove, reparent) with dirty tracking |
| `render` | `SourceMap` + `render_dirty()` — incremental re-rendering of changed regions |
| `decompile` | Model → SysML text + metadata |
| `recompile` | Restore original element IDs from metadata when re-exporting |
| `metadata` | `ImportMetadata`, `ProjectMetadata` — companion JSON for ID round-trip |
| `integrate` | Bridge `Model` ↔ `RootDatabase`: `model_from_symbols()`, `symbols_from_model()` |
| `xmi` | XMI (OMG XML Metadata Interchange) reader/writer |
| `yaml` | YAML format reader/writer |
| `jsonld` | JSON-LD format reader/writer |
| `kpar` | KPAR (Kernel Package Archive) reader/writer |

## Usage

```rust
use syster::hir::{RootDatabase, FileText, parse_file, file_symbols};
use syster::base::FileId;

// Create the Salsa database
let db = RootDatabase::new();

// Set file content (input query)
let file_id = FileId::new(0);
let file_text = FileText::new(&db, file_id, r#"
    package Vehicle {
        part def Car {
            attribute mass : Real;
        }
    }
"#.to_string());

// Parse (memoized — subsequent calls are instant)
let parse_result = parse_file(&db, file_text);
assert!(parse_result.is_ok());

// Extract symbols (also memoized)
if let Some(ast) = parse_result.get_ast() {
    let symbols = file_symbols(file_id, ast);
    // symbols contains: Vehicle (package), Car (part def), mass (attribute)
}
```

### Using the IDE Layer

```rust
use syster::ide::AnalysisHost;

let mut host = AnalysisHost::new();
host.set_file_content("test.sysml", "package Test { part def A; }");

let analysis = host.analysis();  // read-only snapshot
let symbols = analysis.document_symbols(file_id);
let completions = analysis.completions(file_id, position);
```

### Using the Interchange Layer

```rust
use syster::interchange::{Xmi, ModelFormat, ModelHost};

// Load from XMI
let model = Xmi.read(&xmi_bytes)?;

// Or build from SysML text
let host = ModelHost::from_text("package P { part def A; }")?;
for view in host.root_views() {
    println!("{:?}", view.name());
}
```

## Performance

The architecture provides significant performance benefits:

- **Incremental parsing**: rowan green nodes are immutable and shared — only changed subtrees are re-parsed
- **Memoized queries**: Salsa caches all query results; invalidation is automatic and minimal
- **O(1) comparisons**: Interned `FileId` and `Name` enable constant-time equality
- **Reduced allocations**: String interning shares storage across the codebase
- **Incremental rendering**: `render_dirty()` patches only changed model regions

## Building

```bash
# Core library only
cargo build
cargo test

# With interchange support (decompiler, XMI, YAML, JSON-LD, KPAR)
cargo build --features interchange
cargo test --features interchange
```

## Testing

```bash
# All tests (lib + integration, with interchange)
cargo test --features interchange

# Library tests only (425 tests — parser, HIR, IDE, interchange internals)
cargo test --features interchange --lib

# Integration tests only (226 tests — editing, roundtrip, decompiler coverage)
cargo test --test test_editing_integration --features interchange

# Run a specific integration test by name
cargo test --test test_editing_integration --features interchange decompile_drops_anonymous_usage

# Run a group of tests by prefix
cargo test --test test_editing_integration --features interchange decompile_

# Core tests without interchange
cargo test
```

## Linting & Formatting

```bash
# Format code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check

# Clippy with interchange
cargo clippy --all-targets --features interchange -- -D warnings

# Full validation pipeline (format + lint + test)
make run-guidelines
```

## License

MIT
