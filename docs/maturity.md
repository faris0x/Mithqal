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
| Number theory | Partial | Integer domains with documented limits. |
| Statistics | Stable | Implemented in the Mithqal standard library (`lib/statistics.mith`). |
| Standard library and list primitives | Experimental | Bundled Mithqal modules and the generic list primitives they use. |
| Assumptions | Partial | Local sign facts and conjunctions against zero. |
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

## Process gates

A feature is advertised only after it clears these gates:

1. Its documented behavior matches the implementation.
2. Concrete tests cover normal cases, boundaries, and error paths.
3. Randomized property tests cover the invariant that makes the feature correct.
4. Formatting, clippy with warnings denied, and documentation checks pass.
5. Hot paths have a benchmark covered by the CI compile gate, with timing budgets exercised by the scheduled benchmark workflow.
6. Public API changes are recorded in `CHANGELOG.md`.

Areas marked Partial or Experimental must say so in their reference documentation. Unsupported inputs remain unevaluated or return a diagnostic.
