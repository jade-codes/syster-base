# Syster Base

Core library for SysML v2 and KerML parsing, AST, and semantic analysis.

## Features

- **Parser**: Pest-based grammar for SysML v2 and KerML
- **AST**: Complete abstract syntax tree types
- **Semantic Analysis**: Symbol table, reference resolution, import handling
- **Formatter**: Rowan-based code formatter
- **Standard Library**: SysML v2 standard library files

## Usage

```rust
use syster_base::{Workspace, parse_sysml};

let mut workspace = Workspace::new();
workspace.add_file("model.sysml", r#"
    package Vehicle {
        part def Car;
    }
"#);
workspace.populate_all();

// Access symbols
let symbols = workspace.symbol_table();
```

## Modules

- `parser` - Pest grammars and parsing
- `syntax` - AST types for KerML and SysML
- `semantic` - Symbol table, resolver, workspace
- `project` - File loading utilities

## License

MIT
