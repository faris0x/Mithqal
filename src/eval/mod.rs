//! Fixed point evaluator.
//!
//! The current version implements exact arithmetic, listable threading, comparisons,
//! logic, and the boundary for definitions. Definitions and pattern matching
//! are added in the next layer.

mod definitions;
mod pattern;

use core::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::builtins;
use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
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
            _ => return None,
        })
    }
}

#[derive(Copy, Clone)]
enum CalculusHead {
    Differentiate,
    Integrate,
}

impl CalculusHead {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "differentiate" => Self::Differentiate,
            "integrate" => Self::Integrate,
            _ => return None,
        })
    }
}

struct Heads {
    plus: ExprId,
    times: ExprId,
    power: ExprId,
    negate: ExprId,
    list: ExprId,
    if_head: ExprId,
    and: ExprId,
    or: ExprId,
    not: ExprId,
    same: ExprId,
    equal: ExprId,
    not_equal: ExprId,
    less: ExprId,
    less_equal: ExprId,
    greater: ExprId,
    greater_equal: ExprId,
    true_value: ExprId,
    false_value: ExprId,
    undefined: ExprId,
    indeterminate: ExprId,
    failed: ExprId,
    imaginary: ExprId,
    set: ExprId,
    set_delayed: ExprId,
    let_head: ExprId,
    function: ExprId,
    closure: ExprId,
    approx: ExprId,
    interval: ExprId,
    complex: ExprId,
    pi: ExprId,
    e: ExprId,
    null: ExprId,
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
    let mut evaluator = Evaluator::new(arena, definitions);
    evaluator.max_steps = max_steps.max(1);
    evaluator.max_depth = max_depth.max(1);
    evaluator.eval(id)
}

struct Evaluator<'a> {
    arena: &'a mut Arena,
    definitions: &'a mut Definitions,
    heads: Heads,
    locals: Vec<HashMap<SymbolId, ExprId>>,
    steps: u64,
    max_steps: u64,
    depth: u32,
    max_depth: u32,
}

