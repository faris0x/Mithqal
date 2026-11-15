//! Fixed point evaluator.
//!
//! The current version implements exact arithmetic, listable threading, comparisons,
//! logic, and the boundary for definitions. Definitions and pattern matching
//! are added in the next layer.

mod definitions;
mod pattern;

use core::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
use crate::kernel::Heads;
use crate::num::{ComplexMpc, Int, MPFR_RNDD, MPFR_RNDU, Rat, Real};

use definitions::{Definitions, Rule};
use pattern::{Bindings, match_pattern, substitute};

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

    fn eval_interval(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "interval expects two endpoints",
            ));
        }
        let low = self.eval(args[0])?;
        let high = self.eval(args[1])?;
        if let (Some(low_value), Some(high_value)) =
            (self.real_endpoint(low), self.real_endpoint(high))
            && low_value.cmp(&high_value) == Ordering::Greater
        {
            return Ok(self.heads.failed);
        }
        Ok(self.arena.intern_call(self.heads.interval, &[low, high]))
    }

    fn eval_complex(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "complex expects a real and an imaginary part",
            ));
        }
        let real = self.eval(args[0])?;
        let imaginary = self.eval(args[1])?;
        if let Some(value) = self.real_endpoint(imaginary)
            && value.is_zero()
        {
            return Ok(real);
        }
        Ok(self
            .arena
            .intern_call(self.heads.complex, &[real, imaginary]))
    }

    fn real_endpoint(&self, id: ExprId) -> Option<Real> {
        match self.arena.node(id) {
            Node::Int(value) => {
                if let Some(value) = value.to_i64() {
                    Real::from_i64(value, 30).ok()
                } else {
                    let rational = Rat::from_ints(value.clone(), Int::one()).ok()?;
                    Real::from_rat(&rational, 30).ok()
                }
            }
            Node::Rat(value) => Real::from_rat(value, 30).ok(),
            Node::Real(value) => Some(value.clone()),
            _ => None,
        }
    }

    fn interval_parts(&self, id: ExprId) -> Option<(ExprId, ExprId)> {
        match self.arena.node(id) {
            Node::Call { head, args } if *head == self.heads.interval && args.len() == 2 => {
                Some((args[0], args[1]))
            }
            _ => None,
        }
    }

    fn interval_bounds(&self, id: ExprId) -> Option<(Real, Real)> {
        let (low, high) = self.interval_parts(id)?;
        let low = self.real_endpoint(low)?.lower_bound().ok()?;
        let high = self.real_endpoint(high)?.upper_bound().ok()?;
        Some((low, high))
    }

    fn interval_node(&mut self, low: Real, high: Real) -> ExprId {
        let (low, high) = if low.cmp(&high) == Ordering::Greater {
            (high, low)
        } else {
            (low, high)
        };
        let low = self.arena.intern_real(low);
        let high = self.arena.intern_real(high);
        self.arena.intern_call(self.heads.interval, &[low, high])
    }

    fn add_interval(
        &mut self,
        low: &Real,
        high: &Real,
        other_low: &Real,
        other_high: &Real,
    ) -> Result<(Real, Real), Diagnostic> {
        let low = low
            .add_round(other_low, MPFR_RNDD)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "interval addition failed"))?;
        let high = high
            .add_round(other_high, MPFR_RNDU)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "interval addition failed"))?;
        Ok((low, high))
    }

    fn mul_interval(
        &mut self,
        low: &Real,
        high: &Real,
        other_low: &Real,
        other_high: &Real,
    ) -> Result<(Real, Real), Diagnostic> {
        let mut lower_bound: Option<Real> = None;
        let mut upper_bound: Option<Real> = None;
        for (left, right) in [
            (low, other_low),
            (low, other_high),
            (high, other_low),
            (high, other_high),
        ] {
            let lower = left.mul_round(right, MPFR_RNDD).map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval product failed")
            })?;
            let upper = left.mul_round(right, MPFR_RNDU).map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval product failed")
            })?;
            let replace_low = match &lower_bound {
                Some(current) => lower.cmp(current) == Ordering::Less,
                None => true,
            };
            if replace_low {
                lower_bound = Some(lower);
            }
            let replace_high = match &upper_bound {
                Some(current) => upper.cmp(current) == Ordering::Greater,
                None => true,
            };
            if replace_high {
                upper_bound = Some(upper);
            }
        }
        Ok((
            lower_bound.expect("interval product has a lower bound"),
            upper_bound.expect("interval product has an upper bound"),
        ))
    }

    fn plus_intervals(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut scalars = Vec::new();
        let mut intervals = Vec::new();
        for arg in args {
            if let Some(bounds) = self.interval_bounds(*arg) {
                intervals.push(bounds);
            } else {
                scalars.push(*arg);
            }
        }
        let scalar = if scalars.is_empty() {
            Real::zero(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?
        } else {
            let sum = self.plus(&scalars)?;
            let Some(value) = self.real_endpoint(sum) else {
                return Ok(self.arena.intern_call(self.heads.plus, args));
            };
            value
        };
        let mut low = scalar.clone();
        let mut high = scalar;
        for (other_low, other_high) in intervals {
            let (next_low, next_high) = self.add_interval(&low, &high, &other_low, &other_high)?;
            low = next_low;
            high = next_high;
        }
        Ok(self.interval_node(low, high))
    }

    fn times_intervals(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut scalars = Vec::new();
        let mut intervals = Vec::new();
        for arg in args {
            if let Some(bounds) = self.interval_bounds(*arg) {
                intervals.push(bounds);
            } else {
                scalars.push(*arg);
            }
        }
        let scalar = if scalars.is_empty() {
            Real::one(30).map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?
        } else {
            let product = self.times(&scalars)?;
            let Some(value) = self.real_endpoint(product) else {
                return Ok(self.arena.intern_call(self.heads.times, args));
            };
            value
        };
        let mut low = scalar.clone();
        let mut high = scalar;
        for (other_low, other_high) in intervals {
            let (next_low, next_high) = self.mul_interval(&low, &high, &other_low, &other_high)?;
            low = next_low;
            high = next_high;
        }
        Ok(self.interval_node(low, high))
    }

    fn interval_power(&mut self, low: &Real, high: &Real, exponent: i64) -> Option<ExprId> {
        if exponent == 0 {
            let one = Real::one(30).ok()?;
            return Some(self.interval_node(one.clone(), one));
        }
        if exponent < 0 {
            if low.sign() <= 0 && high.sign() >= 0 {
                return None;
            }
            let positive = exponent.unsigned_abs() as i64;
            let (low_power, high_power) = self.interval_positive_power(low, high, positive)?;
            let one = Real::one(30).ok()?;
            let low_result = one.div_round(&high_power, MPFR_RNDD).ok()?;
            let high_result = one.div_round(&low_power, MPFR_RNDU).ok()?;
            return Some(self.interval_node(low_result, high_result));
        }
        let (low_power, high_power) = self.interval_positive_power(low, high, exponent)?;
        Some(self.interval_node(low_power, high_power))
    }

    fn interval_positive_power(
        &mut self,
        low: &Real,
        high: &Real,
        exponent: i64,
    ) -> Option<(Real, Real)> {
        let low_power = low.pow_si_round(exponent, MPFR_RNDD).ok()?;
        let high_power = high.pow_si_round(exponent, MPFR_RNDU).ok()?;
        if exponent % 2 == 0 && low.sign() < 0 && high.sign() > 0 {
            let zero = Real::zero(30).ok()?;
            let low_abs = low.abs().ok()?;
            let high_abs = high.abs().ok()?;
            let high_magnitude = if low_abs.cmp(&high_abs) == Ordering::Greater {
                low_abs
            } else {
                high_abs
            };
            let upper = high_magnitude.pow_si_round(exponent, MPFR_RNDU).ok()?;
            return Some((zero, upper));
        }
        if exponent % 2 == 0 && high.sign() <= 0 {
            let upper = low.pow_si_round(exponent, MPFR_RNDU).ok()?;
            let lower = high.pow_si_round(exponent, MPFR_RNDD).ok()?;
            return Some((lower, upper));
        }
        Some((low_power, high_power))
    }

    fn interval_unary(&mut self, head: ExprId, low: &Real, high: &Real) -> Option<ExprId> {
        let (low_value, high_value) = if head == self.heads.sqrt {
            if high.sign() < 0 {
                return None;
            }
            let zero = Real::zero(30).ok()?;
            let low = if low.sign() < 0 { &zero } else { low };
            (
                low.sqrt_round(MPFR_RNDD).ok()?,
                high.sqrt_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.exp {
            (
                low.exp_round(MPFR_RNDD).ok()?,
                high.exp_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.ln {
            if low.sign() <= 0 {
                return None;
            }
            (
                low.ln_round(MPFR_RNDD).ok()?,
                high.ln_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.abs {
            if low.sign() >= 0 {
                (low.clone(), high.clone())
            } else if high.sign() <= 0 {
                (high.neg().ok()?, low.neg().ok()?)
            } else {
                let low_magnitude = low.neg().ok()?;
                let high_magnitude = if low_magnitude.cmp(high) == Ordering::Greater {
                    low_magnitude
                } else {
                    high.clone()
                };
                (Real::zero(30).ok()?, high_magnitude)
            }
        } else if head == self.heads.sin || head == self.heads.cos {
            let minus_one = Real::from_i64(-1, 30).ok()?;
            let one = Real::one(30).ok()?;
            self.interval_trig(low, high, head == self.heads.sin, &minus_one, &one)?
        } else {
            return None;
        };
        if !low_value.is_finite() || !high_value.is_finite() {
            return None;
        }
        Some(self.interval_node(low_value, high_value))
    }

    fn interval_trig(
        &self,
        low: &Real,
        high: &Real,
        sine: bool,
        minus_one: &Real,
        one: &Real,
    ) -> Option<(Real, Real)> {
        let low_value = low.to_f64();
        let high_value = high.to_f64();
        if !low_value.is_finite() || !high_value.is_finite() {
            return None;
        }
        let magnitude = low_value.abs().max(high_value.abs());
        let epsilon = 1e-12 * magnitude.max(1.0);
        let two_pi = 2.0 * core::f64::consts::PI;
        if high_value - low_value >= two_pi - epsilon || magnitude > 1e15 {
            return Some((minus_one.clone(), one.clone()));
        }
        let (low_lower, low_upper) = if sine {
            (
                low.sin_round(MPFR_RNDD).ok()?,
                low.sin_round(MPFR_RNDU).ok()?,
            )
        } else {
            (
                low.cos_round(MPFR_RNDD).ok()?,
                low.cos_round(MPFR_RNDU).ok()?,
            )
        };
        let (high_lower, high_upper) = if sine {
            (
                high.sin_round(MPFR_RNDD).ok()?,
                high.sin_round(MPFR_RNDU).ok()?,
            )
        } else {
            (
                high.cos_round(MPFR_RNDD).ok()?,
                high.cos_round(MPFR_RNDU).ok()?,
            )
        };
        let mut lower = if low_lower.cmp(&high_lower) == Ordering::Less {
            low_lower
        } else {
            high_lower
        };
        let mut upper = if low_upper.cmp(&high_upper) == Ordering::Greater {
            low_upper
        } else {
            high_upper
        };
        let lower_phase = if sine {
            -core::f64::consts::FRAC_PI_2
        } else {
            core::f64::consts::PI
        };
        let upper_phase = if sine {
            core::f64::consts::FRAC_PI_2
        } else {
            0.0
        };
        if interval_contains_phase(
            low_value - epsilon,
            high_value + epsilon,
            lower_phase,
            two_pi,
        ) {
            lower = minus_one.clone();
        }
        if interval_contains_phase(
            low_value - epsilon,
            high_value + epsilon,
            upper_phase,
            two_pi,
        ) {
            upper = one.clone();
        }
        if lower.cmp(minus_one) == Ordering::Less {
            lower = minus_one.clone();
        }
        if upper.cmp(one) == Ordering::Greater {
            upper = one.clone();
        }
        Some((lower, upper))
    }

    fn eval_approx(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "approx expects an expression and an optional digit count",
            ));
        }
        let digits = if args.len() == 2 {
            let value = self.eval(args[1])?;
            match self.arena.node(value) {
                Node::Int(value) => value.to_i64().unwrap_or(0),
                _ => 0,
            }
        } else {
            16
        };
        if digits <= 0 || digits > 10_000 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "approx digit count must be between 1 and 10000",
            ));
        }
        let expression = self.eval(args[0])?;
        if let Some(value) = self.approximate(expression, digits as u32)? {
            return Ok(self.numeric_expr(value));
        }
        Ok(self.arena.intern_call(self.heads.approx, args))
    }

    fn approximate(
        &mut self,
        expression: ExprId,
        digits: u32,
    ) -> Result<Option<Numeric>, Diagnostic> {
        match self.arena.node(expression) {
            Node::Int(value) => {
                if let Some(value) = value.to_i64() {
                    let real = Real::from_i64(value, digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "real conversion failed")
                    })?;
                    return Ok(Some(Numeric::Real(real)));
                }
                let rational = Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?;
                self.approximate_rat(rational, digits)
            }
            Node::Rat(value) => self.approximate_rat(value.clone(), digits),
            Node::Real(value) => Ok(Some(Numeric::Real(value.clone()))),
            Node::Symbol(_) => {
                if expression == self.heads.pi {
                    return Ok(Some(Numeric::Real(Real::pi(digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "pi failed")
                    })?)));
                }
                if expression == self.heads.e {
                    return Ok(Some(Numeric::Real(Real::e(digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "e failed")
                    })?)));
                }
                if expression == self.heads.imaginary {
                    let zero = Real::zero(digits)
                        .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?;
                    let one = Real::one(digits)
                        .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?;
                    return Ok(Some(Numeric::Complex(zero, one)));
                }
                if self.is_error(expression) {
                    return Ok(Some(Numeric::Error(expression)));
                }
                Ok(None)
            }
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                let function = self
                    .arena
                    .symbol_id(head)
                    .and_then(|symbol| NumericHead::from_name(self.arena.symbol_name(symbol)));
                match function {
                    Some(NumericHead::Plus) => {
                        let mut value = Numeric::Real(Real::zero(digits).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "real failed")
                        })?);
                        for arg in args {
                            let Some(argument) = self.approximate(arg, digits)? else {
                                return Ok(None);
                            };
                            let Some(next) = numeric_add(&value, &argument) else {
                                return Ok(None);
                            };
                            value = next;
                        }
                        Ok(Some(value))
                    }
                    Some(NumericHead::Times) => {
                        let mut value = Numeric::Real(Real::one(digits).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "real failed")
                        })?);
                        for arg in args {
                            let Some(argument) = self.approximate(arg, digits)? else {
                                return Ok(None);
                            };
                            let Some(next) = numeric_mul(&value, &argument) else {
                                return Ok(None);
                            };
                            value = next;
                        }
                        Ok(Some(value))
                    }
                    Some(NumericHead::Negate) if args.len() == 1 => {
                        let Some(argument) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        Ok(numeric_neg(&argument))
                    }
                    Some(NumericHead::Power) if args.len() == 2 => {
                        let exponent = match self.arena.node(args[1]) {
                            Node::Int(value) => value.to_i64(),
                            Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
                            _ => None,
                        };
                        let Some(base) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        if let Some(exponent) = exponent {
                            return Ok(numeric_pow(&base, exponent, digits));
                        }
                        let Some(exponent) = self.approximate(args[1], digits)? else {
                            return Ok(None);
                        };
                        match (base, exponent) {
                            (Numeric::Real(base), Numeric::Real(exponent)) => {
                                match base.pow(&exponent) {
                                    Ok(value) if value.is_finite() => {
                                        Ok(Some(Numeric::Real(value)))
                                    }
                                    _ => {
                                        let Some(zero) = Real::zero(digits).ok() else {
                                            return Ok(None);
                                        };
                                        Ok(numeric_complex_pow(
                                            &(base, zero.clone()),
                                            &(exponent, zero),
                                        ))
                                    }
                                }
                            }
                            (
                                Numeric::Complex(real, imaginary),
                                Numeric::Complex(exponent_real, exponent_imaginary),
                            ) => Ok(numeric_complex_pow(
                                &(real, imaginary),
                                &(exponent_real, exponent_imaginary),
                            )),
                            (
                                Numeric::Real(real),
                                Numeric::Complex(exponent_real, exponent_imaginary),
                            ) => {
                                let Some(zero) = Real::zero(digits).ok() else {
                                    return Ok(None);
                                };
                                Ok(numeric_complex_pow(
                                    &(real, zero),
                                    &(exponent_real, exponent_imaginary),
                                ))
                            }
                            (Numeric::Complex(real, imaginary), Numeric::Real(exponent)) => {
                                let Some(zero) = Real::zero(digits).ok() else {
                                    return Ok(None);
                                };
                                Ok(numeric_complex_pow(&(real, imaginary), &(exponent, zero)))
                            }
                            (Numeric::Error(value), _) | (_, Numeric::Error(value)) => {
                                Ok(Some(Numeric::Error(value)))
                            }
                        }
                    }
                    Some(NumericHead::Complex) if args.len() == 2 => {
                        let Some(real) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        let Some(imaginary) = self.approximate(args[1], digits)? else {
                            return Ok(None);
                        };
                        match (real, imaginary) {
                            (Numeric::Real(real), Numeric::Real(imaginary)) => {
                                Ok(Some(Numeric::Complex(real, imaginary)))
                            }
                            (Numeric::Error(value), _) | (_, Numeric::Error(value)) => {
                                Ok(Some(Numeric::Error(value)))
                            }
                            _ => Ok(None),
                        }
                    }
                    Some(function) if args.len() == 1 && function.is_unary() => {
                        self.approximate_named_unary(args[0], digits, function)
                    }
                    Some(NumericHead::Abs) if args.len() == 1 => {
                        let Some(argument) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        Ok(numeric_abs(&argument))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    fn complex_unary(real: &Real, imaginary: &Real, function: NumericHead) -> Option<(Real, Real)> {
        if matches!(function, NumericHead::Atanh) {
            return Self::complex_atanh(real, imaginary);
        }
        let complex = ComplexMpc::from_parts(real, imaginary)?;
        match function {
            NumericHead::Sqrt => complex.sqrt(),
            NumericHead::Exp => complex.exp(),
            NumericHead::Ln => complex.log(),
            NumericHead::Sin => complex.sin(),
            NumericHead::Cos => complex.cos(),
            NumericHead::Tan => complex.tan(),
            NumericHead::Asin => complex.asin(),
            NumericHead::Acos => complex.acos(),
            NumericHead::Atan => complex.atan(),
            NumericHead::Sinh => complex.sinh(),
            NumericHead::Cosh => complex.cosh(),
            NumericHead::Tanh => complex.tanh(),
            NumericHead::Asinh => complex.asinh(),
            NumericHead::Acosh => complex.acosh(),
            NumericHead::Atanh => None,
            _ => None,
        }
    }

    fn complex_atanh(real: &Real, imaginary: &Real) -> Option<(Real, Real)> {
        let bits = real.bits.max(imaginary.bits);
        let one = Real::one_bits(bits);
        let plus_real = one.add(real).ok()?;
        let plus = ComplexMpc::from_parts(&plus_real, imaginary)?.log()?;
        let minus_real = one.sub(real).ok()?;
        let minus_imaginary = if imaginary.is_zero() {
            Real::from_i64_bits(0, bits).ok()?
        } else {
            imaginary.neg().ok()?
        };
        let minus = ComplexMpc::from_parts(&minus_real, &minus_imaginary)?.log()?;
        let real_difference = plus.0.sub(&minus.0).ok()?;
        let imaginary_difference = plus.1.sub(&minus.1).ok()?;
        let two = Real::from_i64_bits(2, bits).ok()?;
        Some((
            real_difference.div(&two).ok()?,
            imaginary_difference.div(&two).ok()?,
        ))
    }

    fn approximate_named_unary(
        &mut self,
        argument: ExprId,
        digits: u32,
        function: NumericHead,
    ) -> Result<Option<Numeric>, Diagnostic> {
        let Some(argument) = self.approximate(argument, digits)? else {
            return Ok(None);
        };
        match argument {
            Numeric::Real(value) => {
                let result = match function {
                    NumericHead::Sqrt => value.sqrt(),
                    NumericHead::Exp => value.exp(),
                    NumericHead::Ln => value.ln(),
                    NumericHead::Sin => value.sin(),
                    NumericHead::Cos => value.cos(),
                    NumericHead::Tan => value.tan(),
                    NumericHead::Asin => value.asin(),
                    NumericHead::Acos => value.acos(),
                    NumericHead::Atan => value.atan(),
                    NumericHead::Sinh => value.sinh(),
                    NumericHead::Cosh => value.cosh(),
                    NumericHead::Tanh => value.tanh(),
                    NumericHead::Asinh => value.asinh(),
                    NumericHead::Acosh => value.acosh(),
                    NumericHead::Atanh => value.atanh(),
                    NumericHead::Floor => value.floor(),
                    NumericHead::Ceil => value.ceil(),
                    NumericHead::Round => value.round(),
                    NumericHead::Gamma => value.gamma(),
                    NumericHead::Lgamma => value.lgamma(),
                    NumericHead::Erf => value.erf(),
                    NumericHead::Erfc => value.erfc(),
                    _ => return Ok(None),
                };
                if let Ok(real) = result
                    && real.is_finite()
                {
                    return Ok(Some(Numeric::Real(real)));
                }
                let zero = if matches!(function, NumericHead::Asin | NumericHead::Acos) {
                    let Some(one) = Real::one(digits).ok() else {
                        return Ok(None);
                    };
                    if value.cmp(&one) == Ordering::Greater {
                        Real::zero(digits).ok().and_then(|zero| zero.neg().ok())
                    } else {
                        Real::zero(digits).ok()
                    }
                } else {
                    Real::zero(digits).ok()
                };
                let Some(zero) = zero else {
                    return Ok(None);
                };
                Ok(Self::complex_unary(&value, &zero, function)
                    .map(|(real, imaginary)| Numeric::Complex(real, imaginary)))
            }
            Numeric::Error(value) => Ok(Some(Numeric::Error(value))),
            Numeric::Complex(real, imaginary) => {
                Ok(Self::complex_unary(&real, &imaginary, function)
                    .map(|(real, imaginary)| Numeric::Complex(real, imaginary)))
            }
        }
    }

    fn approximate_rat(&mut self, value: Rat, digits: u32) -> Result<Option<Numeric>, Diagnostic> {
        let real = Real::from_rat(&value, digits)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real conversion failed"))?;
        Ok(Some(Numeric::Real(real)))
    }

    fn numeric_expr(&mut self, value: Numeric) -> ExprId {
        match value {
            Numeric::Real(value) => {
                if value.is_finite() {
                    self.arena.intern_real(value)
                } else {
                    self.heads.undefined
                }
            }
            Numeric::Error(value) => value,
            Numeric::Complex(real, imaginary) => {
                if !real.is_finite() || !imaginary.is_finite() {
                    return self.heads.undefined;
                }
                if imaginary.is_zero() {
                    self.arena.intern_real(real)
                } else {
                    let real = self.arena.intern_real(real);
                    let imaginary = self.arena.intern_real(imaginary);
                    self.arena
                        .intern_call(self.heads.complex, &[real, imaginary])
                }
            }
        }
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

    fn eval_definition(&mut self, args: &[ExprId], delayed: bool) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "a definition expects two arguments",
            ));
        }
        let rhs = if delayed {
            args[1]
        } else {
            self.eval(args[1])?
        };
        {
            let arena = &*self.arena;
            self.definitions.define(arena, args[0], rhs, delayed)?;
        }
        Ok(if delayed { self.heads.null } else { rhs })
    }

    fn eval_let(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "let expects two arguments",
            ));
        }
        let bindings = match self.arena.node(args[0]) {
            Node::Call {
                head,
                args: bindings,
            } if *head == self.heads.list => bindings.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "let bindings must be a list",
                ));
            }
        };
        self.locals.push(HashMap::new());
        let result = (|| {
            for binding in bindings {
                let (lhs, rhs) = match self.arena.node(binding) {
                    Node::Call { head, args } if *head == self.heads.set && args.len() == 2 => {
                        (args[0], args[1])
                    }
                    _ => {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalDefinition,
                            "each let binding must use the form name=value",
                        ));
                    }
                };
                let Some(symbol) = self.arena.symbol_id(lhs) else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a let binding name must be a symbol",
                    ));
                };
                let value = self.eval(rhs)?;
                self.locals
                    .last_mut()
                    .expect("let frame exists")
                    .insert(symbol, value);
            }
            self.eval(args[1])
        })();
        self.locals.pop();
        result
    }

    fn eval_if(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "if expects three arguments",
            ));
        }
        let condition = self.eval(args[0])?;
        if condition == self.heads.true_value {
            return self.eval(args[1]);
        }
        if condition == self.heads.false_value {
            return self.eval(args[2]);
        }
        if self.is_error(condition) {
            return Ok(condition);
        }
        Ok(self
            .arena
            .intern_call(self.heads.if_head, &[condition, args[1], args[2]]))
    }

    fn eval_and(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut evaluated = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.eval(*arg)?;
            if value == self.heads.false_value {
                return Ok(self.heads.false_value);
            }
            if self.is_error(value) {
                return Ok(value);
            }
            evaluated.push(value);
        }
        evaluated.retain(|value| *value != self.heads.true_value);
        match evaluated.len() {
            0 => Ok(self.heads.true_value),
            1 => Ok(evaluated[0]),
            _ => Ok(self.arena.intern_call(self.heads.and, &evaluated)),
        }
    }

    fn eval_or(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut evaluated = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.eval(*arg)?;
            if value == self.heads.true_value {
                return Ok(self.heads.true_value);
            }
            if self.is_error(value) {
                return Ok(value);
            }
            evaluated.push(value);
        }
        evaluated.retain(|value| *value != self.heads.false_value);
        match evaluated.len() {
            0 => Ok(self.heads.false_value),
            1 => Ok(evaluated[0]),
            _ => Ok(self.arena.intern_call(self.heads.or, &evaluated)),
        }
    }

    fn eval_not(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "not expects one argument",
            ));
        }
        let value = self.eval(args[0])?;
        if value == self.heads.true_value {
            return Ok(self.heads.false_value);
        }
        if value == self.heads.false_value {
            return Ok(self.heads.true_value);
        }
        if self.is_error(value) {
            return Ok(value);
        }
        Ok(self.arena.intern_call(self.heads.not, &[value]))
    }

    fn thread_listable(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let mut length: Option<usize> = None;
        for arg in args {
            let Some(items) = self.list_items(*arg) else {
                continue;
            };
            match length {
                None => length = Some(items.len()),
                Some(existing) if existing == items.len() => {}
                Some(_) => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "listable arguments have different lengths",
                    ));
                }
            }
        }
        let Some(length) = length else {
            return Ok(None);
        };
        let mut results = Vec::with_capacity(length);
        for index in 0..length {
            let mut call_args = Vec::with_capacity(args.len());
            for arg in args {
                let value = self.list_items(*arg).map_or(*arg, |items| items[index]);
                call_args.push(value);
            }
            let call = self.arena.intern_call(head, &call_args);
            results.push(self.eval(call)?);
        }
        Ok(Some(self.arena.intern_call(self.heads.list, &results)))
    }

    fn list_items(&self, expression: ExprId) -> Option<&[ExprId]> {
        match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => Some(args),
            _ => None,
        }
    }

    fn apply(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if head == self.heads.plus {
            return self.plus(args);
        }
        if head == self.heads.times {
            return self.times(args);
        }
        if head == self.heads.power {
            return self.power(args);
        }
        if head == self.heads.negate {
            return self.negate(args);
        }
        if head == self.heads.list {
            return Ok(self.arena.intern_call(self.heads.list, args));
        }
        if head == self.heads.same {
            return self.same(args);
        }
        if head == self.heads.equal {
            return self.equal(args);
        }
        if head == self.heads.not_equal {
            return self.not_equal(args);
        }
        if head == self.heads.less {
            return self.ordering(args, Ordering::Less, self.heads.less);
        }
        if head == self.heads.less_equal {
            return self.ordering(args, Ordering::Less, self.heads.less_equal);
        }
        if head == self.heads.greater {
            return self.ordering(args, Ordering::Greater, self.heads.greater);
        }
        if head == self.heads.greater_equal {
            return self.ordering(args, Ordering::Greater, self.heads.greater_equal);
        }
        if args.len() == 1 {
            let interval_unary = head == self.heads.abs
                || head == self.heads.sqrt
                || head == self.heads.exp
                || head == self.heads.ln
                || head == self.heads.sin
                || head == self.heads.cos;
            if interval_unary
                && let Some((low, high)) = self.interval_bounds(args[0])
                && let Some(result) = self.interval_unary(head, &low, &high)
            {
                return Ok(result);
            }
            if head == self.heads.abs
                && let Some(result) = self.simplify_abs(args[0])
            {
                return Ok(result);
            }
            if head == self.heads.sqrt
                && let Some(result) = self.simplify_sqrt(args[0])
            {
                return Ok(result);
            }
            if let Some(value) = self.exact_constant_value(args[0]) {
                if head == self.heads.exp && value.is_zero() {
                    return Ok(self.arena.intern_int(Int::one()));
                }
                if head == self.heads.ln && value.is_one() {
                    return Ok(self.arena.intern_int(Int::zero()));
                }
                if (head == self.heads.sin || head == self.heads.tan || head == self.heads.atan)
                    && value.is_zero()
                {
                    return Ok(self.arena.intern_int(Int::zero()));
                }
                if head == self.heads.cos && value.is_zero() {
                    return Ok(self.arena.intern_int(Int::one()));
                }
                if head == self.heads.atan
                    && value.is_one()
                    && let Some(angle) = self.pi_fraction(1, 4)
                {
                    return Ok(angle);
                }
                if head == self.heads.asin {
                    if value.is_zero() {
                        return Ok(self.arena.intern_int(Int::zero()));
                    }
                    if value.is_one()
                        && let Some(angle) = self.pi_fraction(1, 2)
                    {
                        return Ok(angle);
                    }
                    if value.neg().is_one()
                        && let Some(angle) = self.pi_fraction(-1, 2)
                    {
                        return Ok(angle);
                    }
                }
                if head == self.heads.acos {
                    if value.is_one() {
                        return Ok(self.arena.intern_int(Int::zero()));
                    }
                    if value.is_zero()
                        && let Some(angle) = self.pi_fraction(1, 2)
                    {
                        return Ok(angle);
                    }
                    if value.neg().is_one()
                        && let Some(angle) = self.pi_fraction(1, 1)
                    {
                        return Ok(angle);
                    }
                }
            }
        }
        let kernel = self
            .arena
            .symbol_id(head)
            .and_then(|symbol| KernelFunction::from_name(self.arena.symbol_name(symbol)));
        if let Some(kernel) = kernel {
            return self.apply_kernel(kernel, head, args);
        }
        if let Some(symbol) = self.arena.symbol_id(head)
            && let Some(result) = self.apply_user(symbol, head, args)?
        {
            return Ok(result);
        }
        Ok(self.arena.intern_call(head, args))
    }

    fn apply_kernel(
        &mut self,
        function: KernelFunction,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        match function {
            KernelFunction::Algebra(function) => self.apply_algebra(function, head, args),
            KernelFunction::Calculus(function) => self.apply_calculus(function, head, args),
            KernelFunction::Numeric(function) => {
                let original = self.arena.intern_call(head, args);
                Ok(self
                    .apply_exact_numeric(function, args)?
                    .unwrap_or(original))
            }
            KernelFunction::Linear(function) => {
                if args.len() != function.arity() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "linear algebra function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_linear(function, args).unwrap_or(original))
            }
            KernelFunction::NumberTheory(function) => {
                if args.len() != function.arity() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "number theory function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_number_theory(function, args).unwrap_or(original))
            }
            KernelFunction::Logic(function) => {
                let arity = match function {
                    LogicHead::Xor | LogicHead::Implies => 2,
                    LogicHead::All | LogicHead::Any => 1,
                };
                if args.len() != arity {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "logic function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_logic(function, args).unwrap_or(original))
            }
            KernelFunction::Statistics(function) => {
                if args.len() != 1 {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "statistics function expects one list",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_statistics(function, args[0]).unwrap_or(original))
            }
        }
    }

    fn apply_algebra(
        &mut self,
        function: AlgebraHead,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let context = crate::algebra::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        };
        match function {
            AlgebraHead::Expand => self.algebra_unary(args, head, context, crate::algebra::expand),
            AlgebraHead::Factor => self.algebra_unary(args, head, context, crate::algebra::factor),
            AlgebraHead::Together => {
                self.algebra_unary(args, head, context, crate::algebra::together)
            }
            AlgebraHead::Cancel => self.algebra_unary(args, head, context, crate::algebra::cancel),
            AlgebraHead::PolynomialGcd => {
                require_arity(
                    args,
                    3,
                    "polynomialGcd expects two polynomials and a variable",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::polynomial_gcd(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Resultant => {
                require_arity(args, 3, "resultant expects two polynomials and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::resultant(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Solve => {
                require_arity(args, 2, "solve expects an equation and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::solve(self.arena, args[0], args[1], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Degree => {
                require_arity(args, 2, "degree expects a polynomial and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::degree(self.arena, args[0], args[1], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Coefficient => {
                require_arity(
                    args,
                    3,
                    "coefficient expects a polynomial, a variable, and an index",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::coefficient(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::PolynomialDivide => {
                require_arity(
                    args,
                    3,
                    "polynomialDivide expects a dividend, a divisor, and a variable",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(crate::algebra::polynomial_divide(
                    self.arena, args[0], args[1], args[2], &context,
                )
                .unwrap_or(original))
            }
            AlgebraHead::Discriminant => {
                require_arity(args, 2, "discriminant expects a polynomial and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::discriminant(self.arena, args[0], args[1], &context)
                        .unwrap_or(original),
                )
            }
        }
    }

    fn apply_calculus(
        &mut self,
        function: CalculusHead,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let valid = matches!(
            (function, args.len()),
            (CalculusHead::Differentiate, 2)
                | (CalculusHead::Integrate, 2 | 4)
                | (CalculusHead::Limit, 3 | 4)
                | (CalculusHead::Series, 4)
                | (CalculusHead::Converges, 3)
        );
        if !valid {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "calculus function received the wrong number of arguments",
            ));
        }
        let original = self.arena.intern_call(head, args);
        if self.arena.symbol_id(args[1]).is_none() {
            return Ok(original);
        }
        if matches!(function, CalculusHead::Limit) {
            return self.apply_limit(head, args);
        }
        if matches!(function, CalculusHead::Series) {
            return self.apply_series(head, args);
        }
        if matches!(function, CalculusHead::Converges) {
            return self.apply_converges(head, args);
        }
        if matches!(function, CalculusHead::Integrate) && args.len() == 4 {
            return Ok(self
                .definite_integral(args[0], args[1], args[2], args[3])
                .unwrap_or(original));
        }
        let context = crate::calculus::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        };
        let result = match function {
            CalculusHead::Differentiate => {
                crate::calculus::differentiate(self.arena, args[0], args[1], &context)
            }
            CalculusHead::Integrate if args.len() == 4 => None,
            CalculusHead::Integrate => {
                crate::calculus::integrate(self.arena, args[0], args[1], &context)
            }
            CalculusHead::Limit | CalculusHead::Series | CalculusHead::Converges => None,
        };
        let result = result.map(|value| {
            if matches!(function, CalculusHead::Differentiate) {
                let algebra = crate::algebra::Context {
                    plus: self.heads.plus,
                    times: self.heads.times,
                    power: self.heads.power,
                    negate: self.heads.negate,
                    list: self.heads.list,
                    equal: self.heads.equal,
                };
                crate::algebra::cancel(self.arena, value, &algebra).unwrap_or(value)
            } else {
                value
            }
        });
        Ok(result.unwrap_or(original))
    }

    fn definite_integral(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> Option<ExprId> {
        if self.interval_has_interior_pole(expression, variable, lower, upper) {
            return None;
        }
        let context = self.calculus_context();
        let antiderivative =
            crate::calculus::integrate(self.arena, expression, variable, &context)?;
        if self.has_transcendental_singularity(antiderivative, variable, lower, upper) {
            return None;
        }
        let lower_value = self.endpoint_limit(antiderivative, variable, lower, LimitSide::Right)?;
        let upper_value = self.endpoint_limit(antiderivative, variable, upper, LimitSide::Left)?;
        let negative_lower = self.negate_limit_value(lower_value);
        match self.add_limit_values(upper_value, negative_lower)? {
            LimitValue::Finite(value) => Some(value),
            _ => None,
        }
    }

    fn has_transcendental_singularity(
        &mut self,
        antiderivative: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> bool {
        let (Some(lower), Some(upper)) = (self.as_rat(lower), self.as_rat(upper)) else {
            return false;
        };
        let (lower, upper) = if lower <= upper {
            (lower, upper)
        } else {
            (upper, lower)
        };
        let mut arguments = Vec::new();
        self.collect_singularity_arguments(antiderivative, variable, &mut arguments);
        arguments
            .into_iter()
            .any(|argument| self.argument_has_interior_zero(argument, variable, &lower, &upper))
    }

    fn collect_singularity_arguments(
        &self,
        expression: ExprId,
        variable: ExprId,
        arguments: &mut Vec<ExprId>,
    ) {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return;
        };
        let name = self
            .arena
            .symbol_id(*head)
            .map(|symbol| self.arena.symbol_name(symbol));
        if name == Some("ln") && args.len() == 1 && self.expression_contains(args[0], variable) {
            let argument = self.abs_argument(args[0]).unwrap_or(args[0]);
            arguments.push(argument);
        }
        if name == Some("power")
            && args.len() == 2
            && self.expression_contains(args[0], variable)
            && self
                .exact_constant_value(args[1])
                .is_some_and(|exponent| exponent.sign() < 0)
        {
            arguments.push(args[0]);
        }
        for argument in args.iter() {
            self.collect_singularity_arguments(*argument, variable, arguments);
        }
    }

    fn argument_has_interior_zero(
        &mut self,
        argument: ExprId,
        variable: ExprId,
        lower: &Rat,
        upper: &Rat,
    ) -> bool {
        let context = self.algebra_context();
        if let Some(roots) = crate::algebra::univariate_rational_root_multiplicities(
            self.arena, argument, variable, &context,
        ) && roots
            .iter()
            .any(|(root, _)| *root > *lower && *root < *upper)
        {
            return true;
        }
        let samples = 16i64;
        let denominator = Rat::from_i64(samples + 1, 1).ok();
        let span = upper.sub(lower);
        let Some(denominator) = denominator else {
            return false;
        };
        let mut previous_sign = 0i8;
        for index in 1..=samples {
            let fraction = Rat::from_i64(index, 1)
                .ok()
                .and_then(|value| value.div(&denominator).ok());
            let Some(fraction) = fraction else {
                return false;
            };
            let point = lower.add(&span.mul(&fraction));
            let point_expression = self.rat_expression(&point);
            let substituted = self.substitute_bound(argument, variable, point_expression);
            let Ok(value) = self.eval(substituted) else {
                return false;
            };
            let Ok(approximate) = self.approximate(value, 30) else {
                return false;
            };
            let Some(Numeric::Real(real)) = approximate else {
                return matches!(approximate, Some(Numeric::Complex(_, _)));
            };
            let sign = real.sign();
            if sign == 0 {
                return true;
            }
            if previous_sign != 0 && sign != previous_sign {
                return true;
            }
            previous_sign = sign;
        }
        false
    }

    fn interval_has_interior_pole(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> bool {
        let (Some(lower), Some(upper)) = (self.as_rat(lower), self.as_rat(upper)) else {
            return false;
        };
        let (lower, upper) = if lower <= upper {
            (lower, upper)
        } else {
            (upper, lower)
        };
        let cancelled = self.cancel_expression(expression);
        let (_, denominator) = self.split_quotient(cancelled);
        if is_one_value(self.arena, denominator) {
            return false;
        }
        let context = self.algebra_context();
        let Some(roots) = crate::algebra::univariate_rational_root_multiplicities(
            self.arena,
            denominator,
            variable,
            &context,
        ) else {
            return false;
        };
        roots.iter().any(|(root, _)| *root > lower && *root < upper)
    }

    fn endpoint_limit(
        &mut self,
        antiderivative: ExprId,
        variable: ExprId,
        point: ExprId,
        finite_side: LimitSide,
    ) -> Option<LimitValue> {
        let side = match self.limit_target(point) {
            LimitTarget::Finite(_) => finite_side,
            _ => LimitSide::Both,
        };
        self.limit_value(antiderivative, variable, point, side)
    }

    fn apply_limit(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let original = self.arena.intern_call(head, args);
        let side = if args.len() == 4 {
            match self.limit_side(args[3]) {
                Some(side) => side,
                None => return Ok(original),
            }
        } else {
            LimitSide::Both
        };
        match self.limit_value(args[0], args[1], args[2], side) {
            Some(value) => Ok(self.limit_value_expression(value)),
            None => Ok(original),
        }
    }

    fn apply_converges(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let original = self.arena.intern_call(head, args);
        match self.limit_value(args[0], args[1], args[2], LimitSide::Both) {
            Some(LimitValue::Finite(_)) => return Ok(self.heads.true_value),
            Some(_) => return Ok(self.heads.false_value),
            None => {}
        }
        if let LimitTarget::Finite(point) = self.limit_target(args[2]) {
            let left = self.limit_finite_side(args[0], args[1], point, LimitSide::Left);
            let right = self.limit_finite_side(args[0], args[1], point, LimitSide::Right);
            if left.is_some() && right.is_some() {
                return Ok(self.heads.false_value);
            }
        }
        Ok(original)
    }

    fn apply_series(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let original = self.arena.intern_call(head, args);
        let Some(order) = nonnegative_integer(self.arena, args[3]) else {
            return Ok(original);
        };
        if order > 20 {
            return Ok(original);
        }
        let variable = args[1];
        let point = args[2];
        let mut current = args[0];
        let mut terms = Vec::with_capacity(order as usize + 1);
        let mut factorial = Int::one();
        let delta = self.build_delta(variable, point);
        for degree in 0..=order {
            let substituted = self.substitute_bound(current, variable, point);
            let coefficient = match self.eval(substituted) {
                Ok(value) if !self.is_invalid_finite_value(value) => value,
                _ => return Ok(original),
            };
            let term = self.series_term(coefficient, delta, degree, &factorial);
            terms.push(term);
            if degree < order {
                let Some(derivative) = crate::calculus::differentiate(
                    self.arena,
                    current,
                    variable,
                    &self.calculus_context(),
                ) else {
                    return Ok(original);
                };
                current = derivative;
                factorial = factorial.mul(&Int::from_i64(degree as i64 + 1));
            }
        }
        let sum = match terms.len() {
            0 => self.arena.intern_int(Int::zero()),
            1 => terms[0],
            _ => self.arena.intern_call(self.heads.plus, &terms),
        };
        Ok(self.eval(sum).unwrap_or(original))
    }

    fn series_term(
        &mut self,
        coefficient: ExprId,
        delta: ExprId,
        degree: u32,
        factorial: &Int,
    ) -> ExprId {
        let mut factors = Vec::with_capacity(3);
        if !is_one_value(self.arena, coefficient) {
            factors.push(coefficient);
        }
        if degree > 0 {
            let power = if degree == 1 {
                delta
            } else {
                let exponent = self.arena.intern_int(Int::from_i64(degree as i64));
                self.arena.intern_call(self.heads.power, &[delta, exponent])
            };
            factors.push(power);
        }
        let factorial = Rat::from_ints(factorial.clone(), Int::one()).ok();
        if let Some(factorial) = factorial
            && let Ok(reciprocal) = Rat::one().div(&factorial)
            && !reciprocal.is_one()
        {
            factors.push(self.rat_expression(&reciprocal));
        }
        match factors.len() {
            0 => self.arena.intern_int(Int::one()),
            1 => factors[0],
            _ => self.arena.intern_call(self.heads.times, &factors),
        }
    }

    fn build_delta(&mut self, variable: ExprId, point: ExprId) -> ExprId {
        if is_zero_value(self.arena, point) {
            return variable;
        }
        let negative = self.arena.intern_call(self.heads.negate, &[point]);
        let negative = self.eval(negative).unwrap_or(negative);
        let delta = self
            .arena
            .intern_call(self.heads.plus, &[variable, negative]);
        self.eval(delta).unwrap_or(delta)
    }

    fn limit_side(&self, expression: ExprId) -> Option<LimitSide> {
        let symbol = self.arena.symbol_id(expression)?;
        match self.arena.symbol_name(symbol) {
            "left" => Some(LimitSide::Left),
            "right" => Some(LimitSide::Right),
            _ => None,
        }
    }

    fn limit_target(&self, expression: ExprId) -> LimitTarget {
        if let Node::Symbol(symbol) = self.arena.node(expression)
            && self.arena.symbol_name(*symbol) == "inf"
        {
            return LimitTarget::PositiveInfinity;
        }
        if let Node::Call { head, args } = self.arena.node(expression)
            && args.len() == 1
            && *head == self.heads.negate
            && let Node::Symbol(symbol) = self.arena.node(args[0])
            && self.arena.symbol_name(*symbol) == "inf"
        {
            return LimitTarget::NegativeInfinity;
        }
        LimitTarget::Finite(expression)
    }

    fn limit_value(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
        side: LimitSide,
    ) -> Option<LimitValue> {
        if !self.expression_contains(expression, variable) {
            return Some(LimitValue::Finite(expression));
        }
        match self.limit_target(point) {
            LimitTarget::Finite(point) => {
                if side == LimitSide::Both {
                    let left =
                        self.limit_finite_side(expression, variable, point, LimitSide::Left)?;
                    let right =
                        self.limit_finite_side(expression, variable, point, LimitSide::Right)?;
                    if left == right { Some(left) } else { None }
                } else {
                    self.limit_finite_side(expression, variable, point, side)
                }
            }
            LimitTarget::PositiveInfinity => self.limit_infinity(expression, variable, true),
            LimitTarget::NegativeInfinity => self.limit_infinity(expression, variable, false),
        }
    }

    fn limit_finite_side(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
        side: LimitSide,
    ) -> Option<LimitValue> {
        if !self.expression_contains(expression, variable) {
            return Some(LimitValue::Finite(expression));
        }
        if let Some(value) = self.finite_substitution(expression, variable, point) {
            return Some(LimitValue::Finite(value));
        }
        let cancelled = self.cancel_expression(expression);
        if cancelled != expression
            && let Some(value) = self.finite_substitution(cancelled, variable, point)
        {
            return Some(LimitValue::Finite(value));
        }
        if let Some(value) = self.composed_limit(cancelled, variable, point, side) {
            return Some(value);
        }
        let (numerator, denominator) = self.split_quotient(cancelled);
        if let Some(value) = self.abs_quotient_limit(numerator, denominator, variable, point, side)
        {
            return Some(value);
        }
        let numerator_term = self.first_nonzero_term(numerator, variable, point, 8)?;
        let denominator_term = self.first_nonzero_term(denominator, variable, point, 8)?;
        self.quotient_limit_value(numerator_term, denominator_term, side)
    }

    fn abs_quotient_limit(
        &mut self,
        numerator: ExprId,
        denominator: ExprId,
        variable: ExprId,
        point: ExprId,
        side: LimitSide,
    ) -> Option<LimitValue> {
        let (argument, numerator_is_abs) = match self.abs_argument(numerator) {
            Some(argument) => (argument, true),
            None => (numerator, false),
        };
        let argument = if numerator_is_abs {
            if denominator != argument {
                return None;
            }
            argument
        } else {
            let argument = self.abs_argument(denominator)?;
            if numerator != argument {
                return None;
            }
            argument
        };
        let sign = self.approach_sign(argument, variable, point, side)?;
        let one = self.arena.intern_int(Int::one());
        Some(LimitValue::Finite(match sign {
            Sign::Negative => {
                let negative = self.arena.intern_call(self.heads.negate, &[one]);
                self.eval(negative).unwrap_or(negative)
            }
            _ => one,
        }))
    }

    fn abs_argument(&self, expression: ExprId) -> Option<ExprId> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && args.len() == 1
            && self
                .arena
                .symbol_id(*head)
                .map(|symbol| self.arena.symbol_name(symbol))
                == Some("abs")
        {
            return Some(args[0]);
        }
        None
    }

    fn approach_sign(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
        side: LimitSide,
    ) -> Option<Sign> {
        if self.abs_argument(expression).is_some() {
            return Some(Sign::Positive);
        }
        let (order, coefficient) = self.first_nonzero_term(expression, variable, point, 4)?;
        let mut sign = self.definite_sign(coefficient)?;
        if side == LimitSide::Left && order % 2 == 1 {
            sign = flip_sign(sign);
        }
        Some(sign)
    }

    fn composed_limit(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
        side: LimitSide,
    ) -> Option<LimitValue> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if args.len() != 1 {
            return None;
        }
        let head = *head;
        let argument = args[0];
        let name = self
            .arena
            .symbol_id(head)
            .map(|symbol| self.arena.symbol_name(symbol).to_string())?;
        if !matches!(
            name.as_str(),
            "exp"
                | "ln"
                | "atan"
                | "sqrt"
                | "abs"
                | "sin"
                | "cos"
                | "tan"
                | "sinh"
                | "cosh"
                | "tanh"
        ) {
            return None;
        }
        let inner = self.limit_finite_side(argument, variable, point, side)?;
        if name == "ln"
            && let LimitValue::Finite(value) = inner
            && self.expression_sign(value) == Some(Sign::Zero)
        {
            let sign = self.approach_sign(argument, variable, point, side)?;
            if sign == Sign::Negative {
                return None;
            }
            return Some(LimitValue::NegativeInfinity);
        }
        self.apply_unary_limit(&name, inner)
    }

    fn finite_substitution(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
    ) -> Option<ExprId> {
        if self.has_unknown_head(expression) {
            return None;
        }
        let substituted = self.substitute_bound(expression, variable, point);
        let value = self.eval(substituted).ok()?;
        if self.is_invalid_finite_value(value) {
            None
        } else {
            Some(value)
        }
    }

    fn has_unknown_head(&self, expression: ExprId) -> bool {
        if let Node::Call { head, args } = self.arena.node(expression) {
            let known = self.arena.symbol_id(*head).is_some_and(|symbol| {
                matches!(
                    self.arena.symbol_name(symbol),
                    "plus"
                        | "times"
                        | "power"
                        | "negate"
                        | "abs"
                        | "sqrt"
                        | "exp"
                        | "ln"
                        | "sin"
                        | "cos"
                        | "tan"
                        | "asin"
                        | "acos"
                        | "atan"
                        | "sinh"
                        | "cosh"
                        | "tanh"
                        | "asinh"
                        | "acosh"
                        | "atanh"
                )
            });
            if !known {
                return true;
            }
            return args.iter().any(|argument| self.has_unknown_head(*argument));
        }
        false
    }

    fn is_invalid_finite_value(&self, expression: ExprId) -> bool {
        if self.contains_error_or_infinite(expression) {
            return true;
        }
        let Node::Call { head, args } = self.arena.node(expression) else {
            return false;
        };
        if args.len() == 1
            && let Some(name) = self
                .arena
                .symbol_id(*head)
                .map(|symbol| self.arena.symbol_name(symbol))
        {
            let constant = match self.arena.node(args[0]) {
                Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
                Node::Rat(value) => Some(value.clone()),
                _ => None,
            };
            if let Some(constant) = constant {
                if name == "ln" && constant.sign() <= 0 {
                    return true;
                }
                if name == "sqrt" && constant.sign() < 0 {
                    return true;
                }
            }
        }
        args.iter()
            .any(|argument| self.is_invalid_finite_value(*argument))
    }

    fn cancel_expression(&mut self, expression: ExprId) -> ExprId {
        let context = self.algebra_context();
        crate::algebra::cancel(self.arena, expression, &context).unwrap_or(expression)
    }

    fn split_quotient(&mut self, expression: ExprId) -> (ExprId, ExprId) {
        let (head, args) = match self.arena.node(expression) {
            Node::Call { head, args } => (*head, args.to_vec()),
            _ => return (expression, self.arena.intern_int(Int::one())),
        };
        if args.len() == 1 && head == self.heads.negate {
            let (numerator, denominator) = self.split_quotient(args[0]);
            let numerator = self.arena.intern_call(self.heads.negate, &[numerator]);
            return (numerator, denominator);
        }
        if args.len() == 2
            && head == self.heads.power
            && let Some((base, exponent)) = self.negative_power(args[0], args[1])
        {
            let denominator = if matches!(
                self.arena.node(exponent),
                Node::Int(value) if value.is_one()
            ) {
                base
            } else {
                self.arena.intern_call(self.heads.power, &[base, exponent])
            };
            return (self.arena.intern_int(Int::one()), denominator);
        }
        if head == self.heads.times {
            let mut numerator_factors = Vec::new();
            let mut denominator_parts = Vec::new();
            for factor in args {
                let negative = self.negative_factor(factor);
                if let Some((base, exponent)) = negative {
                    denominator_parts.push((base, exponent));
                } else {
                    numerator_factors.push(factor);
                }
            }
            let mut denominator_factors = Vec::with_capacity(denominator_parts.len());
            for (base, exponent) in denominator_parts {
                if exponent == 1 {
                    denominator_factors.push(base);
                } else {
                    let exponent = self.arena.intern_int(Int::from_i64(exponent));
                    denominator_factors
                        .push(self.arena.intern_call(self.heads.power, &[base, exponent]));
                }
            }
            let numerator = match numerator_factors.len() {
                0 => self.arena.intern_int(Int::one()),
                1 => numerator_factors[0],
                _ => self.arena.intern_call(self.heads.times, &numerator_factors),
            };
            let denominator = match denominator_factors.len() {
                0 => self.arena.intern_int(Int::one()),
                1 => denominator_factors[0],
                _ => self
                    .arena
                    .intern_call(self.heads.times, &denominator_factors),
            };
            return (numerator, denominator);
        }
        (expression, self.arena.intern_int(Int::one()))
    }

    fn negative_factor(&self, expression: ExprId) -> Option<(ExprId, i64)> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if *head != self.heads.power || args.len() != 2 {
            return None;
        }
        let exponent = self.negative_exponent(args[1])?;
        Some((args[0], exponent))
    }

    fn negative_exponent(&self, expression: ExprId) -> Option<i64> {
        let value = self.exact_constant_value(expression)?;
        if !value.den().is_one() {
            return None;
        }
        let exponent = value.num().to_i64()?;
        if exponent >= 0 {
            return None;
        }
        exponent.checked_neg()
    }

    fn negative_power(&mut self, base: ExprId, exponent: ExprId) -> Option<(ExprId, ExprId)> {
        let magnitude = self.negative_exponent(exponent)?;
        let positive = self.arena.intern_int(Int::from_i64(magnitude));
        Some((base, positive))
    }

    fn first_nonzero_term(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        point: ExprId,
        limit: u32,
    ) -> Option<(u32, ExprId)> {
        if self.has_unknown_head(expression) {
            return None;
        }
        let mut current = expression;
        for degree in 0..=limit {
            let substituted = self.substitute_bound(current, variable, point);
            let value = self.eval(substituted).ok()?;
            if self.is_invalid_finite_value(value) {
                return None;
            }
            if !is_zero_value(self.arena, value) {
                return Some((degree, value));
            }
            if degree == limit {
                break;
            }
            current = crate::calculus::differentiate(
                self.arena,
                current,
                variable,
                &self.calculus_context(),
            )?;
        }
        None
    }

    fn quotient_limit_value(
        &mut self,
        numerator: (u32, ExprId),
        denominator: (u32, ExprId),
        side: LimitSide,
    ) -> Option<LimitValue> {
        let (numerator_order, numerator_coefficient) = numerator;
        let (denominator_order, denominator_coefficient) = denominator;
        if numerator_order > denominator_order {
            return Some(LimitValue::Finite(self.arena.intern_int(Int::zero())));
        }
        if numerator_order == denominator_order {
            let reciprocal = self.reciprocal(denominator_coefficient);
            let ratio = self
                .arena
                .intern_call(self.heads.times, &[numerator_coefficient, reciprocal]);
            let ratio = self.eval(ratio).ok()?;
            return Some(LimitValue::Finite(ratio));
        }
        let numerator_sign = self.definite_sign(numerator_coefficient)?;
        let denominator_sign = self.definite_sign(denominator_coefficient)?;
        let mut sign = sign_product(numerator_sign, denominator_sign)?;
        if side == LimitSide::Left && (denominator_order - numerator_order) % 2 == 1 {
            sign = flip_sign(sign);
        }
        Some(match sign {
            Sign::Negative => LimitValue::NegativeInfinity,
            _ => LimitValue::PositiveInfinity,
        })
    }

    fn reciprocal(&mut self, expression: ExprId) -> ExprId {
        let exponent = self.arena.intern_int(Int::from_i64(-1));
        self.arena
            .intern_call(self.heads.power, &[expression, exponent])
    }

    fn limit_infinity(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        positive: bool,
    ) -> Option<LimitValue> {
        if !self.expression_contains(expression, variable) {
            return Some(LimitValue::Finite(expression));
        }
        let context = self.algebra_context();
        if let Some(value) = crate::algebra::rational_limit_at_infinity(
            self.arena, expression, variable, &context, positive,
        ) {
            return Some(match value {
                crate::algebra::RationalLimit::Finite(value) => {
                    LimitValue::Finite(self.rat_expression(&value))
                }
                crate::algebra::RationalLimit::Infinite(sign) => {
                    if sign < 0 {
                        LimitValue::NegativeInfinity
                    } else {
                        LimitValue::PositiveInfinity
                    }
                }
            });
        }
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        let head = *head;
        let args = args.to_vec();
        let name = self
            .arena
            .symbol_id(head)
            .map(|symbol| self.arena.symbol_name(symbol).to_string())?;
        if name == "negate" && args.len() == 1 {
            let value = self.limit_infinity(args[0], variable, positive)?;
            return Some(self.negate_limit_value(value));
        }
        if name == "plus" {
            let mut total = LimitValue::Finite(self.arena.intern_int(Int::zero()));
            for arg in args {
                let value = self.limit_infinity(arg, variable, positive)?;
                total = self.add_limit_values(total, value)?;
            }
            return Some(total);
        }
        if name == "times" {
            if self.decays_to_zero(&args, variable, positive)
                || self.logarithm_over_power_decays(&args, variable, positive)
            {
                return Some(LimitValue::Finite(self.arena.intern_int(Int::zero())));
            }
            let mut total = LimitValue::Finite(self.arena.intern_int(Int::one()));
            for arg in args {
                let value = self.limit_infinity(arg, variable, positive)?;
                total = self.multiply_limit_values(total, value)?;
            }
            return Some(total);
        }
        if name == "power" && args.len() == 2 {
            if args[0] == variable {
                let exponent = self.exact_constant_value(args[1])?;
                if exponent.is_zero() {
                    return Some(LimitValue::Finite(self.arena.intern_int(Int::one())));
                }
                if exponent.sign() < 0 {
                    return Some(LimitValue::Finite(self.arena.intern_int(Int::zero())));
                }
                if positive {
                    return Some(LimitValue::PositiveInfinity);
                }
                if exponent.den().is_one() && exponent.num().to_i64()? % 2 != 0 {
                    return Some(LimitValue::NegativeInfinity);
                }
                return None;
            }
            if args[1] == variable {
                let base = self.exact_constant_value(args[0])?;
                if base.sign() <= 0 {
                    return None;
                }
                let grows = if positive {
                    base > Rat::one()
                } else {
                    base < Rat::one()
                };
                if grows {
                    return Some(LimitValue::PositiveInfinity);
                }
                return Some(LimitValue::Finite(self.arena.intern_int(Int::zero())));
            }
        }
        if args.len() == 1 {
            let inner = self.limit_infinity(args[0], variable, positive)?;
            return self.apply_unary_limit(&name, inner);
        }
        None
    }

    fn decays_to_zero(&mut self, factors: &[ExprId], variable: ExprId, positive: bool) -> bool {
        let mut decaying = false;
        for factor in factors {
            if self.is_decaying_exponential(*factor, variable, positive) {
                decaying = true;
                continue;
            }
            if self.is_polynomial_in(*factor, variable) || self.is_logarithm_expression(*factor) {
                continue;
            }
            return false;
        }
        decaying
    }

    fn is_decaying_exponential(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        positive: bool,
    ) -> bool {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return false;
        };
        let head = *head;
        let args = args.to_vec();
        let name = self
            .arena
            .symbol_id(head)
            .map(|symbol| self.arena.symbol_name(symbol));
        if name == Some("exp") && args.len() == 1 {
            if let Some(slope) = self.exponential_slope(args[0], variable) {
                return if positive {
                    slope.sign() < 0
                } else {
                    slope.sign() > 0
                };
            }
            return false;
        }
        if name == Some("power") && args.len() == 2 {
            let Some(exponent) = self.exact_constant_value(args[1]) else {
                return false;
            };
            if exponent.sign() >= 0 || !exponent.den().is_one() {
                return false;
            }
            if let Node::Call { head, args: inner } = self.arena.node(args[0]) {
                let inner_head = *head;
                let inner_args = inner.to_vec();
                if inner_args.len() == 1
                    && self
                        .arena
                        .symbol_id(inner_head)
                        .map(|symbol| self.arena.symbol_name(symbol))
                        == Some("exp")
                    && let Some(slope) = self.exponential_slope(inner_args[0], variable)
                {
                    return if positive {
                        slope.sign() > 0
                    } else {
                        slope.sign() < 0
                    };
                }
            }
        }
        false
    }

    fn exponential_slope(&mut self, argument: ExprId, variable: ExprId) -> Option<Rat> {
        let slope = crate::calculus::differentiate(
            self.arena,
            argument,
            variable,
            &self.calculus_context(),
        )?;
        if self.expression_contains(slope, variable) {
            return None;
        }
        let slope = self.eval(slope).ok()?;
        self.exact_constant_value(slope)
    }

    fn is_logarithm_expression(&self, expression: ExprId) -> bool {
        if let Node::Call { head, args } = self.arena.node(expression) {
            return args.len() == 1
                && self
                    .arena
                    .symbol_id(*head)
                    .map(|symbol| self.arena.symbol_name(symbol))
                    == Some("ln");
        }
        false
    }

    fn logarithm_over_power_decays(
        &mut self,
        factors: &[ExprId],
        variable: ExprId,
        positive: bool,
    ) -> bool {
        if !positive {
            return false;
        }
        let mut has_logarithm = false;
        let mut has_negative_power = false;
        for factor in factors {
            if self.is_logarithm_expression(*factor) {
                has_logarithm = true;
                continue;
            }
            if self.is_negative_variable_power(*factor, variable) {
                has_negative_power = true;
                continue;
            }
            if !self.expression_contains(*factor, variable) {
                continue;
            }
            return false;
        }
        has_logarithm && has_negative_power
    }

    fn is_negative_variable_power(&self, expression: ExprId, variable: ExprId) -> bool {
        if let Node::Call { head, args } = self.arena.node(expression)
            && args.len() == 2
            && *head == self.heads.power
            && args[0] == variable
        {
            if let Node::Int(exponent) = self.arena.node(args[1])
                && let Some(value) = exponent.to_i64()
            {
                return value < 0;
            }
            if let Node::Rat(exponent) = self.arena.node(args[1]) {
                return exponent.den().is_one()
                    && exponent.num().to_i64().is_some_and(|value| value < 0);
            }
        }
        false
    }

    fn is_polynomial_in(&mut self, expression: ExprId, variable: ExprId) -> bool {
        if !self.expression_contains(expression, variable) {
            return true;
        }
        match self.arena.node(expression) {
            Node::Symbol(_) => expression == variable,
            Node::Int(_) | Node::Rat(_) | Node::Real(_) => true,
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.plus || head == self.heads.times || head == self.heads.negate
                {
                    return args
                        .iter()
                        .all(|argument| self.is_polynomial_in(*argument, variable));
                }
                if head == self.heads.power
                    && args.len() == 2
                    && nonnegative_integer(self.arena, args[1]).is_some()
                {
                    return self.is_polynomial_in(args[0], variable);
                }
                false
            }
        }
    }

    fn negate_limit_value(&mut self, value: LimitValue) -> LimitValue {
        match value {
            LimitValue::Finite(value) => {
                let call = self.arena.intern_call(self.heads.negate, &[value]);
                LimitValue::Finite(self.eval(call).unwrap_or(call))
            }
            LimitValue::PositiveInfinity => LimitValue::NegativeInfinity,
            LimitValue::NegativeInfinity => LimitValue::PositiveInfinity,
        }
    }

    fn add_limit_values(&mut self, left: LimitValue, right: LimitValue) -> Option<LimitValue> {
        match (left, right) {
            (LimitValue::Finite(left), LimitValue::Finite(right)) => {
                let value = self.arena.intern_call(self.heads.plus, &[left, right]);
                Some(LimitValue::Finite(self.eval(value).ok()?))
            }
            (LimitValue::Finite(_), infinite) | (infinite, LimitValue::Finite(_)) => Some(infinite),
            (left, right) if left == right => Some(left),
            _ => None,
        }
    }

    fn multiply_limit_values(&mut self, left: LimitValue, right: LimitValue) -> Option<LimitValue> {
        match (left, right) {
            (LimitValue::Finite(left), LimitValue::Finite(right)) => {
                let value = self.arena.intern_call(self.heads.times, &[left, right]);
                Some(LimitValue::Finite(self.eval(value).ok()?))
            }
            (LimitValue::Finite(value), infinite) | (infinite, LimitValue::Finite(value)) => {
                let sign = self.definite_sign(value)?;
                Some(infinite.with_sign(sign))
            }
            (left, right) => {
                let sign = sign_product(left.sign()?, right.sign()?)?;
                Some(match sign {
                    Sign::Negative => LimitValue::NegativeInfinity,
                    _ => LimitValue::PositiveInfinity,
                })
            }
        }
    }

    fn apply_unary_limit(&mut self, name: &str, inner: LimitValue) -> Option<LimitValue> {
        match name {
            "exp" => match inner {
                LimitValue::PositiveInfinity => Some(LimitValue::PositiveInfinity),
                LimitValue::NegativeInfinity => {
                    Some(LimitValue::Finite(self.arena.intern_int(Int::zero())))
                }
                LimitValue::Finite(value) => {
                    let result = self.call_evaluated("exp", &[value])?;
                    Some(LimitValue::Finite(result))
                }
            },
            "ln" => match inner {
                LimitValue::PositiveInfinity => Some(LimitValue::PositiveInfinity),
                LimitValue::NegativeInfinity => None,
                LimitValue::Finite(value) => match self.expression_sign(value)? {
                    Sign::Negative => None,
                    Sign::Zero => Some(LimitValue::NegativeInfinity),
                    _ => {
                        let result = self.call_evaluated("ln", &[value])?;
                        Some(LimitValue::Finite(result))
                    }
                },
            },
            "atan" => match inner {
                LimitValue::PositiveInfinity => Some(LimitValue::Finite(self.half_pi()?)),
                LimitValue::NegativeInfinity => {
                    let half = self.half_pi()?;
                    Some(LimitValue::Finite(
                        self.arena.intern_call(self.heads.negate, &[half]),
                    ))
                }
                LimitValue::Finite(value) => {
                    let result = self.call_evaluated("atan", &[value])?;
                    Some(LimitValue::Finite(result))
                }
            },
            "sqrt" => match inner {
                LimitValue::PositiveInfinity => Some(LimitValue::PositiveInfinity),
                LimitValue::NegativeInfinity => None,
                LimitValue::Finite(value) => match self.expression_sign(value)? {
                    Sign::Negative => None,
                    _ => {
                        let result = self.call_evaluated("sqrt", &[value])?;
                        Some(LimitValue::Finite(result))
                    }
                },
            },
            "abs" => match inner {
                LimitValue::PositiveInfinity | LimitValue::NegativeInfinity => {
                    Some(LimitValue::PositiveInfinity)
                }
                LimitValue::Finite(value) => {
                    let result = self.call_evaluated("abs", &[value])?;
                    Some(LimitValue::Finite(result))
                }
            },
            "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" => match inner {
                LimitValue::Finite(value) => {
                    let result = self.call_evaluated(name, &[value])?;
                    Some(LimitValue::Finite(result))
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn call_evaluated(&mut self, name: &str, args: &[ExprId]) -> Option<ExprId> {
        let symbol = self.arena.symbol_or_intern(name);
        let head = self.arena.symbol_expr(symbol);
        let call = self.arena.intern_call(head, args);
        self.eval(call).ok()
    }

    fn half_pi(&mut self) -> Option<ExprId> {
        self.pi_fraction(1, 2)
    }

    fn pi_fraction(&mut self, numerator: i64, denominator: i64) -> Option<ExprId> {
        let pi = self.arena.builtin_head("pi");
        let factor = Rat::from_i64(numerator, denominator).ok()?;
        let factor = self.rat_expression(&factor);
        let value = self.arena.intern_call(self.heads.times, &[factor, pi]);
        self.eval(value).ok()
    }

    fn definite_sign(&self, expression: ExprId) -> Option<Sign> {
        match self.expression_sign(expression)? {
            Sign::Zero => None,
            Sign::Positive | Sign::NonNegative => Some(Sign::Positive),
            Sign::Negative | Sign::NonPositive => Some(Sign::Negative),
        }
    }

    fn limit_value_expression(&mut self, value: LimitValue) -> ExprId {
        match value {
            LimitValue::Finite(value) => value,
            LimitValue::PositiveInfinity => self.infinity_expression(),
            LimitValue::NegativeInfinity => {
                let infinity = self.infinity_expression();
                self.arena.intern_call(self.heads.negate, &[infinity])
            }
        }
    }

    fn infinity_expression(&mut self) -> ExprId {
        let symbol = self.arena.symbol_or_intern("inf");
        self.arena.symbol_expr(symbol)
    }

    fn rat_expression(&mut self, value: &Rat) -> ExprId {
        if value.den().is_one() {
            self.arena.intern_int(value.num())
        } else {
            self.arena.intern_rat(value.clone())
        }
    }

    fn expression_contains(&self, expression: ExprId, variable: ExprId) -> bool {
        if expression == variable {
            return true;
        }
        match self.arena.node(expression) {
            Node::Call { args, .. } => args
                .iter()
                .any(|argument| self.expression_contains(*argument, variable)),
            _ => false,
        }
    }

    fn substitute_bound(&mut self, expression: ExprId, variable: ExprId, value: ExprId) -> ExprId {
        if expression == variable {
            return value;
        }
        match self.arena.node(expression) {
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                let head = self.substitute_bound(head, variable, value);
                let mut replaced = Vec::with_capacity(args.len());
                for argument in args {
                    replaced.push(self.substitute_bound(argument, variable, value));
                }
                self.arena.intern_call(head, &replaced)
            }
            _ => expression,
        }
    }

    fn contains_error_or_infinite(&self, expression: ExprId) -> bool {
        if let Node::Symbol(symbol) = self.arena.node(expression) {
            return matches!(
                self.arena.symbol_name(*symbol),
                "inf" | "undefined" | "indeterminate" | "failed"
            );
        }
        match self.arena.node(expression) {
            Node::Call { head, args } => {
                self.contains_error_or_infinite(*head)
                    || args
                        .iter()
                        .any(|argument| self.contains_error_or_infinite(*argument))
            }
            _ => false,
        }
    }

    fn calculus_context(&self) -> crate::calculus::Context {
        crate::calculus::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        }
    }

    fn algebra_context(&self) -> crate::algebra::Context {
        crate::algebra::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        }
    }

    fn algebra_unary(
        &mut self,
        args: &[ExprId],
        head: ExprId,
        context: crate::algebra::Context,
        operation: fn(&mut Arena, ExprId, &crate::algebra::Context) -> Option<ExprId>,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "algebra function expects one argument",
            ));
        }
        let original = self.arena.intern_call(head, args);
        Ok(operation(self.arena, args[0], &context).unwrap_or(original))
    }

    fn apply_statistics(&mut self, function: StatisticsHead, expression: ExprId) -> Option<ExprId> {
        match function {
            StatisticsHead::Mean => crate::statistics::mean_stat(self.arena, expression),
            StatisticsHead::Median => crate::statistics::median(self.arena, expression),
            StatisticsHead::Minimum => crate::statistics::minimum(self.arena, expression),
            StatisticsHead::Maximum => crate::statistics::maximum(self.arena, expression),
            StatisticsHead::Range => crate::statistics::range(self.arena, expression),
            StatisticsHead::Total => crate::statistics::total(self.arena, expression),
            StatisticsHead::Count => crate::statistics::count(self.arena, expression),
            StatisticsHead::Variance => crate::statistics::variance_stat(self.arena, expression),
            StatisticsHead::SampleVariance => {
                crate::statistics::sample_variance(self.arena, expression)
            }
            StatisticsHead::StandardDeviation => {
                crate::statistics::standard_deviation(self.arena, expression)
            }
            StatisticsHead::SampleStandardDeviation => {
                crate::statistics::sample_standard_deviation(self.arena, expression)
            }
            StatisticsHead::Frequency => crate::statistics::frequency(self.arena, expression),
            StatisticsHead::Mode => crate::statistics::mode(self.arena, expression),
        }
    }

    fn apply_logic(&mut self, function: LogicHead, args: &[ExprId]) -> Option<ExprId> {
        let boolean = |value| {
            if value {
                self.heads.true_value
            } else {
                self.heads.false_value
            }
        };
        match function {
            LogicHead::Xor => {
                let left = self.boolean_value(args[0])?;
                let right = self.boolean_value(args[1])?;
                Some(boolean(left ^ right))
            }
            LogicHead::Implies => {
                let left = self.boolean_value(args[0])?;
                let right = self.boolean_value(args[1])?;
                Some(boolean(!left || right))
            }
            LogicHead::All | LogicHead::Any => {
                let Node::Call { head, args: values } = self.arena.node(args[0]) else {
                    return None;
                };
                let head = *head;
                if self
                    .arena
                    .symbol_id(head)
                    .map(|symbol| self.arena.symbol_name(symbol))
                    != Some("list")
                {
                    return None;
                }
                let values = values.to_vec();
                let mut result = matches!(function, LogicHead::All);
                for value in values {
                    let value = self.boolean_value(value)?;
                    if matches!(function, LogicHead::All) {
                        result &= value;
                    } else {
                        result |= value;
                    }
                }
                Some(boolean(result))
            }
        }
    }

    fn boolean_value(&self, value: ExprId) -> Option<bool> {
        if let Node::Symbol(symbol) = self.arena.node(value) {
            match self.arena.symbol_name(*symbol) {
                "true" => return Some(true),
                "false" => return Some(false),
                _ => {}
            }
        }
        None
    }

    fn apply_number_theory(
        &mut self,
        function: NumberTheoryHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        match function {
            NumberTheoryHead::IsPrime => crate::numbertheory::is_prime(self.arena, args[0]),
            NumberTheoryHead::NextPrime => crate::numbertheory::next_prime(self.arena, args[0]),
            NumberTheoryHead::FactorInteger => {
                crate::numbertheory::factor_integer(self.arena, args[0])
            }
            NumberTheoryHead::Divisors => crate::numbertheory::divisors(self.arena, args[0]),
            NumberTheoryHead::Totient => crate::numbertheory::totient(self.arena, args[0]),
            NumberTheoryHead::Sigma => crate::numbertheory::sigma(self.arena, args[0]),
            NumberTheoryHead::IsEven => crate::numbertheory::is_even(self.arena, args[0]),
            NumberTheoryHead::IsOdd => crate::numbertheory::is_odd(self.arena, args[0]),
            NumberTheoryHead::Divides => crate::numbertheory::divides(self.arena, args[0], args[1]),
            NumberTheoryHead::Modulo => crate::numbertheory::modulo(self.arena, args[0], args[1]),
            NumberTheoryHead::PowerMod => {
                crate::numbertheory::power_mod(self.arena, args[0], args[1], args[2])
            }
            NumberTheoryHead::PreviousPrime => {
                crate::numbertheory::previous_prime(self.arena, args[0])
            }
            NumberTheoryHead::DivisorCount => {
                crate::numbertheory::divisor_count(self.arena, args[0])
            }
            NumberTheoryHead::Moebius => crate::numbertheory::moebius(self.arena, args[0]),
            NumberTheoryHead::Radical => crate::numbertheory::radical(self.arena, args[0]),
            NumberTheoryHead::IsSquareFree => {
                crate::numbertheory::is_square_free(self.arena, args[0])
            }
            NumberTheoryHead::PrimeCount => crate::numbertheory::prime_count(self.arena, args[0]),
            NumberTheoryHead::Fibonacci => crate::numbertheory::fibonacci(self.arena, args[0]),
            NumberTheoryHead::IsPerfectSquare => {
                crate::numbertheory::is_perfect_square(self.arena, args[0])
            }
            NumberTheoryHead::ModInverse => {
                crate::numbertheory::mod_inverse(self.arena, args[0], args[1])
            }
            NumberTheoryHead::Jacobi => crate::numbertheory::jacobi(self.arena, args[0], args[1]),
        }
    }

    fn apply_linear(&mut self, function: LinearHead, args: &[ExprId]) -> Option<ExprId> {
        match function {
            LinearHead::Determinant => crate::linalg::determinant(self.arena, args[0])
                .or_else(|| crate::linalg::determinant_symbolic(self.arena, args[0])),
            LinearHead::Rank => crate::linalg::rank(self.arena, args[0]),
            LinearHead::Transpose => crate::linalg::transpose(self.arena, args[0])
                .or_else(|| crate::linalg::transpose_symbolic(self.arena, args[0])),
            LinearHead::Inverse => crate::linalg::inverse(self.arena, args[0])
                .or_else(|| crate::linalg::inverse_symbolic(self.arena, args[0])),
            LinearHead::Rref => crate::linalg::rref(self.arena, args[0]),
            LinearHead::Trace => crate::linalg::trace(self.arena, args[0])
                .or_else(|| crate::linalg::trace_symbolic(self.arena, args[0])),
            LinearHead::Matmul => crate::linalg::matmul(self.arena, args[0], args[1])
                .or_else(|| crate::linalg::matmul_symbolic(self.arena, args[0], args[1])),
            LinearHead::Identity => crate::linalg::identity(self.arena, args[0]),
            LinearHead::SolveLinear => crate::linalg::solve_linear(self.arena, args[0], args[1]),
            LinearHead::Dot => crate::linalg::dot(self.arena, args[0], args[1]),
            LinearHead::Cross => crate::linalg::cross(self.arena, args[0], args[1]),
            LinearHead::Plu => crate::linalg::plu(self.arena, args[0]),
            LinearHead::Charpoly => crate::linalg::charpoly(self.arena, args[0]),
            LinearHead::Eigenvalues => crate::linalg::eigenvalues(self.arena, args[0]),
            LinearHead::Nullspace => crate::linalg::nullspace(self.arena, args[0]),
            LinearHead::Eigenvectors => crate::linalg::eigenvectors(self.arena, args[0]),
            LinearHead::Qr => crate::linalg::qr(self.arena, args[0]),
        }
    }

    fn apply_exact_numeric(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let arity = match function {
            ExactNumericHead::Gcd | ExactNumericHead::Lcm => {
                if args.len() < 2 {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "gcd and lcm expect at least two integers",
                    ));
                }
                return Ok(self.apply_fold_integer(function, args));
            }
            ExactNumericHead::Factorial => 1,
            ExactNumericHead::Binomial => 2,
            ExactNumericHead::Floor | ExactNumericHead::Ceil | ExactNumericHead::Round => 1,
        };
        if args.len() != arity {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "numeric function received the wrong number of arguments",
            ));
        }
        Ok(self.apply_simple_numeric(function, args))
    }

    fn apply_fold_integer(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        let mut value = exact_integer(self.arena, args[0])?;
        for arg in &args[1..] {
            let next = exact_integer(self.arena, *arg)?;
            value = match function {
                ExactNumericHead::Gcd => value.gcd(&next),
                ExactNumericHead::Lcm => integer_lcm(&value, &next),
                _ => return None,
            };
        }
        Some(self.arena.intern_int(value))
    }

    fn apply_simple_numeric(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        match function {
            ExactNumericHead::Factorial => {
                let value = exact_integer(self.arena, args[0])?.to_i64()?;
                if !(0..=10_000).contains(&value) {
                    return None;
                }
                Some(self.arena.intern_int(integer_factorial(value)?))
            }
            ExactNumericHead::Binomial => {
                let n = exact_integer(self.arena, args[0])?.to_i64()?;
                let k = exact_integer(self.arena, args[1])?.to_i64()?;
                if n < 0 || k < 0 {
                    return None;
                }
                Some(self.arena.intern_int(integer_binomial(n, k)?))
            }
            ExactNumericHead::Floor | ExactNumericHead::Ceil | ExactNumericHead::Round => {
                if let Node::Real(value) = self.arena.node(args[0]) {
                    let result = match function {
                        ExactNumericHead::Floor => value.floor(),
                        ExactNumericHead::Ceil => value.ceil(),
                        _ => value.round(),
                    };
                    return result.ok().map(|value| self.arena.intern_real(value));
                }
                let rational = exact_rational(self.arena, args[0])?;
                let value = match function {
                    ExactNumericHead::Floor => rational.floor(),
                    ExactNumericHead::Ceil => rational.ceil(),
                    _ => rational.round(),
                };
                Some(self.arena.intern_int(value))
            }
            _ => None,
        }
    }

    fn apply_user(
        &mut self,
        symbol: SymbolId,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let rules: Vec<Rule> = self.definitions.rules(symbol).to_vec();
        if rules.is_empty() {
            return Ok(None);
        }
        let call = self.arena.intern_call(head, args);
        for rule in rules {
            let mut bindings = Bindings::new();
            let mut conditions = Vec::new();
            if !match_pattern(self.arena, rule.lhs, call, &mut bindings, &mut conditions) {
                continue;
            }
            let mut accepted = true;
            for condition in conditions {
                let condition = substitute(self.arena, condition, &bindings);
                let value = self.eval(condition)?;
                if value != self.heads.true_value {
                    accepted = false;
                    break;
                }
            }
            if !accepted {
                continue;
            }
            let rhs = substitute(self.arena, rule.rhs, &bindings);
            return if rule.delayed {
                self.eval(rhs).map(Some)
            } else {
                Ok(Some(rhs))
            };
        }
        Ok(None)
    }

    fn plus(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.iter().any(|arg| self.interval_bounds(*arg).is_some()) {
            return self.plus_intervals(args);
        }
        if let Some(value) = self.error_value(args) {
            return Ok(value);
        }
        let mut has_real = false;
        let mut all_numeric = true;
        for arg in args {
            match self.arena.node(*arg) {
                Node::Real(_) => has_real = true,
                Node::Int(_) | Node::Rat(_) => {}
                _ => all_numeric = false,
            }
        }
        if has_real && all_numeric {
            let mut value = Real::zero(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real addition failed"))?;
            for arg in args {
                let Some(argument) = self.real_endpoint(*arg) else {
                    return Ok(self.arena.intern_call(self.heads.plus, args));
                };
                value = value.add(&argument).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "real addition failed")
                })?;
            }
            return Ok(self.arena.intern_real(value));
        }
        let mut constant = Rat::zero();
        let mut terms: HashMap<ExprId, Rat, FnvBuildHasher> = HashMap::default();
        for arg in args {
            let (coefficient, body) = self.decompose_term(*arg)?;
            match body {
                None => constant = constant.add(&coefficient),
                Some(body) => match terms.get_mut(&body) {
                    Some(value) => *value = value.add(&coefficient),
                    None => {
                        terms.insert(body, coefficient);
                    }
                },
            }
        }

        let mut out = Vec::new();
        if !constant.is_zero() {
            out.push(self.arena.intern_rat(constant));
        }
        for (body, coefficient) in terms {
            if coefficient.is_zero() {
                continue;
            }
            out.push(self.term(body, coefficient));
        }
        Ok(match out.len() {
            0 => self.arena.intern_int(Int::zero()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.plus, &out),
        })
    }

    fn times(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.iter().any(|arg| self.interval_bounds(*arg).is_some()) {
            return self.times_intervals(args);
        }
        let mut has_zero = false;
        let mut undefined = false;
        let mut failed = false;
        let mut has_real = false;
        let mut all_numeric = true;
        for arg in args {
            if self.as_rat(*arg).is_some_and(|value| value.is_zero()) {
                has_zero = true;
            }
            if *arg == self.heads.indeterminate {
                return Ok(self.heads.indeterminate);
            }
            if *arg == self.heads.undefined {
                undefined = true;
            } else if *arg == self.heads.failed {
                failed = true;
            }
            match self.arena.node(*arg) {
                Node::Real(_) => has_real = true,
                Node::Int(_) | Node::Rat(_) => {}
                _ => all_numeric = false,
            }
        }
        if undefined {
            return if has_zero {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.heads.undefined)
            };
        }
        if failed {
            return Ok(self.heads.failed);
        }
        if has_real && all_numeric {
            let mut value = Real::one(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real product failed"))?;
            for arg in args {
                let Some(argument) = self.real_endpoint(*arg) else {
                    return Ok(self.arena.intern_call(self.heads.times, args));
                };
                value = value.mul(&argument).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "real product failed")
                })?;
            }
            return Ok(self.arena.intern_real(value));
        }

        let mut coefficient = Rat::one();
        let mut bases: HashMap<ExprId, Rat, FnvBuildHasher> = HashMap::default();
        for arg in args {
            self.collect_factor(*arg, &mut coefficient, &mut bases)?;
        }
        if coefficient.is_zero() {
            return Ok(self.arena.intern_int(Int::zero()));
        }

        let mut factors = Vec::new();
        for (base, exponent) in bases {
            if exponent.is_zero() {
                continue;
            }
            if exponent.is_one() {
                factors.push(base);
            } else {
                let exponent_node = self.arena.intern_rat(exponent);
                factors.push(
                    self.arena
                        .intern_call(self.heads.power, &[base, exponent_node]),
                );
            }
        }

        let negative = coefficient.sign() < 0;
        let magnitude = coefficient.abs();
        let mut out = Vec::new();
        if !magnitude.is_one() || factors.is_empty() {
            out.push(self.arena.intern_rat(magnitude));
        }
        out.extend(factors);
        let product = match out.len() {
            0 => self.arena.intern_int(Int::one()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.times, &out),
        };
        if negative {
            Ok(self.arena.intern_call(self.heads.negate, &[product]))
        } else {
            Ok(product)
        }
    }

    fn power(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "power expects two arguments",
            ));
        }
        let base = args[0];
        let exponent = args[1];
        if self.is_error(base) {
            return Ok(base);
        }
        if self.is_error(exponent) {
            return Ok(exponent);
        }
        if let Node::Real(real) = self.arena.node(base)
            && let Some(power) = self.integer_exponent(exponent)
        {
            let value = if power == 0 {
                Real::one(30)
            } else {
                real.pow_si(power)
            };
            if let Ok(value) = value {
                return Ok(self.arena.intern_real(value));
            }
        }
        if let (Some((low, high)), Some(power)) =
            (self.interval_bounds(base), self.integer_exponent(exponent))
            && let Some(result) = self.interval_power(&low, &high, power)
        {
            return Ok(result);
        }
        if let Some(exp) = self.as_rat(exponent) {
            return self.power_from_rational(base, exponent, exp);
        }
        if self.as_rat(base).is_some_and(|value| value.is_one()) {
            return Ok(self.arena.intern_int(Int::one()));
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
    }

    fn power_from_rational(
        &mut self,
        base: ExprId,
        exponent: ExprId,
        exp: Rat,
    ) -> Result<ExprId, Diagnostic> {
        if exp.is_zero() {
            return if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.arena.intern_int(Int::one()))
            };
        }
        if exp.is_one() {
            return Ok(base);
        }
        if !exp.den().is_one() {
            if self.as_rat(base).is_some_and(|value| value.is_one()) {
                return Ok(self.arena.intern_int(Int::one()));
            }
            if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                return if exp.sign() < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            return Ok(self.arena.intern_call(self.heads.power, &[base, exponent]));
        }
        let Some(power) = exp.num().to_i64() else {
            return Ok(self.arena.intern_call(self.heads.power, &[base, exponent]));
        };
        if let Some(base_value) = self.as_rat(base) {
            if base_value.is_zero() {
                return if power < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            let value = rat_pow(&base_value, power)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "numeric power failed"))?;
            return Ok(self.arena.intern_rat(value));
        }
        if base == self.heads.imaginary {
            return Ok(self.imaginary_power(power));
        }
        if let Some((real, imaginary)) = self.gaussian(base) {
            if real.is_zero() && imaginary.is_zero() {
                return if power < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            return match gaussian_pow(&real, &imaginary, power) {
                Some((real, imaginary)) => Ok(self.gaussian_node(&real, &imaginary)),
                None => Ok(self.heads.undefined),
            };
        }
        if let Some((inner_base, inner_exponent)) = self.nested_power(base)
            && let Some(inner) = self.as_rat(inner_exponent)
        {
            let combined = inner.mul(&exp);
            let combined_node = self.arena.intern_rat(combined);
            return Ok(self
                .arena
                .intern_call(self.heads.power, &[inner_base, combined_node]));
        }
        if let Some(inner_base) = self.negated_operand(base) {
            let inner = self
                .arena
                .intern_call(self.heads.power, &[inner_base, exponent]);
            let value = self.eval(inner)?;
            return if power % 2 == 0 {
                Ok(value)
            } else {
                Ok(self.arena.intern_call(self.heads.negate, &[value]))
            };
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
    }

    fn integer_exponent(&self, expression: ExprId) -> Option<i64> {
        let value = self.as_rat(expression)?;
        if !value.den().is_one() {
            return None;
        }
        value.num().to_i64()
    }

    fn nested_power(&self, expression: ExprId) -> Option<(ExprId, ExprId)> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && *head == self.heads.power
            && args.len() == 2
        {
            return Some((args[0], args[1]));
        }
        None
    }

    fn negated_operand(&self, expression: ExprId) -> Option<ExprId> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && *head == self.heads.negate
            && args.len() == 1
        {
            return Some(args[0]);
        }
        None
    }

    fn negate(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "negate expects one argument",
            ));
        }
        let value = args[0];
        if self.is_error(value) {
            return Ok(value);
        }
        if let Node::Real(real) = self.arena.node(value) {
            let negated = real
                .neg()
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real negation failed"))?;
            return Ok(self.arena.intern_real(negated));
        }
        if let Some((low, high)) = self.interval_bounds(value) {
            let negated_low = high.neg().map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval negation failed")
            })?;
            let negated_high = low.neg().map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval negation failed")
            })?;
            return Ok(self.interval_node(negated_low, negated_high));
        }
        if let Some(number) = self.as_rat(value) {
            return Ok(self.arena.intern_rat(number.neg()));
        }
        let call = match self.arena.node(value) {
            Node::Call { head, args } => Some((*head, args.to_vec())),
            _ => None,
        };
        if let Some((head, args)) = call {
            if head == self.heads.negate && args.len() == 1 {
                return Ok(args[0]);
            }
            if head == self.heads.plus {
                let mut terms = Vec::with_capacity(args.len());
                for arg in args {
                    terms.push(self.arena.intern_call(self.heads.negate, &[arg]));
                }
                return Ok(self.arena.intern_call(self.heads.plus, &terms));
            }
        }
        Ok(self.arena.intern_call(self.heads.negate, &[value]))
    }

    fn same(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "same expects two arguments",
            ));
        }
        Ok(if args[0] == args[1] {
            self.heads.true_value
        } else {
            self.heads.false_value
        })
    }

    fn equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "equal expects two arguments",
            ));
        }
        if args[0] == args[1] {
            return Ok(self.heads.true_value);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            return Ok(if left == right {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(self.heads.equal, args))
    }

    fn not_equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "notEqual expects two arguments",
            ));
        }
        let equal = self.equal(args)?;
        if equal == self.heads.true_value {
            return Ok(self.heads.false_value);
        }
        if equal == self.heads.false_value {
            return Ok(self.heads.true_value);
        }
        Ok(self.arena.intern_call(self.heads.not_equal, args))
    }

    fn ordering(
        &mut self,
        args: &[ExprId],
        wanted: Ordering,
        head: ExprId,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "comparison expects two arguments",
            ));
        }
        if self.is_error(args[0]) {
            return Ok(args[0]);
        }
        if self.is_error(args[1]) {
            return Ok(args[1]);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            let order = left.cmp(&right);
            let result = match wanted {
                Ordering::Less => {
                    order == Ordering::Less
                        || (head == self.heads.less_equal && order == Ordering::Equal)
                }
                Ordering::Greater => {
                    order == Ordering::Greater
                        || (head == self.heads.greater_equal && order == Ordering::Equal)
                }
                Ordering::Equal => order == Ordering::Equal,
            };
            return Ok(if result {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(head, args))
    }

    fn decompose_term(&mut self, id: ExprId) -> Result<(Rat, Option<ExprId>), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => Ok((
                Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?,
                None,
            )),
            Node::Rat(value) => Ok((value.clone(), None)),
            Node::Real(_) => Ok((Rat::one(), Some(id))),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    let (coefficient, body) = self.decompose_term(args[0])?;
                    return Ok((coefficient.neg(), body));
                }
                if head == self.heads.times {
                    let mut coefficient = Rat::one();
                    let mut body = Vec::new();
                    for arg in args {
                        let (part, part_body) = self.decompose_term(arg)?;
                        coefficient = coefficient.mul(&part);
                        if let Some(part_body) = part_body {
                            body.push(part_body);
                        }
                    }
                    let body = match body.len() {
                        0 => None,
                        1 => Some(body[0]),
                        _ => Some(self.arena.intern_call(self.heads.times, &body)),
                    };
                    return Ok((coefficient, body));
                }
                Ok((Rat::one(), Some(id)))
            }
            Node::Symbol(_) => Ok((Rat::one(), Some(id))),
        }
    }

    fn collect_factor(
        &mut self,
        id: ExprId,
        coefficient: &mut Rat,
        bases: &mut HashMap<ExprId, Rat, FnvBuildHasher>,
    ) -> Result<(), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let value = Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?;
                *coefficient = coefficient.mul(&value);
            }
            Node::Rat(value) => {
                *coefficient = coefficient.mul(value);
            }
            Node::Real(_) => self.add_base(bases, id, Rat::one()),
            Node::Symbol(_) => self.add_base(bases, id, Rat::one()),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    *coefficient = coefficient.neg();
                    return self.collect_factor(args[0], coefficient, bases);
                }
                if head == self.heads.times {
                    for arg in args {
                        self.collect_factor(arg, coefficient, bases)?;
                    }
                    return Ok(());
                }
                if head == self.heads.power
                    && args.len() == 2
                    && let Some(exponent) = self.as_rat(args[1])
                {
                    if !exponent.is_zero() {
                        self.add_base(bases, args[0], exponent);
                    }
                    return Ok(());
                }
                self.add_base(bases, id, Rat::one());
            }
        }
        Ok(())
    }

    fn add_base(
        &self,
        bases: &mut HashMap<ExprId, Rat, FnvBuildHasher>,
        base: ExprId,
        exponent: Rat,
    ) {
        match bases.get_mut(&base) {
            Some(value) => *value = value.add(&exponent),
            None => {
                bases.insert(base, exponent);
            }
        }
    }

    fn term(&mut self, body: ExprId, coefficient: Rat) -> ExprId {
        if coefficient.is_one() {
            return body;
        }
        if coefficient.sign() < 0 {
            let magnitude = coefficient.abs();
            if magnitude.is_one() {
                return self.arena.intern_call(self.heads.negate, &[body]);
            }
            let coefficient_node = self.arena.intern_rat(magnitude);
            let product = self
                .arena
                .intern_call(self.heads.times, &[coefficient_node, body]);
            return self.arena.intern_call(self.heads.negate, &[product]);
        }
        let coefficient_node = self.arena.intern_rat(coefficient);
        self.arena
            .intern_call(self.heads.times, &[coefficient_node, body])
    }

    fn imaginary_power(&mut self, power: i64) -> ExprId {
        let remainder = power.rem_euclid(4);
        match remainder {
            0 => self.arena.intern_int(Int::one()),
            1 => self.heads.imaginary,
            2 => self.arena.intern_int(Int::from_i64(-1)),
            _ => self
                .arena
                .intern_call(self.heads.negate, &[self.heads.imaginary]),
        }
    }

    fn gaussian(&self, id: ExprId) -> Option<(Rat, Rat)> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let real = Rat::from_ints(value.clone(), Int::one()).ok()?;
                Some((real, Rat::zero()))
            }
            Node::Rat(value) => Some((value.clone(), Rat::zero())),
            Node::Real(_) => None,
            Node::Symbol(_) => {
                if id == self.heads.imaginary {
                    Some((Rat::zero(), Rat::one()))
                } else {
                    None
                }
            }
            Node::Call { head, args } => {
                let head = *head;
                if head == self.heads.plus {
                    let mut real = Rat::zero();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        real = real.add(&arg_real);
                        imaginary = imaginary.add(&arg_imaginary);
                    }
                    Some((real, imaginary))
                } else if head == self.heads.times {
                    let mut real = Rat::one();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        let next_real = real.mul(&arg_real).sub(&imaginary.mul(&arg_imaginary));
                        let next_imaginary =
                            real.mul(&arg_imaginary).add(&imaginary.mul(&arg_real));
                        real = next_real;
                        imaginary = next_imaginary;
                    }
                    Some((real, imaginary))
                } else if head == self.heads.negate && args.len() == 1 {
                    let (real, imaginary) = self.gaussian(args[0])?;
                    Some((real.neg(), imaginary.neg()))
                } else if head == self.heads.power && args.len() == 2 {
                    let exponent = match self.arena.node(args[1]) {
                        Node::Int(value) => value.to_i64()?,
                        Node::Rat(value) if value.den().is_one() => value.num().to_i64()?,
                        _ => return None,
                    };
                    let (real, imaginary) = self.gaussian(args[0])?;
                    gaussian_pow(&real, &imaginary, exponent)
                } else {
                    None
                }
            }
        }
    }

    fn gaussian_node(&mut self, real: &Rat, imaginary: &Rat) -> ExprId {
        if imaginary.is_zero() {
            return self.arena.intern_rat(real.clone());
        }
        let imaginary_part = self.term(self.heads.imaginary, imaginary.clone());
        if real.is_zero() {
            imaginary_part
        } else {
            let real_part = self.arena.intern_rat(real.clone());
            self.arena
                .intern_call(self.heads.plus, &[real_part, imaginary_part])
        }
    }

    fn as_rat(&self, id: ExprId) -> Option<Rat> {
        match self.arena.node(id) {
            Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            Node::Rat(value) => Some(value.clone()),
            Node::Real(_) => None,
            _ => None,
        }
    }

    fn is_error(&self, id: ExprId) -> bool {
        id == self.heads.undefined || id == self.heads.indeterminate || id == self.heads.failed
    }

    fn error_value(&self, args: &[ExprId]) -> Option<ExprId> {
        let mut undefined = false;
        let mut failed = false;
        for arg in args {
            if *arg == self.heads.indeterminate {
                return Some(self.heads.indeterminate);
            }
            if *arg == self.heads.undefined {
                undefined = true;
            } else if *arg == self.heads.failed {
                failed = true;
            }
        }
        if undefined {
            Some(self.heads.undefined)
        } else if failed {
            Some(self.heads.failed)
        } else {
            None
        }
    }
}

