# ORTHOS Kernel v0.1

**A Constraint-Topology Description Language**

ORTHOS is not an imperative programming language. It is a constraint solver that defines state spaces and solves for valid histories.

- **Conventional Model:** `Input + Algorithm = Output`
- **ORTHOS Model:** `Constraint + Topology = Manifestation`

## Building

### Prerequisites

- Rust 1.70+ 
- Z3 Theorem Prover (libz3-dev on Ubuntu/Debian)

```bash
# Ubuntu/Debian
sudo apt-get install libz3-dev

# macOS
brew install z3

# Arch Linux
sudo pacman -S z3
```

### Compile

```bash
cd orthos-kernel
cargo build --release
```

## Usage

```bash
# Solve an ORTHOS program and output JSON
./target/release/orthos-kernel examples/safe_division.orth

# Output SMT-LIB2 script (for debugging or external solvers)
./target/release/orthos-kernel examples/safe_division.orth --smt
```

## Import System

Import standard library modules or other `.orth` files:

```orthos
import "std/physics.orth" as Physics
import "std/economics.orth" as Econ

Boundary MySimulation <-> {
    // Use imported boundaries with namespace prefix
    Flux collision = Physics.Collision1D(
        massA=1, velA_before=10,
        massB=1, velB_before=0
    )
    Flux market = Econ.MarketEquilibrium(
        baseSupply=0, supplySlope=2,
        baseDemand=100, demandSlope=1
    )
}
```

## Language Primitives

### Boundary (`|`)
A named scope that separates internal state from external state.

```orthos
Boundary Point {
    Flux x : Int
    Flux y : Int
}
```

### Flux (φ)
A typed quantity that exists within a Boundary. Immutable by default.

```orthos
Flux numerator : Int
Flux result : Bool
```

### Law (λ)
An invariant truth that restricts the domain of Flux. Replaces control flow.

```orthos
Law NonZero : denominator != 0
Law Relation : result * denominator == numerator
```

### Goal (Soft Constraint)
A preference that the solver tries to satisfy, with a weight for prioritization.

```orthos
Goal Efficiency : power < 100 @5      // Weight 5
Goal Comfort : temperature >= 20 @10  // Weight 10 (higher priority)
```

### Manifest
The entry point that triggers the solver.

```orthos
Manifest execution {
    Flux x = 10
    Flux res = SafeMath(numerator=x, denominator=2).result
}
```

## Causal Modes

### Forward Mode (Default) `->`
Inputs are fixed; solver finds outputs.

```orthos
Boundary Calculator {
    // Standard deterministic logic
}
```

### Omni Mode (Retro-causal) `<->`
Solver can modify any Flux to satisfy Laws.

```orthos
Boundary PathFinder <-> {
    // Solver can "work backwards" to find solutions
}
```

## Examples

### Safe Division
```orthos
Boundary SafeMath {
    Flux numerator : Int
    Flux denominator : Int
    Flux result : Int

    Law NonZero : denominator != 0
    Law Relation : result * denominator == numerator
}
```

### Fibonacci (Temporal Unrolling)
```orthos
Boundary Fibonacci {
    Flux f0 : Int
    Flux f1 : Int
    Flux f2 : Int
    
    Law Base0 : f0 == 0
    Law Base1 : f1 == 1
    Law Step2 : f2 == f0 + f1
}
```

## Output Format

### SAT (Satisfiable)
```json
{
  "status": "SAT",
  "model": {
    "Main_x": 10,
    "Main_y": 2,
    "SafeMath_inst_result": 5
  }
}
```

### UNSAT (Contradiction)
```json
{
  "status": "UNSAT",
  "unsat_core": ["NonZero", "Relation"]
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Layer III: Projection                │
│                  (Text Parser / JSON Output)            │
├─────────────────────────────────────────────────────────┤
│                    Layer II: Graph                      │
│              (AST / Boundary-Flux-Law DAG)              │
├─────────────────────────────────────────────────────────┤
│                    Layer I: Kernel                      │
│              (Z3 Theorem Prover Bindings)               │
└─────────────────────────────────────────────────────────┘
```

## Daemon API

The ORTHOS Daemon provides a JSON-RPC interface:

```bash
# Start daemon
./target/release/orthos-daemon

# Initialize with source
echo '{"method":"initialize","params":{"source":"..."},"id":1}' | orthos-daemon

# Query flux values
echo '{"method":"query","params":{"flux":["Main.x"]},"id":2}' | orthos-daemon

# Audit goal satisfaction (explainability)
echo '{"method":"audit","id":3}' | orthos-daemon
```

### Audit Response (Explainability)
```json
{
  "goals": {
    "Comfort.TempMin": {"satisfied": true, "weight": 10, "cost": 0},
    "Economy.SavePower": {"satisfied": false, "weight": 5, "cost": 5}
  },
  "total_cost": 5
}
```

## Limitations (v0.1)

- **Bounded Lists:** Max 64 elements
- **Bounded Horizon:** Max 10 time steps for temporal unrolling
- **No Permeability:** All Flux is public
- **Single Manifest:** One entry point per program
- **Concrete Array Indices:** Dynamic indexing requires known bounds

## License

MIT
