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
factor(2*x^2+3*x*y+y^2)               -> (x+y)*(y+2*x)
factor(4*x^2+4*x*y+y^2)               -> (y+2*x)^2
factor(x^3+3*x^2*y+3*x*y^2+y^3)       -> (x+y)^3
factor((x^2+1)^3)                     -> (1+x^2)^3
factor((x+y^2)*(x+y^3))               -> (x+y^2)*(x+y^3)
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

`factor` handles numeric content, common monomial content, univariate rational roots, univariate square-free decomposition, bounded univariate Hensel lifting, differences of squares, perfect square trinomials, sums of cubes, differences of cubes, homogeneous bivariate forms with rational ratio roots, bivariate quadratics with recursive coefficient constants, generic interpolation for two through five variables, multivariate power extraction, bivariate coefficient content extraction, and grouping for four to eight even-term multivariate polynomials.

Univariate square-free decomposition uses polynomial gcds against the derivative. Repeated irreducible factors are separated and emitted with their multiplicities.

Bounded univariate integer polynomials also pass through modular factorization and Hensel lifting. A small prime supplies a coprime split, the split is lifted above the coefficient bound, and factors are accepted only after exact integer division. Small coefficients use a fast 128 bit path. Larger coefficients and moduli use GMP integers, so the range is not limited by machine word size. This finds repeated and irreducible polynomial factors that rational roots miss.

Homogeneous bivariate forms are factored by solving their ratio polynomial over the rationals. This covers degrees above two, including repeated linear factors.

Bivariate quadratics are evaluated as a polynomial in one variable with coefficients in the other. The homogeneous part is factored first, then the constant coefficients of the two linear factors are solved and checked. Square homogeneous parts use a scalar ratio equation, which also handles distinct linear constants.

The generic interpolation stage handles two through five variables. It factors polynomials that split into linear factors in one variable with polynomial coefficients in the remaining variables. Constant rational leading coefficients are scaled to monic form and restored on a factor. Higher dimensional cases first use sparse total-degree interpolation with a multi-right-hand-side rational solver, then fall back to dense tensor-product interpolation when the sparse system fails. Samples use the exact root-degree bound and the product is verified exactly. The stage retries variable orientations, sample bases, and root orderings.

The multivariate power stage extracts powers from two through twelve of bivariate polynomials by taking exact univariate roots of sampled slices, interpolating the root coefficients, and verifying the full power. Constant rational leading coefficients are scaled to monic form and restored as a scalar factor.

A polynomial-leading quadratic stage handles bivariate quadratics with a general polynomial leading coefficient in one variable. It factors the discriminant when it is a perfect square, builds the two linear factors, distributes factors of the leading polynomial by exact coefficient division, normalises the result, and verifies the product. The stage also retries with the two variable orientations.



A bivariate coefficient-content stage extracts a common polynomial factor across all coefficients of one variable. It computes the gcd of the coefficient polynomials and divides every coefficient by it.

A bivariate specialization root-lift stage samples univariate slices, interpolates a polynomial root branch, verifies that the root annihilates the polynomial, divides by the corresponding linear factor exactly, and recursively factors the quotient.

A bivariate Hensel stage lifts a coprime factor pair from a univariate specialization through coefficient orders in the second variable. It solves each correction with a Bezout identity, rebuilds the two bivariate factors, verifies the product, and recurses. This handles non-linear factors such as quadratic and cubic factors.

The multivariate layer is staged in `src/algebra/multivariate.rs`. The current stages cover binary quadratic forms, homogeneous bivariate forms, recursive bivariate quadratics, polynomial-leading quadratics, bivariate coefficient content, generic interpolation in two through five variables, power extraction through twelve, grouping, bivariate Hensel lifting, and specialization root lifting. Reserved stages cover interpolation over more than five variables and general multivariate factorisation beyond the staged bounds.

Unsupported factorizations stay unchanged. Full general multivariate factorization remains future work.

## Limits

- Exact rational coefficients only.
- `degree`, `coefficient`, `polynomialDivide`, and `discriminant` require univariate input in the chosen variable.
- `discriminant` supports degrees up to three.
- Solving returns exact solutions through degree two.
