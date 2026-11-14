# Semantics

This document defines the expression model and the evaluation rules for the current version.

## Terms and values

An expression is a term. The kernel has two term forms.

- An atom is an exact integer, an exact rational, a symbol, or a string when strings are added.
- A compound term is a call with a head and a sequence of arguments. The head is itself an expression.

Every compound term is represented as `Call(head, args)`. Symbols such as `plus`, `times`, `list`, `rule`, and `blank` are ordinary expressions. Nothing about a term's surface spelling changes its internal representation.

Runtime values include symbolic terms and closures. A closure is represented as a protected `closure(params,body,environment)` call. The parameter list holds symbols, the body holds the expression to evaluate, and the environment holds a list of captured `set(name,value)` bindings. Closures are expressions and can be printed, stored, and hash consed like other calls.

## Exact numbers

The current version supports exact integers and exact rationals.

- Integers use GMP `mpz`.
- Rationals use GMP `mpq` and remain canonical. The denominator is positive. The greatest common divisor of the numerator and denominator is one. Zero has denominator one.
- A decimal literal is exact. `1.23` denotes `123/100`.
- There is no approximate literal. Approximate values arrive through an explicit evaluation boundary in a later version.
- Scientific notation is written with `10^`. The lexer has no exponent literal syntax.

## Approximate reals

Approximate values are `Real` atoms backed by MPFR. Each value carries an absolute error estimate in addition to its binary precision.

`approx(expression,digits)` is the numeric boundary. It evaluates its argument exactly first, then converts numeric subexpressions to `Real` values. The digit count defaults to 16 when omitted.

