# Mithqal

A symbolic math engine.

The public interface is a Rust library. Programs are whitespace free statements separated by newlines. Exact arithmetic uses GMP for arbitrary precision integers and rationals. The engine treats expressions as terms and defines evaluation as deterministic rewriting.

This repository is in early development. The exact number layer, approximate reals with significance tracking, directed rounded interval arithmetic, approximate complex arithmetic and elementary functions, exact polynomial algebra, exact symbolic calculus with definite integrals, exact rational linear algebra, exact integer number theory, exact descriptive statistics, boolean logic, a line-based surface language, expression arena, source reader, printers, arithmetic evaluator, definitions, pattern matching, lexical `let`, closures, diagnostics, public session API, structured IR scaffold, typed instruction virtual machine, and explicit mark and sweep collector are implemented. Full multivariate factorization, improper integration, matrix eigenvalues, and inferential statistics come later.

## Documentation

- [Language](docs/language.md)
- [Algebra](docs/algebra.md)
- [Linear algebra](docs/linalg.md)
- [Number theory](docs/numbertheory.md)
- [Statistics](docs/statistics.md)
- [Logic](docs/logic.md)
- [Semantics](docs/semantics.md)
- [Syntax](docs/syntax.md)
- [Virtual machine](docs/vm.md)
- [Public API](docs/api.md)
- [Structured IR](docs/ir.md)
- [Roadmap](docs/roadmap.md)

## Status

The current version targets Linux on x86 64. The project is named Mithqal. The project has no product name at this stage.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
