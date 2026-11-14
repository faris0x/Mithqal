# cas

A symbolic math engine.

The public interface is a Rust library. Programs are whitespace free statements separated by newlines. Exact arithmetic uses GMP for arbitrary precision integers and rationals. The engine treats expressions as terms and defines evaluation as deterministic rewriting.

This repository is in early development. The exact number layer, expression arena, source reader, printers, arithmetic evaluator, definitions, pattern matching, lexical `let`, diagnostics, public session API, typed instruction virtual machine, and explicit mark and sweep collector are implemented. Closures remain in progress. Numeric approximation, calculus, and algebra come later.

## Documentation

- [Semantics](docs/semantics.md)
- [Syntax](docs/syntax.md)
- [Virtual machine](docs/vm.md)
- [Public API](docs/api.md)
- [Roadmap](docs/roadmap.md)

## Status

The current version targets Linux on x86 64. The crate name `cas` is a placeholder. The project has no product name at this stage.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
