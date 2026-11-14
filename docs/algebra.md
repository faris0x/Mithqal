# Algebra

The algebra layer works with exact polynomials over the rationals. Polynomials are sparse maps from canonical monomials to rational coefficients.

## Functions

| Function | Meaning |
|---|---|
| `expand(expr)` | Expand products and nonnegative integer powers. |
| `factor(expr)` | Extract content and factor supported forms. |
| `together(expr)` | Combine a rational expression. |
| `cancel(expr)` | Cancel common polynomial factors. |
| `polynomialGcd(p,q,var)` | Monic univariate gcd. |
| `resultant(p,q,var)` | Sylvester resultant. |
| `solve(equation,var)` | Exact linear and quadratic solving. |
| `degree(p,var)` | Polynomial degree. |
| `coefficient(p,var,n)` | Coefficient of a power. |
| `polynomialDivide(p,q,var)` | Quotient and remainder. |
| `discriminant(p,var)` | Discriminant up to cubic degree. |

## Examples

```text
expand((x+y)^3)                       -> x^3+y^3+3*x*y^2+3*y*x^2
factor(x^4-1)                         -> (-1+x)*(1+x)*(1+x^2)
factor(x^3+y^3)                       -> (x+y)*(-x*y+x^2+y^2)
cancel((x^2-1)/(x+1))                 -> -1+x
polynomialGcd(x^2-1,x^2+2*x+1,x)      -> 1+x
resultant(x^2-1,x-2,x)                -> 3
solve(x^2-4,x)                        -> [-2,2]
degree(x^3+2*x+1,x)                   -> 3
coefficient(x^3+2*x+1,x,1)            -> 2
polynomialDivide(x^3+1,x-1,x)         -> [1+x+x^2,2]
discriminant(x^2-4,x)                 -> 16
```

## Factoring coverage

`factor` handles numeric content, common monomial content, univariate rational roots, differences of squares, perfect square trinomials, sums of cubes, differences of cubes, and grouping for four to eight even-term multivariate polynomials.

Unsupported factorizations stay unchanged. General multivariate factorization remains future work.

## Limits

- Exact rational coefficients only.
- `degree`, `coefficient`, `polynomialDivide`, and `discriminant` require univariate input in the chosen variable.
- `discriminant` supports degrees up to three.
- Solving returns exact solutions through degree two.
