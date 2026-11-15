# Maturity

This document records the support level of each public area. Stable areas follow the documented contract. Partial areas work for the documented cases and leave unsupported inputs unevaluated. Experimental areas may change without notice.

| Area | State | Notes |
| --- | --- | --- |
| Session, configuration, diagnostics | Stable | Public session methods, error codes, and node-count limits. |
| Expression handles and views | Stable | Generational handles, fallible inspection, and session scoping. |
| Exact integers and rationals | Stable | GMP-backed canonical arithmetic. |
| Approximate real and complex numerics | Partial | MPFR significance, directed intervals, and complex branches. |
| Polynomial expansion and univariate factorisation | Stable | Bounded by the documented term and exponent limits. |
| Multivariate factorisation | Partial | Staged interpolation through eight variables and bounded Hensel lifting. |
| Calculus and limits | Partial | Polynomial, elementary, inverse-function, and supported improper cases. |
| Linear algebra | Partial | Exact rational and symbolic operations with documented decompositions. |
| Number theory and statistics | Partial | Integer and exact list domains with documented limits. |
| Assumptions | Partial | Local sign facts and conjunctions against zero. |
| Surface language, IR, VM, collector | Experimental | Internal surfaces may change while the language design settles. |
| Arena internals | Experimental | Hidden from public documentation and not part of the stable contract. |

## Process gates

A feature is advertised only after it clears these gates:

1. Its documented behavior matches the implementation.
2. Concrete tests cover normal cases, boundaries, and error paths.
3. Randomized property tests cover the invariant that makes the feature correct.
4. Formatting, clippy with warnings denied, and documentation checks pass.
5. Hot paths have a benchmark covered by the CI compile gate, with timing budgets exercised by the scheduled benchmark workflow.
6. Public API changes are recorded in `CHANGELOG.md`.

Areas marked Partial or Experimental must say so in their reference documentation. Unsupported inputs remain unevaluated or return a diagnostic.
