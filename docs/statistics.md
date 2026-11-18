# Statistics

> Status: Partial. See [Maturity](maturity.md).

The statistics layer operates on nonempty lists of exact integers and rationals. Results stay exact.

Statistics is implemented in the Mithqal standard library at `lib/statistics.mith`. The session loads it at construction, so these functions are ordinary Mithqal definitions rather than kernel heads. The module is strict: the public functions are exported and protected, while the helpers (`numeric`, `sum`, `squares`, `runs`, `maxCount`, `pick`) are private to the module. The kernel supplies only the generic list primitives they are built from: `length`, `sort`, `first`, `rest`, `cons`, `nth`, `reverse`, and `isNumber`.

## Functions

| Function | Meaning |
|---|---|
| `mean(data)` | Arithmetic mean |
| `median(data)` | Median |
| `minimum(data)` | Smallest value |
| `maximum(data)` | Largest value |
| `range(data)` | Maximum minus minimum |
| `total(data)` | Sum |
| `count(data)` | Number of values |
| `variance(data)` | Population variance |
| `sampleVariance(data)` | Sample variance |
| `standardDeviation(data)` | Population standard deviation |
| `sampleStandardDeviation(data)` | Sample standard deviation |
| `frequency(data)` | Sorted value and count pairs |
| `mode(data)` | Values with the highest frequency |

## Examples

```text
mean([1,2,3,4])                  -> 5/2
median([1,2,3])                  -> 2
minimum([3,1,2])                 -> 1
maximum([3,1,2])                 -> 3
range([3,1,2])                   -> 2
total([1,2,3,4])                 -> 10
count([1,2,3,4])                 -> 4
variance([1,2,3,4])              -> 5/4
sampleVariance([1,2,3,4])        -> 5/3
standardDeviation([1,3])         -> 1
standardDeviation([1,2,3,4])     -> sqrt(5/4)
frequency([1,2,2,3,3,3])         -> [[1,1],[2,2],[3,3]]
mode([1,2,2,3,3])                -> [2,3]
```

Standard deviations return an exact rational when the variance is a perfect rational square. Otherwise the result is an exact square root expression.

Empty lists, lists containing symbols, and sample statistics with fewer than two values remain unevaluated.

## Limits

- Exact integer and rational entries only.
- Population and sample variance are separate functions.
- The functions are Mithqal definitions; their helpers are private module definitions.
- No distributions, regression, or inferential statistics yet.
