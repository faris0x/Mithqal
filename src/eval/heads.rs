//! Kernel head enums and name lookup.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

pub(crate) enum Numeric {
    Real(Real),
    Complex(Real, Real),
    Error(ExprId),
}

#[derive(Copy, Clone)]
pub(crate) enum NumericHead {
    Plus,
    Times,
    Negate,
    Power,
    Complex,
    Abs,
    Sqrt,
    Exp,
    Ln,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Floor,
    Ceil,
    Round,
    Gamma,
    Lgamma,
    Erf,
    Erfc,
    Erfi,
}

impl NumericHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "plus" => Self::Plus,
            "times" => Self::Times,
            "negate" => Self::Negate,
            "power" => Self::Power,
            "complex" => Self::Complex,
            "abs" => Self::Abs,
            "sqrt" => Self::Sqrt,
            "exp" => Self::Exp,
            "ln" => Self::Ln,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "sinh" => Self::Sinh,
            "cosh" => Self::Cosh,
            "tanh" => Self::Tanh,
            "asinh" => Self::Asinh,
            "acosh" => Self::Acosh,
            "atanh" => Self::Atanh,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "round" => Self::Round,
            "gamma" => Self::Gamma,
            "lgamma" => Self::Lgamma,
            "erf" => Self::Erf,
            "erfc" => Self::Erfc,
            "erfi" => Self::Erfi,
            _ => return None,
        })
    }

    pub(super) fn is_unary(self) -> bool {
        matches!(
            self,
            Self::Sqrt
                | Self::Exp
                | Self::Ln
                | Self::Sin
                | Self::Cos
                | Self::Tan
                | Self::Asin
                | Self::Acos
                | Self::Atan
                | Self::Sinh
                | Self::Cosh
                | Self::Tanh
                | Self::Asinh
                | Self::Acosh
                | Self::Atanh
                | Self::Floor
                | Self::Ceil
                | Self::Round
                | Self::Gamma
                | Self::Lgamma
                | Self::Erf
                | Self::Erfc
                | Self::Erfi
        )
    }
}

#[derive(Copy, Clone)]
pub(crate) enum AlgebraHead {
    Expand,
    Factor,
    Together,
    Cancel,
    PolynomialGcd,
    Resultant,
    Solve,
    Degree,
    Coefficient,
    PolynomialDivide,
    Discriminant,
}

impl AlgebraHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "expand" => Self::Expand,
            "factor" => Self::Factor,
            "together" => Self::Together,
            "cancel" => Self::Cancel,
            "polynomialGcd" => Self::PolynomialGcd,
            "resultant" => Self::Resultant,
            "solve" => Self::Solve,
            "degree" => Self::Degree,
            "coefficient" => Self::Coefficient,
            "polynomialDivide" => Self::PolynomialDivide,
            "discriminant" => Self::Discriminant,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) enum CalculusHead {
    Differentiate,
    Integrate,
    Limit,
    Series,
    Converges,
}

impl CalculusHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "differentiate" => Self::Differentiate,
            "integrate" => Self::Integrate,
            "limit" => Self::Limit,
            "series" => Self::Series,
            "converges" => Self::Converges,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) enum ExactNumericHead {
    Gcd,
    Lcm,
    Factorial,
    Binomial,
    Floor,
    Ceil,
    Round,
    Gamma,
    Lgamma,
}

impl ExactNumericHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "gcd" => Self::Gcd,
            "lcm" => Self::Lcm,
            "factorial" => Self::Factorial,
            "binomial" => Self::Binomial,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "round" => Self::Round,
            "gamma" => Self::Gamma,
            "lgamma" => Self::Lgamma,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
