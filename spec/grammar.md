# Surface Grammar

> Spec version: 0.0.1 (draft)

## Source encoding

A source file is ASCII in this version. Unicode source is reserved. A source
line is terminated by a newline. A trailing newline is optional.

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

## Literals

An integer literal is a sequence of decimal digits.

A decimal literal integrates an optional fractional part and denotes an exact
rational. `1.25` denotes `125/100`. There is no exponent literal. Scientific
notation is written with a power of ten, as in `25*10^-2`.

The approximate reals produced by `approx` have no source literal form in this
version.

## Expressions

The expression forms are:

- a literal;
- an identifier;
- a call `head(arg, ...)`;
- a list `[item, ...]`;
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
- `condition(pattern, expression)` attaches a guard to a pattern.

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

Approximate reals print as decimal text. They are not accepted as source
literals in this version, so canonical round trips are not guaranteed for terms
that contain them.

## Reserved

Qualified names, notation declarations, user-defined operators, Unicode source,
and string literals are reserved for later versions.
