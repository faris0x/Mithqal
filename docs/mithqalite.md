# Mithqalite

> Status: Experimental. See [Maturity](maturity.md).

Mithqalite is the project's independent fork of [Malachite](https://github.com/mhogrefe/malachite). It lives at [faris0x/Mithqalite](https://github.com/faris0x/Mithqalite) and is checked out as a git submodule at `vendor/mithqalite`.

Mithqalite does not follow Malachite upstream. It has its own direction, release cadence, and compatibility decisions. Changes made in Malachite are not expected to be merged into Mithqalite, and Mithqalite is not expected to remain source-compatible with future Malachite releases.

The workspace crates are named `mithqalite`, `mithqalite-base`, `mithqalite-nz`, `mithqalite-q`, and `mithqalite-float`. The fork is the pure Rust numeric core. Mithqal uses it for exact integers, rationals, and approximate real values. Mithqal owns the engine-facing wrappers, complex numbers, hypercomplex numbers, fixed-point types, and special functions. Mithqalite owns changes that require access to the numeric internals.

## Initial setup

After cloning the repository, initialize the fork:

```text
git submodule update --init --recursive
```

The fork's remote is `origin`, which points to `faris0x/Mithqalite`. There is no upstream synchronization remote.

## What belongs in Mithqalite

Changes that need internal access:

- Constants caching for elementary functions.
- Argument reduction and series evaluation.
- Low-level limb, rounding, and precision optimizations.
- Correctness fixes in the float implementation.

Changes that belong in Mithqal:

- The engine-facing `Int`, `Rat`, `Real`, and `Complex` wrappers.
- Complex arithmetic on top of Mithqalite Float.
- Quaternion, octonion, and sedenion types.
- Q31.32 fixed-point arithmetic.
- Gamma, lgamma, erf, erfc, and erfi.
- Display, interning, and expression arena integration.

## Update rules

Every Mithqalite update must have:

- A benchmark or correctness case that demonstrates the need.
- Tests that fail before the update and pass after it.
- A small, reviewable diff.
- A note in Mithqal describing the update.

Do not copy code from another numeric library into Mithqalite without checking its license and documenting the origin. Mithqalite is LGPL-3.0-only. The fork remains LGPL-3.0-only when distributed.

## Version control

The fork is pinned by the submodule commit recorded in Mithqal. Update it deliberately:

```text
cd vendor/mithqalite
git fetch origin
git status
```

Run Mithqalite's tests, then run Mithqal's full test suite and conformance corpus. Commit the new submodule pointer in Mithqal only after both pass.

## CI

CI checks out submodules recursively. If the fork is not initialized, the build fails rather than silently using a registry copy.
