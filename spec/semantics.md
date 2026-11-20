# Evaluation Semantics

> Spec version: 0.0.2 (draft)

## Evaluation

Evaluation is a deterministic rewrite relation. It produces a term or a
diagnostic. Every evaluation carries a step budget and a depth budget.

A term is evaluated as follows.

1. An integer, rational, approximate real, string, byte array, or association
   evaluates to itself.
2. A symbol resolves in this order: a lexical binding, an own value, a
   built-in constant default, the symbol itself. A short name is resolved
   through the current context and `$ContextPath`; a context-qualified name is
   resolved directly in its context.
3. A call evaluates its head. If the head has a lexical binding or an own value,
   the head is evaluated. The head MUST evaluate to a symbol or to a closure.
4. A call with a closure head applies the closure to its evaluated arguments.
5. A control head is evaluated before its arguments. Control heads include
   `if`, `and`, `or`, `not`, `let`, `localDefs`, `function`, `approx`,
   `interval`, `complex`, `map`, `apply`, `fold`, `nest`, `set`, `setDelayed`,
   `hold`, `check`, `quiet`, `off`, `on`, and the loop, return, throw, and catch
   forms.
6. A call whose head holds its arguments returns the call without evaluating
   those arguments. Held arguments are evaluated only by the control form that
   owns them.
7. All remaining arguments are evaluated from left to right.
8. A listable head threads over list arguments.
9. Up values of the head and of symbols in the arguments are tried.
10. The built-in implementation for the head is applied when one exists.
11. User down values for the head are tried in order. The first matching rule
    with a guard that evaluates to `true` is applied.
12. The result is canonicalized and interned. If the result differs from the
    original call, the result is evaluated again.

## Rule selection

Rules for a symbol come from the innermost lexical definition scope that defines
the symbol. A local definition shadows outer scopes and the global definitions
entirely. Within one scope, down values are tried in source order. Up values are
tried before built in implementations and down values.

A rule matches when its left hand side matches the call arguments and every
condition evaluates to `true`. A false condition selects the next rule. If no
rule matches, the call remains unevaluated, unless the arity check below
produces a diagnostic.

Arity is derived from the visible rules. If every visible rule has a fixed
number of parameters, a call with a different argument count produces
`EvalArity`. If any visible rule has a sequence parameter, the function is
variadic and no arity diagnostic is produced. A symbol with no visible rules is
not arity checked.

## Definitions

`set(lhs, rhs)` evaluates `rhs`, installs a definition, and returns the evaluated
`rhs`.

`setDelayed(lhs, rhs)` installs a delayed definition and returns `null`.

A left hand side that is a symbol installs an own value. A left hand side that is
a call installs a down value for its head. Arguments that use pattern syntax are
patterns. A plain symbol argument is a literal symbol.

A definition whose left hand side is a call is delayed in the surface language.
A protected symbol MUST NOT be redefined. A definition whose head name is a
kernel head is rejected with `EvalProtected`, even when the symbol was interned
without attributes.

## Messages

`message(name, args...)` records a non-fatal message and returns `null`.
`check(expr, fallback)` returns the fallback when a message was emitted.
`quiet(expr)` suppresses messages during evaluation. `off(name)` and `on(name)`
disable and enable a named message.

Messages carry the stable code `TGN-MSG-0001` and are exposed separately from
diagnostics. They do not replace evaluation diagnostics.

## Packages and up values

`import` loads a bundled module or a package file. A package file resolves
relative to the importing source and then through `$MithqalPath`. Import cycles
and missing packages are diagnostics.

An up value is a rule associated with a tag symbol. For a call, up values of the
head symbol and of symbols in the arguments are tried before down values and
built in implementations. `tagSet` installs an explicit tag and `upSet` uses the
head of the left hand side.

## Attributes and introspection

`setAttributes` and `clearAttributes` change symbol attributes. `protect` and
`unprotect` manage the protected flag. Built in symbols cannot be unprotected
or given new attributes. Attribute changes invalidate cached results.

`attributes(symbol)` returns the attribute names. `ownValues`, `downValues`, and
`upValues` return rules for own, down, and up values. `subValues` is empty in
this version. `definition` returns the attribute list, own values, and down
values. `fullDefinition` is an alias for the current definition view.

