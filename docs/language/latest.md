# Mithqal Language 0.0.3

> Status: Experimental. Controlled mathematical prose surface.

Version 0.0.3 replaces the line-based surface language with controlled
mathematical prose. Prose is the only source language.

The canonical printed form is Unicode. ASCII aliases remain accepted as input.
Sentences end with a mandatory period. Clauses may appear before or after the
statement. Definitions use `we define`. Local bindings use `let`.

## Statements

The surface supports:

- modules, imports, and exports
- `we define` rules and values
- clauses with `where`, `assuming`, and `provided that`
- structured and variadic patterns
- derivative notation and the `*` global notation escape
- local function definitions through `let` equations
- module-level `let ... be ...` and `print`
- `if ... then:` and `match ... with:` blocks
- structures and instances, including `assumed` instances
- module-level assumptions
- theorem skeletons with checked proof steps

## Lowering and evaluation

Prose modules lower to the evaluator statement AST. Every bundled library
module, integration test source, and conformance row uses prose.

## Proof checking

The checker verifies:

- ground equality claims
- universal symbolic equality claims
- finite-domain universal claims
- finite-domain existential claims
- boolean claims under proof-local assumptions
- `have` and `show` steps
- finite proof by cases
- contradiction from inconsistent assumptions
- named `have` facts referenced with `by <name>`

Symbolic equality assumptions act as rewriting facts. Universal equality claims
use a separate theorem-checking budget.

## Conformance

The `prose` conformance profile covers modules, definitions, guards, local
functions, derivative policy, conditionals, matches, assumptions, structures,
and theorem checking.

## Migration

See [Migrating to 0.0.3](../migration-0.0.3.md) and the
[syntax reference](../syntax.md).
