# Function Capabilities

> Status: Stable. See [Maturity](maturity.md).

This table is the capability matrix for the advertised function surface. `list capabilities` in the REPL prints the same metadata.

Support values: `exact`, `symbolic`, `approx`, `language`. Display values describe the preferred result presentation.

| Function | Category | Support | Signature | Domain | Display |
| --- | --- | --- | --- | --- | --- |
| `gcd` | core | exact | `gcd(a,b,...)` | documented integer and rational inputs | infix |
| `lcm` | core | exact | `lcm(a,b,...)` | documented integer and rational inputs | infix |
| `factorial` | core | exact | `factorial(n)` | non-negative integers up to 10000 | infix |
| `binomial` | core | exact | `binomial(n,k)` | documented integer and rational inputs | infix |
| `floor` | core | exact | `floor(value)` | documented integer and rational inputs | infix |
| `ceil` | core | exact | `ceil(value)` | documented integer and rational inputs | infix |
| `round` | core | exact | `round(value)` | documented integer and rational inputs | infix |
| `expand` | algebra | symbolic | `expand(expr)` | supported expressions in the documented domain | infix |
| `factor` | algebra | symbolic | `factor(expr)` | supported expressions in the documented domain | infix |
| `together` | algebra | symbolic | `together(expr)` | supported expressions in the documented domain | infix |
| `cancel` | algebra | symbolic | `cancel(expr)` | supported expressions in the documented domain | infix |
| `polynomialGcd` | algebra | symbolic | `polynomialGcd(p,q,var)` | supported expressions in the documented domain | infix |
| `resultant` | algebra | symbolic | `resultant(p,q,var)` | supported expressions in the documented domain | infix |
| `solve` | algebra | symbolic | `solve(expr[,var])` | supported expressions in the documented domain | list |
| `degree` | algebra | symbolic | `degree(p,var)` | supported expressions in the documented domain | infix |
| `coefficient` | algebra | symbolic | `coefficient(p,var,n)` | supported expressions in the documented domain | infix |
| `polynomialDivide` | algebra | symbolic | `polynomialDivide(p,q,var)` | supported expressions in the documented domain | infix |
| `discriminant` | algebra | symbolic | `discriminant(p,var)` | supported expressions in the documented domain | infix |
| `differentiate` | calculus | symbolic | `differentiate(expr[,var])` | supported expressions in the documented domain | infix |
| `integrate` | calculus | symbolic | `integrate(expr[,var]) | integrate(expr,a,b) | integrate(expr,var,a,b)` | supported expressions in the documented domain | infix |
| `limit` | calculus | symbolic | `limit(expr[,var],point[,side])` | supported expressions in the documented domain | infix |
| `series` | calculus | symbolic | `series(expr[,var],point,order)` | analytic expressions with supported derivatives | ordered series |
| `converges` | calculus | symbolic | `converges(expr[,var],point)` | supported expressions in the documented domain | infix |
| `assume` | calculus | language | `assume(fact,expr)` | language forms | infix |
| `assuming` | calculus | language | `assuming(fact,expr)` | language forms | infix |
| `det` | linear algebra | exact | `det(matrix)` | exact rational and symbolic matrices | infix |
| `rank` | linear algebra | exact | `rank(matrix)` | exact rational and symbolic matrices | infix |
| `transpose` | linear algebra | exact | `transpose(matrix)` | exact rational and symbolic matrices | infix |
| `inverse` | linear algebra | exact | `inverse(matrix)` | exact rational and symbolic matrices | infix |
| `rref` | linear algebra | exact | `rref(matrix)` | exact rational and symbolic matrices | infix |
| `trace` | linear algebra | exact | `trace(matrix)` | exact rational and symbolic matrices | infix |
| `matmul` | linear algebra | exact | `matmul(a,b)` | exact rational and symbolic matrices | infix |
| `identity` | linear algebra | exact | `identity(n)` | exact rational and symbolic matrices | infix |
| `solveLinear` | linear algebra | exact | `solveLinear(a,b)` | exact rational and symbolic matrices | infix |
| `dot` | linear algebra | exact | `dot(a,b)` | exact rational and symbolic matrices | infix |
| `cross` | linear algebra | exact | `cross(a,b)` | exact rational and symbolic matrices | infix |
| `plu` | linear algebra | exact | `plu(matrix)` | exact rational and symbolic matrices | list |
| `charpoly` | linear algebra | exact | `charpoly(matrix)` | exact rational and symbolic matrices | list |
| `eigenvalues` | linear algebra | exact | `eigenvalues(matrix)` | exact rational and symbolic matrices | list |
| `eigenvectors` | linear algebra | exact | `eigenvectors(matrix)` | exact rational and symbolic matrices | list |
| `nullspace` | linear algebra | exact | `nullspace(matrix)` | exact rational and symbolic matrices | list |
| `kernel` | linear algebra | exact | `kernel(matrix)` | exact rational and symbolic matrices | list |
| `qr` | linear algebra | exact | `qr(matrix)` | exact rational and symbolic matrices | list |
| `isPrime` | number theory | exact | `isPrime(n)` | integers | boolean |
| `nextPrime` | number theory | exact | `nextPrime(n)` | integers | infix |
| `previousPrime` | number theory | exact | `previousPrime(n)` | integers | infix |
| `factorInteger` | number theory | exact | `factorInteger(n)` | nonzero integers | prime product |
| `divisors` | number theory | exact | `divisors(n)` | integers | list |
| `divisorCount` | number theory | exact | `divisorCount(n)` | integers | infix |
| `totient` | number theory | exact | `totient(n)` | integers | infix |
| `sigma` | number theory | exact | `sigma(n)` | integers | infix |
| `isEven` | number theory | exact | `isEven(n)` | integers | boolean |
| `isOdd` | number theory | exact | `isOdd(n)` | integers | boolean |
| `divides` | number theory | exact | `divides(a,b)` | integers | boolean |
| `mod` | number theory | exact | `mod(a,m)` | integers | infix |
| `powerMod` | number theory | exact | `powerMod(a,e,m)` | integers | infix |
| `primeCount` | number theory | exact | `primeCount(n)` | integers | infix |
| `fibonacci` | number theory | exact | `fibonacci(n)` | integers | infix |
| `isPerfectSquare` | number theory | exact | `isPerfectSquare(n)` | integers | boolean |
| `modInverse` | number theory | exact | `modInverse(a,m)` | integers | infix |
| `jacobi` | number theory | exact | `jacobi(a,n)` | integers | infix |
| `mean` | statistics | exact | `mean(list)` | numeric lists | infix |
| `median` | statistics | exact | `median(list)` | numeric lists | infix |
| `minimum` | statistics | exact | `minimum(list)` | numeric lists | infix |
| `maximum` | statistics | exact | `maximum(list)` | numeric lists | infix |
| `range` | statistics | exact | `range(list)` | numeric lists | infix |
| `total` | statistics | exact | `total(list)` | numeric lists | infix |
| `count` | statistics | exact | `count(list)` | numeric lists | infix |
| `variance` | statistics | exact | `variance(list)` | numeric lists | infix |
| `sampleVariance` | statistics | exact | `sampleVariance(list)` | numeric lists | infix |
| `standardDeviation` | statistics | exact | `standardDeviation(list)` | numeric lists | infix |
| `sampleStandardDeviation` | statistics | exact | `sampleStandardDeviation(list)` | numeric lists | infix |
| `frequency` | statistics | exact | `frequency(list)` | numeric lists | list |
| `mode` | statistics | exact | `mode(list)` | numeric lists | list |
| `and` | logic | exact | `and(a,b,...)` | boolean expressions | boolean |
| `or` | logic | exact | `or(a,b,...)` | boolean expressions | boolean |
| `not` | logic | exact | `not(a)` | boolean expressions | boolean |
| `xor` | logic | exact | `xor(a,b)` | boolean expressions | boolean |
| `implies` | logic | exact | `implies(a,b)` | boolean expressions | boolean |
| `all` | logic | exact | `all(list,predicate)` | boolean expressions | boolean |
| `any` | logic | exact | `any(list,predicate)` | boolean expressions | boolean |
| `approx` | numeric | approx | `approx(expr[,digits])` | supported numeric values under approx | infix |
| `interval` | numeric | exact | `interval(low,high)` | supported exact numeric inputs | infix |
| `complex` | numeric | exact | `complex(real,imaginary)` | supported exact numeric inputs | infix |
| `abs` | numeric | exact | `abs(expr)` | supported exact numeric inputs | infix |
| `sqrt` | numeric | exact | `sqrt(expr)` | supported exact numeric inputs | infix |
| `exp` | numeric | exact | `exp(expr)` | supported exact numeric inputs | infix |
| `ln` | numeric | exact | `ln(expr)` | supported exact numeric inputs | infix |
| `sin` | numeric | exact | `sin(expr)` | supported exact numeric inputs | infix |
| `cos` | numeric | exact | `cos(expr)` | supported exact numeric inputs | infix |
| `tan` | numeric | exact | `tan(expr)` | supported exact numeric inputs | infix |
| `asin` | numeric | exact | `asin(expr)` | supported exact numeric inputs | infix |
| `acos` | numeric | exact | `acos(expr)` | supported exact numeric inputs | infix |
| `atan` | numeric | exact | `atan(expr)` | supported exact numeric inputs | infix |
| `sinh` | numeric | exact | `sinh(expr)` | supported exact numeric inputs | infix |
| `cosh` | numeric | exact | `cosh(expr)` | supported exact numeric inputs | infix |
| `tanh` | numeric | exact | `tanh(expr)` | supported exact numeric inputs | infix |
| `asinh` | numeric | exact | `asinh(expr)` | supported exact numeric inputs | infix |
| `acosh` | numeric | exact | `acosh(expr)` | supported exact numeric inputs | infix |
| `atanh` | numeric | exact | `atanh(expr)` | supported exact numeric inputs | infix |
| `gamma` | numeric | approx | `gamma(expr)` | real and complex values under approx | infix |
| `lgamma` | numeric | approx | `lgamma(expr)` | real and complex values under approx | infix |
| `erf` | numeric | approx | `erf(expr)` | supported numeric values under approx | infix |
| `erfc` | numeric | approx | `erfc(expr)` | supported numeric values under approx | infix |
| `set` | language | language | `set(symbol,value)` | language forms | infix |
| `setDelayed` | language | language | `setDelayed(lhs,rhs)` | language forms | infix |
| `function` | language | language | `function(params,body)` | language forms | infix |
| `let` | language | language | `let(binding,expr)` | language forms | infix |
| `if` | language | language | `if(condition,then,else)` | language forms | infix |
| `map` | language | language | `map(list,function)` | language forms | infix |
| `apply` | language | language | `apply(function,list)` | language forms | infix |
| `fold` | language | language | `fold(function,initial,list)` | language forms | infix |
| `nest` | language | language | `nest(function,value,count)` | language forms | infix |