## Assumptions

`assume(fact,expr)` and `assuming(fact,expr)` evaluate `expr` under local facts.
Facts are scoped to `expr` and nested assumptions combine. Facts are keyed by
interned symbol identity, so a qualified name and a short name that resolve to
the same symbol share facts.

The fact language supports sign facts, nonzero facts, exact comparisons,
equality and disequality, integer, rational, real, and complex domains, parity,
conjunction, disjunction, and negation. A contradiction is a diagnostic. An
unsupported fact leaves the `assume` call unevaluated.

`assumptions()` returns the active source facts. `assumptions(symbol)` returns
the active facts that mention that symbol.

## Scoping and hygiene

All scoping is lexical.

A `let` binding is visible in the remainder of its block. An anonymous function
captures the lexical environment by value. A local `def` is visible in its block,
is mutually recursive with the other definitions of that scope, and does not
escape the block.

Rule bodies are instantiated with capture-avoiding substitution. The binding
forms are `function`, `closure`, `let`, `rule`, `ruleDelayed`, `condition`,
`pattern`, and `localDefs`. When an inserted value has a free variable that an
inner binder would capture, the binder is renamed. When an inner binding has the
same name as a rule variable, the inner binding shadows the rule variable.

Fresh names are deterministic and are valid source identifiers.


## Binders

`forall(variable, domain, body)`, `exists(variable, domain, body)`, and `setBuilder(variable, domain, body)` bind `variable` in `body` and evaluate `domain`.

When `domain` evaluates to a finite list, the body is evaluated for each item with the variable bound. `forall` returns `true` when every result is `true` and `false` when one result is `false`. `exists` returns `true` when one result is `true` and `false` when every result is `false`. `setBuilder` returns the canonical set of items whose result is `true`.

An empty domain gives `true` for `forall`, `false` for `exists`, and the empty set for `setBuilder`. A non-list domain, or a body that does not decide to `true` or `false`, leaves the binder symbolic.

Bound variables are renamed to a canonical name when the binder is evaluated. Canonical names are `v0`, `v1`, and so on, chosen so that they do not occur in the body. Alpha-equivalent binders are structurally equal.


## Structures

A structure declaration introduces a constructor, one accessor per field, and a law checker. The constructor returns a dictionary whose first item is the structure name and whose remaining items are the field values in declaration order. An accessor projects a field by position. The law checker binds the fields and evaluates the conjunction of the laws.

An instance evaluates each field value, builds the dictionary in declaration order, and checks the law conjunction. A false result produces a definition diagnostic. A result that is not `true` or `false` also produces a definition diagnostic unless the declaration is marked `assumed`.

Structure and instance declarations are module level statements. Generated names participate in exports and privacy like other definitions.

## Modules

A source is one module. A module with at least one `export` statement is
strict. In a strict module, exported definitions keep their source names, are
installed globally, and are protected. Every other definition is private: it is
installed in a module context and is not callable by its short source name.

A module without an `export` statement keeps every definition visible. This is
the compatibility behavior for small snippets.

An export list MUST name definitions of the same module. Two modules MUST NOT
export the same name.

An `import` declares a dependency on a bundled module or a package file. A
package file resolves relative to the importing source and then through
`$MithqalPath`. Modules load in dependency order. An unknown module or an import
cycle is rejected. Exported names from every loaded module are visible globally
in this version. Qualified access is available through context-qualified names.

`export` and `import` are module level statements. They MUST NOT appear inside a
block.

## Values and diagnostics

Evaluation produces a term. The error values `undefined`, `indeterminate`, and
`failed` are terms and are valid results. A diagnostic reports a condition that
prevents evaluation.

Diagnostic kinds are parse, eval, numeric, and internal. Every diagnostic has a
stable code. Message text is informative and is not normative. A diagnostic MAY
carry a source name, a source span, and a trace of frames. A trace frame carries
a head when known, an expression, and a source location when known.

The error values propagate through arithmetic and comparisons unless a rule
specifies another result.

## Parts and assignment

