# Mithqal Language v2 Design

> Status: Experimental. Target version 0.0.2.

This document records the language-generalisation work for version 0.0.2. The
normative surface grammar remains in `spec/`. The language version and the
Mithqal crate version are the same number.

## Scope

Version 0.0.2 adds the first general-purpose data, naming, and control layer:

- string atoms and string literals;
- byte array atoms and `byteArray(...)` literals;
- association atoms and `<|...|>` literals;
- context-aware symbols, qualified names, and context management;
- indexing with `expr[[...]]`;
- part assignment and mutation helpers;
- evaluation control (`hold`, `evaluate`, `unevaluated`, `sequence`);
- `which`, `switch`, `do`, `while`, `for`, `return`, `break`, `continue`,
  `throw`, and `catch`;
- `unset`, `clear`, `clearAll`, and `remove`.

It does not add whitespace inside expressions, multi-line expressions,
Wolfram Language compatibility, datasets, or new mathematical algorithms.

## Whitespace

Expressions remain whitespace free. A statement still occupies one line.
Whitespace is allowed only where it already was: between a language keyword and
its expression, around the `=` of a definition or binding, before `then`, after
`else`, and on blank or comment lines.

## New atoms

The expression model gains three atom forms.

- A string is a sequence of Unicode scalar values. It is not a symbol and it is
  not evaluated.
- A byte array is an immutable sequence of bytes.
- An association is an ordered mapping from keys to values. Keys are unique and
  are stored in canonical order.

Associations are a first-class node rather than a call. The representation is
intended to support later context-dependent views such as matrices, vectors,
quaternions, and other structured values without changing the node kind.

## Association literal

The surface literal is:

```text
<|key->value,key->value|>
```

The literal contains no whitespace. The canonical printer emits the same form.
The keys are evaluated and canonicalized before construction. Duplicate keys are
rejected with a diagnostic.

## Qualified names

A qualified name uses a semicolon separator:

```text
ctx;name
a;b;c
```

A qualified name names a symbol in a specific context and bypasses the context
path. Semicolons are reserved for qualified names in the current version; they
are not statement separators or expression sequencing operators. Use
`compound(expr1,expr2,...)` for expression sequencing.

## Contexts

The symbol table stores a context and a short name for each symbol. The session
maintains:

- `$Context`, the context used for newly created symbols;
- `$ContextPath`, the ordered list of contexts searched for short names;
- `Context`, `Contexts`, `Begin`, and `End`.

Short names resolve through `$ContextPath`. Qualified names resolve directly.
The canonical printer emits the full context for a non-global symbol. The human
display printer uses the short name when the symbol is in the current context.

## Literals and indexing

Strings use double quotes with `\"`, `\\`, `\n`, `\r`, `\t`, and `\0`
escapes. Byte arrays use `byteArray(0,1,255)`. Associations use
`<|key->value,key->value|>` and are whitespace free.

Indexing is one based and uses double brackets:

```text
[10,20,30][[2]]
x[[1]][[2]]
"abc"[[-1]]
<|a->1|>[[a]]
```

A negative index counts from the end. `span(start,end)` selects an inclusive
range. A missing list or call index leaves the `part` call unevaluated. A
missing association key returns `undefined`.

## Assignment and mutation

Part assignment is immediate:

```text
x=[1,2,3]
x[[2]]=9
```

`setDelayed` on a part is rejected. Mutation helpers are `appendTo`,
`prependTo`, `addTo`, `subtractFrom`, `timesBy`, and `divideBy`.

`unset` removes an own value or a function's rules. `clear`, `clearAll`, and
`remove` remove definitions.

## Evaluation control and control flow

`hold` prevents evaluation; `evaluate` escapes `hold`. `holdForm` and
`holdPattern` are held forms. `unevaluated` suppresses one argument evaluation.
`sequence` splices into the enclosing call arguments.

`which` and `switch` select branches. `do`, `while`, and `for` iterate.
`return` leaves the enclosing function. `break` and `continue` control loops.
`throw` and `catch` transfer a value to the nearest matching catch.

## Attributes and introspection

`setAttributes`, `clearAttributes`, `attributes`, `protect`, and `unprotect`
manage symbol attributes. `ownValues`, `downValues`, `subValues`, and
`upValues` inspect definitions. `definition` and `fullDefinition` combine the
attribute and value views.

## Pattern matching

Patterns support typed blanks such as `x_Integer`, `alternatives`,
`patternTest`, `optional` defaults, `repeated`, `repeatedNull`, `longest`,
`shortest`, `except`, and `verbatim`. Orderless heads use full multiset
matching for sequence patterns. `oneIdentity` permits a call pattern with one
argument to match a bare value.

## Packages and up values

A file package is a module source. `import "path.mith"` loads a file relative to
the importing source, then through `$MithqalPath`. `Session::needs` loads once
and `Session::get` re-evaluates. Import cycles are rejected. A strict module's
private definitions live in a module context and are not visible by short name.

`upSet(lhs,rhs)`, `upSetDelayed(lhs,rhs)`, `tagSet(tag,lhs,rhs)`, and
`tagSetDelayed(tag,lhs,rhs)` install up values. Up values are tried before down
values and built in implementations. `upValues(symbol)` lists them.

## Messages

`message(name, args...)` records a non-fatal message. `messageName(symbol, tag)`
builds a `symbol::tag` name. `check(expr, fallback)` returns the fallback when
the expression emitted a message. `quiet(expr)` suppresses messages. `off(name)`
and `on(name)` disable and enable named messages. `Session::take_messages`
drains recorded messages.

## Compatibility

The changes are additive at the source level except for two reserved token
decisions:

- `;` is now a qualified-name separator.
- `<|` and `|>` begin and end an association literal.

Existing source that does not use these tokens is unaffected.
