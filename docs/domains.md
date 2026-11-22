# Mathematical Domains

> Status: Experimental. See [Maturity](maturity.md).

Mithqal separates kernel mathematics from the bundled standard library. Kernel code provides primitives that need direct access to the expression model, numeric backend, or evaluator. Library domains are written in Mithqal and can be selected per session.

## Domain dependency graph

| Domain | Imports | Exported names |
|---|---|---:|
| `algebra` | lists | 11 |
| `algebraicnumbertheory` | lists, numbertheory, quadraticforms | 20 |
| `analyticnumbertheory` | lists, numbertheory | 16 |
| `calculus` | algebra | 7 |
| `combinatorics` | lists, sets | 7 |
| `cryptonumbertheory` | lists, logic, numbertheory | 14 |
| `decision` | lists, logic, sets, statistics | 5 |
| `ellipticcurves` | lists | 16 |
| `foundations` | lists, logic, sets | 7 |
| `games` | lists, logic, sets, relations, statistics | 14 |
| `graphs` | lists, logic, sets, relations | 20 |
| `information` | lists, sets, probability, relations | 10 |
| `linearalgebra` | lists, logic | 18 |
| `lists` | none | 6 |
| `logic` | lists | 3 |
| `numbertheory` | lists, logic | 88 |
| `probability` | lists, logic, sets, relations, statistics | 9 |
| `quadraticforms` | lists, numbertheory | 7 |
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
- exact integer lattice algorithms: Hermite and Smith normal forms, LLL reduction, and short vector enumeration
- arbitrary precision integer and modular polynomial factorization, including Hensel lifting and Zassenhaus recombination
- exact rational matrix algebra, Sturm signatures, and power basis number field arithmetic
- maximal orders by the p-radical Round 2 step, integral bases, multiplication tables, and p-maximality checks
- codifferent and different lattices, fractional ideal arithmetic, ideal factorization, prime ideal decomposition, small-degree class groups with an independent imaginary-quadratic reduced-form cross-check, order conductors, quadratic narrow class numbers, general ray class group orders for arbitrary finite moduli and real-place conditions with certified congruence-kernel invariants, quadratic and general unit groups with saturation completion and interval regulator bounds on totally real fields, and experimental Hurwitz zeta, Riemann and primitive Dirichlet functional equations, conductor reduction, nonprincipal L(1) limits, Dirichlet L evaluation, numerically certified critical-line zero scanning, and fixed-precision p-adic field arithmetic
- native accelerators behind the library policy for calculus, algebra, linear algebra, and number theory
- limits, series, and the heavier polynomial and eigenvalue kernels
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

## Native accelerator layer

The `numberfields` Cargo feature is enabled by default. It compiles the native accelerator stack under `src/accelerators`: exact integer matrix and lattice kernels, integer and rational polynomial kernels, Sturm isolation, number fields, maximal orders, the different, ideals, prime decomposition, class groups, units, ray class orders, and analytic zeta and L-function evaluation. The current layer covers field validation, signatures, element arithmetic, norms, traces, characteristic and minimal polynomials, integrality tests, integral bases, multiplication tables, p-maximality by the p-radical Round 2 step, codifferent and different lattices, fractional ideal multiplication, inversion, division, powers, ideal factorization, prime ideals with ramification, residue degree, and Frobenius data, and class groups for small-degree fields, together with quadratic, rank-one, and general-rank unit groups, saturation completion for totally real fields, and interval regulator bounds. Ray class group orders are available for arbitrary finite moduli and real-place subsets; their congruence-kernel invariants are exact, and the full group structure is certified when the ordinary class number is one. Experimental analytic evaluation includes Euler-Maclaurin Hurwitz zeta, Dirichlet L-functions through Hurwitz decomposition, and critical-line zero scanning. Full ray class extension structure for nontrivial class groups, certified zero enclosures, large-index prime decomposition, and expression-level field objects are still ahead.

A size-sensitive build can omit the substrate:

```text
cargo build --no-default-features --features std,cli --lib
```

The substrate itself is also checked without the standard library:

```text
cargo check --no-default-features --features numberfields --lib
```

## Current limits

The number theory domains are broad but not a research-level computer algebra system for the entire field. Current limits include:

- primality and factorization primitives operate on machine-sized integers
- primitive-root, multiplicative-order, and discrete-log searches are brute force
- partition and Bell computations use direct recurrences
- analytic number theory is limited to exact arithmetic sums, Bernoulli and Euler numbers, Chebyshev functions, and truncated Dirichlet series
- class field theory and general higher-degree units are not implemented; class groups are computed for small-degree fields with bounded Minkowski radii, unit groups are completed by saturation for totally real fields within the bounded enumeration envelope, regulators have certified interval bounds on totally real fields, general ray class orders and congruence kernels are implemented, zeta and Dirichlet L evaluation exists with the Riemann functional equation, and full ray class extension structure for nontrivial class groups, certified zero theory, large-index non-monogenic decomposition, and expression-level field objects are still ahead
- elliptic curves cover affine arithmetic, point counting, and small-parameter Diffie-Hellman, not modularity, ranks, or descent
- p-adic support covers valuations, norms, units, and truncated expansions, not a general p-adic field
- quadratic forms cover positive definite reduction and class-number counting, not composition or general indefinite forms

The classical layer is intended to grow without pulling more definitions into the kernel.

## Migration order

1. Keep `statistics` as the first selective-loading proof because it has no library imports.
2. `numbertheory` policy now lives in `lib/numbertheory.mith`. Primality, factorization, modular arithmetic, Jacobi symbols, and the sieve used by `primeCount` remain as native accelerators.
3. `algebra` policy lives in `lib/algebra.mith`, and elementary `linearalgebra` operations are in `lib/linearalgebra.mith`. Determinants, inverses, decompositions, and eigenvalues remain native accelerators with public kernel names.
4. Calculus rules are in `lib/calculus.mith`. Differentiation, integration, limits, and series remain native accelerators because they need the evaluator and branch-condition machinery.

