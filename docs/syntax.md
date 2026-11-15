# Syntax

> Status: Experimental. See [Maturity](maturity.md).

This document defines the source language for the current version.

## Source layout

Source is ASCII. Non ASCII input produces a diagnostic in this version.

A program is a sequence of statements. A newline ends a statement. A statement must fit on one line. An empty line is ignored.

A comment is a block that starts with `(*` and ends with `*)`. A comment occupies its own line or several lines. Comment lines may contain spaces, tabs, and newlines. A code line may not contain a comment. A comment inside a statement produces a diagnostic.

```text
(* factorial *)
fact(0):=1
fact(n_):=n*fact(n-1)
```

Code lines contain no spaces or tabs. Whitespace inside a statement produces a deterministic diagnostic with a line and column.

```text
1 + 2
```

is invalid. The valid form is:

```text
1+2
```

Trailing spaces on a code line are invalid. Leading spaces on a code line are invalid. Spaces on comment lines are allowed.

## Identifiers

An identifier starts with an ASCII letter and continues with ASCII letters or digits. Underscores are reserved for pattern syntax. Built in names use dromedaryCase.

```text
x
fooBar
setDelayed
blankSequence
```

A pattern suffix follows an identifier or stands alone:

```text
_        anonymous wildcard
x_       named wildcard, binds x
x__      one or more sequence, binds x
x___     zero or more sequence, binds x
```

Identifiers have no underscores. This keeps `x_` unambiguous.

## Numbers

An integer is a sequence of decimal digits.

```text
0
42
1000000000000000000000000000000
```

A decimal literal is exact and produces a rational.

```text
1.25
```

is `5/4`.

A decimal literal requires a digit before and after the decimal point. `.5` and `1.` are invalid. There is no exponent literal. Write `1.25*10^8` for a large exact value.

Division is the `/` operator. It evaluates to an exact rational when both operands are exact and the divisor is nonzero.

## Lists

A list literal uses square brackets.

```text
[]
[1,2,3]
[[1,2],[3,4]]
```

A list evaluates its elements. It does not flatten nested lists.

## Calls

A call uses parentheses.

```text
f()
f(x)
f(x,y)
sin(x)
```

The head is any expression that evaluates to a callable value. A symbol head can refer to a built in function or a user definition.

Square brackets after a completed expression are reserved for indexing. The current version does not implement indexing and produces a diagnostic. Use a function call such as `part(x,1)` when indexing is added.

## Operators

The surface operators and their canonical heads:

| Surface | Canonical head |
| --- | --- |
| `plus` | `plus` |
| `+` | `plus` |
| `-` | `plus` with `negate` |
| `*` | `times` |
| `/` | `times` with `power(...,-1)` |
| `^` | `power` |
| `&&` | `and` |
| `||` | `or` |
| `!` | `not` |
| `===` | `equal` |
| `==` | `equal` |
| `=/=` | `notEqual` |
| `~` | `same` |
| `<` | `less` |
| `<=` | `lessEqual` |
| `>` | `greater` |
| `>=` | `greaterEqual` |
| `=` | `set` |
| `:=` | `setDelayed` |
| `->` | `rule` |
| `:>` | `ruleDelayed` |

Precedence from lowest to highest:

1. `=` and `:=`
2. `->` and `:>`
3. `||`
4. `&&`
5. `===`, `==`, `=/=`, `~`, `<`, `<=`, `>`, `>=`
6. `+` and `-`
7. `*`, `/`, and implicit multiplication
8. unary `-`
9. `^`
10. calls, lists, pattern suffixes, parentheses

`^` is right associative. `-x^2` parses as `-(x^2)`. `2^3^4` parses as `2^(3^4)`.

## Implicit multiplication

Two adjacent primary expressions multiply when the left expression is complete and the right expression starts with a number, an identifier, or `(`.

```text
2x
2(x+1)
(x+1)(x-1)
2sin(x)
f(x)g(x)
[1,2]x
```

The following rules resolve the ambiguous cases:

