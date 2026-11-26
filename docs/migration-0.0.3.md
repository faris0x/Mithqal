# Migrating to 0.0.3

> Status: Current migration guide for 0.0.3. See [Maturity](maturity.md).

Version 0.0.3 replaces the line-based language with controlled mathematical
prose. The old syntax is not supported. There is no compatibility mode.

## Sentence structure

Sentences end with a period. Clauses may appear before or after the statement.

```text
where x > 0, we define positive(x) = true.
```

## Modules

| 0.0.2 | 0.0.3 |
|---|---|
| `mod name` | `module name.` |
| `end` | `end module.` |
| `import statistics` | `use statistics.` |
| `export f` | `export f.` |

## Definitions

| 0.0.2 | 0.0.3 |
|---|---|
| `def f(x)=x*x` | `we define f(x) = x * x.` |
| `def f(x) when x>0=1` | `we define f(x) = 1, where x > 0.` |
| `def f(x)=...` | `we define f(x) = ....` |

A definition with no parameters is a value:

```text
we define answer = 42.
```

## Bindings and printing

| 0.0.2 | 0.0.3 |
|---|---|
| `let x=3` | `let x be 3.` |
| `prt x` | `print x.` |

## Conditionals

| 0.0.2 | 0.0.3 |
|---|---|
| `if x>0 then` | `if x > 0 then:` |
| `else` | `else:` |
| `end` | `end if.` |

## Matches

| 0.0.2 | 0.0.3 |
|---|---|
| `match x` | `match x with:` |
| `case 0` | `case 0:` |
| `else` | `else:` |
| `end` | `end match.` |

## Structures and instances

| 0.0.2 | 0.0.3 |
|---|---|
| `structure name` | `structure name.` |
| `field carrier` | `field carrier.` |
| `law name: body` | `law name: body.` |
| `end` | `end structure.` |
| `instance name : structure` | `instance name : structure.` |
| `end` | `end instance.` |
| `instance name : structure assumed` | `instance name : structure assumed.` |

## Local definitions

Use local `let` equations:

```text
we define fact(n) =
    let loop(0, acc) = acc,
        loop(k, acc) = loop(k - 1, acc * k)
    in loop(n, 1).
```

## Derivative notation

```text
we define d(sin(u))/dx = cos(u) * d(u)/dx.
```

Use `*d(...)/dx` to select the global derivative notation when a local `d`
shadows it.

## Assumptions

```text
assume x is positive.
```

Assumptions persist for later evaluation in the module.

## Theorems

```text
theorem derivative_of_sine.
for every expression u in x, d(sin(u))/dx = cos(u) * d(u)/dx.
proof:
by the definition of d.
end proof.
```

Proof steps support `assume`, `have`, `show`, and `by` justifications.

## Expressions

Expressions still use ordinary notation. Whitespace is allowed inside prose
formulas. Unicode is canonical in output, while ASCII aliases are accepted as
input.

## Removing the old surface

The old keywords are no longer parsed:

- `mod`
- `def`
- `prt`
- `end`
- `when`
- `if ... then` without the colon
- `match ... case` without the colon

Port every source file before upgrading.
