//! Built in symbol attributes and registration.
//!
//! Built in implementations are registered with the evaluator. This module
//! owns the static attribute table so the parser and evaluator agree on the
//! shape of kernel heads.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::expr::Attributes;

/// The attributes of a built in symbol, or `None` for an ordinary name.
pub fn attributes(name: &str) -> Option<Attributes> {
    use Attributes as A;
    let attrs = match name {
        "plus" | "times" => A::arithmetic()
            .with(A::LISTABLE)
            .with(A::PROTECTED)
            .with(A::ONE_IDENTITY),
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
        "function" | "let" | "closure" | "approx" | "localDefs" => {
            A::new().with(A::HOLD_ALL).with(A::PROTECTED)
        }
        "forall" | "exists" | "setBuilder" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "interval" | "complex" => A::new().with(A::PROTECTED),
        "kernelExpand"
        | "kernelFactor"
        | "kernelTogether"
        | "kernelCancel"
        | "kernelPolynomialGcd"
        | "kernelResultant"
        | "kernelSolve"
        | "kernelDegree"
        | "kernelCoefficient"
        | "kernelPolynomialDivide"
        | "kernelDiscriminant" => A::new().with(A::PROTECTED),
        "differentiate" | "integrate" | "limit" | "series" | "converges" => {
            A::new().with(A::PROTECTED)
        }
        "assume" | "assuming" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "assumptions" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "begin" | "context" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "end" | "contexts" => A::new().with(A::PROTECTED),
        "$Context" | "$ContextPath" => A::new().with(A::PROTECTED).with(A::CONSTANT),
        "$MithqalPath" => A::new().with(A::CONSTANT),
        "byteArray" | "part" | "span" | "evaluate" => A::new().with(A::PROTECTED),
        "unset" | "clear" | "clearAll" | "remove" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "compound" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "hold" | "holdForm" | "holdPattern" | "unevaluated" | "sequence" => {
            A::new().with(A::HOLD_ALL).with(A::PROTECTED)
        }
        "which" | "switch" | "do" | "while" | "for" | "catch" => {
            A::new().with(A::HOLD_ALL).with(A::PROTECTED)
        }
        "return" | "break" | "continue" | "throw" => A::new().with(A::HOLD_ALL).with(A::PROTECTED),
        "appendTo" | "prependTo" | "addTo" | "subtractFrom" | "timesBy" | "divideBy" => {
            A::new().with(A::HOLD_FIRST).with(A::PROTECTED)
        }
        "stringLength"
        | "stringJoin"
        | "stringSplit"
        | "stringReplace"
        | "stringTake"
        | "stringDrop"
        | "stringContains"
        | "stringReverse"
        | "characters"
        | "characterCode"
        | "fromCharacterCode"
        | "toString"
        | "byteArrayLength"
        | "byteArrayToList"
        | "byteArrayFromList"
        | "byteArrayToString"
        | "stringToByteArray"
        | "associationLength"
        | "associationKeys"
        | "associationValues"
        | "lookup"
        | "keyExistsQ"
        | "associationMerge"
        | "associationKeyTake"
        | "associationKeyDrop"
        | "associationFromPairs"
        | "associationToPairs"
        | "associationMap" => A::new().with(A::PROTECTED),
        "readString" | "writeString" | "fileExistsQ" | "fileNames" | "directoryName"
        | "fileNameJoin" | "createDirectory" | "deleteFile" | "importText" | "exportText"
        | "importCsv" | "exportCsv" | "importTsv" | "exportTsv" | "importJson" | "exportJson" => {
            A::new().with(A::PROTECTED)
        }
        "gcd" | "lcm" | "factorial" | "binomial" | "floor" | "ceil" | "round" | "gamma"
        | "lgamma" | "erf" | "erfc" | "erfi" => A::new().with(A::PROTECTED),
        "kernelDet" | "kernelRank" | "kernelTranspose" | "kernelInverse" | "kernelRref"
        | "kernelTrace" | "kernelMatmul" | "kernelIdentity" | "kernelSolveLinear" | "kernelDot"
        | "kernelCross" | "kernelPlu" | "kernelCharpoly" | "kernelEigenvalues"
        | "kernelNullspace" | "kernelEigenvectors" | "kernelQr" => A::new().with(A::PROTECTED),
        "isPrime" | "factorInteger" | "isEven" | "isOdd" | "divides" | "mod" | "powerMod"
        | "kernelPrimeCount" | "isPerfectSquare" | "modInverse" | "jacobi" => {
            A::new().with(A::PROTECTED)
        }
        "xor" | "implies" | "all" | "any" => A::new().with(A::PROTECTED),
        "length" | "sort" | "first" | "rest" | "cons" | "nth" | "reverse" | "isNumber"
        | "kernelIsList" => A::new().with(A::PROTECTED),
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
