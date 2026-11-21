# no_std

> Status: Experimental. See [Maturity](maturity.md).

Mithqal and Mithqalite build without the standard library when the `std` feature is disabled. Both require a global allocator because the expression arena, symbols, strings, and numeric values use `alloc`.

## Building

The default build includes `std` and the command line frontend. To build the library without either:

```text
cargo build --no-default-features --lib
```

The library uses `alloc` and `core` only. The consumer must provide a global allocator and a panic handler in the final binary.

The CLI is std-only. Enabling `cli` enables `std` automatically.

## Targets

The no-std library is checked for:

- `wasm32-unknown-unknown`
- `thumbv7m-none-eabi`

Both checks compile the library without linking a final executable. Embedded binaries still need an allocator, a panic handler, and startup code appropriate to the target.

## Feature contract

| Feature | Effect |
| --- | --- |
| `std` | Enables standard-library collections, file loading, and `mithqalite/std`. Enabled by default. |
| `cli` | Enables the command line frontend and implies `std`. |
| `numberfields` | Compiles exact integer lattice, polynomial, rational matrix, number field arithmetic, maximal orders, the different, fractional ideals, prime decomposition, small-degree class groups, general unit groups with totally real saturation completion and regulator enclosures, general ray class orders and congruence kernels, experimental analytic zeta, Riemann and Dirichlet functional equations, and fixed-precision p-adic field arithmetic. Enabled by default. |
| no `std` | Uses `core`, `alloc`, `hashbrown`, and `libm`. File loading and the CLI are unavailable. |

Mithqalite uses the same pattern. Its numeric crates have their own `std` features, and the constant cache is active only when `std` is enabled.

## Floating-point functions

`core` does not provide every `f64` transcendental function. The no-std build supplies them through `libm`. The std build uses the platform `f64` methods.
