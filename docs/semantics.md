# Semantics

> Status: Stable. See [Maturity](maturity.md).

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
- There is no approximate literal. Approximate values are produced through the explicit `approx` evaluation boundary.
- Scientific notation is written with `10^`. The lexer has no exponent literal syntax.
- Exact integer functions include `gcd`, `lcm`, `factorial`, `binomial`, `floor`, `ceil`, and `round`. The rounding functions also accept approximate reals.

## Approximate reals

Approximate values are `Real` atoms backed by MPFR. Each value carries an absolute error estimate in addition to its binary precision.

`gamma` and `lgamma` evaluate exactly for positive integers and half-integers. `gamma(5)` is `24`, `gamma(1/2)` is `sqrt(pi)`, and `lgamma(5)` is `ln(24)`. Other numeric arguments evaluate at the current precision, including complex values and the principal log gamma branch. Non-positive integers are poles and return `undefined`. General symbolic arguments remain symbolic.

`approx(expression,digits)` is the numeric boundary. It evaluates its argument exactly first, then converts numeric subexpressions to `Real` values. The digit count defaults to 16 when omitted.

The numeric boundary handles exact integers and rationals, `pi`, `e`, `plus`, `times`, `negate`, integer and real `power`, `sqrt`, `exp`, `ln`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `gamma`, `lgamma`, `erf`, `erfc`, and `abs`. Negative real square roots and logarithms produce complex values. A symbolic argument that cannot be converted leaves the `approx` call unevaluated.

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

Interval endpoints are converted to `Real` with a default precision for the operation. Endpoint arithmetic uses MPFR directed rounding: lower bounds round down, upper bounds round up, and products evaluate all four endpoint combinations with directed rounding before selecting the extrema. Interval `sqrt`, `exp`, `ln`, and `abs` use monotonic endpoint rules. Interval `sin` and `cos` evaluate both endpoints with directed rounding and include the relevant extrema when the interval crosses a critical point. Intervals wider than one period, and intervals with very large magnitudes, conservatively return `[-1,1]`. Endpoints also widen by their stored significance error. The result is a conservative enclosure for the modeled error estimates. Fully rigorous enclosures from arbitrary real inputs remain a later refinement because significance estimates are themselves approximate.

## Approximate complex values

An approximate complex value is a call with the protected `complex(real,imaginary)` head. The `approx` boundary recognizes the symbol `i`, converts real and imaginary parts to `Real` values, and returns a `complex` expression when the imaginary part is nonzero. A zero imaginary part normalizes to the real value.

The numeric boundary supports complex addition, subtraction, multiplication, stable division, integer powers, real and complex powers, absolute value, exponential, logarithm, square root, sine, cosine, tangent, inverse sine, inverse cosine, inverse tangent, hyperbolic functions, and inverse hyperbolic functions. MPC supplies the complex elementary functions. Real arguments outside a real domain are promoted to complex values, including square roots and logarithms of negative numbers and inverse trigonometric or hyperbolic arguments outside their real ranges. The principal branch uses the principal logarithm with argument in `(-pi,pi]` and the principal square root with a cut on the negative real axis. Negative real logarithms return positive imaginary parts, and negative real square roots return positive imaginary parts. Promoted inverse trigonometric values follow these cuts. `atanh` is evaluated as `(ln(1+z)-ln(1-z))/2` with the principal logarithm, so real arguments above one return negative imaginary parts. Non finite complex results, such as the logarithm of zero or the inverse tangent of the imaginary unit, leave the `approx` call unevaluated because the stored significance model cannot represent a meaningful enclosure.

## Symbolic constants

The constants are `i`, `pi`, `e`, and `inf`.

- `i` has the built in meaning of the imaginary unit while no user definition shadows it.
- `pi` and `e` are symbolic constants with built in numeric evaluators.
- `ln(e)` simplifies to `1`.
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
- `constant`: marks built in constants such as `i`, `pi`, `e`, and `null`.

Protected symbols in the current version:

