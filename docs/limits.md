# Limits and series

> Status: Partial. See [Maturity](maturity.md).

The calculus layer provides `limit`, `series`, and `converges`. All three work on exact expressions.

## Limits

`limit(expr,var,point)` evaluates a two sided limit. The point may be a finite expression, `inf`, or `-inf`.

```text
limit(x^2+1,x,2)                  -> 5
limit((x^2-1)/(x-1),x,1)          -> 2
limit((2*x^2+1)/(x^2+3),x,inf)    -> 2
limit(exp(-x),x,inf)              -> 0
limit(atan(x),x,-inf)             -> -((1/2)*pi)
```

The fourth argument selects a one sided limit. The sides are `left` and `right`.

```text
limit(1/x,x,0,right)              -> inf
limit(1/x,x,0,left)               -> -inf
limit(1/x^2,x,0)                  -> inf
limit(abs(x)/x,x,0,right)         -> 1
limit(abs(x)/x,x,0,left)          -> -1
limit(exp(1/x),x,0,right)         -> inf
limit(exp(1/x),x,0,left)          -> 0
```

A two sided limit exists only when both sides agree. A mismatch leaves the call unevaluated.

```text
limit(1/x,x,0)                    -> limit(x^(-1),x,0)
limit(abs(x)/x,x,0)               -> limit(abs(x)*x^(-1),x,0)
limit(ln(x),x,0)                  -> limit(ln(x),x,0)
```

## Evaluation rules

Finite points are evaluated through these steps in order:

1. Direct substitution when the substituted expression is finite and defined.
2. Rational cancellation to remove removable singularities.
3. Composition through continuous elementary functions.
4. Local Taylor comparison of numerator and denominator at the point.

The local comparison finds the first nonzero coefficient of each side. It resolves indeterminate forms such as `0/0` and selects the leading term ratio, zero, or an infinite sign. This covers common limits without symbolic L'Hopital bookkeeping.

```text
limit(sin(x)/x,x,0)               -> 1
limit((1-cos(x))/x^2,x,0)         -> 1/2
limit((exp(x)-1)/x,x,0)           -> 1
limit(ln(x)/(x-1),x,1)            -> 1
limit((x-sin(x))/x^3,x,0)         -> 1/6
```

Infinite points use rational degree comparison and growth rules for exponentials and logarithms.

```text
limit((x+1)/(x^2+1),x,inf)        -> 0
limit((x^3+1)/(x+1),x,inf)        -> inf
limit((x^2+1)/(3*x^2),x,-inf)     -> 1/3
limit(x/exp(x),x,inf)             -> 0
limit(ln(x)/x,x,inf)              -> 0
limit(2^x,x,inf)                  -> inf
limit(2^x,x,-inf)                 -> 0
```

Expressions with unknown heads remain unevaluated because continuity is unknown.

## Series

`series(expr,var,point,order)` returns the Taylor polynomial through the requested degree. Coefficients come from repeated exact differentiation evaluated at the point. The variable may be omitted when the expression has one free symbol.

```text
series(exp(x),x,0,3)              -> 1+x+x^2/2!+x^3/3!
series(sin(x),x,0,5)              -> x-x^3/3!+x^5/5!
series(cos(x),x,0,4)              -> 1-x^2/2!+x^4/4!
series(ln(1+x),x,0,3)             -> x-x^2/2!+2*x^3/3!
series(1/(1-x),x,0,3)             -> 1+x+x^2+x^3
```

Terms are returned in degree order. The optional Boolean argument controls whether factorials are evaluated. The default keeps factorials.

```text
series(exp(x),0,4)                -> 1+x+x^2/2!+x^3/3!+x^4/4!
series(exp(x),0,4,true)           -> 1+x+(1/2)*x^2+(1/6)*x^3+(1/24)*x^4
```

Series results work in further algebra. Arithmetic operands and `expand` convert the ordered display form into a normal sum.

```text
expand(series(exp(x),0,2))        -> 1+x+(1/2)*x^2
differentiate(series(exp(x),0,2),x) -> 1+x
```

The point may be nonzero. Terms then use powers of `x-a`.

```text
series(exp(x),x,1,1)              -> exp(1)+exp(1)*(-1+x)
series(x^2,x,1,2)                 -> 1+2*(-1+x)+(-1+x)^2
```

The order must be a nonnegative integer no greater than 20. Unsupported derivatives, singular points, negative or oversized orders, and invalid Boolean flags produce a diagnostic or leave the call unevaluated.

## Convergence

`converges(expr,var,point)` reports whether a finite limit exists.

```text
converges(sin(x)/x,x,0)           -> true
converges(1/x,x,0)                -> false
converges(abs(x)/x,x,0)           -> false
converges(exp(-x),x,inf)          -> true
```

The result is `true` for a finite limit and `false` for divergence or an infinite limit. When the limit cannot be decided, the call remains unevaluated.