fn require_arity(args: &[ExprId], arity: usize, message: &str) -> Result<(), Diagnostic> {
    if args.len() == arity {
        Ok(())
    } else {
        Err(Diagnostic::eval(ErrorCode::EvalArity, message))
    }
}

fn sign_of(value: i8) -> Sign {
    match value.cmp(&0) {
        core::cmp::Ordering::Less => Sign::Negative,
        core::cmp::Ordering::Equal => Sign::Zero,
        core::cmp::Ordering::Greater => Sign::Positive,
    }
}

fn flip_sign(sign: Sign) -> Sign {
    match sign {
        Sign::Negative => Sign::Positive,
        Sign::Zero => Sign::Zero,
        Sign::Positive => Sign::Negative,
        Sign::NonNegative => Sign::NonPositive,
        Sign::NonPositive => Sign::NonNegative,
    }
}

fn multiply_signs(left: Sign, right: Sign) -> Option<Sign> {
    let left = match left {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
        _ => return None,
    };
    let right = match right {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
        _ => return None,
    };
    Some(sign_of((left * right) as i8))
}

fn interval_contains_phase(low: f64, high: f64, phase: f64, period: f64) -> bool {
    let first = ((low - phase) / period).ceil();
    let last = ((high - phase) / period).floor();
    first <= last
}

