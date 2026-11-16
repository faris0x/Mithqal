//! Built in symbol attributes and registration.
//!
//! Built in implementations are registered with the evaluator. This module
//! owns the static attribute table so the parser and evaluator agree on the
//! shape of kernel heads.

use crate::expr::Attributes;

/// The attributes of a built in symbol, or `None` for an ordinary name.
pub fn attributes(name: &str) -> Option<Attributes> {
    use Attributes as A;
    let attrs = match name {
        "plus" | "times" => A::arithmetic().with(A::LISTABLE).with(A::PROTECTED),
        "power" | "negate" => A::new().with(A::LISTABLE).with(A::PROTECTED),
        "list" => A::new().with(A::PROTECTED),
        "seriesSum" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "set" | "setDelayed" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "if" => A::new().with(A::HOLD_REST).with(A::PROTECTED),
        "and" | "or" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "not" => A::new().with(A::PROTECTED),
        "equal" | "notEqual" | "same" | "less" | "lessEqual" | "greater" | "greaterEqual" => {
            A::new().with(A::PROTECTED)
        }
        "map" | "apply" | "fold" | "nest" => A::new().with(A::PROTECTED),
        "function" | "let" | "closure" | "approx" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "interval" | "complex" => A::new().with(A::PROTECTED),
        "expand" | "factor" | "together" | "cancel" | "polynomialGcd" | "resultant" | "solve"
        | "degree" | "coefficient" | "polynomialDivide" | "discriminant" => {
            A::new().with(A::PROTECTED)
        }
        "differentiate" | "integrate" | "limit" | "series" | "converges" => {
            A::new().with(A::PROTECTED)
        }
        "assume" | "assuming" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "gcd" | "lcm" | "factorial" | "binomial" | "floor" | "ceil" | "round" | "gamma"
        | "lgamma" | "erf" | "erfc" => A::new().with(A::PROTECTED),
        "det" | "rank" | "transpose" | "inverse" | "rref" | "trace" | "matmul" | "identity"
        | "solveLinear" | "dot" | "cross" | "plu" | "charpoly" | "eigenvalues" | "nullspace"
        | "kernel" | "eigenvectors" | "qr" => A::new().with(A::PROTECTED),
        "isPrime" | "nextPrime" | "factorInteger" | "divisors" | "totient" | "sigma" | "isEven"
        | "isOdd" | "divides" | "mod" | "powerMod" => A::new().with(A::PROTECTED),
        "previousPrime" | "divisorCount" | "moebius" | "radical" | "isSquareFree"
        | "primeCount" | "fibonacci" | "isPerfectSquare" | "modInverse" | "jacobi" => {
            A::new().with(A::PROTECTED)
        }
        "xor" | "implies" | "all" | "any" => A::new().with(A::PROTECTED),
        "mean"
        | "median"
        | "minimum"
        | "maximum"
        | "range"
        | "total"
        | "count"
        | "variance"
        | "sampleVariance"
        | "standardDeviation"
        | "sampleStandardDeviation"
        | "frequency"
        | "mode" => A::new().with(A::PROTECTED),
        "pattern" | "blank" | "blankSequence" | "blankNullSequence" | "condition" => {
            A::new().with(A::PROTECTED)
        }
        "rule" | "ruleDelayed" => A::new().with(A::PROTECTED),
        "true" | "false" | "emptyset" | "inf" | "undefined" | "indeterminate" | "failed"
        | "null" => A::new().with(A::PROTECTED).with(A::CONSTANT),
        "i" | "pi" | "e" => A::new().with(A::CONSTANT),
        _ => return None,
    };
    Some(attrs)
}
