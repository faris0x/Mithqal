# Surface Grammar Additions

> Spec version: 0.0.3 (draft)

This document records the 0.0.2 additions to the surface grammar. The main
grammar in `grammar.md` remains authoritative for the rest of the language.

## Strings

A string literal is a double quoted sequence. The escape sequences are:

- `\"` for a double quote;
- `\\` for a backslash;
- `\n` for a newline;
- `\r` for a carriage return;
- `\t` for a tab;
- `\0` for a zero byte.

A string literal may contain spaces. Spaces inside a string are data, not
statement whitespace. Source identifiers remain ASCII; string contents may be
Unicode.

## Byte arrays

A byte array literal is a `byteArray(...)` call whose arguments are exact
integers in the range 0 through 255. The canonical printer uses this form. A
future version may add a packed literal syntax.

## Associations

An association literal has the form:

```text
<|key->value,key->value|>
```

The literal contains no whitespace. Keys and values are expressions. The
association constructor evaluates keys and values, canonicalizes keys, sorts
the entries by canonical key order, and rejects duplicate keys.

The canonical printer emits the literal form.

## Qualified names

A qualified name is a sequence of one or more identifiers separated by `;`:

```text
name
ctx;name
a;b;name
```

The final segment is the short symbol name. The preceding segments name the
context path from the root context.

A qualified name contains no whitespace. A semicolon is part of the name, not a
statement separator.

## Context forms

`$Context` is the current context. `$ContextPath` is the list of contexts
searched for a short name. `Begin(name)` enters a context and `End()` returns to
the previous context. `Context(symbol)` returns the context of a symbol and
`Contexts()` returns the known contexts.

## Whitespace

The 0.0.1 whitespace rule is unchanged. No new whitespace is allowed inside an
expression or a statement.
