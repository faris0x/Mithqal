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
- Added own values, down values, protected symbol checks, definition specificity, and immediate and delayed definitions.
- Added pattern matching for blanks, named patterns, sequence patterns, repeated variables, and conditions.
- Added lexical `let` bindings with local shadowing.
- Added integration tests for numbers, expressions, syntax, printing, evaluation, definitions, patterns, and the session API.
- Expanded the suite to more than 200 tests covering normal behavior, boundaries, malformed input, and error propagation.
