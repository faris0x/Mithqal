# Mithqal Language

The Mithqal language is a line based surface language with lowercase keywords. It is not Mathematica compatible.

The first version provides modules, single-line definitions, bindings, conditionals, print statements, comments, and a strict whitespace rule for expressions.

## Keywords

| Purpose | Keyword |
|---|---|
| Module | `mod` |
| Definition | `def` |
| Binding | `let` |
| Conditional | `if` |
| Then | `then` |
| Else | `else` |
| Match | `match` |
| Case | `case` |
| Guard | `when` |
| Block end | `end` |
| Print | `prt` |

Keywords are lowercase ASCII words. User names start with a lowercase ASCII letter and continue with ASCII letters or digits. Names use dromedaryCase.

## Structure

Every source is one module.

```text
mod name
  statements
end
```

`end` closes the innermost block. The same keyword closes `mod` and `if`.

Indentation is optional and ignored. Blank lines are allowed. A source may contain no statements after the module end.

## Definitions

```text
def square(x)=x*x
def add(x,y)=x+y
def value()=42
```

A definition has a name, a parameter list, and one expression after `=`. Parameter lists contain no whitespace. The body is evaluated when the function is called.

Parameters can be plain names or literal patterns.

```text
def fact(0)=1
def fact(n)=n*fact(n-1)
```

Literal patterns are more specific than name patterns, so the base case is selected first.

A definition can include a guard after the parameter list.

```text
def sign(x) when x<0=-1
def sign(x) when x==0=0
def sign(x)=1
```

The guard is a whitespace-free expression. Rules are ordered by specificity, and guards are checked after pattern matching.

Definitions capture the bindings visible at definition time.

## Bindings

```text
let x=3
let y=square(x+1)
```

`let` evaluates its expression immediately and binds the result in the current block. Bindings are lexical. A binding inside `if` or `match` does not escape that block.

## Conditionals

```text
if x>3 then
  prt x
else
  prt 0
end
```

`else` is optional. Nested conditionals share the same `end` keyword and close one block at a time.

A condition must evaluate to `true` or `false`.

## Match blocks

```text
match x
case 0
  prt 100
case n
  prt n+10
else
  prt -1
end
```

A case pattern is either a literal expression or a binding name. Literal cases are tested first in source order until one matches. A name case binds the scrutinee in the case body. `else` is optional.

Match case bindings are scoped to the case body.

## Printing

```text
prt x+1
```

`prt` evaluates the expression, formats it in infix form, and appends the text to the session output. `Session::output` returns the collected lines and `Session::take_output` drains them.

## Expressions

Expressions use the existing Mithqal reader:

```text
x+1
2*x
2x
(x+1)^2
f(x,y)
[1,2,3]
x==4
x!=0
```

`=` belongs to `let` and `def`. `==` is an equation.

Expressions contain no whitespace. Whitespace is allowed between a keyword and an expression, around `=`, before `then`, and after `else`. Parameter lists and call argument lists contain no whitespace.

Valid:

```text
let x = 3
let y=2+3
def square(x) = x*x
if x>0 then
```

Invalid:

```text
let x = 3 + 1
let y=square(x, y)
def square(x, y)=x+y
if x > 0 then
```

A whitespace violation reports the source line and column.

## Comments

`//` starts a comment that runs to the end of the line. Whole-line and trailing comments are allowed.

```text
// whole line
let x=1 // trailing comment
```

## Example

```text
// A small module
mod algebra

def square(x)=x*x
let a=3
let b=square(a+1)

if b>10 then
  prt b
else
  prt 0
end

end
```

The same program works without indentation:

```text
mod algebra
def square(x)=x*x
let a=3
let b=square(a+1)
if b>10 then
prt b
else
prt 0
end
end
```

## Current limits

- One module per source.
- Definition parameters support names and literal patterns.
- Guards are single expressions.
- Match patterns support literals and binding names.
- `prt` is the only output statement.
- List patterns, sequence patterns, and nested constructor patterns are future work.

## Session entry points

```rust
let mut session = mithqal::Session::new();
session.eval_source("mod m\nprt 1+2\nend\n")?;
assert_eq!(session.output(), &["3"]);
```

`eval_source` detects a module source and routes it to `eval_language`. `eval_language` is also public for explicit use.
