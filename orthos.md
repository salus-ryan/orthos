# **PROJECT ORTHOS: Technical Specification (v1.0)**

**Status:** Frozen // **Classification:** Novel Architecture // **Target:** Semantic Field Theory

## **1. Abstract**

ORTHOS is not an imperative programming language. It is a **Constraint-Topology Description Language**. It does not execute instructions; it defines a state space and solves for valid histories.

* **Conventional Model:** `Input + Algorithm = Output`
* **Orthos Model:** `Constraint + Topology = Manifestation`

The "Runtime" is a continuous verification loop (Theorem Prover). The "Hardware" is an energy landscape.

---

## **2. System Architecture**

The system must be built in three distinct layers.

### **Layer I: The Kernel (The Physics)**

* **Role:** The ground truth engine.
* **Implementation Target:** **Rust** (for memory safety and strict typing).
* **Core Dependency:** **Z3 Theorem Prover** (Microsoft Research) or **CVC5**.
* **Function:**
* Accepts a list of logical assertions.
* Returns `SAT` (Satisfiable Model) or `UNSAT` (Contradiction).
* **Strict Requirement:** No interpreter loops. All logic must be transpiled to SMT-LIB2 format for the solver.



### **Layer II: The Graph (The Topology)**

* **Role:** The in-memory representation of the program.
* **Structure:** A high-dimensional Directed Acyclic Graph (DAG) where:
* **Nodes** are `Boundaries`.
* **Edges** are `Flux`.
* **Hyper-edges** are `Laws`.


* **Time Handling:** Time is treated as spatial dimension . Iteration is a geometric spiral along .

### **Layer III: The Projection (The Interface)**

* **Role:** Human-readable I/O.
* **Primary Projection:** Textual (Syntax defined below).
* **Secondary Projection:** Visual (Topological heat map of constraint stress).

---

## **3. The Ontology (Primitive Types)**

There are only three primitives in ORTHOS. All other structures (classes, functions, modules) are derived from these.

### **Primitive A: The Boundary (`|`)**

* **Definition:** A named scope that separates Internal State from External State.
* **Behavior:** Enforces encapsulation.
* **Properties:**
* `Permeability`: Defines what Flux can enter/exit.
* **`CausalMode`**: (See Section 5).



### **Primitive B: The Flux ()**

* **Definition:** A typed quantity that traverses Boundaries.
* **Behavior:** Replaces variables. Flux is immutable by default; "change" is a new Flux instance at .
* **Types:** `Boolean`, `Integer`, `Tensor`, `String` (treated as byte arrays).

### **Primitive C: The Law ()**

* **Definition:** An invariant truth that restricts the Domain of Flux.
* **Behavior:** Replaces Control Flow.
* Instead of `if (x > 0) y = 1`, ORTHOS asserts `Law: (x > 0) implies (y == 1)`.


* **Failure State:** A violation of Law is an `Ontological Error` (the program cannot exist), not a Runtime Error.

---

## **4. The Syntax (Text Projection)**

**Keywords:** `Boundary`, `Flux`, `Law`, `Manifest`, `Diode`.

```orthos
// EXAMPLE: Safe Division with Temporal Diode

Boundary SafeMath {
    // 1. Define Flux (Inputs/Outputs)
    Flux numerator : Int
    Flux denominator : Int
    Flux result : Int

    // 2. Define Laws (Invariants)
    // Attempting to divide by zero makes the timeline invalid.
    Law NonZero : denominator != 0
    
    // The fundamental relation
    Law Relation : result * denominator == numerator
}

// EXAMPLE: Usage
Boundary Main {
    Flux val : Int
    
    // Manifestation (Triggering the Solver)
    Manifest execution {
        Flux x = 10
        Flux y = 2
        
        // This instantiates the logic. 
        // If y were 0, the compiler would reject the UNIVERSE as invalid.
        Flux res = SafeMath(numerator=x, denominator=y).result
    }
}

```

---

## **5. The Novelty: Causal Diodes**

To manage the danger of retro-causality (future constraining the past), all Boundaries must be tagged with a `CausalMode`.

### **Mode A: Forward (Default)**

* **Symbol:** `->`
* **Logic:**  is derived strictly from .
* **Solver Constraint:** Inputs are fixed; Solver finds Outputs.
* **Use Case:** Standard deterministic logic, IO, banking.

### **Mode B: Omni (Retro-causal)**

* **Symbol:** `<->`
* **Logic:**  and  are solved simultaneously.
* **Solver Constraint:** The solver can modify Inputs to satisfy Output Laws.
* **Use Case:** Planning, Inverse Kinematics, Search, Optimization.

### **The Diode Implementation Specification**

The Builder must implement a "Valve" between modes:

1. Information flowing from **Forward**  **Omni** is allowed (Context).
2. Information flowing from **Omni**  **Forward** is **Collapsed**.
* *The Omni scope must collapse to a single concrete value before returning to Forward scope.*



```orthos
// DIODE SYNTAX
Boundary PathFinder <-> {  // Omni Mode enabled
    Flux start : Point
    Flux end : Point
    Flux path : List<Point>

    // The solver is free to mutate 'path' to make this Law true
    Law Connect : path.first == start && path.last == end
}

// Main is Forward by default
Boundary Main {
    Flux route = PathFinder(start=A, end=B).path
    // 'route' is now a fixed constant. The uncertainty is collapsed.
}

```

---

## **6. Build Roadmap**

**Step 1: The Validator (Week 1-2)**

* Build a Rust parser that constructs the Abstract Syntax Tree (AST).
* Map AST nodes to Z3 assertions.
* *Success Metric:* The program `Law: 1 == 2` must fail to compile with output `UNSAT`.

**Step 2: The Timekeeper (Week 3-4)**

* Implement the  dimension.
* Translate "Flux changing" into Static Single Assignment (SSA) form for the solver ().
* *Success Metric:* A Fibonacci sequence generates correctly without loops.

**Step 3: The Diode (Week 5-6)**

* Implement the context switch between Forward and Omni solving strategies.
* *Success Metric:* The solver can "fill in the blanks" for a puzzle in an Omni block, but fails if asked to reverse a hash in a Forward block.

**Step 4: The Metal (Future)**

* Map the "Energy" of the solver (complexity of clauses) to hardware instructions.

---

**End of Specification.**
*Architect: Gemini // Project: ORTHOS*