# Assumptions

> Status: Partial. See [Maturity](maturity.md).

`assume` and its alias `assuming` apply local facts while evaluating an expression.

```text
assume(x>0,abs(x))          -> x
assume(x<0,abs(x))          -> -x
assume(x>=0,sqrt(x^2))      -> x
assume(x<=0,sqrt(x^2))      -> -x
```

Facts are scoped to the second argument. Nested assumptions are allowed.

```text
assume(x>0,assuming(y<0,abs(x)+abs(y))) -> x-y
```

Qualified names address a specific symbol identity.

```text
assume(foo;x>0,abs(foo;x)) -> foo;x
assume(bar;x<0,abs(bar;x)) -> -bar;x
```

`foo;x` and `bar;x` are different symbols, so their facts do not interfere.

## Supported facts

Sign facts:

- `x>0` positive
- `x>=0` nonnegative
- `x<0` negative
- `x<=0` nonpositive
- `x==0` zero
- `x=/=0` nonzero

Named sign predicates:

- `positive(x)`
- `nonnegative(x)`
- `negative(x)`
- `nonpositive(x)`
- `nonzero(x)`

Relations between symbols or exact constants:

- `x>y`, `x>=y`, `x<y`, `x<=y`
- `x==y`, `x=/=y`
- `x>1`, `x<-1`

Relations propagate transitively. For example, `x>y` and `y>0` make `x`
positive.

Domain facts:

- `integer(x)` or `element(x,integers)`
- `rational(x)` or `element(x,rationals)`
- `real(x)` or `element(x,reals)`
- `complex(x)` or `element(x,complexes)`

Parity facts:

- `even(x)`
- `odd(x)`

Parity facts also drive `isEven` and `isOdd` when the argument is symbolic.

Conjunctions use `and` or `&&`. Disjunctions use `or` or `||`. Negation uses
`not`. Negated comparisons are normalized, so `not(x>0)` is treated as
`x<=0`, and `not(x==0)` is treated as `x=/=0`.

```text
assume(and(x>0,y>0),abs(x*y))          -> x*y
assume(or(x>0,x>=0),abs(x))            -> x
assume(not(or(x>0,x<0)),abs(x))        -> 0
```

## Contradictions

Contradictory assumptions produce the diagnostic `TGN-EVAL-0021`.

```text
assume(and(x>0,x<0),abs(x))
assume(and(even(x),odd(x)),abs(x))
```

Unsupported fact forms leave the `assume` call unevaluated.

## Introspection

`assumptions()` returns the active source facts. `assumptions(symbol)` returns
the active facts that mention that symbol.

```text
assume(x>0,assumptions(x)) -> [x>0]
assumptions(x)             -> []
```

## Simplifications

Assumptions affect:

- `abs` of symbols, products, powers, negations, and sums with known sign
- `sqrt(x^2)` to `x`, `-x`, `0`, or `abs(x)` depending on the known sign
- constant simplifications for `exp(0)`, `ln(1)`, `sin(0)`, `cos(0)`, `tan(0)`, and `atan(0)`

Scope ends when the assumption form returns.