The numeric boundary handles exact integers and rationals, `pi`, `e`, `plus`, `times`, `negate`, integer and real `power`, `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, and `abs`. Negative real square roots and logarithms produce complex values. A symbolic argument that cannot be converted leaves the `approx` call unevaluated.

Significance propagation uses first order error estimates:

- Addition and subtraction combine absolute errors in quadrature.
- Multiplication and division combine relative errors in quadrature.
- Integer powers scale relative error by the exponent.
- Elementary functions use their local derivative to propagate error.
- Rounding error from the target precision is added at each operation.

`Real::precision_digits` reports relative decimal precision. `Real::accuracy_digits` reports absolute decimal accuracy. An exact zero has unbounded accuracy when its stored error is zero.

Approximate values are printed as decimal output. They are not accepted as source literals in the current version, and the printer does not provide a round trip for approximate values.

## Intervals

An interval is a call with the protected `interval(low,high)` head. Endpoints may be exact numbers or `Real` values. The constructor rejects a low endpoint greater than the high endpoint by returning `failed`.

Interval arithmetic is available for addition, multiplication, negation, and integer powers. Scalar values combine with intervals by treating the scalar as a zero width interval. Multiplication evaluates the four endpoint products and takes the minimum and maximum. Even powers that cross zero produce a lower bound of zero. A negative power requires an interval that does not contain zero.

Interval endpoints are converted to `Real` with a default precision for the operation. Endpoint arithmetic uses MPFR directed rounding: lower bounds round down, upper bounds round up, and products evaluate all four endpoint combinations with directed rounding before selecting the extrema. Endpoints also widen by their stored significance error. The result is a conservative enclosure for the modeled error estimates. Fully rigorous enclosures from arbitrary real inputs remain a later refinement because significance estimates are themselves approximate.

## Approximate complex values

An approximate complex value is a call with the protected `complex(real,imaginary)` head. The `approx` boundary recognizes the symbol `i`, converts real and imaginary parts to `Real` values, and returns a `complex` expression when the imaginary part is nonzero. A zero imaginary part normalizes to the real value.

The numeric boundary supports complex addition, subtraction, multiplication, stable division, integer powers, real and complex powers, absolute value, exponential, logarithm, square root, sine, cosine, tangent, inverse sine, inverse cosine, inverse tangent, hyperbolic functions, and inverse hyperbolic functions. MPC supplies the complex elementary functions. Real arguments outside a real domain are promoted to complex values, including square roots and logarithms of negative numbers and inverse trigonometric or hyperbolic arguments outside their real ranges. Promoted inverse trigonometric values use the principal branch with the same orientation as Mathematica for real arguments above one. Non finite complex results, such as the logarithm of zero or the inverse tangent of the imaginary unit, leave the `approx` call unevaluated because the stored significance model cannot represent a meaningful enclosure.

## Symbolic constants

The constants are `i`, `pi`, `e`, and `inf`.

- `i` has the built in meaning of the imaginary unit while no user definition shadows it.
- `pi` and `e` are symbolic constants with built in numeric evaluators in a later version.
- `inf` is a protected constant. It represents positive infinity when an infinite limit is expressed. It is not a number.
- `i`, `pi`, and `e` can be shadowed by user definitions.
- `inf` cannot be redefined.

A user definition of `i` changes the meaning of the symbol. With `i=3`, the expression `i^2` evaluates to `9`. The imaginary unit rules apply only while `i` has no own value and no local binding.

## Symbols and attributes

A symbol has a name, attributes, an optional own value, and a list of down values.

Attributes used in the current version:

- `flat`: nested calls with the same head are spliced into one call.
- `orderless`: arguments are sorted into canonical order.
- `holdAll`, `holdFirst`, and `holdRest`: argument evaluation is controlled by the head.
- `listable`: the head threads over arguments that are lists.
- `protected`: user definitions of the symbol are rejected.
- `constant`: reserved for later numeric work.

Protected symbols in the current version:

- All kernel heads: `plus`, `times`, `power`, `negate`, `list`, `set`, `setDelayed`, `if`, `and`, `or`, `not`, `map`, `apply`, `fold`, `nest`, `function`, `let`, `pattern`, `blank`, `blankSequence`, `blankNullSequence`, `condition`, `rule`, `ruleDelayed`.
- Truth values: `true`, `false`.
- Structural constants: `emptyset`, `inf`.
- Error values: `undefined`, `indeterminate`, `failed`.
- The no value result: `null`.

`i`, `pi`, `e`, and ordinary user symbols are not protected.

## Evaluation

Evaluation is a deterministic rewrite process. The evaluator repeatedly applies the rules below until the expression stops changing or the step budget is exhausted.

For an atom, the evaluator performs symbol resolution.

1. A local binding is returned directly.
2. If the symbol has an own value, that value is evaluated.
3. A built in constant default is used when no own value exists.
4. An ordinary symbol evaluates to itself.

For a call, the evaluator proceeds as follows.

1. Evaluate the head.
2. Evaluate each argument in order, unless an attribute holds it.
3. Apply listable threading when the head has the `listable` attribute and an argument is a list.
4. Apply the built in implementation for the head when one exists.
5. Try user down values for the head.
6. Canonicalize the result.
7. Evaluate the result again when it changed.

The head is evaluated before the arguments so that own values and aliases resolve correctly. Held arguments are evaluated by the control form that owns them.

A call whose head evaluates to a closure invokes the closure with the evaluated arguments. A call whose head evaluates to another value produces a structured diagnostic.

### Evaluation budget

Every evaluation carries a deterministic step budget. The budget counts rule applications and matcher steps. It does not depend on wall clock time. When the budget is exhausted, evaluation returns a diagnostic with the current expression attached. The same input with the same budget always produces the same result.

## Canonical order

Canonical order is a total order over expressions. It depends on content and never on memory addresses, insertion order, or hash table layout.

The order proceeds by expression kind, then by content:

- Exact numbers sort before symbols.
- Symbols sort by name, then by sort information when sorts exist.
- Calls sort by head, then by arguments from left to right.
- A shorter argument list sorts before a longer list with an equal prefix.

Arguments of an `orderless` head are sorted with this order before interning.

## Automatic simplification

Automatic simplification is limited to canonical normalization and exact arithmetic. It never distributes a product over a sum. It never factors a sum. It never applies a trigonometric identity.

### `plus`

- Splice nested `plus` calls.
- Remove exact zero terms.
- Combine exact rational terms.
- Combine like terms. Two terms are alike when their non numeric parts are the same canonical expression.
- Sort the remaining terms.

Examples:

```text
x+x
```

evaluates to `times(2,x)`.

```text
2*x+3*x
```

evaluates to `times(5,x)`.

### `times`

- Splice nested `times` calls.
- Remove exact one factors.
- Return zero when any factor is exact zero.
- Combine exact rational factors.
- Combine powers with the same base.
- Move a negative sign to an outer `negate` or to the rational coefficient.
- Sort the remaining factors.

Examples:

```text
x*x
```

evaluates to `power(x,2)`.

```text
x*x^-1
```

evaluates to `1`.

The cancellation in the previous example assumes generic symbolic values. Users can obtain conditional simplification through assumptions in a later version.

### `power`

- Exact rational bases with integer exponents evaluate exactly.
- `power(x,0)` evaluates to `1` when `x` is not zero.
- `power(x,1)` evaluates to `x`.
- `power(0,0)` evaluates to `indeterminate`.
- `power(0,n)` for a negative integer `n` evaluates to `undefined`.
- Rational exponents remain symbolic unless the base is `0`, `1`, or an exact perfect power.
- `i` powers reduce by the imaginary unit rules when `i` is unshadowed.

### `negate`

- Exact rational negation evaluates immediately.
- A double negation cancels.
- Negation distributes over `plus`.
- A negative result of `times` is represented with an outer `negate` or a negative coefficient.

### `list`

A list evaluates its elements. A list does not flatten nested lists. A list does not evaluate its own structure with arithmetic rules.

## Exact complex normalization

Expressions built from exact rationals and the unshadowed symbol `i` using `plus`, `times`, `negate`, integer `power`, and division are Gaussian rationals. They normalize automatically to the form `a+b*i`, where `a` and `b` are exact rationals.

Examples:

```text
(1+i)*(1-i)
```

evaluates to `2`.

```text
(2+3*i)/(1-i)
```

evaluates to `-1/2+5/2*i`.

```text
i^3
```

evaluates to `-i`.

The normalization includes rationalizing a denominator. A zero denominator produces `undefined` or `indeterminate` according to the usual zero rules.

This normalization applies only to closed Gaussian rational expressions. It does not expand expressions that contain `pi`, `e`, free symbols, or other numeric functions. For example, `(a+b*i)*(c+d*i)` remains a product.

## Listable heads

A head with the `listable` attribute threads over list arguments.

```text
plus([1,2],3)
```

evaluates to `[4,5]`.

```text
times([1,2],[3,4])
```

evaluates to `[3,8]`.

When several arguments are lists, their lengths must agree. A length mismatch produces a diagnostic. A non list argument is reused for each element. The `list` head is not listable. Comparison heads are not listable in the current version.

## Definitions

`set(lhs,rhs)` evaluates `rhs`, installs a definition, and returns the evaluated right hand side.

`setDelayed(lhs,rhs)` holds `rhs`, installs a delayed definition, and returns `null`.

A left hand side that is a symbol installs an own value. A left hand side that is a call installs a down value for its head. Arguments in the left hand side are patterns when they use pattern syntax. A plain symbol argument is a literal symbol.

Example:

```text
f(x_):=x^2
```

defines a down value for `f`. Calling `f(3)` returns `9`.

Down values are tried in this order:

1. Literal arguments.
2. Named patterns.
3. Anonymous blanks.
4. Sequence patterns.
5. Definition order within the same class.

A condition is checked after structural matching. When the condition does not evaluate to `true`, the next definition is tried.

## Patterns

Pattern expressions use these heads:

- `blank()` for `_`.
- `pattern(x,blank())` for `x_`.
- `pattern(x,blankSequence())` for `x__`.
- `pattern(x,blankNullSequence())` for `x___`.
- `condition(p,c)` for a condition on pattern `p`.

A named pattern binds the matched value. A repeated name requires equal bindings. A sequence pattern binds a sequence of values. `blankSequence` matches one or more arguments. `blankNullSequence` matches zero or more arguments.

Matching runs against evaluated and canonical arguments. For an `orderless` head, arguments are sorted before matching. Fixed arity patterns use deterministic backtracking over candidate assignments. Sequence patterns on `flat` and `orderless` heads use a contiguous segment in canonical order in the current version. Full multiset partitioning is deferred.

## Comparisons

`~` is structural equality. It always returns `true` or `false`.

- `1~1` returns `true`.
- `x~x` returns `true`.
- `x~y` returns `false`.
- `f(x)~f(x)` returns `true`.

`===` is mathematical equality for exact values. It returns `true` or `false` when the relation is decidable. A symbolic relation that cannot be decided remains as an `equal` call.

- `1===1` returns `true`.
- `1===2` returns `false`.
- `x===y` remains `equal(x,y)`.

`=/=` is the negation of `===`. It remains unevaluated when the corresponding `===` expression remains unevaluated.

The comparisons `<`, `<=`, `>`, and `>=` accept exact real numbers. They return an unevaluated comparison for symbolic arguments. They propagate `undefined`, `indeterminate`, and `failed`.

## Logic and control

`if(cond,then,else)` evaluates only the selected branch. A condition of `true` selects the first branch. A condition of `false` selects the second branch. A symbolic condition returns an unevaluated `if` call. The values `undefined`, `indeterminate`, and `failed` propagate.

`and` and `or` short circuit. They propagate the error values. `not` propagates the error values.

`true` and `false` are protected symbols.

## Scoping

All scoping is lexical.

- `let` creates lexical bindings.
- `fn` creates a closure over the current lexical environment.
- Pattern variables are lexical within the rule body.
- A global definition resolves at application time. A delayed right hand side sees the current global definition of a symbol that is not captured locally.
- A closure captures local bindings by value.
- There is no dynamic scoping form.

## Error values and diagnostics

The symbols `undefined`, `indeterminate`, and `failed` are expression values.

- Division by exact zero produces `undefined` in ordinary arithmetic.
- The expression `0/0` produces `indeterminate`.
- An operation that cannot produce a result can return `failed`.

A `Diagnostic` is a Rust error. It reports parse failures, protected symbol definitions, budget exhaustion, non callable heads, and internal numeric failures. Domain results such as `undefined` remain ordinary expression values.

Error values propagate through arithmetic and comparisons unless a rule specifies another result.

## Determinism

The engine guarantees deterministic evaluation for a fixed version, input, configuration, and budget.

- Rule order is fixed.
- Canonical order depends on content.
- Hash tables do not influence visible rule order.
- Symbol identifiers are assigned in creation order within a session.
- Garbage collection cannot change evaluation results.
- Wall clock time, randomness, and process state do not affect symbolic evaluation.
