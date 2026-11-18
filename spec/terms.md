# Abstract Terms

> Spec version: 0.0.1 (draft)

## Terms

An expression is one of the following:

- an exact integer;
- an exact rational;
- an approximate real;
- a symbol;
- a call, consisting of a head term and an ordered sequence of argument terms.

Every expression is a term. Terms are immutable. A session interns terms, so
two equal terms have the same identity within that session.

Strings and byte sequences are reserved for a later version.

## Values

Every term is a value. Evaluation produces a term. The symbols `undefined`,
`indeterminate`, and `failed` are error values and are ordinary terms. The
symbol `null` is the no-value result.

## Identity and equality

Term identity is structural. Two terms are the same term when their interned
identities are equal.

The head `same` compares structural identity and always returns `true` or
`false`.

The head `equal` is mathematical equality. It returns `true` or `false` when the
specification defines a decision procedure for the operands. It remains an
unevaluated call when the decision procedure is unavailable.

## Canonical order

Canonical order is a total order over terms. It depends on term content and
never on addresses, insertion order, or hash layout.

The order compares term kinds first:

1. exact integer
2. exact rational
3. approximate real
4. symbol
5. call

Within a kind, the order compares content. Integers compare by value.
Rationals compare by value. Approximate reals compare by value. Symbols compare
by name. Calls compare by head, then by arguments from left to right. A shorter
argument list sorts before a longer list that has it as a prefix.

Arguments of an `orderless` head are sorted into canonical order before
interning.

## Canonical form

Canonicalization is the normalization applied before a call is interned. It
includes flattening `flat` heads, sorting `orderless` arguments, and the
arithmetic normalizations defined in `semantics.md`.

Canonical form is the basis of structural equality and of the canonical
printer. It does not implement algebraic identities beyond those listed in
`semantics.md`.

## Reserved

Finite sets and structure dictionaries are available as ordinary library terms.
Structure declarations and instances are part of the language. Proof terms are
reserved.
