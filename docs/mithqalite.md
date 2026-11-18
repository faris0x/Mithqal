# Mithqalite

> Status: Experimental. See [Maturity](maturity.md).

Mithqalite is the project's fork of [Malachite](https://github.com/mhogrefe/malachite). It lives at [faris0x/Mithqalite](https://github.com/faris0x/Mithqalite) and is checked out as a git submodule at `vendor/mithqalite`.

The fork is the pure Rust numeric core. Mithqal uses it for exact integers, rationals, and approximate real values. It is not a general-purpose dumping ground for every experimental numeric idea. Mithqal owns the engine-facing wrappers, complex numbers, hypercomplex numbers, fixed-point types, and special functions. Mithqalite owns changes that require access to Malachite internals.

## Initial setup

After cloning the repository, initialize the fork:

```text
git submodule update --init --recursive
```

The local fork has two remotes:

- `origin` points to `faris0x/Mithqalite`.
- `upstream` points to `mhogrefe/malachite`.

A fresh submodule checkout only has `origin`. Add the upstream remote with:

```text
sh scripts/setup-mithqalite.sh
```

## What belongs in Mithqalite

Changes that need internal access:

- Constants caching for elementary functions.
- Argument reduction and series evaluation.
- Low-level limb, rounding, and precision optimizations.
- Correctness fixes in Malachite's float implementation.

Changes that belong in Mithqal:

- The engine-facing `Int`, `Rat`, `Real`, and `Complex` wrappers.
- Complex arithmetic on top of Malachite Float.
- Quaternion, octonion, and sedenion types.
- Q31.32 fixed-point arithmetic.
- Gamma, lgamma, erf, and erfc.
- Display, interning, and expression arena integration.

## Patch rules

Every Mithqalite patch must have:

- A benchmark or correctness case that demonstrates the need.
- Tests that fail before the patch and pass after it.
- A small, reviewable diff.
- A note in Mithqal describing the patch and its upstream status.

Prefer an upstream pull request for patches that are useful to Malachite users. Drop a local patch once upstream accepts an equivalent change.

Do not copy code from another numeric library into Mithqalite without checking its license and documenting the origin. Malachite is LGPL-3.0-only. The fork remains LGPL-3.0-only when distributed.

## Synchronization

The fork is pinned by the submodule commit recorded in Mithqal. Update it deliberately:

```text
cd vendor/mithqalite
git fetch origin
git fetch upstream
git status
```

Rebase or merge upstream changes on the `mithqal` branch, run Malachite's tests, then run Mithqal's full test suite and conformance corpus. Commit the new submodule pointer in Mithqal only after both pass.

## CI

CI checks out submodules recursively. If the fork is not initialized, the build fails rather than silently using a registry copy.
