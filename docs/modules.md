# Library Modules

> Status: Experimental. See [Maturity](maturity.md).

The standard library is written in Mithqal and loaded when a session is created. A session can instead load selected domains with `Session::with_modules`. The dependency graph and selection rules are documented in [Mathematical domains](domains.md). Each module is strict: exported names are protected and the remaining definitions are private. The core capability matrix is in [Functions](functions.md). This page documents the definitional modules.

Finite sets are represented by canonical sorted lists without duplicates. Set equality is structural equality of the canonical list. Use `setOf` to build a set from arbitrary items. A relation is a set of two item lists. A structure is a dictionary whose first item is a tag and whose remaining items are the structure data.

The law checkers quantify over a finite carrier. They verify a model and do not prove a law.

## lists

| Function | Meaning |
|---|---|
| `append(xs,ys)` | concatenation of two lists |
| `concat(xss)` | concatenation of a list of lists |
| `contains(xs,value)` | structural membership test |
| `unique(xs)` | sorted list without duplicates |
| `filter(xs,predicate)` | items that satisfy the predicate |
| `rangeList(n)` | the list `0` through `n-1` |

## logic

| Function | Meaning |
|---|---|
| `allOf(items,predicate)` | true when every item satisfies the predicate |
| `exists(items,predicate)` | true when some item satisfies the predicate |
| `countWhere(items,predicate)` | number of items that satisfy the predicate |

## sets

| Function | Meaning |
|---|---|
| `setOf(items)` | canonical set from arbitrary items |
| `emptySet()` | the empty set |
| `singleton(value)` | set with one item |
| `union(left,right)` | union |
| `intersection(left,right)` | intersection |
| `difference(left,right)` | items of the left set absent from the right set |
| `member(items,value)` | membership test |
| `subset(left,right)` | true when every item of the left set is in the right set |
| `cardinality(items)` | number of items |
| `isEmpty(items)` | true when the set is empty |
| `powerSet(items)` | set of all subsets |
| `cartesian(left,right)` | set of ordered pairs |

## relations

| Function | Meaning |
|---|---|
| `relationOf(pairs)` | canonical relation from a list of pairs |
| `domainOf(relation)` | set of first coordinates |
| `rangeOf(relation)` | set of second coordinates |
| `imageOf(relation,items)` | second coordinates whose first coordinate is in the items |
| `inverseRelation(relation)` | relation with every pair swapped |
| `composeRelations(first,second)` | relational composition |
| `isFunction(relation)` | true when every first coordinate occurs once |
| `applyRelation(relation,value)` | unique second coordinate, or `undefined` |
| `isReflexive(relation,items)` | true when every item is related to itself |
| `isSymmetric(relation)` | true when every pair has its swap |
| `isTransitive(relation)` | true when every composable pair has its composite |

## structures

| Function | Meaning |
|---|---|
| `monoid(carrier,op,identity)` | monoid dictionary |
| `group(carrier,op,identity,inverse)` | group dictionary |
| `ring(carrier,add,mul,zero,one,negate)` | ring dictionary |
| `carrierOf(structure)` | carrier set |
| `opOf(structure)` | binary operation of a monoid or group |
| `identityOf(structure)` | identity element |
| `inverseOf(structure)` | inverse operation of a group |
| `addOf(structure)` | addition of a ring |
| `mulOf(structure)` | multiplication of a ring |
| `zeroOf(structure)` | additive identity of a ring |
| `oneOf(structure)` | multiplicative identity of a ring |
| `negateOf(structure)` | additive inverse of a ring |
| `associativeOn(op,items)` | finite associativity check |
| `commutativeOn(op,items)` | finite commutativity check |
| `identityOn(op,identity,items)` | finite identity check |
| `inverseOn(op,identity,inverse,items)` | finite inverse check |
| `distributiveOn(add,mul,items)` | finite distributivity check |
| `monoidHolds(structure)` | finite monoid law check |
| `groupHolds(structure)` | finite group law check |
| `ringHolds(structure)` | finite ring law check |

## foundations

| Function | Meaning |
|---|---|
| `peanoZero()` | zero of the Peano interface |
| `peanoSucc(n)` | successor |
| `peanoAdd(a,b)` | addition |
| `peanoMul(a,b)` | multiplication |
| `checkPeano(n)` | finite check of the Peano laws |
| `vonNeumann(n)` | von Neumann numeral as a finite set |
| `checkVonNeumann(n)` | finite check that the numeral has `n` elements |


