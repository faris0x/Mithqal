# Library Modules

> Status: Experimental. See [Maturity](maturity.md).

The standard library is written in Mithqal and loaded when a session is created. Each module is strict: exported names are protected and the remaining definitions are private. The core capability matrix is in [Functions](functions.md). This page documents the definitional modules.

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


## calculus

The calculus module expresses differentiation and integration rules in Mithqal. Forms that the rules do not handle fall back to the native `differentiate` and `integrate` kernels.

| Function | Meaning |
|---|---|
| `derive(expression,variable)` | derivative from the Mithqal rule layer |
| `antiderivative(expression,variable)` | indefinite integral from the Mithqal rule layer |

## Notes

`setOf` and `relationOf` canonicalize their input, so equal sets and relations are the same term. Membership uses structural equality. The finite law checkers use closures and finite quantification, so a law that fails over the carrier is reported as `false`.
