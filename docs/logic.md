# Logic

> Status: Stable. See [Maturity](maturity.md).

The logic layer provides boolean values and the usual connectives. Predicates from the number theory layer return `true` or `false` and compose with these forms.

## Connectives

| Function | Meaning |
|---|---|
| `not(a)` | Negation. |
| `and(a,b,...)` | Conjunction. |
| `or(a,b,...)` | Disjunction. |
| `xor(a,b)` | Exclusive or. |
| `implies(a,b)` | Material implication. |
| `all([a,b,...])` | True when every element is true. |
| `any([a,b,...])` | True when at least one element is true. |

`not`, `and`, and `or` are held forms that simplify known booleans. `xor`, `implies`, `all`, and `any` evaluate their arguments and require boolean values.

## Examples

```text
xor(false,true)                     -> true
xor(true,true)                      -> false
implies(false,false)                -> true
implies(true,false)                 -> false
all([true,true,true])               -> true
all([true,false,true])              -> false
any([false,false,true])             -> true
and(isPrime(7),isOdd(7))            -> true
or(isPrime(4),isEven(4))            -> true
```

Symbolic operands leave the connective unevaluated when a result cannot be determined.

The language conditional accepts these booleans directly:

```text
mod logic
let n=17
if and(isPrime(n),isOdd(n)) then
  prt 1
else
  prt 0
end
end
```
