# Mithqal Language

> Status: Experimental. See [Maturity](maturity.md).
>
> This page teaches the language. The normative definition is in `spec/`.

The Mithqal language is a line based surface language with lowercase keywords. It is not Mathematica compatible.

The language provides modules with exports and imports, definitions with single-line or block bodies, local definitions, lexical bindings, conditionals, match blocks, canonical binders, structure and instance declarations, print statements, comments, and a strict whitespace rule for expressions.

Version 0.0.2 adds strings, byte arrays, associations, indexing, part assignment, contexts, qualified names, package loading, user attributes, up values, messages, and the control forms `which`, `switch`, `do`, `while`, `for`, `return`, `break`, `continue`, `throw`, and `catch`. See [Language v2 design](language-v2.md) and [Syntax](syntax.md) for the full surface.

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

A definition has a name, a parameter list, and a body. Parameter lists contain no whitespace. The body is evaluated when the function is called.

A body is either one expression on the definition line or a block terminated by `end`:

```text
def variance(xs)=
let n=length(xs)
let s=total(xs)
let q=total(map(xs,fn(x)=>x*x))
(n*q-s*s)/(n*n)
end
```

A block body contains local `def` statements, `let` bindings, `if` statements, and one final expression. The final expression supplies the value. An `if` statement may stand in final position; otherwise it must be the last statement. `prt`, `match`, `export`, `import`, `structure`, and `instance` are not allowed in a body in this version.

Local definitions must appear before the body's other statements. Definitions with the same name are mutually recursive and are tried in source order. A local definition is visible only inside its body, shadows a global definition of the same name for that body, and cannot shadow a protected symbol. It captures the enclosing parameters lexically.

```text
def factorial(n)=
def loop(0,acc)=acc
def loop(k,acc)=loop(k-1,acc*k)
loop(n,1)
end
```

Parameters can be names, literal patterns, sequence patterns, or nested constructor patterns.

```text
def fact(0)=1
def fact(n)=n*fact(n-1)

def sum(x__)=fold(fn(a,b)=>a+b,0,list(x))
def head2([a_,b_])=a
```

Literal patterns are not ranked above name patterns. Definitions are tried in source order, so a base case must be defined before a more general rule.

A definition can include a guard after the parameter list.

```text
def sign(x) when x<0=-1
def sign(x) when x==0=0
def sign(x)=1
```

The guard is a whitespace-free expression. Rules are tried in source order, and guards are checked after pattern matching. A rule whose guard does not evaluate to `true` is skipped and the next rule is tried.

Definitions capture the bindings visible at definition time.

Rule bodies are instantiated with capture-avoiding substitution. When a matched value would be captured by a closure parameter, a `let` binding, or a pattern variable of a rule constructed by the body, that binder is renamed. An inner binding shadows a rule variable of the same name.

## Arity

A function whose definitions all have a fixed number of parameters reports an arity diagnostic when it is called with a different number of arguments. A function with a sequence parameter accepts any count in its range and is not arity checked; an unmatched call stays unevaluated.


## Binders

Three expression forms bind a variable:

```text
forall(x,[1,2,3],x>0)          -> true
exists(x,[1,2,3],x>2)          -> true
setBuilder(x,[1,2,3,4],isEven(x)) -> [2,4]
```

`forall(variable, domain, body)` is true when the body is true for every item of a finite domain. `exists` is true when the body is true for at least one item. `setBuilder` returns the canonical set of items for which the body is true.

An empty domain makes `forall` true, `exists` false, and `setBuilder` empty. A body that is not decidable for some item leaves the binder symbolic. Bindings are lexical and are renamed to a canonical name, so alpha-equivalent binders are the same term.

## Bindings

```text
let x=3
let y=square(x+1)
```

`let` evaluates its expression immediately and binds the result in the current block. Bindings are lexical. A binding inside `if` or `match` does not escape that block.

## Higher-order list forms

`fn(params)=>body` creates an anonymous function. Higher-order list forms accept that function value:

```text
map([1,2,3],fn(x)=>x^2)        -> [1,4,9]
apply(fn(x,y)=>x+y,[3,4])      -> 7
fold(fn(x,y)=>x+y,0,[1,2,3])   -> 6
nest(fn(x)=>x+1,0,5)           -> 5
```