impl<'a> Evaluator<'a> {
    fn new(arena: &'a mut Arena, definitions: &'a mut Definitions) -> Self {
        let heads = Heads {
            plus: head(arena, "plus"),
            times: head(arena, "times"),
            power: head(arena, "power"),
            negate: head(arena, "negate"),
            list: head(arena, "list"),
            if_head: head(arena, "if"),
            and: head(arena, "and"),
            or: head(arena, "or"),
            not: head(arena, "not"),
            same: head(arena, "same"),
            equal: head(arena, "equal"),
            not_equal: head(arena, "notEqual"),
            less: head(arena, "less"),
            less_equal: head(arena, "lessEqual"),
            greater: head(arena, "greater"),
            greater_equal: head(arena, "greaterEqual"),
            true_value: head(arena, "true"),
            false_value: head(arena, "false"),
            undefined: head(arena, "undefined"),
            indeterminate: head(arena, "indeterminate"),
            failed: head(arena, "failed"),
            imaginary: head(arena, "i"),
            set: head(arena, "set"),
            set_delayed: head(arena, "setDelayed"),
            let_head: head(arena, "let"),
            function: head(arena, "function"),
            closure: head(arena, "closure"),
            approx: head(arena, "approx"),
            interval: head(arena, "interval"),
            complex: head(arena, "complex"),
            pi: head(arena, "pi"),
            e: head(arena, "e"),
            null: head(arena, "null"),
        };
        Evaluator {
            arena,
            definitions,
            heads,
            locals: Vec::new(),
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
            Node::Call { head, args } => (*head, args.to_vec()),
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
            return self.apply_closure(&closure, &args);
        }
        if !matches!(self.arena.node(head), Node::Symbol(_)) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalNotCallable,
                "a call head must evaluate to a symbol",
            ));
        }

        if head == self.heads.function {
            return self.eval_function(&args);
        }
        if head == self.heads.approx {
            return self.eval_approx(&args);
        }
        if head == self.heads.interval {
            return self.eval_interval(&args);
        }
        if head == self.heads.complex {
            return self.eval_complex(&args);
        }
        if head == self.heads.set {
            return self.eval_definition(&args, false);
        }
        if head == self.heads.set_delayed {
            return self.eval_definition(&args, true);
        }
        if head == self.heads.let_head {
            return self.eval_let(&args);
        }
        if head == self.heads.if_head {
            return self.eval_if(&args);
        }
        if head == self.heads.and {
            return self.eval_and(&args);
        }
        if head == self.heads.or {
            return self.eval_or(&args);
        }
        if head == self.heads.not {
            return self.eval_not(&args);
        }

        let attrs = self.arena.head_attributes(head);
        if attrs.has(Attributes::HOLD_ALL)
            || attrs.has(Attributes::HOLD_FIRST)
            || attrs.has(Attributes::HOLD_REST)
        {
            return Ok(id);
        }

        let mut evaluated = Vec::with_capacity(args.len());
        for arg in &args {
            evaluated.push(self.eval(*arg)?);
        }

        if head == self.heads.plus || head == self.heads.times || head == self.heads.negate {
            let candidate = self.arena.intern_call(head, &evaluated);
            if let Some((real, imaginary)) = self.gaussian(candidate) {
                return Ok(self.gaussian_node(&real, &imaginary));
            }
        }

        if attrs.has(Attributes::LISTABLE) {
            if let Some(result) = self.thread_listable(head, &evaluated)? {
                return self.finish(id, result);
            }
        }

        let result = self.apply(head, &evaluated)?;
        self.finish(id, result)
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
        {
            if low_value.cmp(&high_value) == Ordering::Greater {
                return Ok(self.heads.failed);
            }
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
        if let Some(value) = self.real_endpoint(imaginary) {
            if value.is_zero() {
                return Ok(real);
            }
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
                return self.approximate_rat(rational, digits);
            }
            Node::Rat(value) => return self.approximate_rat(value.clone(), digits),
            Node::Real(value) => return Ok(Some(Numeric::Real(value.clone()))),
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
                return Ok(None);
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
            NumericHead::Atanh => complex.atanh(),
            _ => None,
        }
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
                    _ => return Ok(None),
                };
                if let Ok(real) = result {
                    if real.is_finite() {
                        return Ok(Some(Numeric::Real(real)));
                    }
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
            if let Node::Call {
                head: list_head,
                args: items,
            } = self.arena.node(*arg)
            {
                if *list_head == self.heads.list {
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
            }
        }
        let Some(length) = length else {
            return Ok(None);
        };
        let mut results = Vec::with_capacity(length);
        for index in 0..length {
            let mut call_args = Vec::with_capacity(args.len());
            for arg in args {
                if let Node::Call {
                    head: list_head,
                    args: items,
                } = self.arena.node(*arg)
                {
                    if *list_head == self.heads.list {
                        call_args.push(items[index]);
                        continue;
                    }
                }
                call_args.push(*arg);
            }
            let call = self.arena.intern_call(head, &call_args);
            results.push(self.eval(call)?);
        }
        Ok(Some(self.arena.intern_call(self.heads.list, &results)))
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
        let algebra_head = self
            .arena
            .symbol_id(head)
            .and_then(|symbol| AlgebraHead::from_name(self.arena.symbol_name(symbol)));
        if let Some(algebra_head) = algebra_head {
            let algebra = crate::algebra::Context {
                plus: self.heads.plus,
                times: self.heads.times,
                power: self.heads.power,
                negate: self.heads.negate,
                list: self.heads.list,
                equal: self.heads.equal,
            };
            match algebra_head {
                AlgebraHead::Expand => {
                    return self.algebra_unary(args, head, algebra, crate::algebra::expand);
                }
                AlgebraHead::Factor => {
                    return self.algebra_unary(args, head, algebra, crate::algebra::factor);
                }
                AlgebraHead::Together => {
                    return self.algebra_unary(args, head, algebra, crate::algebra::together);
                }
                AlgebraHead::Cancel => {
                    return self.algebra_unary(args, head, algebra, crate::algebra::cancel);
                }
                AlgebraHead::PolynomialGcd => {
                    if args.len() != 3 {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalArity,
                            "polynomialGcd expects two polynomials and a variable",
                        ));
                    }
                    let original = self.arena.intern_call(head, args);
                    return Ok(crate::algebra::polynomial_gcd(
                        self.arena, args[0], args[1], args[2], &algebra,
                    )
                    .unwrap_or(original));
                }
                AlgebraHead::Resultant => {
                    if args.len() != 3 {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalArity,
                            "resultant expects two polynomials and a variable",
                        ));
                    }
                    let original = self.arena.intern_call(head, args);
                    return Ok(crate::algebra::resultant(
                        self.arena, args[0], args[1], args[2], &algebra,
                    )
                    .unwrap_or(original));
                }
                AlgebraHead::Solve => {
                    if args.len() != 2 {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalArity,
                            "solve expects an equation and a variable",
                        ));
                    }
                    let original = self.arena.intern_call(head, args);
                    return Ok(
                        crate::algebra::solve(self.arena, args[0], args[1], &algebra)
                            .unwrap_or(original),
                    );
                }
            }
        }
        if let Some(symbol) = self.arena.symbol_id(head) {
            if let Some(result) = self.apply_user(symbol, head, args)? {
                return Ok(result);
            }
        }
        let calculus_head = self
            .arena
            .symbol_id(head)
            .and_then(|symbol| CalculusHead::from_name(self.arena.symbol_name(symbol)));
        if let Some(calculus_head) = calculus_head {
            if args.len() != 2 {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalArity,
                    "calculus function expects an expression and a variable",
                ));
            }
            let original = self.arena.intern_call(head, args);
            if self.arena.symbol_id(args[1]).is_none() {
                return Ok(original);
            }
            let context = crate::calculus::Context {
                plus: self.heads.plus,
                times: self.heads.times,
                power: self.heads.power,
                negate: self.heads.negate,
                list: self.heads.list,
                equal: self.heads.equal,
            };
            let result = match calculus_head {
                CalculusHead::Differentiate => {
                    crate::calculus::differentiate(self.arena, args[0], args[1], &context)
                }
                CalculusHead::Integrate => {
                    crate::calculus::integrate(self.arena, args[0], args[1], &context)
                }
            };
            let result = result.map(|value| {
                if matches!(calculus_head, CalculusHead::Differentiate) {
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
            return Ok(result.unwrap_or(original));
        }
        Ok(self.arena.intern_call(head, args))
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
            if !match_pattern(&self.arena, rule.lhs, call, &mut bindings, &mut conditions) {
                continue;
            }
            let mut accepted = true;
            for condition in conditions {
                let condition = substitute(&mut self.arena, condition, &bindings);
                let value = self.eval(condition)?;
                if value != self.heads.true_value {
                    accepted = false;
                    break;
                }
            }
            if !accepted {
                continue;
            }
            let rhs = substitute(&mut self.arena, rule.rhs, &bindings);
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
        if args
            .iter()
            .any(|arg| matches!(self.arena.node(*arg), Node::Real(_)))
            && args.iter().all(|arg| {
                matches!(
                    self.arena.node(*arg),
                    Node::Int(_) | Node::Rat(_) | Node::Real(_)
                )
            })
        {
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
        let has_zero = args
            .iter()
            .any(|arg| self.as_rat(*arg).is_some_and(|v| v.is_zero()));
        if args.iter().any(|arg| *arg == self.heads.indeterminate) {
            return Ok(self.heads.indeterminate);
        }
        if args.iter().any(|arg| *arg == self.heads.undefined) {
            return if has_zero {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.heads.undefined)
            };
        }
        if args.iter().any(|arg| *arg == self.heads.failed) {
            return Ok(self.heads.failed);
        }
        if args
            .iter()
            .any(|arg| matches!(self.arena.node(*arg), Node::Real(_)))
            && args.iter().all(|arg| {
                matches!(
                    self.arena.node(*arg),
                    Node::Int(_) | Node::Rat(_) | Node::Real(_)
                )
            })
        {
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
        if let Node::Real(real) = self.arena.node(base) {
            if let Some(exp) = self.as_rat(exponent) {
                if exp.den().is_one() {
                    if let Some(power) = exp.num().to_i64() {
                        let value = if power == 0 {
                            Real::one(30)
                        } else {
                            real.pow_si(power)
                        };
                        if let Ok(value) = value {
                            return Ok(self.arena.intern_real(value));
                        }
                    }
                }
            }
        }
        if let (Some((low, high)), Some(exp)) = (self.interval_bounds(base), self.as_rat(exponent))
        {
            if exp.den().is_one() {
                if let Some(power) = exp.num().to_i64() {
                    if let Some(result) = self.interval_power(&low, &high, power) {
                        return Ok(result);
                    }
                }
            }
        }
        if let Some(exp) = self.as_rat(exponent) {
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
            if exp.den().is_one() {
                if let Some(power) = exp.num().to_i64() {
                    if let Some(base_value) = self.as_rat(base) {
                        if base_value.is_zero() {
                            return if power < 0 {
                                Ok(self.heads.undefined)
                            } else {
                                Ok(self.arena.intern_int(Int::zero()))
                            };
                        }
                        let value = rat_pow(&base_value, power).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "numeric power failed")
                        })?;
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
                    let nested = match self.arena.node(base) {
                        Node::Call { head, args }
                            if *head == self.heads.power && args.len() == 2 =>
                        {
                            Some((args[0], args[1]))
                        }
                        _ => None,
                    };
                    if let Some((inner_base, inner_exponent)) = nested {
                        if let Some(inner) = self.as_rat(inner_exponent) {
                            let combined = inner.mul(&exp);
                            let combined_node = self.arena.intern_rat(combined);
                            return Ok(self
                                .arena
                                .intern_call(self.heads.power, &[inner_base, combined_node]));
                        }
                    }
                    let negated_base = match self.arena.node(base) {
                        Node::Call { head, args }
                            if *head == self.heads.negate && args.len() == 1 =>
                        {
                            Some(args[0])
                        }
                        _ => None,
                    };
                    if let Some(inner_base) = negated_base {
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
                }
            } else {
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
            }
        } else if self.as_rat(base).is_some_and(|value| value.is_one()) {
            return Ok(self.arena.intern_int(Int::one()));
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
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
                if head == self.heads.power && args.len() == 2 {
                    if let Some(exponent) = self.as_rat(args[1]) {
                        if !exponent.is_zero() {
                            self.add_base(bases, args[0], exponent);
                        }
                        return Ok(());
                    }
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
        if args.iter().any(|arg| *arg == self.heads.indeterminate) {
            return Some(self.heads.indeterminate);
        }
        if args.iter().any(|arg| *arg == self.heads.undefined) {
            return Some(self.heads.undefined);
        }
        if args.iter().any(|arg| *arg == self.heads.failed) {
            return Some(self.heads.failed);
        }
        None
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

fn head(arena: &mut Arena, name: &str) -> ExprId {
    let attrs = builtins::attributes(name).unwrap_or_else(Attributes::new);
    let symbol = arena.intern_symbol(name, attrs);
    arena.symbol_expr(symbol)
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
