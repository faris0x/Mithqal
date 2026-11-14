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
- Added an explicit non moving mark and sweep collector with root tracking, free slot reuse, and intern table rebuilding.
- Added a typed instruction compiler and virtual machine with conditional control flow and a compiled session path.
- Added lexical closures with parameter binding, captured environments, nested closures, and recursive closure calls.
- Added integration tests for numbers, expressions, syntax, printing, evaluation, definitions, patterns, closures, garbage collection, the compiled path, and the session API.
- Added automatic top level collection when the live expression count crosses the configured threshold.
- Optimized hot paths with binary limb keys for interning, FNV hashed maps, hash based like term and base collection, a head resolution fast path, and a release profile with thin LTO and one codegen unit.
- Added a benchmark harness for parsing, evaluation, pattern calls, closure calls, compiled execution, and collection.
- Expanded the suite to more than 230 tests covering normal behavior, boundaries, malformed input, and error propagation.