- `x(y)` is a call of `x` with argument `y`.
- `f(x)(y)` is `f(x)*y`. A call applies to a bare identifier. A call result is multiplied instead.
- `x^2y` is `(x^2)*y`. Use `x^(2y)` for the other meaning.
- `2x^2` is `2*(x^2)`.
- `x2` is one identifier. It is not `x*2`.
- `23` is one number. It is not `2*3`.
- `[` after a completed expression starts indexing. `2[1,2]` is an indexing form and produces a diagnostic in the current version. Write `2*[1,2]` for scalar multiplication of a list.
- `[1,2][3]` is indexing and produces a diagnostic in the current version.
- A comma separates arguments or list elements. It never implies multiplication.

Implicit multiplication has the same precedence as explicit `*` and associates to the left.

## Lists, sets, and empty values

`[]` is the empty list. The empty set is the protected symbol `emptyset`. These are different values.

The error values are `undefined`, `indeterminate`, and `failed`. The result of a delayed definition is `null`.

## Definition forms

Immediate definition:

```text
a=3^2
```

Delayed function definition:

```text
f(x_):=x^2
```

Literal argument definition:

```text
f(0):=1
```

Multiple definitions are allowed:

```text
fact(0):=1
fact(n_):=n*fact(n-1)
```

Rules can be written directly:

```text
x_->x^2
x_:>x^2
```

`->` evaluates the right hand side when the rule is constructed. `:>` holds the right hand side.

## Patterns and conditions

Patterns use the suffix forms:

```text
f(_):=1
f(x_):=x^2
f(x_,x_):=x
f(x__):=x
f(x___):=x
```

A condition uses the `when` surface form. It maps to the `condition` head.

```text
f(when(x_,x>0)):=x
```

A condition is evaluated after the pattern matches. It must produce `true` for the definition to apply.

## Local bindings and functions

`let` introduces lexical bindings. The first argument is a list of bindings. Each binding uses `name=value`. The second argument is the body.

```text
let([x=1],x+1)
let([x=1,y=2],x+y)
```

`fn` creates a closure. Parameters are written in parentheses and the body follows `=>`.

```text
fn(x)=>x+1
fn(x,y)=>x+y
```

`let` and `fn` are syntax forms. Their canonical heads are `let` and `function`.

## Numeric approximation

`approx(expression,digits)` converts a numeric expression to an approximate real with a requested decimal digit count.

```text
approx(1/3,30)
approx(pi,30)
approx(sqrt(2),40)
approx(sin(pi/6),30)
```

The digit count is optional and defaults to 16. Approximate values print as decimal output. They do not have a source literal syntax in the current version.

`interval(low,high)` constructs an approximate interval. Intervals support addition, multiplication, negation, and integer powers.

```text
interval(1,2)+3
interval(1,2)*interval(3,4)
-interval(1,2)
interval(-2,3)^2
```

`complex(real,imaginary)` constructs an approximate complex value. The `approx` boundary recognizes `i` and produces complex values for expressions with a nonzero imaginary part.

```text
complex(1,2)
approx(1+2*i,30)
approx((2+3*i)*(4-i),30)
approx((1+i)/(1-i),30)
```

## Control forms

`if` is a normal call with three arguments. It holds the branch arguments.

```text
if(x>0,x,-x)
```

`and`, `or`, and `not` have surface spellings:

```text
a&&b
a||b
!a
```

## Comments and whitespace diagnostics

A code line with whitespace produces a diagnostic with a stable code and a source span.

```text
error TGN-PARSE-0001: whitespace is not allowed inside a statement
  --> <input>:1:4
   |
 1 | 1 + 2
   |    ^
   = note: write 1+2
```

A code line with an inline comment produces a separate diagnostic.

```text
error TGN-PARSE-0002: comments must occupy their own lines
  --> <input>:1:4
   |
 1 | 1+2(*sum*)
   |    ^
```

## File fixtures

Test fixtures use the `.expr` extension for the current version. The project has no file reader in this version. The final source extension will be chosen with the project name.