fn sign_product(left: Sign, right: Sign) -> Option<Sign> {
    multiply_signs(left, right)
}

fn is_zero_value(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.is_zero(),
        Node::Rat(value) => value.is_zero(),
        Node::Real(value) => value.sign() == 0,
        _ => false,
    }
}

fn is_one_value(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.to_i64() == Some(1),
        Node::Rat(value) => value.num().to_i64() == Some(1) && value.den().is_one(),
        Node::Real(value) => value.to_f64() == 1.0,
        _ => false,
    }
}

fn nonnegative_integer(arena: &Arena, expression: ExprId) -> Option<u32> {
    let value = match arena.node(expression) {
        Node::Int(value) => value.to_i64()?,
        Node::Rat(value) if value.den().is_one() => value.num().to_i64()?,
        _ => return None,
    };
    if value < 0 || value > u32::MAX as i64 {
        return None;
    }
    Some(value as u32)
}

fn integer_sqrt(value: &Int) -> Option<Int> {
    let value = value.to_i64()?;
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(Int::from_i64(middle));
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

fn exact_integer(arena: &Arena, expression: ExprId) -> Option<Int> {
    match arena.node(expression) {
        Node::Int(value) => Some(value.clone()),
        Node::Rat(value) if value.den().is_one() => Some(value.num()),
        _ => None,
    }
}

fn exact_rational(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

fn integer_lcm(left: &Int, right: &Int) -> Int {
    if left.is_zero() || right.is_zero() {
        return Int::zero();
    }
    let gcd = left.gcd(right);
    left.mul(right)
        .divrem(&gcd)
        .map(|(quotient, _)| quotient.abs())
        .unwrap_or_else(|_| Int::one())
}

fn integer_factorial(value: i64) -> Option<Int> {
    let mut result = Int::one();
    let mut factor = 2i64;
    while factor <= value {
        result = result.mul(&Int::from_i64(factor));
        factor += 1;
    }
    Some(result)
}

fn integer_binomial(n: i64, k: i64) -> Option<Int> {
    if k > n {
        return Some(Int::zero());
    }
    let k = k.min(n - k);
    if k > 100_000 {
        return None;
    }
    let mut result = Int::one();
    let mut index = 1i64;
    while index <= k {
        result = result.mul(&Int::from_i64(n - k + index));
        let (quotient, remainder) = result.divrem(&Int::from_i64(index)).ok()?;
        if !remainder.is_zero() {
            return None;
        }
        result = quotient;
        index += 1;
    }
    Some(result)
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

fn numeric_add(left: &Numeric, right: &Numeric) -> Option<Numeric> {
    match (left, right) {
        (Numeric::Error(value), _) | (_, Numeric::Error(value)) => Some(Numeric::Error(*value)),
        (Numeric::Real(a), Numeric::Real(b)) => a.add(b).ok().map(Numeric::Real),
        (Numeric::Real(a), Numeric::Complex(c, d)) => {
            let real = a.add(c).ok()?;
            Some(Numeric::Complex(real, d.clone()))
        }
        (Numeric::Complex(c, d), Numeric::Real(a)) => {
            let real = c.add(a).ok()?;
            Some(Numeric::Complex(real, d.clone()))
        }
        (Numeric::Complex(a, b), Numeric::Complex(c, d)) => {
            let real = a.add(c).ok()?;
            let imaginary = b.add(d).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
    }
}

fn numeric_mul(left: &Numeric, right: &Numeric) -> Option<Numeric> {
    match (left, right) {
        (Numeric::Error(value), _) | (_, Numeric::Error(value)) => Some(Numeric::Error(*value)),
        (Numeric::Real(a), Numeric::Real(b)) => a.mul(b).ok().map(Numeric::Real),
        (Numeric::Real(a), Numeric::Complex(c, d)) => {
            let real = a.mul(c).ok()?;
            let imaginary = a.mul(d).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
        (Numeric::Complex(c, d), Numeric::Real(a)) => {
            let real = c.mul(a).ok()?;
            let imaginary = d.mul(a).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
        (Numeric::Complex(a, b), Numeric::Complex(c, d)) => {
            let real = a.mul(c).ok()?.sub(&b.mul(d).ok()?).ok()?;
            let imaginary = a.mul(d).ok()?.add(&b.mul(c).ok()?).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
    }
}

fn numeric_neg(value: &Numeric) -> Option<Numeric> {
    match value {
        Numeric::Real(value) => Some(Numeric::Real(value.neg().ok()?)),
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Complex(real, imaginary) => {
            Some(Numeric::Complex(real.neg().ok()?, imaginary.neg().ok()?))
        }
    }
}

fn numeric_abs(value: &Numeric) -> Option<Numeric> {
    match value {
        Numeric::Real(value) => Some(Numeric::Real(value.abs().ok()?)),
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Complex(real, imaginary) => {
            let complex = ComplexMpc::from_parts(real, imaginary)?;
            Some(Numeric::Real(complex.abs()?))
        }
    }
}

fn numeric_complex_pow(base: &(Real, Real), exponent: &(Real, Real)) -> Option<Numeric> {
    let base = ComplexMpc::from_parts(&base.0, &base.1)?;
    let exponent = ComplexMpc::from_parts(&exponent.0, &exponent.1)?;
    let (real, imaginary) = base.pow(&exponent)?;
    Some(Numeric::Complex(real, imaginary))
}

fn numeric_complex_div(
    numerator: &(Real, Real),
    denominator: &(Real, Real),
    digits: u32,
) -> Option<Numeric> {
    let (a, b) = numerator;
    let (c, d) = denominator;
    let c_abs = c.abs().ok()?;
    let d_abs = d.abs().ok()?;
    if c.is_zero() && d.is_zero() {
        return None;
    }
    let (real, imaginary) = if c_abs.cmp(&d_abs) != Ordering::Less {
        let ratio = d.div(c).ok()?;
        let real_denominator = c.add(&d.mul(&ratio).ok()?).ok()?;
        if real_denominator.is_zero() {
            return None;
        }
        let real_numerator = a.add(&b.mul(&ratio).ok()?).ok()?;
        let imaginary_numerator = b.sub(&a.mul(&ratio).ok()?).ok()?;
        (
            real_numerator.div(&real_denominator).ok()?,
            imaginary_numerator.div(&real_denominator).ok()?,
        )
    } else {
        let ratio = c.div(d).ok()?;
        let real_denominator = c.mul(&ratio).ok()?.add(d).ok()?;
        if real_denominator.is_zero() {
            return None;
        }
        let real_numerator = a.mul(&ratio).ok()?.add(b).ok()?;
        let imaginary_numerator = b.mul(&ratio).ok()?.sub(a).ok()?;
        (
            real_numerator.div(&real_denominator).ok()?,
            imaginary_numerator.div(&real_denominator).ok()?,
        )
    };
    let _ = digits;
    Some(Numeric::Complex(real, imaginary))
}

fn numeric_pow(value: &Numeric, exponent: i64, digits: u32) -> Option<Numeric> {
    if exponent == 0 {
        return Some(Numeric::Real(Real::one(digits).ok()?));
    }
    match value {
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Real(value) => {
            if value.is_zero() && exponent < 0 {
                return None;
            }
            Some(Numeric::Real(value.pow_si(exponent).ok()?))
        }
        Numeric::Complex(real, imaginary) => {
            let mut power = exponent.unsigned_abs();
            let inverted = exponent < 0;
            let mut base = Numeric::Complex(real.clone(), imaginary.clone());
            if inverted {
                let one = (Real::one(digits).ok()?, Real::zero(digits).ok()?);
                base = numeric_complex_div(&one, &(real.clone(), imaginary.clone()), digits)?;
            }
            let mut result = Numeric::Complex(Real::one(digits).ok()?, Real::zero(digits).ok()?);
            while power > 0 {
                if power & 1 == 1 {
                    result = numeric_mul(&result, &base)?;
                }
                power >>= 1;
                if power > 0 {
                    base = numeric_mul(&base, &base)?;
                }
            }
            Some(result)
        }
    }
}

fn gaussian_pow(real: &Rat, imaginary: &Rat, power: i64) -> Option<(Rat, Rat)> {
    if power == 0 {
        return Some((Rat::one(), Rat::zero()));
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut base = (real.clone(), imaginary.clone());
    let mut result = (Rat::one(), Rat::zero());
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = gaussian_mul(&result, &base);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = gaussian_mul(&base, &base);
        }
    }
    if negative {
        let norm = result.0.mul(&result.0).add(&result.1.mul(&result.1));
        if norm.is_zero() {
            return None;
        }
        let inverse = Rat::one().div(&norm).ok()?;
        Some((result.0.mul(&inverse), result.1.neg().mul(&inverse)))
    } else {
        Some(result)
    }
}

fn gaussian_mul(left: &(Rat, Rat), right: &(Rat, Rat)) -> (Rat, Rat) {
    let real = left.0.mul(&right.0).sub(&left.1.mul(&right.1));
    let imaginary = left.0.mul(&right.1).add(&left.1.mul(&right.0));
    (real, imaginary)
}

fn rat_pow(base: &Rat, power: i64) -> Result<Rat, ()> {
    if power == 0 {
        return Ok(Rat::one());
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut result = Rat::one();
    let mut factor = base.clone();
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.mul(&factor);
        }
        exponent >>= 1;
        if exponent > 0 {
            factor = factor.mul(&factor);
        }
    }
    if negative {
        if result.is_zero() {
            return Err(());
        }
        Rat::one().div(&result).map_err(|_| ())
    } else {
        Ok(result)
    }
}
