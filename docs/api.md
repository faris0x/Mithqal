# Public API

> Status: Stable. See [Maturity](maturity.md).

This document defines the stable library surface for the current version.

## Implementation status

The session, parsing, printing, exact arithmetic, comparisons, logic, error values, definitions, attributes, pattern matching, lexical `let`, closures, contexts, package loading, up values, messages, the typed instruction virtual machine, and the explicit collector are active. The example at the end of this document works with the current definition and pattern implementation.

## Design rules

Public items use Rust naming conventions. Rust methods use snake_case. Source identifiers use dromedaryCase.

The stable surface exposes sessions, expression handles, expression views, diagnostics, and configuration. Internal modules such as `expr` are hidden from public documentation and are not part of the stable contract.

Handles are valid until an explicit or automatic garbage collection removes them. The session roots its definitions, its last result, active language scopes, and any caller-supplied roots.

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
    pub fn eval_named_source(&mut self, name: &str, src: &str) -> Result<ExprId, Diagnostic>;
    pub fn eval_file(&mut self, path: &str) -> Result<ExprId, Diagnostic>;
    #[cfg(feature = "std")]
    pub fn needs(&mut self, name: &str) -> Result<ExprId, Diagnostic>;
    #[cfg(feature = "std")]
    pub fn get(&mut self, path: &str) -> Result<ExprId, Diagnostic>;
    pub fn eval_language(&mut self, src: &str) -> Result<ExprId, Diagnostic>;
    pub fn eval(&mut self, expr: ExprId) -> Result<ExprId, Diagnostic>;
    pub fn eval_compiled(&mut self, expr: ExprId) -> Result<ExprId, Diagnostic>;
    pub fn eval_compiled_statement(&mut self, src: &str) -> Result<ExprId, Diagnostic>;

    pub fn output(&self) -> &[String];
    pub fn take_output(&mut self) -> Vec<String>;

    pub fn format(&self, expr: ExprId) -> Result<String, Diagnostic>;
    pub fn format_infix(&self, expr: ExprId) -> Result<String, Diagnostic>;
    pub fn view(&self, expr: ExprId) -> Result<ExprView<'_>, Diagnostic>;
    pub fn is_live(&self, expr: ExprId) -> bool;

    pub fn collect_garbage(&mut self, roots: &[ExprId]);
    pub fn live_expressions(&self) -> usize;
    pub fn last_result(&self) -> Option<ExprId>;
    pub fn take_messages(&mut self) -> Vec<Message>;
}
```

`eval_source` dispatches module-shaped source to `eval_language`. Ordinary statement source is parsed and evaluated in order. It stops at the first diagnostic and returns the value of the final statement. `parse_source` returns one handle per statement.

`eval_named_source` is `eval_source` with a source name attached to every diagnostic and trace frame. `eval_file` reads a file and calls `eval_named_source` with the path.

`Session::new()` and `Session::with_config` load the bundled standard library before returning. Modules load in dependency order. A strict module, one with an `export` statement, exposes only its exports; other definitions are private and are installed in a module context. Exported names are protected and carry a declared arity. Descriptive statistics is implemented this way; see [Statistics](statistics.md).

`eval_language` parses and evaluates a surface language module, including exports, imports, structure and instance declarations, and local definitions. `output` returns the text collected by `prt` statements, and `take_output` moves that text out of the session.

`format` produces canonical source. The output can be parsed back into an equal expression. `format_infix` produces the human form. It may use operators and implicit multiplication. Both methods return `EvalNotLive` for a stale handle.

`view` borrows the session. The borrow ends before the next mutation. It returns `EvalNotLive` for a stale handle.

`is_live` reports whether a handle still refers to a live expression.

`eval_compiled` and `eval_compiled_statement` use the typed instruction compiler and machine. They share the arena and definitions with the symbolic path.

`collect_garbage` marks definitions, the last result, symbol nodes, active language scope values, and the supplied roots. `live_expressions` reports the number of live expression nodes. The session collects after a top level evaluation when the live node count exceeds the configured collection threshold, except while a language evaluation is active.

The session caches parsed statements, compiled code, and evaluation results. Evaluation caches are keyed by the current definition version, so installing or changing a definition invalidates them. Collection clears all caches because their handles are not extra roots.

## Expression handles

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExprId(/* private generational handle */);
```

Symbolic expressions are hash consed, so equal expressions have equal handles. Closures are expressions and follow the same interning rule.

An `ExprId` carries an arena generation. A handle that is not anchored by the session or by an explicit root becomes invalid after `collect_garbage`. `view`, `format`, `format_infix`, and `eval` return `EvalNotLive` instead of panicking. The `expr` module is an unstable internal surface and is not part of the stable API contract.

