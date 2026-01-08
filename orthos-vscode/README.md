# Orthos VS Code Extension

**Constraint-Topology Description Language** support for Visual Studio Code.

## Features

- **Syntax Highlighting** for `.orth` files
- **Daemon Integration** - Spawn and communicate with `orthos-daemon`
- **Interactive Commands**:
  - `Orthos: Initialize Universe` - Load the current file into the daemon
  - `Orthos: Create Checkpoint` - Save solver state for hypothetical reasoning
  - `Orthos: Restore Checkpoint` - Rollback to last checkpoint
  - `Orthos: Add Constraint` - Dynamically add constraints
  - `Orthos: Query Flux` - Query variable values from the solver
  - `Orthos: Shutdown Daemon` - Stop the daemon process

## Requirements

- `orthos-daemon` binary (build from `orthos-daemon/` with `cargo build --release`)

## Installation

1. Build the daemon:
   ```bash
   cd orthos-daemon
   cargo build --release
   ```

2. Install the extension:
   ```bash
   cd orthos-vscode
   npm install
   npm run compile
   ```

3. Open VS Code in the orthos workspace and press F5 to launch Extension Development Host

## Usage

1. Open any `.orth` file
2. The extension auto-initializes (or run `Orthos: Initialize Universe`)
3. Use the command palette (Ctrl+Shift+P) to access Orthos commands
4. Check the **Orthos** output channel for logs
5. Status bar shows current state: SAT, UNSAT, Checkpoint, etc.

## Settings

- `orthos.daemonPath`: Path to `orthos-daemon` binary (auto-detected if empty)
- `orthos.autoInitialize`: Auto-initialize when opening `.orth` files (default: true)

## The Paradigm

Orthos is not a programming language. It's a **Constraint-Topology Description Language**.

```orthos
Boundary SafeMath {
    Flux numerator : Int
    Flux denominator : Int
    Flux result : Int
    
    Law NonZero : denominator != 0
    Law Relation : result * denominator == numerator
}
```

The daemon doesn't "execute" code - it maintains a satisfiable universe of constraints.
