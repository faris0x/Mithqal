# Mithqal

A symbolic math engine.

The public interface is a Rust library with an optional command line frontend. Expressions are whitespace free within a statement. Programs use newline separated statements and language blocks. Exact arithmetic uses GMP for arbitrary precision integers and rationals. The engine treats expressions as terms and defines evaluation as deterministic rewriting.

This repository is in early development. The exact number layer, approximate reals with significance tracking, directed rounded interval arithmetic, approximate complex arithmetic and elementary functions, exact polynomial algebra with staged multivariate factorisation through eight variables, exact symbolic calculus with definite and supported improper integrals, exact rational linear algebra with eigenvalues and eigenvectors, exact integer number theory, exact descriptive statistics, boolean logic, a line-based surface language, internal expression arena, source reader, printers, arithmetic evaluator, definitions, pattern matching, lexical `let`, closures, diagnostics, public session API, experimental structured IR, typed instruction virtual machine, and explicit mark and sweep collector are implemented. General multivariate factorisation beyond the staged bounds, broader assumption domains, and inferential statistics remain open.

## Building

```text
cargo build
cargo run
```

Use `cargo build --no-default-features --lib` to build the engine without the command line frontend. The `cli` feature controls the frontend and the `gui` feature is reserved for a future graphical frontend.

The supported build target is Linux on x86-64. A native build requires:

- The Rust toolchain pinned in `rust-toolchain.toml`.
- A C compiler such as GCC or Clang.
- GNU Make.
- The standard build tools required by the vendored GMP, MPFR, and MPC configure scripts.

The first build compiles the vendored numeric libraries and can take several minutes. Cargo caches the result in `target`.

## Documentation

- [Language](docs/language.md)
- [Command line interface](docs/cli.md)
- [Algebra](docs/algebra.md)
- [Assumptions](docs/assumptions.md)
- [Limits and series](docs/limits.md)
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
- [Maturity](docs/maturity.md)
- [Testing](docs/testing.md)

## Status

The current version targets Linux on x86 64. The project is named Mithqal. The project has no product name at this stage.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
