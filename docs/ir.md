# Structured IR

> Status: Experimental. See [Maturity](maturity.md).

The structured IR is a versioned, validated representation of a whole Mithqal program. It lives in `src/ir` and is separate from the expression arena and from the flat virtual machine instructions.

## Purpose

The evaluator currently consumes canonical expressions and the runtime compiler emits flat instructions. Both are useful for execution. A structured IR adds an explicit layer for lexical blocks, bindings, definitions, patterns, and control flow. Later passes can analyze that structure before lowering it to the existing virtual machine.

The IR is a scaffold. Parsing into it and lowering from it arrive in later work.

## Non-compatibility

Mithqal is not Mathematica compatible. The structured IR makes that separation explicit.

- Attribute names are Mithqal names such as `INERT`, `LISTABLE`, `FLAT`, `ORDERLESS`, `CONSTANT`, and `PROTECTED`. They are not Mathematica attribute spellings.
- Definitions use explicit `Definition` and `Rule` records. There are no down value or own value objects.
- Patterns use `Wildcard`, `Bind`, `Constructor`, `Sequence`, `Alternative`, and `Guard`. There are no blank classes or pattern test names from Mathematica.
- Binding and evaluation modes are explicit enum values. The IR does not borrow Mathematica evaluation order or hold attributes.
- The surface language and its printed forms remain Mithqal designs. Source compatibility with Mathematica is out of scope.

The shared idea is a symbolic call with a head and arguments. That idea is common to many symbolic systems and does not imply compatibility.

## Structure

`Module` owns these arenas:

- `symbols`, a symbol table with names, kinds, and attributes.
- `scopes`, lexical scopes with parent links and local symbols.
- `patterns`, a pattern arena.
- `expressions`, a structured expression arena.
- `blocks`, lexical statement sequences with a scope and references.
- `definitions`, rules keyed by a symbol.
- `entry`, the entry block.

Every reference is a typed newtype over `u32`. Validation rejects an index outside its arena.

Expressions cover unit, booleans, exact integers, rationals, approximate reals, text, symbols, lists, calls, lambdas, conditionals, matches, and quoted expressions.

Statements cover `let`, `define`, `assign`, expression evaluation, and `return`. Blocks own a scope, so lexical structure is visible before any lowering step.

Patterns cover wildcard matching, binding, literal matching, constructor matching, sequence matching, alternatives, and guarded patterns.

## Validation

`Module::validate` checks the format version, the entry block, every arena reference, duplicate symbol names, empty names, empty definitions, scope parent links, and scope cycles. It also enforces a combined node limit.

`Builder` is the intended construction path. It appends nodes in deterministic order and returns typed handles. `build` validates the finished module.

Forward references across arenas are allowed. Only the final module has to be well formed.

## What Plasma suggests

The Plasma IR offers several ideas worth keeping:

- A format version is part of the data, not an undocumented convention.
- Node encodings are append-only. New variants receive new discriminants.
- The frontend and the backends share one representation instead of translating ad hoc.
- Fixed limits make memory behavior predictable.
- Control flow is explicit and machine readable.

Mithqal cannot copy the flat bytecode shape directly. Plasma encodes control flow as `(offset, length)` ranges into a fixed operation array, which suits a heap-free quantum circuit frontend. Mithqal needs named definitions, nested lexical scopes, patterns, and value-producing control flow. The structured arena preserves those relationships and leaves flat lowering to a later pass.

Plasma's constant pool and versioned program record are still useful models. Mithqal uses arenas with typed handles, and keeps the version constant beside the module.

## Future lowering

The intended pipeline is:

```text
source text
  to reader
  to structured IR
  to typed core expressions
  to flat virtual machine instructions
```

The structured IR is the analysis point. Name resolution, pattern checks, scope validation, and definition checks can run there. Lowering can then target the existing evaluator and virtual machine.

## Example shape

```rust
let mut builder = Builder::new();
let x = builder.symbol("x", SymbolKind::Local, Attributes::new());
let x_expr = builder.expr(Expr::Symbol(x));
let one = builder.expr(Expr::Integer("1".into()));
let plus = builder.symbol("plus", SymbolKind::Builtin, Attributes::new());
let head = builder.expr(Expr::Symbol(plus));
let sum = builder.expr(Expr::Call {
    head,
    arguments: vec![x_expr, one],
    mode: CallMode::Strict,
});
let block = builder.block(Block {
    scope: ScopeId::new(0),
    statements: vec![Stmt::Expr(sum)],
});
builder.set_entry(block);
let module = builder.build()?;
```

The example omits imports and uses the current builder API.
