# Virtual machine

This document defines the execution model for version 0.0.1.

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
    Closure { code: CodeId, env: EnvId },
}
```

Integers, rationals, symbols, and calls are hash consed. A closure is allocated with a unique identity and is never interned. Two closures with the same body and different captured environments are different values.

`ValueSlice` points into the arena and is immutable.

## Instructions

Version 0.0.1 uses a typed instruction vector. A packed byte stream is a later optimization.

```rust
enum Instr {
    Const(ConstId),
    LoadLocal(u16),
    StoreLocal(u16),
    LoadGlobal(SymbolId),

    Eval,
    CallSymbol(SymbolId, u16),
    CallValue(u16),
    CallBuiltin(BuiltinId, u16),
    MakeCall(Value, u16),
    MakeClosure(CodeId, u16),

    Jump(u32),
    JumpIfFalse(u32),
    Return,

    ApplyRules,
    Safepoint,
    Budget(u32),
}
```

`Eval` evaluates the value on the top of the stack through the symbolic evaluator. It is the bridge for dynamically constructed expressions.

`CallSymbol` dispatches by symbol. The dispatcher checks built in implementations and user down values. `CallBuiltin` is an optional direct path chosen by the compiler for a known built in head.

`CallValue` applies a closure or another callable value.

`ApplyRules` asks the matcher to apply user definitions to a call. The matched right hand side is a code object executed with pattern bindings installed in locals.

## Code objects

A code object contains instructions, a constant pool, local slot count, and source metadata for diagnostics.

```rust
struct Code {
    instrs: Vec<Instr>,
    constants: Vec<Value>,
    locals: u16,
    name: Option<String>,
}
```

Constants reference expressions in the arena. The garbage collector treats active code constant pools as roots.

Definitions are compiled when installed. A delayed definition compiles its right hand side once. An immediate definition evaluates its right hand side at installation time. A top level statement is compiled before execution.

## Frames and closures

The virtual machine has an operand stack and a frame stack.

```rust
struct Frame {
    code: CodeId,
    pc: usize,
    locals: Vec<Value>,
    env: EnvId,
    base: usize,
}
```

The operand stack stores `Value` handles. A frame owns its local slots. A closure stores a code identifier and a captured environment.

An environment is a traced heap object containing captured values. Closures capture lexical bindings by value. Global symbols resolve through the session at application time.

Sequence pattern bindings use a separate binding representation because they hold a sequence of values. The matcher produces bindings, and the compiler maps named bindings to local slots. Sequence bindings occupy a sequence slot that can be read by RHS code.

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

`and` and `or` compile to short circuit jumps. `let` evaluates each binding in order and stores the result in a local slot. `fn` emits `MakeClosure` with the body code and the captured environment.

Pattern definitions are not control forms at runtime. The matcher handles them inside `ApplyRules`, and the right hand side receives bindings as locals.

## Garbage collection

Version 0.0.1 uses explicit, non moving mark and sweep collection. Explicit collection keeps handle validity predictable for library users.

Roots:

- The symbol table, including own values and down values.
- Pattern and rule records.
- The last result held by the session.
- The operand stack.
- Every frame's local slots and environment.
- Closure environments.
- Constant pools of active code objects.
- Extra roots supplied by the caller.
- Parser temporaries during a parse operation.

Safepoints:

- `CallSymbol`, `CallValue`, `CallBuiltin`.
- `MakeClosure` and `MakeCall`.
- `Eval`.
- Backward `Jump`.
- Explicit `Safepoint`.

The collector marks reachable nodes, frees unmarked nodes, and rebuilds the intern table from surviving nodes. Collection does not move surviving nodes, so rooted handles remain valid.

The caller invokes collection through the public API with extra roots. The session automatically roots definitions and the last result. Any handle not rooted at collection time becomes invalid.

## Determinism

Execution is deterministic for a fixed program, session state, configuration, and budget.

- Instructions execute in a fixed order.
- Symbol identifiers are sequential within a session.
- Canonical order depends on expression content.
- Hash table iteration has no effect on rule order or results.
- The budget counts instructions, matcher steps, and rule applications.
- Garbage collection has no observable effect other than handle lifetime.

The collector can run at every safepoint during differential tests. Test results must match execution with collection disabled.

## Reference interpreter

The reference interpreter evaluates the same expressions with a tree walker. It uses the same evaluator, matcher, and built in registry. It does not use the virtual machine.

Tests generate a corpus of expressions, evaluate with both paths, and compare canonical output. The corpus includes arithmetic, patterns, definitions, closures, listable calls, error values, and budget exhaustion.

## Target

Version 0.0.1 targets Linux on x86 64 with a static GMP build. The virtual machine uses portable Rust and has no architecture specific code. Packed instruction encoding, bytecode serialization, and a JIT are later work.