## combinatorics

| Function | Meaning |
|---|---|
| `permutationCount(n,k)` | ordered selections of `k` items from `n` |
| `multinomial(counts)` | arrangements of a multiset with the given multiplicities |
| `catalan(n)` | the `n`th Catalan number |
| `derangementCount(n)` | permutations with no fixed point |
| `stirlingSecond(n,k)` | partitions of an `n` element set into `k` blocks |
| `permutationsOf(items)` | all orderings of the items |
| `combinationsOf(items,k)` | all `k` element subsets |

## probability

| Function | Meaning |
|---|---|
| `distribution(pairs)` | canonical distribution from outcome and probability pairs |
| `isDistribution(distribution)` | true when outcomes are unique and probabilities are positive and sum to one |
| `uniform(items)` | uniform distribution over the distinct items |
| `support(distribution)` | set of outcomes |
| `probabilityOf(distribution,event)` | probability of an event |
| `expectation(distribution,observable)` | expected value of an observable |
| `varianceOf(distribution,observable)` | variance of an observable |
| `conditional(distribution,event)` | conditional distribution, or `undefined` for an impossible event |
| `productDistribution(left,right)` | independent product of two distributions |


## decision

| Function | Meaning |
|---|---|
| `expectedUtility(distribution,action,utility)` | expected utility of an action |
| `bestActions(distribution,actions,utility)` | actions with maximal expected utility |
| `decisionValue(distribution,actions,utility)` | maximal expected utility |
| `expectedRegret(distribution,action,actions,utility)` | expected regret of an action |
| `minimaxRegretActions(distribution,actions,utility)` | actions with minimal expected regret |

## games

A game is a list of row actions, column actions, and a set of payoff entries. A payoff entry is `[[rowAction,colAction],[rowPayoff,colPayoff]]`.

| Function | Meaning |
|---|---|
| `game(rowActions,colActions,payoffs)` | build a game |
| `rowActionsOf(game)` | row actions |
| `colActionsOf(game)` | column actions |
| `payoffsOf(game)` | payoff entries |
| `payoff(game,rowAction,colAction)` | payoff pair for an outcome |
| `rowPayoff(game,rowAction,colAction)` | first payoff of an outcome |
| `colPayoff(game,rowAction,colAction)` | second payoff of an outcome |
| `bestRows(game,colAction)` | row best responses to a column action |
| `bestColumns(game,rowAction)` | column best responses to a row action |
| `pureNash(game)` | pure Nash equilibria |
| `dominantRowActions(game)` | row actions that are best against every column action |
| `dominantColumnActions(game)` | column actions that are best against every row action |
| `paretoOptimal(game)` | outcomes not strictly dominated |
| `isZeroSum(game)` | true when every outcome payoff pair sums to zero |


## graphs

A graph is a list of kind, vertices, and edges. An undirected edge is normalized by sorting its endpoints. A directed edge is an arc from the first endpoint to the second.

| Function | Meaning |
|---|---|
| `graph(vertices,edges)` | undirected graph |
| `directedGraph(vertices,arcs)` | directed graph |
| `verticesOf(graph)` | vertex set |
| `edgesOf(graph)` | edge set |
| `isDirected(graph)` | true for a directed graph |
| `isUndirected(graph)` | true for an undirected graph |
| `adjacency(graph)` | adjacency relation |
| `neighbors(graph,vertex)` | adjacent vertices |
| `hasEdge(graph,from,to)` | edge test |
| `degreeOf(graph,vertex)` | number of neighbors |
| `inDegree(graph,vertex)` | incoming arcs |
| `outDegree(graph,vertex)` | outgoing arcs |
| `reachable(graph,start)` | vertices reachable from a start vertex |
| `pathExists(graph,start,target)` | reachability test |
| `isConnected(graph)` | true when an undirected graph is connected |
| `components(graph)` | connected components |
| `isTree(graph)` | connected undirected graph with `n-1` edges |
| `isComplete(graph)` | every pair of distinct vertices is adjacent |
| `isAcyclic(graph)` | true when no arc closes a cycle |
| `reachabilityRelation(graph)` | relation of reachable vertex pairs |


## information

Information quantities are in bits. `log2` is exact when the probability is an integer power of two. Other probabilities remain symbolic through `ln`.

