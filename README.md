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

## Development

### DevContainer Setup (Recommended)

This project includes a DevContainer configuration for a consistent development environment.

**Using VS Code:**
1. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. Open this repository in VS Code
3. Click "Reopen in Container" when prompted (or use Command Palette: "Dev Containers: Reopen in Container")

**What's included:**
- Rust 1.x with toolchain
- rust-analyzer, clippy
- GitHub CLI
- All VS Code extensions pre-configured

### Manual Setup

If not using DevContainer:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build --release
```