- All kernel heads: `plus`, `times`, `power`, `negate`, `list`, `set`, `setDelayed`, `if`, `and`, `or`, `not`, `map`, `apply`, `fold`, `nest`, `function`, `let`, `forall`, `exists`, `setBuilder`, `pattern`, `blank`, `blankSequence`, `blankNullSequence`, `condition`, `rule`, `ruleDelayed`.
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

The cancellation in the previous example assumes generic symbolic values. Users can obtain conditional simplification through `assume` and `assuming`.

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

## Polynomial algebra

Exact polynomial operations live in a separate algebra layer. Polynomials are sparse maps from canonical monomials to rational coefficients. This representation is independent of the expression arena and is converted back to canonical `plus`, `times`, and `power` forms at the boundary.

The algebra boundary provides these heads:

- `expand` distributes products over sums and expands nonnegative integer powers.
- `factor` extracts numeric content, common monomial content, univariate rational roots, differences of squares, and perfect square trinomials.
- `together` combines a rational expression over a common denominator.
- `cancel` removes common factors from a rational expression.
- `polynomialGcd` returns the monic greatest common divisor of two univariate polynomials.
- `resultant` computes the Sylvester resultant of two univariate polynomials.
- `solve` returns a list of exact solutions for linear and quadratic equations.
- `degree`, `coefficient`, `polynomialDivide`, and `discriminant` provide univariate polynomial inspection and division.

`factor` and `cancel` cover common cases rather than the general multivariate factorization problem. A polynomial that cannot be factored by the supported rules is returned unchanged. A univariate factor that has no rational roots is retained as one factor, so `factor(x^4+1)` returns the original polynomial.

`solve` accepts an equation or a polynomial. A bare polynomial is solved as an equation with zero. Rational roots are extracted first. A remaining quadratic is solved with the quadratic formula, including exact irrational and complex radicals. Higher degree equations are left unevaluated when the remaining factor has degree above two.

The algebra layer rejects exponents above 4096, expanded forms above 20000 terms, and resultant inputs with a combined degree above 64. Products with more than two million coefficient pairs are rejected before multiplication starts. These limits keep a bad input from exhausting memory in a single evaluation. Rejected inputs stay in their original unevaluated form.

## Symbolic calculus

The bundled standard library also provides `derive` and `antiderivative`, which express the elementary rules in Mithqal and fall back to `differentiate` and `integrate` for forms they do not cover. See [Library modules](modules.md).

The calculus layer provides `differentiate`, `integrate`, `limit`, `series`, and `converges`. Differentiation, integration, and limits take an expression and a variable. `integrate` accepts either a variable plus two bounds or two bounds with an inferred variable. `limit`, `series`, and `converges` also infer a variable when the variable argument is omitted, and `limit` accepts an optional side argument.

`differentiate` applies exact differentiation rules for sums, products, powers, the elementary functions, and the inverse trigonometric and hyperbolic functions. Unknown function calls remain unevaluated. A symbol other than the differentiation variable is treated as a constant. `differentiate`, `integrate`, `limit`, `series`, `converges`, and `solve` accept an omitted variable when the expression has exactly one free symbol. An ambiguous expression produces an arity diagnostic.

`integrate` covers polynomials, powers with constant exponents, linear substitutions, `exp`, `sin`, `cos`, `tan`, `sinh`, `cosh`, `tanh`, `ln`, reciprocal linear terms, `1/(1+x^2)`, `1/sqrt(1-x^2)`, and the inverse trigonometric and hyperbolic functions `asin`, `acos`, `atan`, `asinh`, `acosh`, and `atanh`. It also performs integration by parts for polynomial and elementary factor products, and partial fractions for reciprocal polynomials with distinct rational roots. Products of dependent factors are expanded when the algebra layer can expand them. An antiderivative constant is omitted. Unsupported integrals remain unevaluated.

