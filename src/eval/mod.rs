//! Fixed point evaluator.
//!
//! The current version implements exact arithmetic, listable threading, comparisons,
//! logic, and the boundary for definitions. Definitions and pattern matching
//! are added in the next layer.

mod arithmetic;
mod control;
mod definitions;
mod dispatch;
mod limits;
mod numeric;
mod pattern;
mod support;

use core::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
use crate::kernel::Heads;
use crate::num::{ComplexMpc, Int, MPFR_RNDD, MPFR_RNDU, Rat, Real};

use definitions::{Definitions, Rule};
use pattern::{Bindings, match_pattern, substitute};
use support::*;

pub(crate) use definitions::Definitions as SessionDefinitions;

enum Numeric {
    Real(Real),
    Complex(Real, Real),
    Error(ExprId),
}

#[derive(Copy, Clone)]
enum NumericHead {
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
}

impl NumericHead {
    fn from_name(name: &str) -> Option<Self> {
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
            _ => return None,
        })
    }

    fn is_unary(self) -> bool {
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
        )
    }
}

#[derive(Copy, Clone)]
enum AlgebraHead {
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
    fn from_name(name: &str) -> Option<Self> {
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
enum CalculusHead {
    Differentiate,
    Integrate,
    Limit,
    Series,
    Converges,
}

impl CalculusHead {
    fn from_name(name: &str) -> Option<Self> {
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
enum ExactNumericHead {
    Gcd,
    Lcm,
    Factorial,
    Binomial,
    Floor,
    Ceil,
    Round,
}

impl ExactNumericHead {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "gcd" => Self::Gcd,
            "lcm" => Self::Lcm,
            "factorial" => Self::Factorial,
            "binomial" => Self::Binomial,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "round" => Self::Round,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
enum LinearHead {
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
    fn from_name(name: &str) -> Option<Self> {
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

    fn arity(self) -> usize {
        match self {
            Self::Matmul | Self::SolveLinear | Self::Dot | Self::Cross => 2,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone)]
enum NumberTheoryHead {
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
    fn from_name(name: &str) -> Option<Self> {
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

    fn arity(self) -> usize {
        match self {
            Self::Divides | Self::Modulo | Self::ModInverse | Self::Jacobi => 2,
            Self::PowerMod => 3,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone)]
enum LogicHead {
    Xor,
    Implies,
    All,
    Any,
}

impl LogicHead {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "xor" => Self::Xor,
            "implies" => Self::Implies,
            "all" => Self::All,
            "any" => Self::Any,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
enum StatisticsHead {
    Mean,
    Median,
    Minimum,
    Maximum,
    Range,
    Total,
    Count,
    Variance,
    SampleVariance,
    StandardDeviation,
    SampleStandardDeviation,
    Frequency,
    Mode,
}

impl StatisticsHead {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "mean" => Self::Mean,
            "median" => Self::Median,
            "minimum" => Self::Minimum,
            "maximum" => Self::Maximum,
            "range" => Self::Range,
            "total" => Self::Total,
            "count" => Self::Count,
            "variance" => Self::Variance,
            "sampleVariance" => Self::SampleVariance,
            "standardDeviation" => Self::StandardDeviation,
            "sampleStandardDeviation" => Self::SampleStandardDeviation,
            "frequency" => Self::Frequency,
            "mode" => Self::Mode,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
enum KernelFunction {
    Algebra(AlgebraHead),
    Calculus(CalculusHead),
    Numeric(ExactNumericHead),
    Linear(LinearHead),
    NumberTheory(NumberTheoryHead),
    Logic(LogicHead),
    Statistics(StatisticsHead),
}

impl KernelFunction {
    fn from_name(name: &str) -> Option<Self> {
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
        StatisticsHead::from_name(name).map(Self::Statistics)
    }
}

/// Evaluate an expression to a normal form.
pub fn evaluate(arena: &mut Arena, id: ExprId) -> Result<ExprId, Diagnostic> {
    let mut definitions = Definitions::new();
    evaluate_with_definitions(arena, &mut definitions, id, 1_000_000, 2048)
}

/// Evaluate an expression with explicit budget and depth limits.
pub fn evaluate_with_limits(
    arena: &mut Arena,
    id: ExprId,
    max_steps: u64,
    max_depth: u32,
) -> Result<ExprId, Diagnostic> {
    let mut definitions = Definitions::new();
    evaluate_with_definitions(arena, &mut definitions, id, max_steps, max_depth)
}

pub(crate) fn evaluate_with_definitions(
    arena: &mut Arena,
    definitions: &mut Definitions,
    id: ExprId,
    max_steps: u64,
    max_depth: u32,
) -> Result<ExprId, Diagnostic> {
    if max_steps >= 16
        && max_depth >= 4
        && let Some(value) = fold_numeric_call(arena, id)
    {
        return Ok(value);
    }
    let mut evaluator = Evaluator::new(arena, definitions);
    evaluator.max_steps = max_steps.max(1);
    evaluator.max_depth = max_depth.max(1);
    evaluator.eval(id)
}

struct SmallArgs {
    inline: [ExprId; 6],
    len: usize,
    heap: Vec<ExprId>,
}

impl SmallArgs {
    fn new() -> Self {
        SmallArgs {
            inline: [ExprId::default(); 6],
            len: 0,
            heap: Vec::new(),
        }
    }

    fn from_slice(args: &[ExprId]) -> Self {
        if args.len() <= 6 {
            let mut inline = [ExprId::default(); 6];
            inline[..args.len()].copy_from_slice(args);
            SmallArgs {
                inline,
                len: args.len(),
                heap: Vec::new(),
            }
        } else {
            SmallArgs {
                inline: [ExprId::default(); 6],
                len: 0,
                heap: args.to_vec(),
            }
        }
    }

    fn push(&mut self, value: ExprId) {
        if self.heap.is_empty() && self.len < 6 {
            self.inline[self.len] = value;
            self.len += 1;
        } else {
            if self.heap.is_empty() {
                self.heap.extend_from_slice(&self.inline[..self.len]);
            }
            self.heap.push(value);
        }
    }

    fn as_slice(&self) -> &[ExprId] {
        if self.heap.is_empty() {
            &self.inline[..self.len]
        } else {
            &self.heap
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Sign {
    Negative,
    Zero,
    Positive,
    NonNegative,
    NonPositive,
}

#[derive(Copy, Clone)]
struct Assumption {
    symbol: SymbolId,
    sign: Sign,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitSide {
    Both,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitTarget {
    Finite(ExprId),
    PositiveInfinity,
    NegativeInfinity,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitValue {
    Finite(ExprId),
    PositiveInfinity,
    NegativeInfinity,
}

impl LimitValue {
    fn sign(self) -> Option<Sign> {
        match self {
            LimitValue::PositiveInfinity => Some(Sign::Positive),
            LimitValue::NegativeInfinity => Some(Sign::Negative),
            LimitValue::Finite(_) => None,
        }
    }

    fn with_sign(self, sign: Sign) -> LimitValue {
        if !matches!(
            self,
            LimitValue::PositiveInfinity | LimitValue::NegativeInfinity
        ) {
            return self;
        }
        let current = match self {
            LimitValue::NegativeInfinity => Sign::Negative,
            _ => Sign::Positive,
        };
        match sign_product(current, sign) {
            Some(Sign::Negative) => LimitValue::NegativeInfinity,
            _ => LimitValue::PositiveInfinity,
        }
    }
}

struct Evaluator<'a> {
    arena: &'a mut Arena,
    definitions: &'a mut Definitions,
    heads: Heads,
    locals: Vec<HashMap<SymbolId, ExprId>>,
    assumptions: Vec<Assumption>,
    steps: u64,
    max_steps: u64,
    depth: u32,
    max_depth: u32,
}

impl<'a> Evaluator<'a> {
    fn new(arena: &'a mut Arena, definitions: &'a mut Definitions) -> Self {
        let heads = arena.heads();
        Evaluator {
            arena,
            definitions,
            heads,
            locals: Vec::new(),
            assumptions: Vec::new(),
            steps: 0,
            max_steps: 1_000_000,
            depth: 0,
            max_depth: 2048,
        }
    }

    fn eval(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(Diagnostic::eval(
                ErrorCode::EvalBudget,
                "evaluation budget exhausted",
            ));
        }
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(Diagnostic::eval(
                ErrorCode::EvalBudget,
                "evaluation depth exhausted",
            ));
        }
        let result = self.eval_inner(id);
        self.depth -= 1;
        result
    }

    fn eval_inner(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        let (head, args) = match self.arena.node(id) {
            Node::Int(_) | Node::Rat(_) | Node::Real(_) => return Ok(id),
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if let Some(value) = self.local_value(symbol) {
                    return Ok(value);
                }
                if let Some(own) = self.definitions.own_value(symbol) {
                    return self.eval(own.value);
                }
                return Ok(id);
            }
            Node::Call { head, args } => (*head, SmallArgs::from_slice(args)),
        };
        let head = if let Node::Symbol(symbol) = self.arena.node(head) {
            let symbol = *symbol;
            if self.local_value(symbol).is_none() && self.definitions.own_value(symbol).is_none() {
                head
            } else {
                self.eval(head)?
            }
        } else {
            self.eval(head)?
        };
        let closure = match self.arena.node(head) {
            Node::Call {
                head: closure_head,
                args,
            } if *closure_head == self.heads.closure => Some(args.to_vec()),
            _ => None,
        };
        if let Some(closure) = closure {
            return self.apply_closure(&closure, args.as_slice());
        }
        if !matches!(self.arena.node(head), Node::Symbol(_)) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalNotCallable,
                "a call head must evaluate to a symbol",
            ));
        }

        if head == self.heads.list
            && args.as_slice().iter().all(|arg| {
                matches!(
                    self.arena.node(*arg),
                    Node::Int(_) | Node::Rat(_) | Node::Real(_)
                )
            })
        {
            return Ok(id);
        }
        if head == self.heads.assume || head == self.heads.assuming {
            return self.eval_assume(args.as_slice());
        }
        if head == self.heads.function {
            return self.eval_function(args.as_slice());
        }
        if head == self.heads.approx {
            return self.eval_approx(args.as_slice());
        }
        if head == self.heads.interval {
            return self.eval_interval(args.as_slice());
        }
        if head == self.heads.complex {
            return self.eval_complex(args.as_slice());
        }
        if head == self.heads.set {
            return self.eval_definition(args.as_slice(), false);
        }
        if head == self.heads.set_delayed {
            return self.eval_definition(args.as_slice(), true);
        }
        if head == self.heads.let_head {
            return self.eval_let(args.as_slice());
        }
        if head == self.heads.if_head {
            return self.eval_if(args.as_slice());
        }
        if head == self.heads.and {
            return self.eval_and(args.as_slice());
        }
        if head == self.heads.or {
            return self.eval_or(args.as_slice());
        }
        if head == self.heads.not {
            return self.eval_not(args.as_slice());
        }

        let attrs = self.arena.head_attributes(head);
        if attrs.has(Attributes::HOLD_ALL)
            || attrs.has(Attributes::HOLD_FIRST)
            || attrs.has(Attributes::HOLD_REST)
        {
            return Ok(id);
        }

        let mut evaluated = SmallArgs::new();
        for arg in args.as_slice() {
            evaluated.push(self.eval(*arg)?);
        }

        if head == self.heads.plus || head == self.heads.times || head == self.heads.negate {
            let candidate = self.arena.intern_call(head, evaluated.as_slice());
            if let Some((real, imaginary)) = self.gaussian(candidate) {
                return Ok(self.gaussian_node(&real, &imaginary));
            }
        }

        if attrs.has(Attributes::LISTABLE)
            && let Some(result) = self.thread_listable(head, evaluated.as_slice())?
        {
            return self.finish(id, result);
        }

        let result = self.apply(head, evaluated.as_slice())?;
        self.finish(id, result)
    }

    fn eval_assume(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "assume expects an assumption and an expression",
            ));
        }
        let Some(assumptions) = self.parse_assumptions(args[0]) else {
            return Ok(self.arena.intern_call(self.heads.assume, args));
        };
        let length = self.assumptions.len();
        self.assumptions.extend(assumptions);
        let result = self.eval(args[1]);
        self.assumptions.truncate(length);
        result
    }

    fn parse_assumptions(&self, expression: ExprId) -> Option<Vec<Assumption>> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && *head == self.heads.and
        {
            let mut assumptions = Vec::new();
            for argument in args.iter() {
                assumptions.extend(self.parse_assumptions(*argument)?);
            }
            return (!assumptions.is_empty()).then_some(assumptions);
        }
        self.parse_assumption(expression)
            .map(|assumption| vec![assumption])
    }

    fn parse_assumption(&self, expression: ExprId) -> Option<Assumption> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if args.len() != 2 {
            return None;
        }
        let name = self
            .arena
            .symbol_id(*head)
            .map(|symbol| self.arena.symbol_name(symbol))?;
        let symbol = self.arena.symbol_id(args[0])?;
        let zero = match self.arena.node(args[1]) {
            Node::Int(value) => value.is_zero(),
            Node::Rat(value) => value.is_zero(),
            _ => false,
        };
        if !zero {
            return None;
        }
        let sign = match name {
            "greater" => Sign::Positive,
            "greaterEqual" => Sign::NonNegative,
            "less" => Sign::Negative,
            "lessEqual" => Sign::NonPositive,
            "equal" => Sign::Zero,
            _ => return None,
        };
        Some(Assumption { symbol, sign })
    }

    fn expression_sign(&self, expression: ExprId) -> Option<Sign> {
        match self.arena.node(expression) {
            Node::Int(value) => Some(sign_of(value.sign())),
            Node::Rat(value) => Some(sign_of(value.sign())),
            Node::Real(value) => Some(sign_of(value.sign())),
            Node::Symbol(symbol) => {
                if let Some(sign) = self
                    .assumptions
                    .iter()
                    .rev()
                    .find(|assumption| assumption.symbol == *symbol)
                    .map(|assumption| assumption.sign)
                {
                    return Some(sign);
                }
                match self.arena.symbol_name(*symbol) {
                    "pi" | "e" => Some(Sign::Positive),
                    _ => None,
                }
            }
            Node::Call { head, args } => {
                let name = self
                    .arena
                    .symbol_id(*head)
                    .map(|symbol| self.arena.symbol_name(symbol))?;
                match name {
                    "negate" if args.len() == 1 => self.expression_sign(args[0]).map(flip_sign),
                    "abs" | "sqrt" if args.len() == 1 => Some(Sign::NonNegative),
                    "exp" if args.len() == 1 => Some(Sign::Positive),
                    "times" => {
                        let mut exact = Sign::Positive;
                        for arg in args.iter() {
                            exact = multiply_signs(exact, self.expression_sign(*arg)?)?;
                        }
                        Some(exact)
                    }
                    "power" if args.len() == 2 => {
                        let exponent = match self.arena.node(args[1]) {
                            Node::Int(value) => value.to_i64()?,
                            _ => return None,
                        };
                        let base = self.expression_sign(args[0])?;
                        if exponent % 2 == 0 {
                            Some(match base {
                                Sign::Zero => Sign::Zero,
                                Sign::Negative | Sign::Positive => Sign::Positive,
                                Sign::NonNegative | Sign::NonPositive => Sign::NonNegative,
                            })
                        } else {
                            Some(base)
                        }
                    }
                    "plus" => {
                        let mut all_nonnegative = true;
                        let mut all_nonpositive = true;
                        for arg in args.iter() {
                            let sign = self.expression_sign(*arg)?;
                            all_nonnegative &=
                                matches!(sign, Sign::Positive | Sign::Zero | Sign::NonNegative);
                            all_nonpositive &=
                                matches!(sign, Sign::Negative | Sign::Zero | Sign::NonPositive);
                        }
                        if all_nonnegative {
                            Some(Sign::NonNegative)
                        } else if all_nonpositive {
                            Some(Sign::NonPositive)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        }
    }

    fn exact_constant_value(&self, expression: ExprId) -> Option<Rat> {
        match self.arena.node(expression) {
            Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            Node::Rat(value) => Some(value.clone()),
            _ => None,
        }
    }

    fn simplify_abs(&mut self, argument: ExprId) -> Option<ExprId> {
        match self.expression_sign(argument) {
            Some(Sign::Zero) => Some(self.arena.intern_int(Int::zero())),
            Some(Sign::Positive) | Some(Sign::NonNegative) => Some(argument),
            Some(Sign::Negative) | Some(Sign::NonPositive) => {
                Some(self.arena.intern_call(self.heads.negate, &[argument]))
            }
            None => None,
        }
    }

    fn simplify_sqrt(&mut self, argument: ExprId) -> Option<ExprId> {
        if let Node::Call { head, args } = self.arena.node(argument)
            && args.len() == 2
            && self
                .arena
                .symbol_id(*head)
                .map(|symbol| self.arena.symbol_name(symbol))
                == Some("power")
            && let Node::Int(exponent) = self.arena.node(args[1])
            && exponent.to_i64() == Some(2)
        {
            let base = args[0];
            return match self.expression_sign(base) {
                Some(Sign::NonNegative) | Some(Sign::Positive) | Some(Sign::Zero) => Some(base),
                Some(Sign::NonPositive) | Some(Sign::Negative) => {
                    Some(self.arena.intern_call(self.heads.negate, &[base]))
                }
                None => None,
            };
        }
        if let Node::Int(value) = self.arena.node(argument)
            && let Some(root) = integer_sqrt(value)
        {
            return Some(self.arena.intern_int(root));
        }
        None
    }

    fn finish(&mut self, original: ExprId, result: ExprId) -> Result<ExprId, Diagnostic> {
        if result == original {
            Ok(original)
        } else {
            self.eval(result)
        }
    }

    fn local_value(&self, symbol: SymbolId) -> Option<ExprId> {
        for frame in self.locals.iter().rev() {
            if let Some(value) = frame.get(&symbol) {
                return Some(*value);
            }
        }
        None
    }

    fn eval_function(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "fn expects parameters and a body",
            ));
        }
        self.parameter_list(args[0])?;
        let captured: Vec<(SymbolId, ExprId)> = self
            .locals
            .iter()
            .flat_map(|frame| frame.iter().map(|(symbol, value)| (*symbol, *value)))
            .collect();
        let mut bindings = Vec::with_capacity(captured.len());
        for (symbol, value) in captured {
            let name = self.arena.symbol_expr(symbol);
            bindings.push(self.arena.intern_call(self.heads.set, &[name, value]));
        }
        let environment = self.arena.intern_call(self.heads.list, &bindings);
        Ok(self
            .arena
            .intern_call(self.heads.closure, &[args[0], args[1], environment]))
    }

    fn apply_closure(
        &mut self,
        closure: &[ExprId],
        call_args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if closure.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "malformed closure value",
            ));
        }
        let params = self.parameter_list(closure[0])?;
        if params.len() != call_args.len() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "closure called with the wrong number of arguments",
            ));
        }
        let mut frame = HashMap::new();
        self.read_environment(closure[2], &mut frame)?;
        for (symbol, argument) in params.into_iter().zip(call_args.iter()) {
            let value = self.eval(*argument)?;
            frame.insert(symbol, value);
        }
        let outer = core::mem::take(&mut self.locals);
        self.locals.push(frame);
        let result = self.eval(closure[1]);
        self.locals = outer;
        result
    }

