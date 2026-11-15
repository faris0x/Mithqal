# Virtual machine

> Status: Experimental. See [Maturity](maturity.md).

This document defines the execution model.

## Implementation status

The compiler and typed instruction machine are active. They execute `Const`, `Eval`, `CallSymbol`, jumps, `Return`, `Safepoint`, and `Budget`. The `if` form compiles to control flow. Calls into the rewrite engine remain the semantic path for symbolic work. Closures are active in the evaluator. Machine call frames are active for symbol calls. Machine locals, compiled closure bodies, `ApplyRules` as a dedicated instruction, and packed instruction encoding remain future work.

## Role of the virtual machine

The evaluator owns the rewrite semantics. The virtual machine executes compiled statements, function bodies, and closures. Built in rules remain Rust functions in a registry. The virtual machine removes the cost of walking the same function body as a tree on every call.

This split keeps symbolic rewriting correct and testable while giving user code a compact execution path. The reference interpreter implements the same semantics as a tree walker. Tests compare the two on a fixed corpus.

## Values

A runtime value is a handle into the node arena.

```rust
pub type Value = u32;
```

The node arena stores these forms:

```rust
enum Node {
    Integer(Int),
    Rational(Rat),
    Symbol(SymbolId),
    Call { head: Value, args: ValueSlice },
}
```

Closures are calls with the protected `closure` head. They carry a parameter list, a body expression, and a captured environment list. Because they are ordinary expressions, the collector reaches them through normal call arguments.

`ValueSlice` points into the arena and is immutable.

## Instructions

The current version uses a typed instruction vector. A packed byte stream is a later optimization.

```rust
enum Instr {
    Const(u16),
    Eval,
    CallSymbol(SymbolId, u16),
    Jump(u32),
    JumpIfFalse(u32),
    JumpIfNotBoolean(u32),
    Return,
    Safepoint,
    Budget(u32),
}
```

`Eval` evaluates the value on the top of the stack through the symbolic evaluator. It is the bridge for dynamically constructed expressions.

`CallSymbol` builds a call from the symbol head and the values on the stack, then runs the symbolic evaluator. The evaluator checks built in implementations and user down values. Dedicated call and built in instructions are future work.

## Code objects

A code object contains instructions, a constant pool, and an optional name.

```rust
struct Code {
    instrs: Vec<Instr>,
    constants: Vec<ExprId>,
    name: Option<String>,
}
```

Constants reference expressions in the arena. The compiler emits one constant per literal or fallback expression.

Definitions are evaluated by the symbolic engine. Compiled top level statements are available through the session compiled path. Compiling every definition body once and caching the result is future work.

## Frames and closures

The machine keeps an operand stack and a call frame stack. Each symbol call records the operand stack base for the callee, evaluates the call, restores the caller stack, and pushes the result. Machine locals and compiled closure bodies are future work. Closures are evaluated by the symbolic engine into `closure` calls. Applying a closure builds a lexical frame from the captured environment and parameter bindings, evaluates the body in that frame, then restores the caller frame.

Sequence pattern bindings are produced by the matcher and substituted into right hand sides. A local slot mapping for compiled right hand sides is future work.

## Execution loop

The machine runs a fetch, decode, execute loop over the instruction vector.

1. Read the instruction at `pc`.
2. Advance `pc`.
3. Perform the operation on the operand stack and current frame.
4. Check the step budget on calls and backward jumps.
5. Reach a safepoint when the instruction can allocate or call the garbage collector.

Arithmetic fast paths are optional. A fast path checks operand tags, performs exact arithmetic, and falls back to `CallSymbol(plus,2)` or another generic path when the tags do not match. Every fast path has a generic equivalent.

## Compiling control forms

Control forms compile to jumps so their evaluation order is explicit.

```text
if(cond,then,else)
```

compiles to a sequence that evaluates `cond`, branches when the result is `false`, evaluates one branch, and returns.

`and`, `or`, `let`, and `fn` currently fall back to the symbolic evaluator. They keep their held arguments correct without duplicate control logic in the compiler. Short circuit jump lowering and closure code generation are future work.

Pattern definitions are handled by the symbolic evaluator. The compiler treats definition forms as fallback expressions.

## Garbage collection

The current version uses explicit, non moving mark and sweep collection. Explicit collection keeps handle validity predictable for library users.

Roots:

- The symbol table node cache.
- Own values and down value rules.
- The last result held by the session.
- Extra roots supplied by the caller.

The collector marks reachable nodes, frees unmarked nodes, and rebuilds the intern table from surviving nodes. Collection does not move surviving nodes, so rooted handles remain valid. Freed slots are reused by later allocations.

Collection can be explicit, and the session also collects after a top level evaluation when the live node count exceeds the configured threshold. The session automatically roots definitions and the last result. Any handle not rooted at collection time becomes invalid. The machine emits `Safepoint` instructions. Collection during machine execution is future work.

## Determinism

Execution is deterministic for a fixed program, session state, configuration, and budget.

- Instructions execute in a fixed order.
- Symbol identifiers are sequential within a session.
- Canonical order depends on expression content.
- Hash table iteration has no effect on rule order or results.
- The budget counts instructions, matcher steps, and rule applications.
- Garbage collection has no observable effect other than handle lifetime.

Collection tests cover reclamation, explicit roots, definition survival, intern table rebuilding, and repeated collection with bounded live node counts.

## Reference interpreter

The symbolic evaluator is the reference path. The compiled path uses the typed instruction machine and delegates symbolic calls to the evaluator.

Tests generate a corpus of expressions, evaluate with both paths, and compare canonical output. The corpus includes arithmetic, conditionals, logic, definitions, recursion, patterns, listable calls, error values, and budget exhaustion.

## Target

The current version targets Linux on x86 64 with a static GMP build. The virtual machine uses portable Rust and has no architecture specific code. Packed instruction encoding, bytecode serialization, and a JIT are later work.
