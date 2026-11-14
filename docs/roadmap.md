# Roadmap

The project ships small versions. The current version establishes the kernel and the public library surface. There is no target date for a version 1.0.

## Current scope

Included:

- Exact integers and rationals through GMP.
- Symbol table, attributes, canonical order, and hash consed expressions.
- Source reader for whitespace free statements with newline separators and whole line comments.
- Canonical and infix printers.
- Fixed point evaluator with deterministic budgets.
- Pattern matcher for the current pattern set.
- Definitions, lexical `let`, closures, listable arithmetic, comparisons, and logic.
- Symbolic `Q(i)` normalization.
- Approximate reals through MPFR with significance tracking and a numeric boundary.
- Approximate interval arithmetic with directed endpoint rounding and significance widening.
- Approximate complex arithmetic, elementary functions, inverse functions, powers, reciprocals, and absolute value.
- Typed instruction virtual machine and a reference interpreter.
- Explicit mark and sweep garbage collection.
- Structured diagnostics with stable codes and source spans.
- Public library API and documentation.

Excluded:

- Branch cut conformance for complex inverse functions.
- Calculus, algebra, linear algebra, and assumptions.
- Strings, files, and sets beyond `emptyset`.
- A command line interface.
- Unicode source and LaTeX rendering.
- Parallel evaluation and a JIT.
- Windows and macOS builds.

## Implementation order

1. Vendor GMP and write the FFI layer.
2. Implement `Int` and `Rat` wrappers with exact arithmetic tests.
3. Implement the expression arena, symbol table, interning, and canonical order.
4. Implement the lexer, parser, canonical printer, and infix printer.
5. Implement the evaluator, attributes, patterns, definitions, scoping, listable threading, and diagnostics.
6. Implement the reference interpreter.
7. Implement the compiler, virtual machine, and differential tests.
8. Implement explicit garbage collection and stress tests.
9. Stabilize the public API and complete the documentation.

Each step ends with a green test suite. The next step does not begin while the previous step has failing tests.

## Testing

The test suite covers these areas:

- GMP wrapper correctness against known values and small integer oracles.
- Parser round trips through canonical output.
- Evaluation idempotence and determinism across sessions.
- Pattern matching with repeated variables, conditions, and sequence patterns.
- `Q(i)` normalization and shadowing of `i`.
- Listable threading and length errors.
- Diagnostics for whitespace, inline comments, protected symbols, and budget overflow.
- Virtual machine equivalence with the reference interpreter.
- Garbage collection with forced collection at every safepoint.
- Memory stability under repeated evaluation.

Fixtures use the `.expr` extension in the current version.

## Later layers

The next numeric layer adds MPFR and MPC, significance tracking, and interval values. After that, the engine gains polynomial arithmetic, expansion, factoring, differentiation, and simplification under assumptions. A command line interface and a graphical interface follow once the library surface is stable.

The project remains a library at its core. Front ends are consumers of the public API.

## Constraints

- Target Linux on x86 64 for the current version.
- Link GMP from vendored source.
- Avoid external Rust crates.
- Keep `unsafe` inside the FFI layer and arena internals.
- Keep evaluation deterministic.
- Document every public item.
- Do not use Git commands during the current work session.