| Function | Meaning |
|---|---|
| `log2(p)` | base two logarithm |
| `entropy(distribution)` | Shannon entropy |
| `jointEntropy(distribution)` | entropy of a joint distribution |
| `marginal(distribution,projection)` | marginal distribution over a projection |
| `marginalLeft(distribution)` | marginal over the first coordinate |
| `marginalRight(distribution)` | marginal over the second coordinate |
| `conditionalEntropy(distribution,projection)` | conditional entropy |
| `mutualInformation(distribution)` | mutual information |
| `klDivergence(p,q)` | Kullback-Leibler divergence |
| `crossEntropy(p,q)` | cross entropy |


## numbertheory

Classical and computational number theory. The kernel supplies primality, factorization, divisors, modular powers, modular inverses, Jacobi symbols, Fibonacci numbers, and exact arithmetic. This module supplies the broader classical layer over those primitives.

| Function | Meaning |
|---|---|
| `coprime` | true when the gcd is one (``coprime(a,b)``) |
| `extendedGcd` | Bezout coefficients `[g,x,y]` with `a*x+b*y=g` (``extendedGcd(a,b)``) |
| `quotient` | floor of the integer quotient (``quotient(a,b)``) |
| `remainder` | nonnegative remainder (``remainder(a,b)``) |
| `crt` | two-modulus Chinese remainder solution (``crt(a1,m1,a2,m2)``) |
| `modAdd` | modular addition (``modAdd(a,b,m)``) |
| `modSub` | modular subtraction (``modSub(a,b,m)``) |
| `modMul` | modular multiplication (``modMul(a,b,m)``) |
| `modPow` | modular exponentiation (``modPow(a,e,m)``) |
| `legendreSymbol` | Legendre symbol through the Jacobi algorithm (``legendreSymbol(a,p)``) |
| `isQuadraticResidue` | quadratic residue test for odd primes (``isQuadraticResidue(a,p)``) |
| `multiplicativeOrder` | least positive exponent with `a^k=1 mod n` (``multiplicativeOrder(a,n)``) |
| `primitiveRoot` | least primitive root modulo a prime (``primitiveRoot(p)``) |
| `discreteLog` | brute-force discrete logarithm modulo a prime (``discreteLog(g,h,p)``) |
| `carmichaelLambda` | Carmichael lambda function (``carmichaelLambda(n)``) |
| `jordanTotient` | Jordan totient `J_k(n)` (``jordanTotient(n,k)``) |
| `dedekindPsi` | Dedekind psi function (``dedekindPsi(n)``) |
| `liouville` | Liouville lambda function (``liouville(n)``) |
| `omega` | number of distinct prime factors (``omega(n)``) |
| `bigOmega` | number of prime factors with multiplicity (``bigOmega(n)``) |
| `divisorSigma` | sum of the `k`th powers of the divisors (``divisorSigma(k,n)``) |
| `unitaryDivisors` | divisors whose gcd with their cofactor is one (``unitaryDivisors(n)``) |
| `unitaryDivisorCount` | number of unitary divisors (``unitaryDivisorCount(n)``) |
| `aliquotSum` | sum of proper divisors (``aliquotSum(n)``) |
| `isPerfect` | perfect number test (``isPerfect(n)``) |
| `isAbundant` | abundant number test (``isAbundant(n)``) |
| `isDeficient` | deficient number test (``isDeficient(n)``) |
| `isAmicable` | amicable pair test (``isAmicable(a,b)``) |
| `primeFactors` | prime factors with multiplicity (``primeFactors(n)``) |
| `distinctPrimeFactors` | distinct prime factors (``distinctPrimeFactors(n)``) |
| `primeExponent` | exponent of `p` in the factorization of `n` (``primeExponent(n,p)``) |
| `valuation` | p-adic valuation (``valuation(n,p)``) |
| `nthPrime` | the `n`th prime, one based (``nthPrime(n)``) |
| `primeGap` | distance from `n` to the next prime (``primeGap(n)``) |
| `isTwinPrime` | prime with a prime gap of two (``isTwinPrime(p)``) |
| `isCousinPrime` | prime with a prime gap of four (``isCousinPrime(p)``) |
| `isSexyPrime` | prime with a prime gap of six (``isSexyPrime(p)``) |
| `isSophieGermainPrime` | prime `p` with `2p+1` prime (``isSophieGermainPrime(p)``) |
| `isSafePrime` | prime `p` with `(p-1)/2` prime (``isSafePrime(p)``) |
| `isMersennePrime` | Mersenne prime test (``isMersennePrime(n)``) |
| `mersenneNumber` | `2^n-1` (``mersenneNumber(n)``) |
| `fermatNumber` | `2^(2^n)+1` (``fermatNumber(n)``) |
| `isFermatPrime` | Fermat prime test (``isFermatPrime(n)``) |
| `isPrimePower` | prime power test (``isPrimePower(n)``) |
| `isPerfectPower` | perfect power test (``isPerfectPower(n)``) |
| `lucasNumber` | Lucas number (``lucasNumber(n)``) |
| `tribonacci` | tribonacci number (``tribonacci(n)``) |
| `pellNumber` | Pell number (``pellNumber(n)``) |
| `jacobsthalNumber` | Jacobsthal number (``jacobsthalNumber(n)``) |
| `triangularNumber` | triangular number (``triangularNumber(n)``) |
| `pentagonalNumber` | pentagonal number (``pentagonalNumber(n)``) |
| `hexagonalNumber` | hexagonal number (``hexagonalNumber(n)``) |
| `polygonalNumber` | `s`-gonal number (``polygonalNumber(s,n)``) |
| `bellNumber` | Bell number (``bellNumber(n)``) |
| `partitionNumber` | integer partition count (``partitionNumber(n)``) |
| `harmonicNumber` | exact harmonic number (``harmonicNumber(n)``) |
| `mertens` | Mertens function (``mertens(n)``) |
| `summatoryTotient` | sum of totients through `n` (``summatoryTotient(n)``) |
| `summatorySigma` | sum of divisor sums through `n` (``summatorySigma(n)``) |
| `pythagoreanTriples` | primitive Pythagorean triples with hypotenuse at most `limit` (``pythagoreanTriples(limit)``) |
| `isPythagorean` | Pythagorean triple test (``isPythagorean(a,b,c)``) |
| `solveLinearDiophantine` | one solution of `a*x+b*y=c` (``solveLinearDiophantine(a,b,c)``) |
| `sumOfTwoSquares` | one representation as a sum of two squares (``sumOfTwoSquares(n)``) |
| `isSumOfTwoSquares` | sum of two squares test (``isSumOfTwoSquares(n)``) |
| `egyptianFraction` | greedy Egyptian fraction expansion (``egyptianFraction(a,b)``) |
| `continuedFraction` | continued fraction of an exact rational (``continuedFraction(a,b)``) |
| `convergents` | convergents of a continued fraction (``convergents(cf)``) |
| `gaussianNorm` | Gaussian integer norm (``gaussianNorm(a,b)``) |
| `gaussianMultiply` | Gaussian integer product (``gaussianMultiply(a,b,c,d)``) |
| `gaussianIsPrime` | Gaussian prime test (``gaussianIsPrime(a,b)``) |
| `floorSqrt` | integer square root (``floorSqrt(n)``) |
| `primeList` | primes below `n` (``primeList(n)``) |
| `primeSum` | sum of primes below `n` (``primeSum(n)``) |
| `primeProduct` | product of primes below `n` (``primeProduct(n)``) |
| `radicalList` | distinct prime factors (``radicalList(n)``) |
| `mobiusSum` | Mertens function alias (``mobiusSum(n)``) |
| `divisorSum` | sum of positive divisors (``divisorSum(n)``) |

