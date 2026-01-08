# Orthos Language Server (orthos-lsp)

A Language Server Protocol implementation for the Orthos constraint language.

## Features

- **Real-time Diagnostics**: Red squiggles appear instantly when constraints are unsatisfiable (UNSAT)
- **Inlay Hints**: Ghost text shows solved values for Flux variables (e.g., `Flux x : Int  = 42`)
- **Parse Error Reporting**: Syntax errors are highlighted as you type

## Architecture

The LSP links directly to `orthos-kernel` and Z3 for maximum performance. No separate daemon process is required.

```
┌─────────────────┐     stdio      ┌─────────────────┐
│   VS Code       │◄──────────────►│   orthos-lsp    │
│   Extension     │                │   (Rust binary) │
└─────────────────┘                └────────┬────────┘
                                            │
                                   ┌────────▼────────┐
                                   │  orthos-kernel  │
                                   │     + Z3        │
                                   └─────────────────┘
```

## Building

```bash
cargo build --release --package orthos-lsp
```

The binary will be at `target/release/orthos-lsp`.

## Usage with VS Code

1. Build the LSP binary
2. Install the orthos-vscode extension
3. The extension auto-detects the LSP binary in `target/release/orthos-lsp`
4. Open any `.orth` file

## Configuration

In VS Code settings:

- `orthos.lspPath`: Path to orthos-lsp binary (auto-detected if empty)
- `orthos.enableLsp`: Enable/disable the language server (default: true)

## LSP Capabilities

| Feature | Method | Description |
|---------|--------|-------------|
| Sync | `textDocument/didOpen`, `didChange` | Full document sync |
| Diagnostics | `textDocument/publishDiagnostics` | UNSAT/parse errors |
| Inlay Hints | `textDocument/inlayHint` | Solved Flux values |

## Example

```orthos
Boundary Math {
    Flux x : Int
    Flux y : Int
    Law Sum : x + y == 100
    Law Positive : x > 0 && y > 0
}
```

With the LSP running, you'll see:
- `Flux x : Int  = 1` (inlay hint showing solved value)
- `Flux y : Int  = 99` (inlay hint showing solved value)

If you add `Law Impossible : x > 100`:
- Red squiggle on the conflicting law
- Diagnostic: "UNSAT: Constraint 'Impossible' contributes to contradiction"
