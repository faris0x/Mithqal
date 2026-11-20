# Mithqal

فَمَن يَعْمَلْ **مِثْقَالَ** ذَرَّةٍ خَيْرًا يَرَهُ

"So whosoever does good equal to the weight of an atom shall see it" (Qur'an 99:7, Hilali and Khan)

/mɪθqɔːl/

A symbolic math engine.

The public interface is a Rust library with a Mithqal language and an optional command line frontend. Expressions are whitespace free within a statement. Programs use newline separated statements and language blocks. Exact arithmetic uses Mithqalite for arbitrary precision integers and rationals. The engine treats expressions as terms and defines evaluation as deterministic rewriting.

Version 0.0.2 adds strings, byte arrays, associations, indexing, part assignment, contexts, packages, user attributes, up values, messages, and file text/CSV/TSV/JSON input and output. This repository is in early development. The exact number layer, approximate reals with significance tracking, directed rounded interval arithmetic, approximate complex arithmetic and elementary functions, exact polynomial algebra with staged multivariate factorisation through eight variables, exact symbolic calculus with definite and supported improper integrals, exact rational linear algebra with eigenvalues and eigenvectors, exact integer number theory, a bundled Mithqal standard library covering descriptive statistics, finite sets and relations, structure dictionaries, combinatorics, probability, decision theory, games, graphs, information theory, and calculus rules, boolean logic, a line-based surface language with modules, exports, local definitions, canonical binders, and structure declarations, internal expression arena, source reader, printers, arithmetic evaluator, definitions, pattern matching, lexical `let`, closures, diagnostics, public session API, experimental structured IR, typed instruction virtual machine, and explicit mark and sweep collector are implemented. General multivariate factorisation beyond the staged bounds, deeper assumption inference beyond the documented fact language, inferential statistics, proofs, and notation declarations remain open.

## Building

```text
cargo build
cargo run
```

Use `cargo build --no-default-features --lib` to build the engine without the command line frontend. The `cli` feature controls the frontend and the `gui` feature is reserved for a future graphical frontend.

The supported desktop build target is Linux on x86-64. A native build requires the Rust toolchain pinned in `rust-toolchain.toml`. Numeric work is pure Rust; no C compiler, Make, or native numeric libraries are required. The first build compiles Mithqalite from `vendor/mithqalite` and caches the result in `target`.

The library also builds without the standard library:

```text
cargo build --no-default-features --lib
```

The no-std build uses `core` and `alloc`, and requires the final binary to provide a global allocator and panic handler. The CLI is std-only. See [no_std](./docs/no_std.md).


## Usage

Evaluate an expression from the command line:

```text
$ cargo run -- -e "2+3"
5
```

Start the interactive calculator with `cargo run`. The prompt accepts ordinary expressions and definitions:

```text
$ cargo run
> differentiate(x^3,x)
3*x^2
```

A surface language module looks like this:

```text
mod example
def polynomial(x)=x^3+2*x
prt polynomial(2)
prt derive(polynomial(x),x)
end
```

The module prints:

```text
12
2+3*x^2
```

## Documentation

- [Language (latest)](./docs/language/latest.md)
- [Language 0.0.1 archive](./docs/language/language-0.0.1.md)
- [Migrating to 0.0.2](./docs/migration-0.0.2.md)
- [Language specification](./spec/README.md)
- [Command line interface](./docs/cli.md)
- [Function capabilities](./docs/functions.md)
- [Library modules](./docs/modules.md)
- [Mathematical domains](./docs/domains.md)
- [Algebra](./docs/algebra.md)
- [Assumptions](./docs/assumptions.md)
- [Limits and series](./docs/limits.md)
- [Linear algebra](./docs/linalg.md)
- [Number theory](./docs/numbertheory.md)
- [Statistics](./docs/statistics.md)
- [Logic](./docs/logic.md)
- [Semantics](./docs/semantics.md)
- [Syntax](./docs/syntax.md)
- [Virtual machine](./docs/vm.md)
- [Public API](./docs/api.md)
- [Structured IR](./docs/ir.md)
- [Roadmap](./docs/roadmap.md)
- [Maturity](./docs/maturity.md)
- [Testing](./docs/testing.md)
- [Mithqalite numeric fork](./docs/mithqalite.md)
- [no_std](./docs/no_std.md)

## Status

Mithqal is in early development at version 0.0.2. The supported desktop build target is Linux on x86-64. The library also builds without the standard library for `wasm32-unknown-unknown` and `thumbv7m-none-eabi`; those targets are compile-checked, not full product targets. Windows and macOS desktop builds are not yet supported. See [no_std](./docs/no_std.md).

## License

GPL-3.0-only. See [LICENSE](./LICENSE).

## AI Declaration

I use large language models through the OpenCode coding assistant while developing Mithqal. I use the assistant for design discussion, implementation drafts, test construction, refactoring, and documentation editing. I review and accept every change, run the test suite, and check the format, lint, and documentation gates. I remain responsible for correctness, licensing, and provenance. I do not list an AI system as an author, and I do not include generated datasets or third-party copyrighted text.