## analyticnumbertheory

| Function | Meaning |
|---|---|
| `bernoulliNumber` | Bernoulli number `B_n` in the `B_1=-1/2` convention |
| `bernoulliPolynomial` | Bernoulli polynomial `B_n(x)` |
| `vonMangoldt` | von Mangoldt function |
| `mangoldt` | Alias for `vonMangoldt` |
| `chebyshevTheta` | Chebyshev theta sum of `log p` |
| `chebyshevPsi` | Chebyshev psi sum of von Mangoldt values |
| `summatoryLiouville` | summatory Liouville function |
| `summatoryMobius` | summatory Moebius function |
| `zetaInteger` | partial sum of the zeta series at an integer exponent |
| `dirichletEta` | truncated Dirichlet eta series |
| `eulerNumber` | Euler secant number |
| `primeHarmonic` | sum of reciprocals of primes below a bound |

## ellipticcurves

| Function | Meaning |
|---|---|
| `ellipticDiscriminant` | discriminant `-16(4a^3+27b^2)` |
| `ellipticJInvariant` | j-invariant |
| `ellipticIsOnCurve` | rational point test |
| `ellipticIsOnCurveMod` | finite-field point test |
| `ellipticIsSingular` | singular curve test |
| `ellipticNegate` | negation of a point |
| `ellipticAdd` | affine point addition over the rationals |
| `ellipticDouble` | point doubling over the rationals |
| `ellipticScalarMul` | scalar multiplication over the rationals |
| `ellipticPointOrder` | order search for a rational point |
| `ellipticPointCount` | point count over a prime field |
| `ellipticAddMod` | point addition modulo a prime |
| `ellipticDoubleMod` | point doubling modulo a prime |
| `ellipticScalarMulMod` | scalar multiplication modulo a prime |
| `ellipticCurveDiffieHellman` | elliptic-curve Diffie-Hellman shared point |
| `ellipticTwist` | quadratic twist of a short Weierstrass curve |

