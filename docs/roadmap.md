# Roadmap

The project ships small versions. The current version establishes the kernel and the public library surface. There is no target date for a version 1.0.

## Current scope

Included:

- Exact integers and rationals through Mithqalite.
- Symbol table, attributes, canonical order, and hash consed expressions.
- Source reader for whitespace free statements with newline separators and whole line comments.
- Canonical and infix printers.
- Fixed point evaluator with deterministic budgets.
- Pattern matcher for the current pattern set.
- Definitions, lexical `let`, closures, listable arithmetic, comparisons, and logic.
- Symbolic `Q(i)` normalization.
- Approximate reals through Mithqalite Float with significance tracking and a numeric boundary.
- Approximate interval arithmetic with directed endpoint rounding and significance widening.
- Approximate complex arithmetic, elementary functions, inverse functions, powers, reciprocals, absolute value, gamma, log gamma, and error functions.
- Exact polynomial expansion, factoring, polynomial gcd, resultants, rational function cancellation, and linear or quadratic solving.
- Exact symbolic differentiation, integration by parts, partial fractions, inverse function integrals, and definite and simple improper integrals.
- Exact limits at finite points and at infinity, one sided limits, truncated Taylor series, and limit convergence checks.
- Principal branch conventions for complex logarithms, powers, and inverse functions.
- One-sided endpoint limits for improper integrals, transcendental interior singularity checks, and a staged multivariate factorization pipeline with binary, homogeneous, recursive quadratic, polynomial-leading, content extraction, specialization root lifting, bivariate Hensel lifting, generic multivariate interpolation through eight variables, and power extraction stages. It also includes bounded bivariate Hensel lifting to total degree sixteen.
- Bounded Hensel lifting for univariate integer polynomials.
- Local assumptions over interned symbol identity, including signs, domains, parity, equality, ordering, conjunction, disjunction, negation, introspection, and conditional simplification.
- A controlled mathematical prose surface with modules, exports, imports, definitions and clauses, local definitions, literal and constructor patterns, guards, lexical bindings, conditionals, match blocks, canonical binders, structure and instance declarations, assumptions, and theorem skeletons.
- Exact gcd, lcm, factorial, binomial, floor, ceil, and round functions.
- Exact rational and symbolic matrix linear algebra with determinants, rank, inverses, row reduction, matrix products, PLU factorization, characteristic polynomials, eigenvalues, eigenvectors, null spaces, QR factorization, and linear solving.
- Broader polynomial algebra with degrees, coefficients, polynomial division, discriminants, and sum or difference of cubes factoring.
- Exact integer number theory with primality, factorization, divisors, totients, sigma, modular arithmetic, and modular powers.
- Broader number theory with previous primes, prime counting, divisor counts, Moebius values, radicals, square-free tests, Fibonacci numbers, modular inverses, and Jacobi symbols.
- Exact descriptive statistics with means, medians, extrema, ranges, totals, variances, deviations, frequencies, and modes.
- A standard library written in Mithqal and loaded by default when a session is constructed, covering statistics, finite sets and relations, structures, foundations, combinatorics, probability, decision theory, games, graphs, information theory, and calculus rules. Selected domains can be loaded with `Session::with_modules`.
- Generic list primitives for library code: length, sort, first, rest, cons, nth, reverse, and a number predicate.
- Inferred arities for definitions, with arity diagnostics on mismatched fixed-arity calls.
- Definition bodies that span lines with `let` and `if` statements ending in one expression.
- Pattern parameters for definitions, including sequence patterns and nested constructor patterns.
- Arity inferred from each definition; fixed-arity functions report arity diagnostics and variadic functions stay unchecked.
- Capture-avoiding substitution for rule-constructing bodies, including `rule`, `ruleDelayed`, `condition`, and `pattern` binders.
- Lexically scoped local definitions inside blocks and definition bodies, with mutual recursion and protected-symbol checks.
- Strict modules with declared exports, private internal definitions, and import-declared dependency ordering.
- A definitional library layer with finite sets, relations, finite quantifiers, structure dictionaries with finite law checks, and a Peano and von Neumann foundations module.
- Exact combinatorics with permutations, combinations, Catalan numbers, derangements, and Stirling numbers.
- Finite probability with exact rational distributions, conditioning, expectation, variance, and independent products.
- Exact decision theory with expected utility, best actions, and minimax regret.
- Finite two player games with pure Nash equilibria, dominant strategies, zero-sum detection, and Pareto optimal outcomes.
- Finite graph theory with adjacency, reachability, components, trees, complete graphs, and acyclicity.
- Information theory with entropy, marginals, mutual information, and divergences over exact distributions.
- A Mithqal calculus rule layer for differentiation and integration, with the native kernels as fallback.
- Canonical binders and structure and instance declarations with finite law checks.
- Boolean logic with exclusive or, implication, list quantification, and predicate composition.
- A versioned structured IR scaffold for a future language frontend.
- Typed instruction virtual machine and a reference interpreter.
- Explicit mark and sweep garbage collection.
- Structured diagnostics with stable codes and source spans.
- Public library API and documentation.
- A feature-gated command line frontend with a persistent session, assignments, commands, completion, and history.
- Strings, byte arrays, associations, indexing, part assignment, and mutation helpers.
- Contexts, qualified names, package loading, and user attributes.
- Typed patterns, alternatives, tests, optionals, repetitions, orderless multiset matching, and up values.
- Non-fatal messages, message controls, and session message collection.
- File text, CSV, TSV, and JSON import and export under the `std` feature.

Excluded:

- Full multivariate factorization beyond the staged coverage and interpolation above eight variables.
- Further matrix decompositions and deeper assumption inference beyond the documented fact language.
- Infinite sets.
- Unicode source identifiers and LaTeX rendering.
- Parallel evaluation and a JIT.
- Windows and macOS builds.
- A graphical or notebook frontend.

## Implementation order

1. Vendor Mithqalite and write the numeric backend layer.
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

- Mithqalite wrapper correctness against known values and small integer oracles.
- Parser round trips through canonical output.
- Evaluation idempotence and determinism across sessions.
- Pattern matching with repeated variables, conditions, and sequence patterns.
- `Q(i)` normalization and shadowing of `i`.
- Listable threading and length errors.
- Diagnostics for whitespace, inline comments, protected symbols, and budget overflow.
- Virtual machine equivalence with the reference interpreter.
- Garbage collection with forced collection at every safepoint.
- Memory stability under repeated evaluation.
- Differential algebra, calculus, and linear algebra fixtures generated from SymPy.

## Remaining work

Remaining work includes broader improper integration, general multivariate factorization beyond the staged bounds, further matrix decompositions, deeper assumption inference beyond the documented fact language, proofs, notation declarations, multi-module source files, nested same-head `flat` pattern grouping, and deeper up value dispatch. A graphical interface follows once the language surface is stable.

The engine remains a small core. Mathematics lives in Mithqal modules. Frontends consume the public API and the language contract.

## Constraints

- Support Linux on x86-64 as the desktop build target and compile-check the library for `wasm32-unknown-unknown` and `thumbv7m-none-eabi` without the standard library.
- Link Mithqalite from the Mithqalite fork.
- Keep external Rust crates limited to the numeric core and optional frontends.
- Keep `unsafe` confined to arena internals.
- Keep evaluation deterministic.
- Document every public item.
