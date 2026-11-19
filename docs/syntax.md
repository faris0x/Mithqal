# Syntax

> Status: Experimental. See [Maturity](maturity.md).

This document defines the source language for the current version.

## Source layout

Source is ASCII except inside string literals, which may contain Unicode text. Non ASCII input outside a string produces a diagnostic in this version.

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

An identifier starts with an ASCII letter and continues with ASCII letters or digits. Underscores are reserved for pattern syntax. Built in names use dromedaryCase. System names such as `$Context` begin with `$` and continue with letters or digits; a bare `$` is not an identifier.

```text
x
fooBar
setDelayed
blankSequence
$Context
$ContextPath
```

A context-qualified name uses a semicolon separator and contains no whitespace.

```text
ctx;name
a;b;name
```

The final segment is the short name. The preceding segments form the context. Semicolons are reserved for qualified names in this version; they are not statement separators.

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

## Strings, bytes, and associations

A string literal uses double quotes and the escapes `\"`, `\\`, `\n`, `\r`, `\t`, and `\0`.

```text
"hello"
"line\nbreak"
```

A byte array uses `byteArray(...)` with integers from 0 through 255.

```text
byteArray(0,1,255)
```

An association literal uses `<|...|>` with `->` entries. Keys are unique and sorted canonically.

```text
<|a->1,b->2|>
```

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

Double square brackets index a completed expression. Indexing is one based, and negative indices count from the end.

```text
[10,20,30][[2]]
"abc"[[-1]]
<|a->1|>[[a]]
[10,20,30][[span(2,3)]]
```

## Contexts

Short names resolve through the current context and `$ContextPath`. A qualified name bypasses the path and names a symbol in a specific context.

```text
foo
foo;bar
a;b;bar
```

The context forms are `begin`, `end`, `context`, and `contexts`. `$Context` and `$ContextPath` report the current context and the search path.

```text
begin(foo)
x=1
context(x)
end()
```

The expression is whitespace free, including qualified names.

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
- `[[` after a completed expression starts indexing. `2[[1,2]]` indexes the expression `2`. Write `2*[1,2]` for scalar multiplication of a list.
- `[1,2][[3]]` is indexing.
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

## Messages

Messages are ordinary call forms:

```text
message(foo,1)
message(messageName(f,tag),1)
check(message(foo,1),2)
quiet(message(foo,1))
off(foo)
on(foo)
```

Messages are non-fatal and are collected separately from diagnostics. The CLI
prints them before the result.

## Package imports and up values

An import names a bundled module or a quoted package path.

```text
import lists
import "packages/stats.mith"
```

Up values are installed with `upSet`, `upSetDelayed`, `tagSet`, and
`tagSetDelayed`.

```text
tagSet(f,g(f(x_)),x+1)
upSet(g(f(x_)),x+10)
upValues(f)
```

## Attributes and definitions

`setAttributes`, `clearAttributes`, `attributes`, `protect`, and `unprotect`
manage symbol attributes. `ownValues`, `downValues`, `subValues`, `upValues`,
`definition`, and `fullDefinition` inspect definitions.

```text
setAttributes(f,orderless)
attributes(f)
protect(f)
downValues(f)
```

## Pattern forms

Typed blanks, alternatives, tests, optionals, and repetitions are call forms:

```text
x_Integer
alternatives(1,2)
patternTest(x_Integer,isEven)
optional(y_,10)
repeated(x__)
repeatedNull(x___)
longest(x__)
shortest(x__)
except(1)
verbatim(2)
```

Orderless heads match sequence patterns by multiset. A head with `oneIdentity`
allows a one argument call pattern to match a bare value.

## Assignment, evaluation control, and control flow

Part assignment is immediate:

```text
x=[1,2,3]
x[[2]]=9
```

`appendTo`, `prependTo`, `addTo`, `subtractFrom`, `timesBy`, and `divideBy`
update a value in place. `unset`, `clear`, `clearAll`, and `remove` remove
definitions.

`hold` prevents evaluation, `evaluate` escapes `hold`, `holdForm` and
`holdPattern` are held forms, `unevaluated` suppresses one argument evaluation,
and `sequence` splices into the enclosing call.

`which`, `switch`, `do`, `while`, `for`, `return`, `break`, `continue`,
`throw`, and `catch` provide control flow. All are ordinary call forms and
remain whitespace free.

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

The expression reader has no file interface of its own. A session can evaluate a file with `Session::eval_file`, and package sources can be loaded with `import "path.mith"`, `Session::needs`, or `Session::get`. The CLI evaluates source passed with `--eval`.
