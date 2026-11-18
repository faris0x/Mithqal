# Numbers

> Spec version: 0.0.1 (draft)

## Numeric tower

The numeric terms are:

- exact integers;
- exact rationals;
- approximate reals;
- intervals;
- complex values;
- symbolic numeric expressions.

The exact layer is always available. The approximate layer is produced only at
the explicit `approx` boundary. The interval layer is produced by the
`interval` constructor or by interval operations.

## Exact integers and rationals

Integers are arbitrary precision. Rationals are canonical: the denominator is
positive, the greatest common divisor of numerator and denominator is one, and
zero has denominator one.

A decimal literal is exact. `1.25` denotes `125/100`. There is no approximate
literal and no exponent literal.

Exact arithmetic is closed over addition, subtraction, multiplication, division
by a nonzero value, and integer powers. Division by exact zero produces
`undefined`. The expression `0/0` produces `indeterminate`.

## Approximate reals

`approx(expression, digits)` is the numeric boundary. It evaluates the
expression exactly first, then converts numeric subexpressions to approximate
reals at the requested decimal precision. The default precision is 16 digits.

An approximate real carries a binary precision and an absolute error estimate.
Significance propagation uses first order error estimates:

- addition and subtraction combine absolute errors in quadrature;
- multiplication and division combine relative errors in quadrature;
- an integer power scales relative error by the exponent;
- an elementary function uses its local derivative to propagate error;
- rounding error is added at each operation.

The reported precision is relative decimal precision. The reported accuracy is
absolute decimal accuracy. An exact zero has unbounded accuracy when its stored
error is zero.

The boundary supports exact integers and rationals, `pi`, `e`, arithmetic,
integer and real powers, roots, exponentials, logarithms, trigonometric
functions, inverse trigonometric functions, hyperbolic functions, inverse
hyperbolic functions, `gamma`, `lgamma`, `erf`, `erfc`, and `abs`. A symbolic
argument that cannot be converted leaves the `approx` call unevaluated.

Approximate values are not accepted as source literals in this version.

## Intervals

An interval is a call with the protected head `interval(low, high)`. Endpoints
are exact numbers or approximate reals. A constructor with a low endpoint
greater than its high endpoint produces `failed`.

Interval arithmetic is defined for addition, multiplication, negation, integer
powers, `sqrt`, `exp`, `ln`, `abs`, `sin`, and `cos`. Endpoint operations use
directed rounding: lower bounds round down and upper bounds round up. Products
evaluate all endpoint combinations and select the extrema. A negative power
requires an interval that does not contain zero.

Endpoints widen by their stored significance error. The result is a conservative
enclosure for the modeled error. A fully rigorous enclosure from arbitrary real
inputs is reserved for a later version.

## Complex values

An approximate complex value is a call with the protected head
`complex(real, imaginary)`. A zero imaginary part normalizes to the real value.

The symbol `i` denotes the imaginary unit while no definition or binding
shadows it. Exact expressions built from rationals and `i` with addition,
multiplication, negation, integer powers, and division normalize to the form
`a+b*i` with exact rational `a` and `b`.

The numeric boundary supports complex arithmetic, integer and real powers,
stable division, absolute value, exponential, logarithm, square root,
trigonometric functions, inverse trigonometric functions, hyperbolic functions,
and inverse hyperbolic functions. Principal branches use the principal
logarithm with argument in `(-pi, pi]` and the principal square root with a cut
on the negative real axis.

Non-finite results leave the `approx` call unevaluated.

## Comparison and equality

Exact numbers compare exactly. Approximate reals compare with their stored
significance. Intervals are not ordered as scalars. A comparison with a
symbolic operand remains unevaluated.

Structural identity is available through `same`. Mathematical equality is
available through `equal` where a decision procedure exists.

## Printing

Exact integers and rationals print in canonical rational form. Approximate
reals print as decimal text. Intervals print as `interval(low,high)`. Complex
values print as `complex(real,imaginary)` in canonical form and in the
conventional infix form for display.

Because approximate reals have no source literal, canonical round trips are
guaranteed only for terms that contain no approximate reals.

## Reserved

A unified ball arithmetic layer, a Rust-native arbitrary-precision backend, and
correctly rounded transcendental functions are reserved for later versions.