pub(crate) enum LinearHead {
    Determinant,
    Rank,
    Transpose,
    Inverse,
    Rref,
    Trace,
    Matmul,
    Identity,
    SolveLinear,
    Dot,
    Cross,
    Plu,
    Charpoly,
    Eigenvalues,
    Nullspace,
    Eigenvectors,
    Qr,
}
impl LinearHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "det" => Self::Determinant,
            "rank" => Self::Rank,
            "transpose" => Self::Transpose,
            "inverse" => Self::Inverse,
            "rref" => Self::Rref,
            "trace" => Self::Trace,
            "matmul" => Self::Matmul,
            "identity" => Self::Identity,
            "solveLinear" => Self::SolveLinear,
            "dot" => Self::Dot,
            "cross" => Self::Cross,
            "plu" => Self::Plu,
            "charpoly" => Self::Charpoly,
            "eigenvalues" => Self::Eigenvalues,
            "nullspace" | "kernel" => Self::Nullspace,
            "eigenvectors" => Self::Eigenvectors,
            "qr" => Self::Qr,
            _ => return None,
        })
    }

    pub(super) fn arity(self) -> usize {
        match self {
            Self::Matmul | Self::SolveLinear | Self::Dot | Self::Cross => 2,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum NumberTheoryHead {
    IsPrime,
    NextPrime,
    FactorInteger,
    Divisors,
    Totient,
    Sigma,
    IsEven,
    IsOdd,
    Divides,
    Modulo,
    PowerMod,
    PreviousPrime,
    DivisorCount,
    Moebius,
    Radical,
    IsSquareFree,
    PrimeCount,
    Fibonacci,
    IsPerfectSquare,
    ModInverse,
    Jacobi,
}

impl NumberTheoryHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "isPrime" => Self::IsPrime,
            "nextPrime" => Self::NextPrime,
            "factorInteger" => Self::FactorInteger,
            "divisors" => Self::Divisors,
            "totient" => Self::Totient,
            "sigma" => Self::Sigma,
            "isEven" => Self::IsEven,
            "isOdd" => Self::IsOdd,
            "divides" => Self::Divides,
            "mod" => Self::Modulo,
            "powerMod" => Self::PowerMod,
            "previousPrime" => Self::PreviousPrime,
            "divisorCount" => Self::DivisorCount,
            "moebius" => Self::Moebius,
            "radical" => Self::Radical,
            "isSquareFree" => Self::IsSquareFree,
            "primeCount" => Self::PrimeCount,
            "fibonacci" => Self::Fibonacci,
            "isPerfectSquare" => Self::IsPerfectSquare,
            "modInverse" => Self::ModInverse,
            "jacobi" => Self::Jacobi,
            _ => return None,
        })
    }

    pub(super) fn arity(self) -> usize {
        match self {
            Self::Divides | Self::Modulo | Self::ModInverse | Self::Jacobi => 2,
            Self::PowerMod => 3,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum LogicHead {
    Xor,
    Implies,
    All,
    Any,
}

impl LogicHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "xor" => Self::Xor,
            "implies" => Self::Implies,
            "all" => Self::All,
            "any" => Self::Any,
            _ => return None,
        })
    }
}

/// Generic list primitives available to library code.
#[derive(Copy, Clone)]
pub(crate) enum ListHead {
    Length,
    Sort,
    First,
    Rest,
    Cons,
    Nth,
    Reverse,
    IsNumber,
}