## Expression views

```rust
#[non_exhaustive]
pub enum ExprView<'a> {
    Integer(&'a Int),
    Rational(&'a Rat),
    Real(&'a Real),
    String(&'a str),
    Bytes(&'a [u8]),
    Symbol(&'a str),
    Call { head: ExprId, args: &'a [ExprId] },
    Association(&'a [AssociationEntry]),
}
```

A closure is represented as a call with the protected `closure` head. Its parameters, body, and captured environment are ordinary arguments. A caller can inspect the closure through `ExprView::Call`.

## Numbers

```rust
pub struct Int { /* Mithqalite Integer */ }
pub struct Rat { /* Mithqalite Rational */ }
```

`Int` and `Rat` implement `Clone`, `Debug`, and equality with the same type. They are not `Copy` because they own heap storage. The API provides construction from Rust integer types and decimal string conversion.

The numeric wrappers are plain Rust values with no manual initialization or cleanup.

```rust
pub struct Real { /* Mithqalite Float, binary precision, absolute error */ }
```

`Real` exposes `to_decimal`, `precision_digits`, `accuracy_digits`, `is_zero`, `sign`, and arithmetic operations. It is created by `approx` or by the public constructors `Real::from_rat`, `Real::pi`, and `Real::e`. It owns a Mithqalite Float value and carries an absolute error estimate.

`approx(expression,digits)` is the session level numeric boundary. It returns a `Real` expression when the input can be evaluated numerically.

Intervals are expressions with the protected `interval(low,high)` head. Their endpoints are exact values or `Real` values. The evaluator supplies addition, multiplication, negation, and integer powers for intervals.

Approximate complex values are expressions with the protected `complex(real,imaginary)` head. Their components are exact values or `Real` values. Numeric approximation supplies complex arithmetic, integer powers, reciprocals, and absolute value.

## Messages

`message(name, args...)` records a non-fatal `Message` instead of returning a `Diagnostic`. Messages carry a stable code such as `TGN-MSG-0001`, a name such as `f::tag`, and formatted text. `Session::take_messages` drains them. `check`, `quiet`, `off`, and `on` control message handling.

```rust
pub struct Message {
    pub code: &'static str,
    pub name: String,
    pub text: String,
}
```

## Diagnostics

```rust
pub struct Diagnostic {
    pub code: ErrorCode,
    pub kind: ErrorKind,
    pub message: Box<str>,
    pub span: Option<Box<Span>>,
    pub source: Option<Box<str>>,
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
    pub source: Option<String>,
}

#[non_exhaustive]
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

Parser diagnostics carry exact spans. Language evaluation diagnostics carry a source name and a trace whose frames point at the failing statements. `eval_named_source` and `eval_file` attach the source name to every diagnostic and frame. A frame produced inside a definition carries the definition site, so an error reports the definition line as well as the call site.

## Configuration

```rust
pub struct Config {
    pub max_steps: u64,
    pub max_depth: u32,
    pub initial_arena_capacity: usize,
    pub gc_threshold_nodes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_depth: 2048,
            initial_arena_capacity: 16_384,
            gc_threshold_nodes: 131_072,
        }
    }
}
```

The step budget counts evaluator and matcher steps. The depth limit protects the Rust stack. `initial_arena_capacity` preallocates arena slots. `gc_threshold_nodes` sets the live node count that triggers automatic collection after a top level evaluation.

Configuration does not introduce randomness or wall clock dependence.

`$MithqalPath` is initialized from the current directory and, under `std`, the colon separated `MITHQAL_PATH` environment variable. `Session::needs` and `Session::get` load package files.

## Error values

The symbols `undefined`, `indeterminate`, and `failed` are ordinary expression values. They appear in `Result::Ok` when they are valid results of a computation.

`Diagnostic` reports conditions that prevent evaluation, such as a parse failure, a protected symbol definition, a budget overflow, or a non callable head.

## Threading

A `Session` is owned by one thread at a time. It is not `Send` or `Sync` in the current version. Multiple sessions can run on multiple threads with independent arenas.

## Stability

Version 0.0.x allows breaking changes. Every breaking change is recorded in `CHANGELOG.md`. From version 0.1.0, the public API follows semantic versioning.

`ExprView`, `ErrorKind`, `ErrorCode`, and `NumError` are `#[non_exhaustive]`. Private modules are not part of the API contract.

## Example

```rust
use mithqal::Session;

fn main() -> Result<(), mithqal::Diagnostic> {
    let mut session = Session::new();
    let value = session.eval_statement("f(x_):=x^2+1")?;
    let _ = value;
    let result = session.eval_statement("f(3)")?;
    assert_eq!(session.format(result)?, "10");
    Ok(())
}
```