`integrate` also accepts lower and upper bounds as extra arguments. The definite form evaluates the antiderivative at the endpoints through one-sided limits and forms the difference. Symbolic bounds are allowed. Finite singular endpoints are handled when the antiderivative has a one-sided limit. Improper integrals with `inf` or `-inf` endpoints use the same limit engine, including rational, exponential, logarithmic, and arctangent growth rules. Rational denominators with roots strictly inside the interval leave the integral unevaluated. Logarithmic antiderivative arguments are checked for rational interior zeros and for sign changes across sampled interior points, which catches cases such as `tan(x)` across a pole. Divergent improper integrals remain unevaluated.

Differentiation results are passed through rational cancellation so quotient rule results collapse to a single fraction. Integration does not attach branch conditions or assumptions.

`limit` evaluates two sided and one sided limits at finite points and at `inf` or `-inf`. The fourth argument selects `left` or `right`. Finite points use direct substitution, rational cancellation, continuous composition, and local Taylor comparison. Infinite points use rational degree comparison plus exponential and logarithmic growth rules. Expressions with unknown heads remain unevaluated.

`series` returns a truncated Taylor polynomial through a requested degree. Coefficients come from repeated exact differentiation evaluated at the point. The order must be a nonnegative integer no greater than 20.

`converges` reports whether a finite limit exists. It returns `true` for a finite limit, `false` for divergence or an infinite limit, and remains unevaluated when the limit cannot be decided.

## Linear algebra

Matrices are lists of rows and vectors are plain lists. The linear algebra layer accepts exact rational entries only.

- `det` computes a determinant through exact elimination.
- `rank` and `rref` use exact Gauss-Jordan elimination.
- `inverse` augments with the identity and reduces.
- `matmul`, `trace`, `transpose`, `identity`, `dot`, and `cross` provide the basic matrix and vector operations.
- `solveLinear` returns the unique exact solution of a consistent system.
- `plu` returns permutation, lower, and upper factors for a full-rank square exact matrix.
- `charpoly` returns characteristic polynomial coefficients in ascending order.
- `eigenvalues` handles exact and symbolic two by two matrices, plus diagonal and triangular matrices.
- `eigenvectors` handles exact two by two matrices with rational eigenvalues, plus diagonal and triangular matrices.
- `nullspace` and its `kernel` alias return an exact basis for the null space.
- `qr` returns exact thin Q and R factors for full column rank matrices.

Symbolic entries are supported by determinants, transposes, traces, products, and inverses up to three by three. Symbolic determinant expansion is available up to five by five.

A singular matrix, an inconsistent system, an underdetermined system, a dimension mismatch, or a symbolic entry leaves the call unevaluated. Full parametrized solution sets and decompositions are later work.

## Number theory and logic

Number theory functions accept exact integers. Primality, next prime, factorization, divisors, totient, and sigma accept signed 64 bit inputs. Modular remainder and modular exponentiation accept arbitrary precision integers.

`isPrime` uses deterministic Miller-Rabin bases for machine sized values. `factorInteger` uses Pollard rho and returns ordered prime and exponent pairs. `divisors` returns positive divisors of the absolute value. `mod` normalizes into the nonnegative range of the modulus.

Broader number theory adds previous primes, prime counting through a sieve, divisor counts, Moebius values, radicals, square-free tests, Fibonacci numbers, perfect square tests, modular inverses, and Jacobi symbols.

Descriptive statistics operate on nonempty numeric lists. `mean`, `median`, `minimum`, `maximum`, `range`, `total`, `count`, `variance`, `sampleVariance`, `standardDeviation`, `sampleStandardDeviation`, `frequency`, and `mode` keep exact rational results. Standard deviations simplify exact rational squares and otherwise remain exact square root expressions. These functions are defined in the Mithqal standard library rather than in the kernel.

## List primitives

The kernel provides generic list primitives that library code is built from:

- `length(list)` returns the item count.
- `sort(list)` orders items by numeric value, with nonnumeric items after numbers in canonical order.
- `first(list)` returns the first item of a nonempty list and otherwise remains unevaluated.
- `rest(list)` returns the list without its first item; `rest([])` is `[]`.
- `cons(item,list)` prepends an item.
- `nth(list,index)` returns a zero based item and remains unevaluated out of range.
- `reverse(list)` reverses a list.
- `isNumber(value)` returns `true` for integers, rationals, and approximate reals.

