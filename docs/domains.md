# Mathematical Domains

> Status: Experimental. See [Maturity](maturity.md).

Mithqal separates kernel mathematics from the bundled standard library. Kernel code provides primitives that need direct access to the expression model, numeric backend, or evaluator. Library domains are written in Mithqal and can be selected per session.

## Domain dependency graph

| Domain | Imports | Exported names |
|---|---|---:|
| `calculus` | none | 2 |
| `algebraicnumbertheory` | lists, numbertheory | 16 |
| `analyticnumbertheory` | lists | 12 |
| `cryptonumbertheory` | lists, logic, numbertheory | 14 |
| `ellipticcurves` | lists | 16 |
| `quadraticforms` | lists, numbertheory | 7 |
| `combinatorics` | lists, sets | 7 |
| `decision` | lists, logic, sets, statistics | 5 |
| `foundations` | lists, logic, sets | 7 |
| `games` | lists, logic, sets, relations, statistics | 14 |
| `graphs` | lists, logic, sets, relations | 20 |
| `information` | lists, sets, probability, relations | 10 |
| `lists` | none | 6 |
| `logic` | lists | 3 |
| `numbertheory` | lists, logic | 77 |
| `probability` | lists, logic, sets, relations, statistics | 9 |
| `relations` | lists, logic, sets | 11 |
| `sets` | lists, logic | 12 |
| `statistics` | none | 13 |
| `structures` | lists, logic | 20 |

`Session::with_modules` loads only the requested domains and their import closure. `Session::with_config_and_modules` does the same with explicit evaluation limits. `Session::new` and `Session::with_config` load every domain.

## Kernel versus library

Kernel mathematics stays in Rust when it requires:

- canonical expression construction or interning
- exact integer, rational, real, or complex numeric primitives
- rewrite, pattern, or assumption integration
- the numeric boundary and significance tracking
- resource budgets and determinism controls

Library domains stay in `lib/*.mith` when they are ordinary definitions over the public language.

The current audit indicates that the following areas still contain substantial kernel mathematics:

- exact polynomial algebra and factorization
- calculus kernels and limits
- exact rational linear algebra
- integer number theory
- approximate numeric dispatch and special functions

The existing library already covers descriptive statistics, finite sets and relations, structure dictionaries, combinatorics, probability, decision theory, games, graphs, information theory, calculus rules, classical and computational number theory, analytic number theory, elliptic curves, quadratic fields, p-adic expansions, binary quadratic forms, and cryptographic number theory.

## Domain manifests

A future generated build should read a manifest per domain. The intended shape is:

```text
[statistics]
requires = []
numeric = ["exact"]
optional = ["distributions"]
embedded = true
```

The manifest will drive dependency resolution, size estimates, compatibility checks, and website build options. The current implementation derives dependencies from `import` declarations and uses `Session::with_modules` for selection.

## Current limits

The number theory domains are broad but not a research-level computer algebra system for the entire field. Current limits include:

- primality and factorization primitives operate on machine-sized integers
- primitive-root, multiplicative-order, and discrete-log searches are brute force
- partition and Bell computations use direct recurrences
- analytic number theory is limited to exact arithmetic sums, Bernoulli and Euler numbers, Chebyshev functions, and truncated Dirichlet series
- class field theory, general number fields, and ideal arithmetic are not implemented
- elliptic curves cover affine arithmetic, point counting, and small-parameter Diffie-Hellman, not modularity, ranks, or descent
- p-adic support covers valuations, norms, units, and truncated expansions, not a general p-adic field
- quadratic forms cover positive definite reduction and class-number counting, not composition or general indefinite forms

The classical layer is intended to grow without pulling more definitions into the kernel.

## Migration order

1. Keep `statistics` as the first selective-loading proof because it has no library imports.
2. Keep `numbertheory` as the first large selectable mathematical domain. Its classical layer is in `lib/numbertheory.mith`; performance-critical primality, factorization, divisor, modular, Jacobi, and Fibonacci primitives remain in the kernel.
3. Move linear algebra and polynomial algebra only after the manifest and size-reporting path exists.
4. Keep calculus kernels in the kernel until the library has a stable numeric and symbolic interface for series, limits, and branch conditions.

