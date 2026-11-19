# Surface Grammar

> Spec version: 0.0.2 (draft)

## Source encoding

A source file is ASCII except inside string literals, which may contain Unicode
text. A source line is terminated by a newline. A trailing newline is optional.

## Comments

The sequence `//` begins a comment that extends to the end of the line. A
comment is allowed on its own line or after a statement.

## Lines and whitespace

A program is a sequence of lines. Blank lines are allowed. In a module, the
first non-comment line MUST be a module declaration.

Expressions contain no whitespace. Whitespace is allowed between a keyword and
its expression, around the `=` of a definition or binding, before the `then` of
a conditional, and after `else`. A parameter list and a call argument list
contain no whitespace. Whitespace around commas in an export list is allowed.

## Identifiers

An identifier begins with a lowercase ASCII letter and continues with ASCII
letters and digits. The conventional style is dromedaryCase. Keywords are
lowercase ASCII words and are not reserved outside their statement position.

A context-qualified name is a sequence of identifiers separated by `;`, as in
`ctx;name` or `a;b;name`. It contains no whitespace. The final identifier is the
short name and the preceding identifiers form the context.

System names such as `$Context` and `$ContextPath` begin with `$` and continue
with letters and digits. A bare `$` is not an identifier.

## Literals

An integer literal is a sequence of decimal digits.

A decimal literal integrates an optional fractional part and denotes an exact
rational. `1.25` denotes `125/100`. There is no exponent literal. Scientific
notation is written with a power of ten, as in `25*10^-2`.

A string literal is double quoted. The escapes are `\"`, `\\`, `\n`, `\r`,
`\t`, and `\0`.

A byte array literal is `byteArray(0,1,255)`, with integer arguments from 0
through 255.

An association literal is `<|key->value,key->value|>`. It contains no whitespace.
Keys are unique and sorted into canonical order.

The approximate reals produced by `approx` have no source literal form in this
version.

## Expressions

The expression forms are:

- a literal;
- an identifier;
- a call `head(arg, ...)`;
- a list `[item, ...]`;
- an association literal `<|key->value, ...|>`;
- an indexed expression `expr[[index, ...]]`;
- an anonymous function `fn(name, ...)=>expression`;
- a binder `forall(variable, domain, body)`, `exists(variable, domain, body)`, or `setBuilder(variable, domain, body)`;
- a parenthesized expression.

Juxtaposition denotes multiplication, so `2x` denotes `2*x`.

Operator precedence, from tightest to loosest:

1. `^`
2. unary `-` and unary `!`
3. `*`, `/`, and juxtaposition
4. `+` and `-`
5. `==`, `===`, `=/=`, `~`, `<`, `<=`, `>`, `>=`
6. `&&`
7. `||`
8. `=`, `:=`, `->`, `:>`

Comparisons do not chain.

## Patterns

Pattern syntax is available in definition parameter lists and in call
arguments:

- `_` is an anonymous blank.
- `name_` is a named blank.
- `name__` is a named sequence of one or more arguments.
- `name___` is a named sequence of zero or more arguments.
- `name_Type` is a typed blank. The type is a head name.
- `condition(pattern, expression)` attaches a guard to a pattern.
- `alternatives(pattern, ...)` matches any alternative.
- `patternTest(pattern, test)` adds a test.
- `optional(pattern[,default])` may consume zero or one argument.
- `repeated(pattern)` and `repeatedNull(pattern)` bind a sequence.
- `longest(pattern)` and `shortest(pattern)` choose sequence order.
- `except(pattern)` matches when the pattern does not.
- `verbatim(expression)` compares without pattern interpretation.

A sequence pattern is allowed only as a direct argument of a call pattern.

## Statements

A module has the form:

```text
mod name
statements
end
```

The statements are:

- `def name(parameters)=expression`
- `def name(parameters) when guard=expression`
- `def name(parameters)=` followed by a body block and `end`
- `let name=expression`
- `if condition then` body `else` body `end`, where `else` is optional
- `match expression` with `case` blocks and an optional `else`
- `prt expression`
- `export name,name,...`
- `import module`
- `structure name` followed by `field name` and `law name: expression` lines and `end`
- `instance name : structure` followed by `field = expression` lines and `end`, with optional `assumed`
- `end`

A body block contains local `def` statements, `let` statements, `if`
statements, and one final expression. A local `def` may itself have a body
block. `prt`, `match`, `export`, `import`, `structure`, and `instance` are not allowed in a body block.

Indentation is ignored. `end` closes the innermost block.

## Printing

Two printers are normative.

The canonical printer emits source that the reader accepts and that denotes an
equal term. It is the stable interchange form. It is guaranteed for terms that
contain no approximate reals.

The display printer emits an infix form intended for people. It uses operators,
implicit multiplication, and the conventional mathematical layout. It is not
required to round trip, and its exact spacing is implementation-defined.

## Messages

`message(name,args...)`, `messageName(symbol,tag)`, `check(expr,fallback)`,
`quiet(expr)`, `off(name)`, and `on(name)` are ordinary call forms.

## Packages and up values

`import` names a bundled module, an identifier resolved through the package
path, or a quoted file path. A quoted path is relative to the importing source.

`upSet(lhs,rhs)`, `upSetDelayed(lhs,rhs)`, `tagSet(tag,lhs,rhs)`, and
`tagSetDelayed(tag,lhs,rhs)` install up values.

## Assignment, evaluation control, and control flow

The following are ordinary call forms and remain whitespace free:

- `part(expr,index,...)` and the surface form `expr[[index,...]]`;
- `hold`, `holdForm`, `holdPattern`, `evaluate`, `unevaluated`, and `sequence`;
- `compound`, `which`, `switch`, `do`, `while`, `for`, `return`, `break`,
  `continue`, `throw`, and `catch`;
- `unset`, `clear`, `clearAll`, and `remove`;
- `appendTo`, `prependTo`, `addTo`, `subtractFrom`, `timesBy`, and `divideBy`.

Part assignment uses `set` with a part target, as in `x[[2]]=9`. A part target
is not accepted by `setDelayed`.

Approximate reals print as decimal text. They are not accepted as source
literals in this version, so canonical round trips are not guaranteed for terms
that contain them.

## Context forms

`$Context` is the current context and `$ContextPath` is the ordered context
search path. `begin(name)` enters a context, `end()` leaves it, `context(x)`
reports the context of a symbol, and `contexts()` lists known contexts.

## Reserved

Notation declarations, user-defined operators, Unicode source identifiers, and
string literals with interpolation are reserved for later versions.
