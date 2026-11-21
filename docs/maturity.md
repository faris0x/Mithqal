# Maturity

This document records the support level of each public area. Stable areas follow the documented contract. Partial areas work for the documented cases and leave unsupported inputs unevaluated. Experimental areas may change without notice.

| Area | State | Notes |
| --- | --- | --- |
| Session, configuration, diagnostics | Stable | Public session methods, error codes, and node-count limits. |
| Expression handles and views | Stable | Generational handles, fallible inspection, and session scoping. |
| Exact integers and rationals | Stable | Mithqalite-backed canonical arithmetic. |
| Approximate real and complex numerics | Partial | Mithqalite significance, directed intervals, and complex branches. |
| Polynomial expansion and univariate factorisation | Stable | Bounded by the documented term and exponent limits. |
| Multivariate factorisation | Partial | Staged interpolation through eight variables and bounded Hensel lifting. |
| Calculus and limits | Partial | Polynomial, elementary, inverse-function, and supported improper cases. |
| Linear algebra | Partial | Exact rational and symbolic operations with documented decompositions. |
| Number theory | Partial | Kernel primitives plus classical, analytic, algebraic, elliptic, quadratic-form, and cryptographic library domains with documented limits. |
| Analytic number theory | Experimental | Bernoulli and Euler numbers, Chebyshev functions, summatory functions, and truncated zeta and eta series. |
| Elliptic curves and quadratic forms | Experimental | Affine curve arithmetic, point counting, reduction, class-number counting, and documented small-parameter limits. |
| Algebraic and p-adic number theory | Experimental | Quadratic fields, Eisenstein integers, valuations, norms, units, and truncated p-adic expansions. |
| Number field kernel layer | Experimental | Internal construction, Sturm signatures, power basis arithmetic, norms, traces, characteristic and minimal polynomials, integrality tests, maximal orders by the p-radical Round 2 step, multiplication tables, codifferent and different lattices, fractional ideal arithmetic, ideal factorization, prime decomposition, small-degree class groups, and quadratic, rank-one, and rank-two unit groups with regulators, and experimental Hurwitz zeta and Dirichlet L evaluation; rank-three unit lattices, certified zero theory, and large-index decomposition are not implemented yet. |
| Statistics | Stable | Implemented in the Mithqal standard library (`lib/statistics.mith`). |
| Standard library and list primitives | Experimental | Bundled Mithqal modules with dependency-aware selective loading through `Session::with_modules`. |
| Assumptions | Partial | Local sign, domain, parity, equality, and ordered facts over interned symbols, with conjunction, disjunction, and negation. |
| Surface language, IR, VM, collector | Experimental | Internal surfaces may change while the language design settles. |
| Arena internals | Experimental | Hidden from public documentation and not part of the stable contract. |
| Command line frontend | Stable | REPL evaluation, assignments, commands, completion, history, and license reporting. |
| Language specification and conformance corpus | Experimental | `spec/` is normative. The corpus runs in `tests/conformance.rs`. |
| Finite sets, relations, and finite logic | Experimental | Library modules over canonical lists. |
| Structure dictionaries and finite law checks | Experimental | A structure is a list; laws are checked over a finite carrier. |
| Peano interface and von Neumann numerals | Experimental | A `foundations` module over native integers and finite sets. |
| Combinatorics and finite probability | Experimental | Exact counts, finite enumerations, and distributions over exact rationals. |
| Decision theory and finite games | Experimental | Expected utility, regret, pure Nash equilibria, dominant strategies, and Pareto outcomes. |
| Graph theory | Experimental | Finite directed and undirected graphs over relations. |
| Information theory | Experimental | Entropy, marginals, mutual information, and divergences over exact distributions. |
| Calculus rule layer | Experimental | Mithqal differentiation and integration rules with native fallback. |
| Canonical binders and structure declarations | Experimental | `forall`, `exists`, `setBuilder`, and structure and instance declarations with finite law checks. |
| Strings, byte arrays, associations, and text functions | Experimental | Exact string, byte, and association operations. |
| File text, CSV, TSV, and JSON I/O | Experimental | `std` only and unsandboxed in this version. |
| Contexts, qualified names, and packages | Experimental | Semicolon-qualified names, module contexts, relative package imports, once-only loading, and cycle detection. |
| User attributes and definition introspection | Experimental | Attribute controls, protection, own values, down values, and up values. |
| Advanced pattern matching | Experimental | Typed blanks, alternatives, tests, optionals, repetitions, orderless multiset matching, and `oneIdentity`. |
| Up values | Experimental | Tagged definitions and dispatch before down values and built in implementations. |
| Messages | Experimental | Non-fatal messages, `check`, `quiet`, `off`, and `on`. |

## Process gates

A feature is advertised only after it clears these gates:

1. Its documented behavior matches the implementation.
2. Concrete tests cover normal cases, boundaries, and error paths.
3. Randomized property tests cover the invariant that makes the feature correct.
4. Formatting, clippy with warnings denied, and documentation checks pass.
5. Hot paths have a benchmark covered by the CI compile gate, with timing budgets exercised by the scheduled benchmark workflow.
6. Public API changes are recorded in `CHANGELOG.md`.

Areas marked Partial or Experimental must say so in their reference documentation. Unsupported inputs remain unevaluated or return a diagnostic.