impl ListHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "length" => Self::Length,
            "sort" => Self::Sort,
            "first" => Self::First,
            "rest" => Self::Rest,
            "cons" => Self::Cons,
            "nth" => Self::Nth,
            "reverse" => Self::Reverse,
            "isNumber" => Self::IsNumber,
            _ => return None,
        })
    }

    pub(super) fn arity(self) -> usize {
        match self {
            Self::Cons | Self::Nth => 2,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum TextHead {
    StringLength,
    StringJoin,
    StringSplit,
    StringReplace,
    StringTake,
    StringDrop,
    StringContains,
    StringReverse,
    Characters,
    CharacterCode,
    FromCharacterCode,
    ToString,
    ByteArrayLength,
    ByteArrayToList,
    ByteArrayFromList,
    ByteArrayToString,
    StringToByteArray,
    AssociationLength,
    AssociationKeys,
    AssociationValues,
    Lookup,
    KeyExistsQ,
    AssociationMerge,
    AssociationKeyTake,
    AssociationKeyDrop,
    AssociationFromPairs,
    AssociationToPairs,
    AssociationMap,
}

impl TextHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "stringLength" => Self::StringLength,
            "stringJoin" => Self::StringJoin,
            "stringSplit" => Self::StringSplit,
            "stringReplace" => Self::StringReplace,
            "stringTake" => Self::StringTake,
            "stringDrop" => Self::StringDrop,
            "stringContains" => Self::StringContains,
            "stringReverse" => Self::StringReverse,
            "characters" => Self::Characters,
            "characterCode" => Self::CharacterCode,
            "fromCharacterCode" => Self::FromCharacterCode,
            "toString" => Self::ToString,
            "byteArrayLength" => Self::ByteArrayLength,
            "byteArrayToList" => Self::ByteArrayToList,
            "byteArrayFromList" => Self::ByteArrayFromList,
            "byteArrayToString" => Self::ByteArrayToString,
            "stringToByteArray" => Self::StringToByteArray,
            "associationLength" => Self::AssociationLength,
            "associationKeys" => Self::AssociationKeys,
            "associationValues" => Self::AssociationValues,
            "lookup" => Self::Lookup,
            "keyExistsQ" => Self::KeyExistsQ,
            "associationMerge" => Self::AssociationMerge,
            "associationKeyTake" => Self::AssociationKeyTake,
            "associationKeyDrop" => Self::AssociationKeyDrop,
            "associationFromPairs" => Self::AssociationFromPairs,
            "associationToPairs" => Self::AssociationToPairs,
            "associationMap" => Self::AssociationMap,
            _ => return None,
        })
    }

    pub(super) fn arity(self) -> (usize, usize) {
        match self {
            Self::StringJoin | Self::AssociationMerge => (0, usize::MAX),
            Self::StringSplit => (1, 2),
            Self::Lookup => (2, 3),
            Self::StringReplace => (3, 3),
            Self::StringTake
            | Self::StringDrop
            | Self::StringContains
            | Self::KeyExistsQ
            | Self::AssociationKeyTake
            | Self::AssociationKeyDrop
            | Self::AssociationMap => (2, 2),
            _ => (1, 1),
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum IoHead {
    ReadString,
    WriteString,
    FileExistsQ,
    FileNames,
    DirectoryName,
    FileNameJoin,
    CreateDirectory,
    DeleteFile,
    ImportText,
    ExportText,
    ImportCsv,
    ExportCsv,
    ImportTsv,
    ExportTsv,
    ImportJson,
    ExportJson,
}

impl IoHead {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "readString" => Self::ReadString,
            "writeString" => Self::WriteString,
            "fileExistsQ" => Self::FileExistsQ,
            "fileNames" => Self::FileNames,
            "directoryName" => Self::DirectoryName,
            "fileNameJoin" => Self::FileNameJoin,
            "createDirectory" => Self::CreateDirectory,
            "deleteFile" => Self::DeleteFile,
            "importText" => Self::ImportText,
            "exportText" => Self::ExportText,
            "importCsv" => Self::ImportCsv,
            "exportCsv" => Self::ExportCsv,
            "importTsv" => Self::ImportTsv,
            "exportTsv" => Self::ExportTsv,
            "importJson" => Self::ImportJson,
            "exportJson" => Self::ExportJson,
            _ => return None,
        })
    }

    pub(super) fn arity(self) -> (usize, usize) {
        match self {
            Self::WriteString
            | Self::ExportText
            | Self::ExportCsv
            | Self::ExportTsv
            | Self::ExportJson => (2, 2),
            Self::FileNames => (0, 1),
            _ => (1, 1),
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) enum KernelFunction {
    Algebra(AlgebraHead),
    Calculus(CalculusHead),
    Numeric(ExactNumericHead),
    Linear(LinearHead),
    NumberTheory(NumberTheoryHead),
    Logic(LogicHead),
    List(ListHead),
    Text(TextHead),
    Io(IoHead),
}

impl KernelFunction {
    pub(super) fn from_name(name: &str) -> Option<Self> {
        if let Some(function) = AlgebraHead::from_name(name) {
            return Some(Self::Algebra(function));
        }
        if let Some(function) = CalculusHead::from_name(name) {
            return Some(Self::Calculus(function));
        }
        if let Some(function) = ExactNumericHead::from_name(name) {
            return Some(Self::Numeric(function));
        }
        if let Some(function) = LinearHead::from_name(name) {
            return Some(Self::Linear(function));
        }
        if let Some(function) = NumberTheoryHead::from_name(name) {
            return Some(Self::NumberTheory(function));
        }
        if let Some(function) = LogicHead::from_name(name) {
            return Some(Self::Logic(function));
        }
        if let Some(function) = ListHead::from_name(name) {
            return Some(Self::List(function));
        }
        if let Some(function) = TextHead::from_name(name) {
            return Some(Self::Text(function));
        }
        IoHead::from_name(name).map(Self::Io)
    }
}
