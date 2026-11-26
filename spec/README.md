# Mithqal Language Specification

> Spec version: 0.0.3 (draft)

This directory is the normative definition of the Mithqal language. The
reference documentation in `docs/` teaches the language. The specification
states what an implementation must do. The conformance corpus in
`conformance/` is the executable form of the specification.

## Documents

1. `terms.md` defines abstract terms, values, identity, and canonical order.
2. `grammar.md` defines the surface source language and printing.
3. `semantics.md` defines evaluation, definitions, scoping, hygiene, modules,
   and the structural primitives.
4. `numbers.md` defines the numeric tower and the approximation boundary.
5. `profiles.md` defines conformance profiles and their dependencies.
6. `versioning.md` defines the version policy and the frozen surface.
7. `conformance/README.md` defines the corpus format and case kinds.

When prose and corpus disagree, the corpus records the intended behavior and
the prose is corrected in a patch release. A change to the intended behavior
requires a language version change.

## Conformance

An implementation declares the profiles it supports. A conforming
implementation MUST pass every corpus case in each declared profile and in the
profiles that profile depends on. A case names one profile. The corpus is data,
and a runner may be written in any language.

The specification uses MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY as defined
by RFC 2119.

## Normative areas

The following are normative: abstract terms, the surface grammar, evaluation
results, rule selection, scoping and hygiene, module privacy, canonical
printing, error codes, exact numeric results, and determinism under a budget.

## Implementation-defined areas

The following are implementation-defined but observable: resource thresholds,
performance, internal symbol numbering, whether a library is bundled, diagnostic
message text, memory layout, and the order of independent internal tables.

## Unspecified areas

Anything not observable through the abstract terms, the surface language, the
diagnostics, or the corpus is unspecified.

## Status of this version

Version 0.0.3 describes the implemented language: a controlled mathematical
prose reader, exact and approximate numbers, evaluation, definitions and
clauses, scoping, hygiene, modules, binders, structures, instances, conditionals,
matches, assumptions, theorem skeletons, and the structural primitives. Notation
declarations remain reserved for later versions.