`map` applies one argument to each list item. `apply` passes the list items as call arguments. `fold` carries an accumulator from left to right. `nest` repeats a unary function a fixed number of times.

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

## Modules

Every source is one module. A module can export names and import other modules.

```text
mod statistics

export mean,median,frequency
import list

def mean(xs)=sum(xs)/count(xs)
def sum(xs)=fold(fn(a,b)=>a+b,0,xs)
end
```

A module with at least one `export` statement is **strict**. Exported definitions keep their source names and are protected. Every other definition is private to the module: it is installed in a module context, so two modules can use the same helper name without colliding. A module without an `export` statement keeps every definition visible, which is the compatibility behavior for small snippets.

`import` declares a dependency on another bundled module or a package file. Modules load in dependency order; an unknown module or an import cycle is rejected. Imported names are not gated per module yet: every exported name from every loaded module is visible. Context-qualified access such as `ctx;name` is available in expressions.

Exporting a name that the module does not define is an error. Two modules cannot export the same name; the second definition reports a protected-symbol diagnostic.


## Structures and instances

A structure declares a carrier, its fields, and named laws. An instance supplies the fields and checks the laws.

```text
structure monoid
field carrier
field op
field identity
law associative: forall(a,carrier,forall(b,carrier,forall(c,carrier,same(op(op(a,b),c),op(a,op(b,c))))))
law neutral: forall(a,carrier,same(op(a,identity),a))
end

instance cyclic : monoid
carrier = [0,1,2]
op = fn(a,b)=>mod(a+b,3)
identity = 0
end
```

A structure declaration defines a constructor with the structure name, an accessor for each field, and a law checker. For the example, these are `monoid`, `monoidCarrier`, `monoidOp`, `monoidIdentity`, and `monoidHolds`. Accessor names are the structure name followed by the capitalized field name.

An instance evaluates its field values, builds the structure dictionary, and checks the laws over the carrier. A false law rejects the instance. An undecidable law rejects the instance unless the declaration uses `assumed`.

```text
instance symbolic : monoid assumed
carrier = y
op = fn(a,b)=>a+b
identity = 0
end
```

Structure and instance declarations appear at module level. Generated names participate in module exports and privacy like other definitions.

## Standard library

The engine bundles a standard library written in the surface language. A session loads every module when it is constructed, so library definitions are available to ordinary statements without an explicit import.

```text
prt mean([1,2,3,4])
```

The first module is `statistics`, which defines `mean`, `median`, `minimum`, `maximum`, `range`, `total`, `count`, `variance`, `sampleVariance`, `standardDeviation`, `sampleStandardDeviation`, `frequency`, and `mode`. Library symbols are protected, and a library function declares a fixed arity: calling one with the wrong number of arguments reports an arity diagnostic.

Library modules are part of the crate source and are not user editable at run time.

## Current limits

- One module per source.
- Definition parameters support names, literal patterns, typed blanks, alternatives, tests, optionals, repetitions, sequence patterns, and nested constructor patterns.
- Guards are single expressions.
- Match blocks support literal and binding cases; general patterns are available in rule and definition parameters.
- `prt` is the only output statement.
- Definition bodies support local `def`, `let`, `if`, and one final expression.
- Local definitions are lexically scoped, mutually recursive, and cannot shadow protected symbols.
- Strict modules export a declared set of names and keep the rest private.
- Imports declare bundled-module or package-file dependencies and are resolved in dependency order. Package files resolve relative to the importing source, then through `$MithqalPath`.
- Qualified names use a semicolon separator, as in `ctx;name`.
- Contexts are managed with `begin`, `end`, `context`, and `contexts`.
- Structure laws must decide to `true` or `false` over the carrier, or the instance must be declared `assumed`.

## Session entry points

```rust
let mut session = mithqal::Session::new();
session.eval_source("mod m\nprt 1+2\nend\n")?;
assert_eq!(session.output(), &["3"]);
```

`eval_source` detects a module source and routes it to `eval_language`. `eval_language` is also public for explicit use.