## algebraicnumbertheory

| Function | Meaning |
|---|---|
| `quadraticConjugate` | conjugate in a quadratic field |
| `quadraticNorm` | field norm |
| `quadraticTrace` | field trace |
| `quadraticAdd` | addition of quadratic field elements |
| `quadraticMultiply` | multiplication of quadratic field elements |
| `quadraticInverse` | inverse of a quadratic field element |
| `quadraticIsUnit` | unit test from the norm |
| `quadraticPower` | power of a quadratic field element |
| `eisensteinNorm` | Eisenstein integer norm |
| `eisensteinMultiply` | Eisenstein integer product |
| `eisensteinIsPrime` | Eisenstein prime test |
| `padicValuation` | p-adic valuation |
| `padicNorm` | p-adic norm |
| `padicUnit` | p-adic unit part |
| `padicExpansion` | truncated p-adic digit expansion |
| `padicDigit` | one p-adic digit |

## quadraticforms

| Function | Meaning |
|---|---|
| `binaryFormDiscriminant` | discriminant of a binary quadratic form |
| `isReducedBinaryForm` | reduced positive definite form test |
| `reduceBinaryForm` | Gauss reduction |
| `reducedForms` | reduced forms of a negative discriminant |
| `binaryFormClassNumber` | class number by counting reduced forms |
| `binaryFormValue` | value of a binary quadratic form |
| `binaryFormRepresents` | small-value representation search |

## cryptonumbertheory

| Function | Meaning |
|---|---|
| `millerRabinWitness` | Miller-Rabin witness test |
| `isStrongPseudoprime` | strong pseudoprime test |
| `isCarmichael` | Carmichael number test |
| `isPoulet` | Fermat pseudoprime base two |
| `lucasLehmer` | Lucas-Lehmer test |
| `isMersennePrimeLucasLehmer` | Lucas-Lehmer Mersenne prime test |
| `trialDivisionFactor` | smallest factor by trial division |
| `rsaEncrypt` | RSA encryption exponentiation |
| `rsaDecrypt` | RSA decryption exponentiation |
| `diffieHellman` | finite-field Diffie-Hellman shared value |
| `isWieferichPrime` | Wieferich prime test |
| `isWilsonPrime` | Wilson prime test |
| `isEulerPseudoprime` | Euler pseudoprime test |
| `orderOfElement` | multiplicative order alias |

## calculus

The calculus module expresses differentiation and integration rules in Mithqal. Forms that the rules do not handle fall back to the native `differentiate` and `integrate` kernels.

| Function | Meaning |
|---|---|
| `derive(expression,variable)` | derivative from the Mithqal rule layer |
| `antiderivative(expression,variable)` | indefinite integral from the Mithqal rule layer |

## Package files and loading

A package file is a module source. `import "path.mith"` loads a file relative to
the importing source and then through `$MithqalPath`. `Session::needs` loads a
package once, and `Session::get` re-evaluates it. Import cycles and missing
packages are rejected.

A strict module's private definitions are installed in a context named after the
module. Exported definitions stay visible under their source names. A private
name is not callable by its short name, but remains reachable with a qualified
name such as `statistics;helper`.

## Notes

`setOf` and `relationOf` canonicalize their input, so equal sets and relations are the same term. Membership uses structural equality. The finite law checkers use closures and finite quantification, so a law that fails over the carrier is reported as `false`.
