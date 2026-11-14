# Public API

This document defines the stable library surface for the current version.

## Implementation status

The session, parsing, printing, exact arithmetic, comparisons, logic, error values, definitions, pattern matching, lexical `let`, closures, the typed instruction virtual machine, and the explicit collector are active. The example at the end of this document works with the current definition and pattern implementation.

## Design rules

Public items use Rust naming conventions. Rust methods use snake_case. Source identifiers use dromedaryCase.

The public surface exposes sessions, expression handles, expression views, diagnostics, and configuration. The arena, hash table, matcher, virtual machine, and garbage collector remain private.

Handles are valid until an explicit garbage collection removes them. The session roots its definitions and its last result. Callers pass additional roots when they collect.

## Session

```rust
pub struct Session {
    // private
}

impl Session {
    pub fn new() -> Self;
    pub fn with_config(config: Config) -> Self;

    pub fn parse_statement(&mut self, src: &str) -> Result<ExprId, Diagnostic>;
    pub fn parse_source(&mut self, src: &str) -> Result<Vec<ExprId>, Diagnostic>;

    pub fn eval_statement(&mut self, src: &str) -> Result<ExprId, Diagnostic>;
    pub fn eval_source(&mut self, src: &str) -> Result<ExprId, Diagnostic>;
    pub fn eval(&mut self, expr: ExprId) -> Result<ExprId, Diagnostic>;
    pub fn eval_compiled(&mut self, expr: ExprId) -> Result<ExprId, Diagnostic>;
    pub fn eval_compiled_statement(&mut self, src: &str) -> Result<ExprId, Diagnostic>;

    pub fn format(&self, expr: ExprId) -> String;
    pub fn format_infix(&self, expr: ExprId) -> String;
    pub fn view(&self, expr: ExprId) -> ExprView<'_>;

    pub fn collect_garbage(&mut self, roots: &[ExprId]);
    pub fn live_expressions(&self) -> usize;
    pub fn last_result(&self) -> Option<ExprId>;
}
```

`eval_source` evaluates statements in order. It stops at the first diagnostic and returns the value of the final statement. `parse_source` returns one handle per statement.

`format` produces canonical source. The output can be parsed back into an equal expression. `format_infix` produces the human form. It may use operators and implicit multiplication.

`view` borrows the session. The borrow ends before the next mutation.

`eval_compiled` and `eval_compiled_statement` use the typed instruction compiler and machine. They share the arena and definitions with the symbolic path.

`collect_garbage` marks definitions, the last result, symbol nodes, and the supplied roots. `live_expressions` reports the number of live expression nodes. The session also collects after a top level evaluation when the live node count exceeds the configured collection threshold.

The session caches parsed statements, compiled code, and evaluation results. Evaluation caches are keyed by the current definition version, so installing or changing a definition invalidates them. Collection clears all caches because their handles are not extra roots.

## Expression handles

```rust
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExprId(u32);
```

Symbolic expressions are hash consed, so equal expressions have equal handles. Closures are expressions and follow the same interning rule.

An `ExprId` that is not anchored by the session or by an explicit root can become invalid after `collect_garbage`.

## Expression views

```rust
pub enum ExprView<'a> {
    Integer(&'a Int),
    Rational(&'a Rat),
    Real(&'a Real),
    Symbol(&'a str),
    Call { head: ExprId, args: &'a [ExprId] },
}
```

A closure is represented as a call with the protected `closure` head. Its parameters, body, and captured environment are ordinary arguments. A caller can inspect the closure through `ExprView::Call`.

## Numbers

```rust
pub struct Int { /* GMP mpz */ }
pub struct Rat { /* GMP mpq */ }
```

`Int` and `Rat` implement `Clone`, `Debug`, and equality with the same type. They are not `Copy` because they own GMP storage. The API provides construction from Rust integer types and decimal string conversion.

The numeric wrappers enforce GMP initialization and cleanup through `Drop`. No public method exposes a raw pointer.

```rust
pub struct Real { /* MPFR value, binary precision, absolute error */ }
```

`Real` exposes `to_decimal`, `precision_digits`, `accuracy_digits`, `is_zero`, `sign`, and arithmetic operations. It is created by `approx` or by the public constructors `Real::from_rat`, `Real::pi`, and `Real::e`. It enforces MPFR initialization and cleanup through `Drop`.

`approx(expression,digits)` is the session level numeric boundary. It returns a `Real` expression when the input can be evaluated numerically.

## Diagnostics

```rust
pub struct Diagnostic {
    pub code: ErrorCode,
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub trace: Vec<Frame>,
    pub notes: Vec<String>,
}

pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

pub struct Frame {
    pub head: Option<String>,
    pub expression: String,
    pub span: Option<Span>,
}

pub enum ErrorKind {
    Parse,
    Eval,
    Numeric,
    Internal,
}
```

`ErrorCode` is a stable identifier such as `TGN-PARSE-0001` or `TGN-EVAL-0010`. Codes are public and documented. Messages are deterministic and contain no locale dependent text.

`Diagnostic` implements `Display`, `Debug`, and `std::error::Error`. Its display form includes the code, message, source span, source line, and trace.

```text
error TGN-EVAL-0010: cannot call a non callable value
  --> <input>:1:1
   |
 1 | 3(x)
   | ^
   = trace:
      1. eval call(3,[x])
      2. apply: head evaluated to 3
```

Parser diagnostics carry exact spans. Evaluator diagnostics carry a trace of heads and expressions. Full source mapping for evaluator frames arrives with a located expression layer later.

## Configuration

```rust
pub struct Config {
    pub max_steps: u64,
    pub max_depth: u32,
    pub initial_arena_bytes: usize,
    pub gc_threshold_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        // deterministic defaults
    }
}
```

The step budget counts evaluator and matcher steps. The depth limit protects the Rust stack. The arena settings control initial allocation and the automatic collection threshold used when automatic collection is enabled in a later version.

Configuration does not introduce randomness or wall clock dependence.

## Error values

The symbols `undefined`, `indeterminate`, and `failed` are ordinary expression values. They appear in `Result::Ok` when they are valid results of a computation.

`Diagnostic` reports conditions that prevent evaluation, such as a parse failure, a protected symbol definition, a budget overflow, or a non callable head.

## Threading

A `Session` is owned by one thread at a time. It is not `Send` or `Sync` in the current version. Multiple sessions can run on multiple threads with independent arenas.

## Stability

Version 0.0.x allows breaking changes. Every breaking change is recorded in `CHANGELOG.md`. From version 0.1.0, the public API follows semantic versioning.

Public enums are `#[non_exhaustive]` where extension is expected. Private modules are not part of the API contract.

## Example

```rust
use cas::Session;

fn main() -> Result<(), cas::Diagnostic> {
    let mut session = Session::new();
    let value = session.eval_statement("f(x_):=x^2+1")?;
    let _ = value;
    let result = session.eval_statement("f(3)")?;
    assert_eq!(session.format(result), "10");
    Ok(())
}
```