`part(expr,index,...)` indexes lists, calls, strings, byte arrays, and
associations. List and call indices are one based. Index zero selects the call
head. Negative indices count from the end. String and byte array indices are one
based and negative. A missing association key returns `undefined`. A missing
list or call index leaves the `part` call unevaluated.

`set(part(...),value)` updates an existing value. A list or call index MUST be in
range. An association assignment inserts a missing key when it is the final
index. A string part requires a one character string. A byte part requires an
integer from 0 through 255. `setDelayed` MUST reject a part target.

`unset` removes an own value or the rules of a function head. `clear`,
`clearAll`, and `remove` remove definitions. Protected symbols reject all three.

## Evaluation control and control flow

`hold` returns its arguments unevaluated. A direct `evaluate(arg)` inside
`hold` is evaluated. `holdForm` and `holdPattern` remain unevaluated.
`unevaluated` suppresses one evaluation of a direct argument. `sequence`
splices its elements into the enclosing argument list.

`compound` evaluates its arguments in order and returns the last value. `which`
evaluates condition and value pairs in order and returns the first selected
value. `switch` compares its evaluated scrutinee with held patterns and returns
the first matching value or the optional default.

`do`, `while`, and `for` iterate. `return` exits the enclosing function.
`break` and `continue` control loops. `throw(value[,tag])` transfers control to
the nearest `catch(body[,tag])` whose tag matches. An uncaught control signal
produces a diagnostic.

## Determinism and limits

For a fixed language version, input, configuration, and budget, evaluation is
deterministic. Rule order is fixed. Canonical order depends on content.
Garbage collection MUST NOT change a result. Wall clock time, randomness, and
process state MUST NOT affect a result.

The configuration carries a maximum step count and a maximum depth. Exhausting
either produces `EvalBudget`. The default step count is 1000000 and the default
depth is 2048. Thresholds are implementation-defined.

## Structural primitives

The following heads are normative. A call remains unevaluated when its domain
condition fails.

| Head | Semantics |
|---|---|
| `length(list)` | returns the number of items |
| `sort(list)` | orders items by numeric value, with nonnumeric items after numbers in canonical order |
| `first(list)` | returns the first item of a nonempty list |
| `rest(list)` | returns the list without its first item; `rest([])` is `[]` |
| `cons(item, list)` | returns a list with the item prepended |
| `nth(list, index)` | returns the item at a zero based index in range |
| `reverse(list)` | returns the list in reverse order |
| `isNumber(value)` | returns `true` for integers, rationals, and approximate reals |
| `map(list, function)` | applies the function to every item |
| `apply(function, list)` | applies the function to the items as arguments |
| `fold(function, initial, list)` | folds left over the list |
| `nest(function, value, count)` | applies the function `count` times; a count above 10000 remains unevaluated |
| `if(condition, then, else)` | evaluates one branch when the condition is `true` or `false` |
| `and(a, ...)` | short circuits on `false` |
| `or(a, ...)` | short circuits on `true` |
| `not(a)` | boolean negation |
| `plus(a, ...)` | flattens nested `plus`, combines exact numeric terms, combines like terms, and sorts |
| `times(a, ...)` | flattens nested `times`, combines exact numeric factors, combines equal bases, and sorts |
| `power(base, exponent)` | exact for exact bases and integer exponents; symbolic otherwise |
| `negate(a)` | exact negation, double negation elimination, and distribution over `plus` |
| `list(a, ...)` | evaluates items and does not flatten nested lists |
| `part(expr, index, ...)` | one based part access; missing list or call index remains unevaluated |
| `same(a, b)` | structural identity |
| `equal(a, b)` | mathematical equality where decidable |
| `less`, `lessEqual`, `greater`, `greaterEqual` | exact numeric comparison where decidable |

The control forms `let`, `localDefs`, `function`, and the pattern heads are
language forms. `localDefs` is internal to compiled body blocks and is not part
of the surface language.

All other heads are library definitions or native compute kernels. Their
contract is recorded by the reference documentation and by certificates where
available. The bundled library provides finite sets, relations, structure
dictionaries, finite law checks, combinatorics, finite probability,
information theory, decision theory, finite games, finite graphs, and a
calculus rule layer as ordinary Mithqal definitions.