    fn parameter_list(&self, expression: ExprId) -> Result<Vec<SymbolId>, Diagnostic> {
        let args = match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => args.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be a list",
                ));
            }
        };
        let mut symbols = Vec::with_capacity(args.len());
        let mut seen = HashSet::new();
        for arg in args {
            let Some(symbol) = self.arena.symbol_id(arg) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be symbols",
                ));
            };
            if !seen.insert(symbol) {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be distinct",
                ));
            }
            symbols.push(symbol);
        }
        Ok(symbols)
    }

    fn read_environment(
        &self,
        expression: ExprId,
        frame: &mut HashMap<SymbolId, ExprId>,
    ) -> Result<(), Diagnostic> {
        let bindings = match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => args.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "closure environment must be a list",
                ));
            }
        };
        for binding in bindings {
            let (name, value) = match self.arena.node(binding) {
                Node::Call { head, args } if *head == self.heads.set && args.len() == 2 => {
                    (args[0], args[1])
                }
                _ => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "closure environment binding is malformed",
                    ));
                }
            };
            let Some(symbol) = self.arena.symbol_id(name) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "closure environment name must be a symbol",
                ));
            };
            frame.insert(symbol, value);
        }
        Ok(())
    }
}

pub(crate) fn fold_numeric_call(arena: &mut Arena, id: ExprId) -> Option<ExprId> {
    match arena.node(id) {
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => Some(id),
        Node::Call { head, args } => {
            let symbol = arena.symbol_id(*head)?;
            let name = arena.symbol_name(symbol);
            if name != "plus" && name != "times" && name != "negate" {
                return None;
            }
            let args = args.as_ref();
            if args.is_empty() {
                return None;
            }
            if name == "negate" {
                if args.len() != 1 {
                    return None;
                }
                return match arena.node(args[0]) {
                    Node::Int(value) => Some(arena.intern_int(value.neg())),
                    Node::Rat(value) => Some(arena.intern_rat(value.neg())),
                    _ => None,
                };
            }
            if args
                .iter()
                .all(|arg| matches!(arena.node(*arg), Node::Int(_)))
            {
                let mut value = if name == "plus" {
                    Int::zero()
                } else {
                    Int::one()
                };
                for arg in args {
                    let Node::Int(number) = arena.node(*arg) else {
                        return None;
                    };
                    value = if name == "plus" {
                        value.add(number)
                    } else {
                        value.mul(number)
                    };
                }
                return Some(arena.intern_int(value));
            }
            let mut value = if name == "plus" {
                Rat::zero()
            } else {
                Rat::one()
            };
            for arg in args {
                let number = match arena.node(*arg) {
                    Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok()?,
                    Node::Rat(value) => value.clone(),
                    _ => return None,
                };
                value = if name == "plus" {
                    value.add(&number)
                } else {
                    value.mul(&number)
                };
            }
            Some(arena.intern_rat(value))
        }
        _ => None,
    }
}
