# Number Theory

> Status: Partial. See [Maturity](maturity.md).

The number theory layer works on exact integers. Factorization and primality operate on values that fit a signed 64 bit integer. Modular arithmetic uses arbitrary precision integers.

## Functions

| Function | Meaning |
|---|---|
| `isPrime(n)` | Deterministic primality test. |
| `nextPrime(n)` | Smallest prime greater than `n`. |
| `factorInteger(n)` | Prime factorization as `[[prime,exponent],...]`. |
| `divisors(n)` | Sorted positive divisors of `n`. |
| `totient(n)` | Euler totient. |
| `sigma(n)` | Sum of positive divisors. |
| `isEven(n)` | Parity predicate. |
| `isOdd(n)` | Parity predicate. |
| `divides(a,b)` | True when `a` divides `b`. |
| `mod(a,m)` | Nonnegative remainder. |
| `powerMod(a,e,m)` | Modular exponentiation. |
| `previousPrime(n)` | Largest prime less than `n`. |
| `divisorCount(n)` | Number of positive divisors. |
| `moebius(n)` | Moebius function. |
| `radical(n)` | Product of distinct prime factors. |
| `isSquareFree(n)` | Square-free predicate. |
| `primeCount(n)` | Number of primes up to `n`. |
| `fibonacci(n)` | Fibonacci number. |
| `isPerfectSquare(n)` | Perfect square predicate. |
| `modInverse(a,m)` | Modular inverse. |
| `jacobi(a,n)` | Jacobi symbol. |

## Examples

```text
isPrime(97)                 -> true
nextPrime(100)              -> 101
factorInteger(360)          -> [[2,3],[3,2],[5,1]]
factorInteger(-12)          -> [[-1,1],[2,2],[3,1]]
divisors(36)                -> [1,2,3,4,6,9,12,18,36]
totient(36)                 -> 12
sigma(12)                   -> 28
isEven(-2)                  -> true
isOdd(-3)                   -> true
divides(3,12)               -> true
mod(-7,3)                   -> 2
powerMod(2,10,1000)         -> 24
previousPrime(10)           -> 7
divisorCount(36)            -> 9
moebius(30)                 -> -1
radical(360)                -> 30
isSquareFree(30)            -> true
primeCount(100)             -> 25
fibonacci(20)               -> 6765
isPerfectSquare(144)        -> true
modInverse(3,11)            -> 4
jacobi(2,15)                -> 1
```

`factorInteger(1)` is the empty list. `factorInteger(0)` remains unevaluated. Negative values include `[-1,1]` as the first factor.

`divisors` returns positive divisors of the absolute value. `mod` normalizes the result into the nonnegative range of the modulus. `powerMod` accepts nonnegative exponents.

## Limits

- Primality, factorization, prime counting, and Fibonacci accept signed 64 bit inputs within their documented ranges.
- `primeCount` accepts values up to five million.
- `fibonacci` accepts values up to 10000.
- Modular remainder and modular exponentiation accept arbitrary precision integers.
- Modular inverses and Jacobi symbols use signed 64 bit inputs.
- Symbolic arguments leave the call unevaluated.
