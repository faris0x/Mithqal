# Changelog

## 0.0.1

Initial kernel work.

- Added project documentation for semantics, syntax, the virtual machine, the public API, and the roadmap.
- Added a single crate skeleton.
- Added vendored GMP 6.3.0 and a build script.
- Added exact `Int` and `Rat` wrappers with RAII cleanup.
- Added integer and rational arithmetic tests, including large divisor division.
- Added the hash consed expression arena, symbol table, attributes, and canonical order.
- Added the line based lexer and recursive descent parser, including implicit multiplication, comments, patterns, and exact decimals.
- Added canonical and infix printers with round trip tests.
- Added the exact arithmetic evaluator with listable threading, comparisons, logic, error values, and Gaussian rational normalization.
- Added the public `Session`, `ExprId`, `ExprView`, `Config`, and diagnostic surface.
- Added integration tests for numbers, expressions, syntax, printing, evaluation, and the session API.
