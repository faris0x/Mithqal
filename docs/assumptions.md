# Assumptions

`assume` and its alias `assuming` apply a local domain fact while evaluating an expression.

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

## Supported facts

- `x>0` positive.
- `x>=0` nonnegative.
- `x<0` negative.
- `x<=0` nonpositive.
- `x==0` zero.

The assumption parser accepts a symbol compared against zero. Unsupported assumption forms leave the `assume` call unevaluated.

## Simplifications

Assumptions affect:

- `abs` of symbols, products, powers, negations, and sums with known sign.
- `sqrt(x^2)` under a known sign for `x`.
- Constant simplifications for `exp(0)`, `ln(1)`, `sin(0)`, `cos(0)`, `tan(0)`, and `atan(0)`.

Scope ends when the assumption form returns.
