# Command Line Interface

> Status: Stable. See [Maturity](maturity.md).

The `mithqal` binary provides a line-oriented calculator. It is built with the engine by default through the `cli` Cargo feature. The engine library keeps its empty dependency graph. The frontend uses `rustyline` for line editing and history with Tab completion.

## Building

```text
cargo build
cargo run
```

The frontend can be removed from a build:

```text
cargo build --no-default-features --lib
```

A future graphical frontend will use the same module and feature arrangement under a `gui` feature.

## Modes

```text
mithqal
mithqal --eval 'integrate(x^2,0,5)'
mithqal --license
mithqal --help
mithqal --version
```

## Interactive commands

```text
help
help FUNCTION
list functions [TEXT]
list capabilities [TEXT]
set
set timing on|off
set pretty on|off
license
quit
```

An assignment can be written with `=`:

```text
> x = 5
5
> x^2
25
```

The REPL keeps one session for the whole run, so definitions remain available. Errors print a diagnostic and the session continues. Non-fatal messages print to stderr before the result.

`factorInteger` results use a prime product in the default display:

```text
> factorInteger(360)
2^3*3^2*5
```

With `set pretty off`, the underlying list is printed:

```text
> factorInteger(360)
[[2,3],[3,2],[5,1]]
```

Timing output:

```text
> set timing on
timing: on
> 5+3
8 (12.14 us)
```

## Completion and history

`list capabilities` prints the support matrix for the advertised functions. `help FUNCTION` prints the same metadata for one function. `set` shows the current settings. `set timing` adds the processing time to each result. `set pretty` controls display formatting such as prime products. Tab completes command names and built-in function names. History is stored in `$XDG_DATA_HOME/mithqal/history` or `$HOME/.local/share/mithqal/history`. Ctrl-C clears the current line. Ctrl-D exits.

## Licensing

The `license` command and the `--license` flag print a summary that covers the project license and the dependency licenses. The complete list, license texts, and repository links are in [license/LICENSES.md](../license/LICENSES.md).
