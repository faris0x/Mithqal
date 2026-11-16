# Function Capabilities

> Status: Stable. See [Maturity](maturity.md).

This table is the capability matrix for the advertised function surface. `list capabilities` in the REPL prints the same metadata.

Support values: `exact`, `symbolic`, `approx`, `language`. Example values describe the expected documented example behavior.

| Function | Category | Support | Signature | Domain | Display | Example | Example behavior |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `gcd` | core | exact | `gcd(a,b,...)` | documented integer and rational inputs | infix | `gcd(84,30)` | evaluates |
| `lcm` | core | exact | `lcm(a,b,...)` | documented integer and rational inputs | infix | `lcm(12,18)` | evaluates |
| `factorial` | core | exact | `factorial(n)` | non-negative integers up to 10000 | infix | `factorial(10)` | evaluates |
| `binomial` | core | exact | `binomial(n,k)` | documented integer and rational inputs | infix | `binomial(20,10)` | evaluates |
| `floor` | core | exact | `floor(value)` | documented integer and rational inputs | infix | `floor(7/2)` | evaluates |
| `ceil` | core | exact | `ceil(value)` | documented integer and rational inputs | infix | `ceil(7/2)` | evaluates |
| `round` | core | exact | `round(value)` | documented integer and rational inputs | infix | `round(7/2)` | evaluates |
| `expand` | algebra | symbolic | `expand(expr)` | supported expressions in the documented domain | infix | `expand((x+y)^3)` | evaluates |
| `factor` | algebra | symbolic | `factor(expr)` | supported expressions in the documented domain | infix | `factor(x^6-1)` | evaluates |
| `together` | algebra | symbolic | `together(expr)` | supported expressions in the documented domain | infix | `together(1/x+1/(x+1))` | evaluates |
| `cancel` | algebra | symbolic | `cancel(expr)` | supported expressions in the documented domain | infix | `cancel((x^2-1)/(x-1))` | evaluates |
| `polynomialGcd` | algebra | symbolic | `polynomialGcd(p,q,var)` | supported expressions in the documented domain | infix | `polynomialGcd(x^2-1,x^2-2*x+1,x)` | evaluates |
| `resultant` | algebra | symbolic | `resultant(p,q,var)` | supported expressions in the documented domain | infix | `resultant(x^2-1,x-1,x)` | evaluates |
| `solve` | algebra | symbolic | `solve(expr[,var])` | supported expressions in the documented domain | list | `solve(x^2-4)` | evaluates |
| `degree` | algebra | symbolic | `degree(p,var)` | supported expressions in the documented domain | infix | `degree(x^3-2*x+1,x)` | evaluates |
| `coefficient` | algebra | symbolic | `coefficient(p,var,n)` | supported expressions in the documented domain | infix | `coefficient(x^3-2*x+1,x,1)` | evaluates |
| `polynomialDivide` | algebra | symbolic | `polynomialDivide(p,q,var)` | supported expressions in the documented domain | infix | `polynomialDivide(x^2-1,x-1,x)` | evaluates |
| `discriminant` | algebra | symbolic | `discriminant(p,var)` | supported expressions in the documented domain | infix | `discriminant(x^2-4,x)` | evaluates |
| `differentiate` | calculus | symbolic | `differentiate(expr[,var])` | supported expressions in the documented domain | infix | `differentiate(sin(x))` | evaluates |
| `integrate` | calculus | symbolic | `integrate(expr[,var]) | integrate(expr,a,b) | integrate(expr,var,a,b)` | supported expressions in the documented domain | infix | `integrate(x^2*exp(x))` | evaluates |
| `limit` | calculus | symbolic | `limit(expr[,var],point[,side])` | supported expressions in the documented domain | infix | `limit(sin(x)/x,0)` | evaluates |
| `series` | calculus | symbolic | `series(expr[,var],point,order[,evaluateFactorials])` | analytic expressions with supported derivatives | ordered series | `series(exp(x),0,4)` | evaluates |
| `converges` | calculus | symbolic | `converges(expr[,var],point)` | supported expressions in the documented domain | infix | `converges(1/x,inf)` | evaluates |
| `assume` | calculus | language | `assume(fact,expr)` | language forms | infix | `assume(x>0,abs(x))` | symbolic |
| `assuming` | calculus | language | `assuming(fact,expr)` | language forms | infix | `assuming(x<0,sqrt(x^2))` | symbolic |
| `det` | linear algebra | exact | `det(matrix)` | exact rational and symbolic matrices | infix | `det([[1,2],[3,4]])` | evaluates |
| `rank` | linear algebra | exact | `rank(matrix)` | exact rational and symbolic matrices | infix | `rank([[1,2],[2,4]])` | evaluates |
| `transpose` | linear algebra | exact | `transpose(matrix)` | exact rational and symbolic matrices | infix | `transpose([[1,2],[3,4]])` | evaluates |
| `inverse` | linear algebra | exact | `inverse(matrix)` | exact rational and symbolic matrices | infix | `inverse([[1,2],[3,4]])` | evaluates |
| `rref` | linear algebra | exact | `rref(matrix)` | exact rational and symbolic matrices | infix | `rref([[1,2],[3,4]])` | evaluates |
| `trace` | linear algebra | exact | `trace(matrix)` | exact rational and symbolic matrices | infix | `trace([[1,2],[3,4]])` | evaluates |
| `matmul` | linear algebra | exact | `matmul(a,b)` | exact rational and symbolic matrices | infix | `matmul([[1,2],[3,4]],[[5,6],[7,8]])` | evaluates |
| `identity` | linear algebra | exact | `identity(n)` | exact rational and symbolic matrices | infix | `identity(3)` | evaluates |
| `solveLinear` | linear algebra | exact | `solveLinear(a,b)` | exact rational and symbolic matrices | infix | `solveLinear([[2,1],[1,3]],[5,7])` | evaluates |
| `dot` | linear algebra | exact | `dot(a,b)` | exact rational and symbolic matrices | infix | `dot([1,2,3],[4,5,6])` | evaluates |
| `cross` | linear algebra | exact | `cross(a,b)` | exact rational and symbolic matrices | infix | `cross([1,0,0],[0,1,0])` | evaluates |
| `plu` | linear algebra | exact | `plu(matrix)` | exact rational and symbolic matrices | list | `plu([[2,1],[4,3]])` | evaluates |
| `charpoly` | linear algebra | exact | `charpoly(matrix)` | exact rational and symbolic matrices | list | `charpoly([[1,2],[3,4]])` | evaluates |
| `eigenvalues` | linear algebra | exact | `eigenvalues(matrix)` | exact rational and symbolic matrices | list | `eigenvalues([[1,2],[3,4]])` | evaluates |
| `eigenvectors` | linear algebra | exact | `eigenvectors(matrix)` | exact rational and symbolic matrices | list | `eigenvectors([[1,1],[0,2]])` | evaluates |
| `nullspace` | linear algebra | exact | `nullspace(matrix)` | exact rational and symbolic matrices | list | `nullspace([[1,2],[2,4]])` | evaluates |
| `kernel` | linear algebra | exact | `kernel(matrix)` | exact rational and symbolic matrices | list | `kernel([[1,2],[2,4]])` | evaluates |
| `qr` | linear algebra | exact | `qr(matrix)` | exact rational and symbolic matrices | list | `qr([[1,0],[0,1]])` | evaluates |
| `isPrime` | number theory | exact | `isPrime(n)` | integers | boolean | `isPrime(1000000007)` | evaluates |
| `nextPrime` | number theory | exact | `nextPrime(n)` | integers | infix | `nextPrime(100)` | evaluates |
| `previousPrime` | number theory | exact | `previousPrime(n)` | integers | infix | `previousPrime(100)` | evaluates |
| `factorInteger` | number theory | exact | `factorInteger(n)` | nonzero integers | prime product | `factorInteger(360)` | evaluates |
| `divisors` | number theory | exact | `divisors(n)` | integers | list | `divisors(36)` | evaluates |
| `divisorCount` | number theory | exact | `divisorCount(n)` | integers | infix | `divisorCount(360)` | evaluates |
| `totient` | number theory | exact | `totient(n)` | integers | infix | `totient(360)` | evaluates |
| `sigma` | number theory | exact | `sigma(n)` | integers | infix | `sigma(36)` | evaluates |
| `isEven` | number theory | exact | `isEven(n)` | integers | boolean | `isEven(42)` | evaluates |
| `isOdd` | number theory | exact | `isOdd(n)` | integers | boolean | `isOdd(43)` | evaluates |
| `divides` | number theory | exact | `divides(a,b)` | integers | boolean | `divides(3,12)` | evaluates |
| `mod` | number theory | exact | `mod(a,m)` | integers | infix | `mod(-7,3)` | evaluates |
| `powerMod` | number theory | exact | `powerMod(a,e,m)` | integers | infix | `powerMod(2,10,1000)` | evaluates |
| `moebius` | number theory | exact | `moebius(n)` | integers | infix | `moebius(30)` | evaluates |
| `radical` | number theory | exact | `radical(n)` | integers | infix | `radical(360)` | evaluates |
| `isSquareFree` | number theory | exact | `isSquareFree(n)` | integers | boolean | `isSquareFree(30)` | evaluates |
| `primeCount` | number theory | exact | `primeCount(n)` | integers | infix | `primeCount(1000)` | evaluates |
| `fibonacci` | number theory | exact | `fibonacci(n)` | integers | infix | `fibonacci(20)` | evaluates |
| `isPerfectSquare` | number theory | exact | `isPerfectSquare(n)` | integers | boolean | `isPerfectSquare(144)` | evaluates |
| `modInverse` | number theory | exact | `modInverse(a,m)` | integers | infix | `modInverse(3,7)` | evaluates |
| `jacobi` | number theory | exact | `jacobi(a,n)` | integers | infix | `jacobi(5,21)` | evaluates |
| `mean` | statistics | exact | `mean(list)` | numeric lists | infix | `mean([1,2,3,4])` | evaluates |
| `median` | statistics | exact | `median(list)` | numeric lists | infix | `median([1,2,3,4])` | evaluates |
| `minimum` | statistics | exact | `minimum(list)` | numeric lists | infix | `minimum([3,1,2])` | evaluates |
| `maximum` | statistics | exact | `maximum(list)` | numeric lists | infix | `maximum([3,1,2])` | evaluates |
| `range` | statistics | exact | `range(list)` | numeric lists | infix | `range([3,1,2])` | evaluates |
| `total` | statistics | exact | `total(list)` | numeric lists | infix | `total([1,2,3,4])` | evaluates |
| `count` | statistics | exact | `count(list)` | numeric lists | infix | `count([1,2,3,4])` | evaluates |
| `variance` | statistics | exact | `variance(list)` | numeric lists | infix | `variance([1,2,3,4])` | evaluates |
| `sampleVariance` | statistics | exact | `sampleVariance(list)` | numeric lists | infix | `sampleVariance([1,2,3,4])` | evaluates |
| `standardDeviation` | statistics | exact | `standardDeviation(list)` | numeric lists | infix | `standardDeviation([4,4,4,4])` | evaluates |
| `sampleStandardDeviation` | statistics | exact | `sampleStandardDeviation(list)` | numeric lists | infix | `sampleStandardDeviation([4,4,4,4])` | evaluates |
| `frequency` | statistics | exact | `frequency(list)` | numeric lists | list | `frequency([1,1,2])` | evaluates |
| `mode` | statistics | exact | `mode(list)` | numeric lists | list | `mode([1,1,2])` | evaluates |
| `and` | logic | exact | `and(a,b,...)` | boolean expressions | boolean | `and(true,false)` | evaluates |
| `or` | logic | exact | `or(a,b,...)` | boolean expressions | boolean | `or(true,false)` | evaluates |
| `not` | logic | exact | `not(a)` | boolean expressions | boolean | `not(true)` | evaluates |
| `xor` | logic | exact | `xor(a,b)` | boolean expressions | boolean | `xor(true,false)` | evaluates |
| `implies` | logic | exact | `implies(a,b)` | boolean expressions | boolean | `implies(true,false)` | evaluates |
| `all` | logic | exact | `all(list)` | boolean expressions | boolean | `all([true,true])` | evaluates |
| `any` | logic | exact | `any(list)` | boolean expressions | boolean | `any([false,true])` | evaluates |
| `approx` | numeric | approx | `approx(expr[,digits])` | supported numeric values under approx | infix | `approx(pi,30)` | approximate |
| `interval` | numeric | exact | `interval(low,high)` | supported exact numeric inputs | infix | `interval(1,2)+3` | evaluates |
| `complex` | numeric | exact | `complex(real,imaginary)` | supported exact numeric inputs | infix | `approx(2+3*i,30)` | evaluates |
| `abs` | numeric | exact | `abs(expr)` | supported exact numeric inputs | infix | `abs(x)` | symbolic |
| `sqrt` | numeric | exact | `sqrt(expr)` | supported exact numeric inputs | infix | `sqrt(x^2)` | symbolic |
| `exp` | numeric | exact | `exp(expr)` | supported exact numeric inputs | infix | `exp(x)` | symbolic |
| `ln` | numeric | exact | `ln(expr)` | supported exact numeric inputs | infix | `ln(x)` | symbolic |
| `sin` | numeric | exact | `sin(expr)` | supported exact numeric inputs | infix | `sin(x)` | symbolic |
| `cos` | numeric | exact | `cos(expr)` | supported exact numeric inputs | infix | `cos(x)` | symbolic |
| `tan` | numeric | exact | `tan(expr)` | supported exact numeric inputs | infix | `tan(x)` | symbolic |
| `asin` | numeric | exact | `asin(expr)` | supported exact numeric inputs | infix | `asin(x)` | symbolic |
| `acos` | numeric | exact | `acos(expr)` | supported exact numeric inputs | infix | `acos(x)` | symbolic |
| `atan` | numeric | exact | `atan(expr)` | supported exact numeric inputs | infix | `atan(x)` | symbolic |
| `sinh` | numeric | exact | `sinh(expr)` | supported exact numeric inputs | infix | `sinh(x)` | symbolic |
| `cosh` | numeric | exact | `cosh(expr)` | supported exact numeric inputs | infix | `cosh(x)` | symbolic |
| `tanh` | numeric | exact | `tanh(expr)` | supported exact numeric inputs | infix | `tanh(x)` | symbolic |
| `asinh` | numeric | exact | `asinh(expr)` | supported exact numeric inputs | infix | `asinh(x)` | symbolic |
| `acosh` | numeric | exact | `acosh(expr)` | supported exact numeric inputs | infix | `acosh(x)` | symbolic |
| `atanh` | numeric | exact | `atanh(expr)` | supported exact numeric inputs | infix | `atanh(x)` | symbolic |
| `gamma` | numeric | exact | `gamma(expr)` | language forms | infix | `gamma(5)` | evaluates |
| `lgamma` | numeric | exact | `lgamma(expr)` | language forms | infix | `lgamma(5)` | evaluates |
| `erf` | numeric | approx | `erf(expr)` | supported numeric values under approx | infix | `approx(erf(1),30)` | approximate |
| `erfc` | numeric | approx | `erfc(expr)` | supported numeric values under approx | infix | `approx(erfc(1),30)` | approximate |
| `set` | language | language | `set(symbol,value)` | language forms | infix | `set(f,3)` | language |
| `setDelayed` | language | language | `setDelayed(lhs,rhs)` | language forms | infix | `setDelayed(f(x_),x^2)` | language |
| `fn` | language | language | `fn(params)=>body` | language forms | infix | `fn(x)=>x^2` | language |
| `let` | language | language | `let(binding,expr)` | language forms | infix | `let([x=2],x)` | language |
| `if` | language | language | `if(condition,then,else)` | language forms | infix | `if(true,1,2)` | language |
| `map` | language | language | `map(list,function)` | language forms | infix | `map([1,2],fn(x)=>x^2)` | language |
| `apply` | language | language | `apply(function,list)` | language forms | infix | `apply(fn(x,y)=>x+y,[3,4])` | language |
| `fold` | language | language | `fold(function,initial,list)` | language forms | infix | `fold(fn(x,y)=>x+y,0,[1,2,3])` | language |
| `nest` | language | language | `nest(function,value,count)` | language forms | infix | `nest(fn(x)=>x+1,0,5)` | language |
