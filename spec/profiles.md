# Conformance Profiles

> Spec version: 0.0.1 (draft)

A profile names a set of language features. An implementation declares the
profiles it supports and MUST pass every corpus case in each declared profile,
together with the cases of every profile that profile depends on.

| Profile | Depends on | Required corpus |
|---|---|---|
| `core` | none | `core.tsv` |
| `exact` | `core` | `core.tsv`, `exact.tsv` |
| `numeric` | `core` | `core.tsv`, `numeric.tsv` |
| `modules` | `core` | `core.tsv`, `modules.tsv` |
| `embedded` | `core`, `exact` | `core.tsv`, `exact.tsv`, `embedded.tsv` |

## core

The `core` profile covers the reader, abstract terms, evaluation, definitions,
scoping, hygiene, diagnostics, resource limits, exact integers and rationals,
the control forms, and the structural primitives.

An implementation of `core` MUST provide:

- the surface grammar in `grammar.md`;
- the term model in `terms.md`;
- the evaluation relation and rule selection in `semantics.md`;
- exact integer and rational arithmetic from `numbers.md`;
- stable error codes and deterministic diagnostics;
- the structural primitives listed in `semantics.md`;
- the binder and structure declarations in `grammar.md` and `semantics.md`.

## exact

The `exact` profile adds the exact arithmetic and number theory heads that the
reference documentation advertises as exact, including `gcd`, `lcm`,
`factorial`, `binomial`, `floor`, `ceil`, and `round`, together with exact
algebraic simplification and cancellation.

## numeric

The `numeric` profile adds approximate reals, the `approx` boundary, intervals,
complex values, and the principal branch conventions in `numbers.md`.

## modules

The `modules` profile adds strict modules with export lists, private internal
names, protected exports, import dependencies, dependency order, and the module
level restrictions on `export` and `import`.

A `core` implementation may support modules. The `modules` profile records the
additional conformance cases that exercise privacy and import behavior.

## embedded

The `embedded` profile targets fixed-capacity devices. It requires `core` and
`exact`, an explicit step budget, an explicit depth budget, and a documented
subset of the structural primitives. It does not require the `numeric` profile.
It does not require bundled modules, and it does not require a standard library.

An embedded implementation MUST document its resource limits and the primitives
it provides. A call outside the documented subset remains unevaluated.

## Declaring conformance

A conformance report states the implementation name, the implemented spec
version, the declared profiles, the number of cases run, and the number of
failures. A report with failures is not a conformance report.
