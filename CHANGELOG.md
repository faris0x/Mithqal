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
- Added vendored MPFR and MPC sources with static linking in the build script.
- Added approximate `Real` values with MPFR storage, significance tracking, precision and accuracy reporting, and first order error propagation.
- Added the `approx` numeric boundary with exact rational conversion, `pi` and `e`, arithmetic, powers, roots, exponentials, logarithms, and trigonometric functions.
- Added approximate value views, printing, collection, compiled path support, and numeric tests.
- Renamed the project and crate to Mithqal.
- Added approximate interval arithmetic with addition, multiplication, negation, integer powers, validation, collection support, and tests.
- Added approximate complex arithmetic with a `complex(real,imaginary)` value, `i` recognition in `approx`, addition, multiplication, division, integer powers, reciprocals, absolute value, collection support, compiled path support, and tests.
- Added fast small integer construction for real endpoints and reused real zero and real one values in interval and approximation paths.
- Added all numeric real folding in `plus` and `times`, interval zero power normalization, correct negative interval powers, and shorter precision aware real interning keys.
- Added general real powers through MPFR, complex non-integer powers through MPC, and complex exponential, logarithm, square root, sine, cosine, and tangent functions.
- Added stable scaled complex division, complex power handling, negative real square root and logarithm promotion to complex values, and conservative interval endpoint widening from significance error.
- Added real derivative corrections for sine, cosine, and integer powers.
- Expanded numeric, interval, and complex tests to cover normal values, boundaries, malformed input, errors, precision changes, collection, and the compiled path.
- Added own values, down values, protected symbol checks, definition specificity, and immediate and delayed definitions.
- Added pattern matching for blanks, named patterns, sequence patterns, repeated variables, and conditions.
- Added lexical `let` bindings with local shadowing.
- Added an explicit non moving mark and sweep collector with root tracking, free slot reuse, and intern table rebuilding.
- Added a typed instruction compiler and virtual machine with conditional control flow and a compiled session path.
- Added lexical closures with parameter binding, captured environments, nested closures, and recursive closure calls.
- Added integration tests for numbers, expressions, syntax, printing, evaluation, definitions, patterns, closures, garbage collection, the compiled path, and the session API.
- Added automatic top level collection when the live expression count crosses the configured threshold.
- Optimized hot paths with binary limb keys for interning, FNV hashed maps, hash based like term and base collection, a head resolution fast path, and a release profile with thin LTO and one codegen unit.
- Added parse, compiled code, and definition versioned evaluation caches. Definition changes invalidate the result caches, and collection clears every cache.
- Added constant folding of numeric atoms in the compiler so integer and rational literals avoid an evaluation instruction.
- Added a machine call frame stack that isolates caller and callee operand stack regions for symbol calls.
- Added a benchmark harness for cold parsing, cached evaluation, pattern calls, closure calls, compiled execution, and collection.
- Expanded the suite to more than 300 tests covering normal behavior, boundaries, malformed input, numeric significance, intervals, complex arithmetic, and error propagation.
