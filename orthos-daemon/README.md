# ORTHOS Daemon (The Nervous System)

**Phase 2 of Project ORTHOS** - A long-running Oracle Server for incremental constraint solving.

## Overview

`orthos-daemon` transforms the ORTHOS Kernel from a one-shot CLI tool into a persistent runtime service. It maintains a Z3 solver context across requests, enabling:

- **Incremental Solving**: Add constraints without re-parsing the entire program
- **Hypothetical Reasoning**: Checkpoint/restore for "what-if" scenarios
- **Auto-Rollback**: Failed constraints automatically revert to preserve universe stability

## Protocol

- **Transport**: Stdio (line-delimited JSON)
- **Format**: JSON-RPC style messages

## API Methods

### 1. `initialize`

Load an ORTHOS program and create the universe.

**Request:**
```json
{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int Law Positive : x > 0 }"}, "id": 1}
```

**Response (Success):**
```json
{"result": {"status": "READY", "flux_list": ["Main.x"]}, "id": 1}
```

**Response (Ontological Error):**
```json
{"error": {"code": -1, "message": "Ontological Error", "core": ["Law: Contradiction"]}, "id": 1}
```

### 2. `checkpoint`

Save the current universe state (calls `solver.push()`).

**Request:**
```json
{"method": "checkpoint", "id": 2}
```

**Response:**
```json
{"result": "OK", "id": 2}
```

### 3. `constrain`

Add new constraints to the universe. Auto-rollbacks on UNSAT.

**Request:**
```json
{"method": "constrain", "params": {"laws": ["Main.x > 10", "Main.x < 100"]}, "id": 3}
```

**Response (SAT):**
```json
{"result": {"status": "SAT"}, "id": 3}
```

**Response (UNSAT - auto-rolled back):**
```json
{"result": {"status": "UNSAT", "core": ["Main.Positive", "Dynamic: Main.x < 0"]}, "id": 3}
```

### 4. `query`

Get current values of Flux variables from the model.

**Request:**
```json
{"method": "query", "params": {"flux": ["Main.x", "Main.y"]}, "id": 4}
```

**Response:**
```json
{"result": {"values": {"Main.x": 42, "Main.y": 0}}, "id": 4}
```

### 5. `restore`

Revert to the last checkpoint (calls `solver.pop()`).

**Request:**
```json
{"method": "restore", "id": 5}
```

**Response:**
```json
{"result": "OK", "id": 5}
```

### 6. `shutdown`

Terminate the daemon.

**Request:**
```json
{"method": "shutdown", "id": 99}
```

## Usage

### Build
```bash
cargo build --package orthos-daemon
```

### Run
```bash
./target/debug/orthos-daemon
```

### Example Session
```bash
echo '{"method": "initialize", "params": {"source": "Boundary Main { Flux x : Int }"}, "id": 1}
{"method": "constrain", "params": {"laws": ["Main.x == 42"]}, "id": 2}
{"method": "query", "params": {"flux": ["Main.x"]}, "id": 3}
{"method": "shutdown", "id": 99}' | ./target/debug/orthos-daemon
```

Output:
```json
{"result":{"flux_list":["Main.x"],"status":"READY"},"id":1}
{"result":{"status":"SAT"},"id":2}
{"result":{"values":{"Main.x":42}},"id":3}
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    orthos-daemon                        │
├─────────────────────────────────────────────────────────┤
│  stdin ──► JSON Parser ──► Dispatcher                   │
│                              │                          │
│                              ▼                          │
│                        DaemonState                      │
│                    ┌─────────────────┐                  │
│                    │  Z3 Context     │                  │
│                    │  (leaked 'static)│                 │
│                    ├─────────────────┤                  │
│                    │  Solver         │◄── push/pop     │
│                    │  int_vars       │                  │
│                    │  bool_vars      │                  │
│                    │  law_names      │                  │
│                    └─────────────────┘                  │
│                              │                          │
│                              ▼                          │
│  stdout ◄── JSON Serializer ◄── Response               │
└─────────────────────────────────────────────────────────┘
```

## Constraint Syntax

Dynamic constraints use dot notation for variable references:
- `Main.x > 10` - Variable `x` in boundary `Main`
- `Main.x + Main.y == 100` - Arithmetic expressions
- `Main.x > 0 && Main.y < 10` - Logical operators

## Next Steps (Phase 3)

- **`universe.simulate(duration)`**: Temporal unrolling (currently stubbed)
- **VS Code Extension**: Real-time constraint visualization
- **TCP Transport**: Network access for distributed agents