These primitives leave a call unevaluated when its list argument is not a list. `map`, `apply`, `fold`, and `nest` evaluate their list and count arguments before use, so a computed list such as `map(...)` can be passed directly.

Logic provides `xor`, `implies`, `all`, and `any` alongside the existing `and`, `or`, and `not`. Predicates compose directly with these forms. Symbolic operands leave a connective unevaluated when the result is undetermined.

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

A protected symbol cannot be redefined. A definition whose head is a kernel head is rejected even when the symbol was interned without attributes.

Example:

```text
f(x_):=x^2
```

defines a down value for `f`. Calling `f(3)` returns `9`.

Down values are tried in definition order. A more general rule defined first shadows a later literal rule; write specific cases first.

A condition is checked after structural matching. When the condition does not evaluate to `true`, the next definition is tried.

Rule bodies are instantiated with capture-avoiding substitution. Closure parameters, `let` bindings, and pattern variables of rules constructed by the body are renamed when an inserted value would otherwise be captured. An inner binding shadows a rule variable of the same name. Renaming uses fresh deterministic names derived from the bound name.

A function with a fixed arity reports `EvalArity` when called with a different number of arguments. A definition with a sequence pattern is variadic and is not arity checked. Kernel heads always check their own arity.

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


## Binders

`forall(variable, domain, body)`, `exists(variable, domain, body)`, and `setBuilder(variable, domain, body)` bind a variable in the body. The domain is evaluated. The body is evaluated once for each item of a finite domain with the variable bound.

`forall` returns `true` when every item satisfies the body and `false` when one item does not. `exists` returns `true` when one item satisfies the body and `false` when none does. `setBuilder` returns the canonical set of satisfying items.

An empty domain gives `true` for `forall`, `false` for `exists`, and the empty set for `setBuilder`. A non-list domain or a body that does not decide to `true` or `false` leaves the binder symbolic.

Bound variables are renamed to a canonical name when the binder is evaluated, so alpha-equivalent binders are structurally equal. The canonical names are `v0`, `v1`, and so on, chosen so that they do not occur in the body.


## Structures

A structure declaration introduces a constructor, one accessor per field, and a law checker. The constructor returns a dictionary whose first item is the structure name and whose remaining items are the field values in declaration order. An accessor projects a field by position. The law checker binds the fields from the dictionary and evaluates the conjunction of the declared laws.

An instance evaluates each field value, builds the dictionary in declaration order, and checks the law conjunction. A false result rejects the instance with a definition diagnostic. A result that is not `true` or `false` also rejects the instance unless the declaration is marked `assumed`.

Structure and instance declarations are module level statements. Generated names are ordinary definitions for export and privacy purposes.

## Scoping

All scoping is lexical.

- `let` creates lexical bindings.
- A local `def` inside a block creates a lexical function scope. The innermost definition scope shadows outer scopes and global definitions entirely.
- Mutual recursion is available among definitions in the same local scope.
- Local definitions cannot shadow protected symbols.
- A strict module's private definitions are installed under deterministic internal names and are not callable by their source name. Exported names are protected.
- `fn` creates a closure over the current lexical environment.
- Pattern variables are lexical within the rule body.
- Rule bodies are substituted with capture avoidance: a closure parameter or `let` binding is renamed when an inserted value would otherwise be captured.
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

A diagnostic carries a stable code, an optional source name, an optional span, and a trace. Trace frames point at the statement that failed and at the enclosing statements. The normative diagnostic contract is in `spec/semantics.md`.

## Determinism

The engine guarantees deterministic evaluation for a fixed version, input, configuration, and budget.

- Rule order is fixed.
- Canonical order depends on content.
- Hash tables do not influence visible rule order.
- Symbol identifiers are assigned in creation order within a session.
- Garbage collection cannot change evaluation results.
- Wall clock time, randomness, and process state do not affect symbolic evaluation.
