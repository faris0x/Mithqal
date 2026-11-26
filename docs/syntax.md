# Syntax

> Status: Experimental. See [Maturity](maturity.md).

This document defines the current source language. The surface language is
controlled mathematical prose. Sentences end with a period. Formulas inside
sentences use ordinary mathematical notation.

## Source layout

Source is Unicode. Line endings may be LF or CRLF. A source file is a sequence
of sentences.

A sentence ends with a period. The period is mandatory. A period inside a
decimal number does not end a sentence. A period inside parentheses or brackets
does not end a sentence.

```text
module arithmetic.

we define square(x) = x * x.

print square(5).
end module.
```

Comments are line comments beginning with `//` or block comments delimited by
`(*` and `*)`. Comments may appear between sentences.

## Modules

A module begins with:

```text
module name.
```

and ends with:

```text
end module.
```

Imports and exports are module statements:

```text
use statistics.
use "path/to/package.mith".
export mean, median.
```

A module must end before any further statements. Statements after `end module`
are rejected.

## Definitions

A named definition uses `we define`:

```text
we define square(x) = x * x.
```

Definitions may be guarded:

```text
we define sign(x) = 1, where x > 0.
we define sign(x) = -1, where x < 0.
we define sign(x) = 0.
```

Clauses may appear before or after the definition:

```text
where x > 0, we define positive(x) = true.
```

A definition with no parameters defines a value:

```text
we define pi2 = 2 * pi.
```

Local bindings use `let ... be ...`:

```text
let answer be 42.
```

Local functions use `let` equations inside a formula:

```text
we define fact(n) =
    let loop(0, acc) = acc,
        loop(k, acc) = loop(k - 1, acc * k)
    in loop(n, 1).
```

## Patterns

The left side of a definition is a pattern.

- A bare lowercase name binds one value.
- A name followed by `...` binds a sequence.
- A call pattern such as `sin(u)` matches that head and binds its arguments.
- A list pattern such as `[a, b]` matches a two-element list.
- A literal such as `0` matches that value.

```text
we define firstOf(plus(a, rest...)) = a.
```

Pattern heads are constructors. For example, `g(x)` matches a call with head
`g`, while `x` is a pattern variable.

## Derivative notation

`d(expression)/dx` is the derivative of an expression with respect to `x`.

```text
we define d(sin(u))/dx = cos(u) * d(u)/dx.
```

`d/dx (expression)` is accepted as an alternative form.

The leading asterisk selects the global notation when a local `d` shadows it:

```text
*d(f)/dx
```

## Conditionals and matches

Conditional blocks:

```text
if condition then:
    ...
else:
    ...
end if.
```

The `else` branch is optional.

Match blocks:

```text
match expression with:
case pattern:
    ...
case pattern:
    ...
else:
    ...
end match.
```

Case patterns use the same pattern forms as definitions.

## Structures and instances

Structures declare fields and laws:

```text
structure monoid.
field carrier.
field op.
field identity.
law associative: forall(a, carrier, forall(b, carrier, forall(c, carrier, same(op(op(a, b), c), op(a, op(b, c)))))).
end structure.
```

Instances provide field values:

```text
instance cyclic : monoid.
carrier = [0, 1, 2].
op = fn(a, b) => mod(a + b, 3).
identity = 0.
end instance.
```

An instance may skip law checking:

```text
instance symbolic : monoid assumed.
carrier = y.
op = fn(a, b) => a + b.
identity = 0.
end instance.
```

## Assumptions

Module-level assumptions:

```text
assume x is positive.
```

The assumption is active for later evaluation in the module. Supported forms
include positivity, negativity, nonzero, integer, rational, real, and arbitrary
formula assumptions.

## Theorems and proofs

```text
theorem derivative_of_sine.
for every expression u in x, d(sin(u))/dx = cos(u) * d(u)/dx.
proof:
by the definition of d.
end proof.
```

Proof steps include:

- `assume condition.`
- `have name: claim.`
- `show claim.`
- `by definition.`
- `by the definition of name.`
- `by arithmetic.`
- `by assumption.`
- `by rewriting.`

Ground and universal equality claims are checked. Boolean claims are evaluated
under the active assumptions. Finite-domain existential claims of the form
`there exists n in domain such that condition` are checked by evaluation.

## Expressions

Expression forms:

- integer, rational, real, string, and byte-array literals;
- identifiers and context-qualified names;
- calls `head(arg, ...)`;
- lists `[item, ...]`;
- associations `<|key->value, ...|>`;
- indexing `expr[[index]]`;
- anonymous functions `fn(x, ...) => expression`;
- binders `forall(x, domain, body)`, `exists(x, domain, body)`, and
  `setBuilder(x, domain, body)`;
- parenthesized expressions.

Function application binds tighter than arithmetic. Operator precedence, from
tightest to loosest:

1. indexing and calls
2. `^`
3. unary `-` and `!`
4. `*`, `/`, and juxtaposition
5. `+` and `-`
6. comparisons
7. `&&`
8. `||`

## Unicode and ASCII

Unicode is canonical in printed output. ASCII aliases are accepted as input.

| Unicode | ASCII |
|---|---|
| `×`, `·` | `*` |
| `÷` | `/` |
| `−` | `-` |
| `≤` | `<=` |
| `≥` | `>=` |
| `≠` | `=/=` |
| `∧` | `&&` |
| `∨` | `||` |
| `¬` | `!` |
| `π` | `pi` |
| `∞` | `inf` |

Identifiers may contain Unicode letters. Normalization uses NFC.

## Identifiers

An identifier begins with a letter or `_` and continues with letters, digits,
and underscores. Built-in names use dromedaryCase. Context-qualified names use a
semicolon:

```text
ctx;name
```

System names begin with `$`, such as `$Context` and `$ContextPath`.

## Errors

The parser reports the sentence, line, and column of the first error. Missing
periods, unclosed delimiters, misplaced block terminators, invalid patterns, and
unbound names are diagnosed before evaluation.
