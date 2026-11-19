# Migrating to 0.0.2

> Status: Experimental. See [Maturity](maturity.md).

Version 0.0.2 keeps the 0.0.1 whitespace rule: expressions contain no
whitespace. A statement still occupies one line.

## Reserved tokens

- `;` is now a qualified-name separator, as in `ctx;name`. It is not a statement
  separator.
- `<|` and `|>` delimit an association literal.
- `[[` and `]]` delimit indexing.

Existing source that does not use these tokens is unaffected.

## New expression forms

- Strings, byte arrays, and associations.
- Indexing with one-based and negative indices, plus `span(start,end)`.
- Part assignment, `unset`, `clear`, `clearAll`, and mutation helpers.
- `hold`, `evaluate`, `unevaluated`, `sequence`, and `compound`.
- `which`, `switch`, `do`, `while`, `for`, `return`, `break`, `continue`,
  `throw`, and `catch`.
- Contexts and qualified names.
- User attributes and definition introspection.
- Typed blanks, alternatives, pattern tests, optionals, repetitions, `except`,
  `verbatim`, and orderless sequence matching.
- Package loading and up values.
- Messages and message controls.

## Contexts and packages

Short names resolve through the current context and `$ContextPath`. Strict
module private definitions live in a module context and are hidden from short
name lookup. Use `module;name` for qualified access.

Package files load with `import "path.mith"`. `Session::needs` loads once and
`Session::get` re-evaluates.

## Compatibility notes

- Private module names changed from mangled source names to module contexts.
- Up values are tried before down values and built in implementations.
- I/O remains `std` only and unsandboxed in this version.
- Nested same-head `flat` grouping is not implemented yet.
