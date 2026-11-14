# Linear Algebra

The linear algebra layer operates on dense matrices and vectors with exact rational entries. Matrices are written as lists of rows.

```text
[[1,2],[3,4]]
```

Vectors are plain lists.

```text
[1,2,3]
```

Symbolic entries and approximate entries are not accepted by this layer yet. An unsupported argument leaves the call unevaluated.

## Functions

| Function | Meaning |
|---|---|
| `det(a)` | Determinant of a square matrix. |
| `rank(a)` | Rank through exact row reduction. |
| `transpose(a)` | Matrix transpose. |
| `inverse(a)` | Exact inverse, when it exists. |
| `rref(a)` | Reduced row echelon form. |
| `trace(a)` | Sum of the diagonal entries. |
| `matmul(a,b)` | Matrix product. |
| `identity(n)` | The `n` by `n` identity matrix. |
| `solveLinear(a,b)` | Unique solution of `a*x=b`. |
| `dot(a,b)` | Vector dot product. |
| `cross(a,b)` | Three dimensional vector cross product. |
| `plu(a)` | Permutation, lower, and upper factors. |
| `charpoly(a)` | Characteristic polynomial coefficients. |

## Examples

```text
det([[1,2],[3,4]])                  -> -2
rank([[1,2],[2,4]])                 -> 1
transpose([[1,2,3],[4,5,6]])        -> [[1,4],[2,5],[3,6]]
inverse([[1,2],[3,4]])              -> [[-2,1],[3/2,-1/2]]
rref([[1,2],[3,4]])                 -> [[1,0],[0,1]]
trace([[1,2],[3,4]])                -> 5
matmul([[1,2],[3,4]],[[5,6],[7,8]]) -> [[19,22],[43,50]]
identity(2)                         -> [[1,0],[0,1]]
solveLinear([[2,1],[1,3]],[5,7])    -> [8/5,9/5]
dot([1,2,3],[4,5,6])                -> 32
cross([1,0,0],[0,1,0])              -> [0,0,1]
det([[a,b],[c,d]])                  -> -b*c+a*d
matmul([[a,b],[c,d]],[[e,f],[g,h]]) -> [[a*e+b*g,a*f+b*h],[c*e+d*g,c*f+d*h]]
plu([[2,1],[4,3]])                  -> [[[1,0],[0,1]],[[1,0],[2,1]],[[2,1],[0,1]]]
charpoly([[1,2],[3,4]])             -> [-2,-5,1]
```

## Exactness

All elimination is performed over exact rationals. Entries remain integers or rationals, and no approximate conversion happens inside this layer.

Symbolic entries are supported by `det`, `transpose`, `trace`, `matmul`, and `inverse`. Symbolic determinants use cofactor expansion up to five by five. Symbolic inverses use cofactors and the determinant up to three by three. Symbolic results are returned in canonical expression form and simplified by the evaluator.

`inverse` and `solveLinear` return unevaluated calls when the matrix is singular, the system is inconsistent, or the system has more than one solution. Full parametrized solution sets are future work.

Dimension mismatches and non-square inputs also leave the call unevaluated. Wrong arity is a diagnostic.

## Limits

- Dense storage only.
- `rank`, `rref`, and `solveLinear` accept exact rational entries.
- `plu` requires a square full-rank exact matrix.
- `charpoly` accepts exact rational matrices up to eight by eight and returns coefficients in ascending order.
- `identity` accepts sizes from zero to 256.
- Eigenvalues and sparse matrices arrive in later work.
