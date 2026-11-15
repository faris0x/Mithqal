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
- Approximate complex arithmetic, elementary functions, inverse functions, powers, reciprocals, absolute value, gamma, log gamma, and error functions.
- Exact polynomial expansion, factoring, polynomial gcd, resultants, rational function cancellation, and linear or quadratic solving.
- Exact symbolic differentiation, integration by parts, partial fractions, inverse function integrals, and definite and simple improper integrals.
- Exact limits at finite points and at infinity, one sided limits, truncated Taylor series, and limit convergence checks.
- Principal branch conventions for complex logarithms, powers, and inverse functions.
- One-sided endpoint limits for improper integrals, transcendental interior singularity checks, and a staged multivariate factorization scaffold with binary, homogeneous, recursive quadratic, polynomial-leading, content extraction, specialization root lifting, bivariate Hensel lifting, generic multivariate interpolation through eight variables, and power extraction stages. It also includes bounded bivariate Hensel lifting to total degree sixteen.
- Bounded Hensel lifting for univariate integer polynomials.
- Local sign assumptions, conjunctive assumptions, and conditional simplification.
- A line based surface language with modules, definitions, literal patterns, guards, lexical bindings, conditionals, match blocks, and print statements.
- Exact gcd, lcm, factorial, binomial, floor, ceil, and round functions.
- Exact rational and symbolic matrix linear algebra with determinants, rank, inverses, row reduction, matrix products, PLU factorization, characteristic polynomials, eigenvalues, eigenvectors, null spaces, QR factorization, and linear solving.
- Broader polynomial algebra with degrees, coefficients, polynomial division, discriminants, and sum or difference of cubes factoring.
- Exact integer number theory with primality, factorization, divisors, totients, sigma, modular arithmetic, and modular powers.
- Broader number theory with previous primes, prime counting, divisor counts, Moebius values, radicals, square-free tests, Fibonacci numbers, modular inverses, and Jacobi symbols.
- Exact descriptive statistics with means, medians, extrema, ranges, totals, variances, deviations, frequencies, and modes.
- Boolean logic with exclusive or, implication, list quantification, and predicate composition.
- A versioned structured IR scaffold for a future language frontend.
- Typed instruction virtual machine and a reference interpreter.
- Explicit mark and sweep garbage collection.
- Structured diagnostics with stable codes and source spans.
- Public library API and documentation.

Excluded:

- Full multivariate factorization beyond the staged coverage and interpolation above eight variables.
- Further matrix decompositions and wider assumption domains.
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

The next layers add broader improper integration, full multivariate factorization, further matrix decompositions, and wider assumption domains. A command line interface and a graphical interface follow once the library surface is stable.

The project remains a library at its core. Front ends are consumers of the public API.

## Constraints

- Target Linux on x86 64 for the current version.
- Link GMP from vendored source.
- Avoid external Rust crates.
- Keep `unsafe` inside the FFI layer and arena internals.
- Keep evaluation deterministic.
- Document every public item.
- Do not use Git commands during the current work session.
