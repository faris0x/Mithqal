//! Limit, series, and convergence evaluation helpers.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

mod infinity;
mod series;

impl<'a> Evaluator<'a> {
    pub(super) fn apply_limit(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let original = self.held_public_call(head, args);
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

    pub(super) fn limit_side(&self, expression: ExprId) -> Option<LimitSide> {
        let symbol = self.arena.symbol_id(expression)?;
        match self.arena.symbol_name(symbol) {
            "left" => Some(LimitSide::Left),
            "right" => Some(LimitSide::Right),
            _ => None,
        }
    }

    pub(super) fn limit_target(&self, expression: ExprId) -> LimitTarget {
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

    pub(super) fn limit_value(
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

    pub(super) fn limit_finite_side(
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

    pub(super) fn abs_quotient_limit(
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

    pub(super) fn abs_argument(&self, expression: ExprId) -> Option<ExprId> {
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

    pub(super) fn approach_sign(
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

    pub(super) fn composed_limit(
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

    pub(super) fn finite_substitution(
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

    pub(super) fn has_unknown_head(&self, expression: ExprId) -> bool {
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

    pub(super) fn is_invalid_finite_value(&self, expression: ExprId) -> bool {
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

    pub(super) fn cancel_expression(&mut self, expression: ExprId) -> ExprId {
        let context = self.algebra_context();
        crate::algebra::cancel(self.arena, expression, &context).unwrap_or(expression)
    }

    pub(super) fn split_quotient(&mut self, expression: ExprId) -> (ExprId, ExprId) {
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

    pub(super) fn negative_factor(&self, expression: ExprId) -> Option<(ExprId, i64)> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if *head != self.heads.power || args.len() != 2 {
            return None;
        }
        let exponent = self.negative_exponent(args[1])?;
        Some((args[0], exponent))
    }

    pub(super) fn negative_exponent(&self, expression: ExprId) -> Option<i64> {
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

    pub(super) fn negative_power(
        &mut self,
        base: ExprId,
        exponent: ExprId,
    ) -> Option<(ExprId, ExprId)> {
        let magnitude = self.negative_exponent(exponent)?;
        let positive = self.arena.intern_int(Int::from_i64(magnitude));
        Some((base, positive))
    }

    pub(super) fn first_nonzero_term(
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

    pub(super) fn quotient_limit_value(
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

    pub(super) fn reciprocal(&mut self, expression: ExprId) -> ExprId {
        let exponent = self.arena.intern_int(Int::from_i64(-1));
        self.arena
            .intern_call(self.heads.power, &[expression, exponent])
    }

    pub(super) fn negate_limit_value(&mut self, value: LimitValue) -> LimitValue {
        match value {
            LimitValue::Finite(value) => {
                let call = self.arena.intern_call(self.heads.negate, &[value]);
                LimitValue::Finite(self.eval(call).unwrap_or(call))
            }
            LimitValue::PositiveInfinity => LimitValue::NegativeInfinity,
            LimitValue::NegativeInfinity => LimitValue::PositiveInfinity,
        }
    }

    pub(super) fn add_limit_values(
        &mut self,
        left: LimitValue,
        right: LimitValue,
    ) -> Option<LimitValue> {
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

    pub(super) fn multiply_limit_values(
        &mut self,
        left: LimitValue,
        right: LimitValue,
    ) -> Option<LimitValue> {
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

    pub(super) fn apply_unary_limit(
        &mut self,
        name: &str,
        inner: LimitValue,
    ) -> Option<LimitValue> {
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

    pub(super) fn call_evaluated(&mut self, name: &str, args: &[ExprId]) -> Option<ExprId> {
        let attrs = crate::builtins::attributes(name).unwrap_or_default();
        let symbol = self.arena.intern_symbol(name, attrs);
        let head = self.arena.symbol_expr(symbol);
        let call = self.arena.intern_call(head, args);
        self.eval(call).ok()
    }

    pub(super) fn half_pi(&mut self) -> Option<ExprId> {
        self.pi_fraction(1, 2)
    }

    pub(super) fn pi_fraction(&mut self, numerator: i64, denominator: i64) -> Option<ExprId> {
        let pi = self.arena.builtin_head("pi");
        let factor = Rat::from_i64(numerator, denominator).ok()?;
        let factor = self.rat_expression(&factor);
        let value = self.arena.intern_call(self.heads.times, &[factor, pi]);
        self.eval(value).ok()
    }

    pub(super) fn definite_sign(&self, expression: ExprId) -> Option<Sign> {
        match self.expression_sign(expression)? {
            Sign::Zero => None,
            Sign::Positive | Sign::NonNegative => Some(Sign::Positive),
            Sign::Negative | Sign::NonPositive => Some(Sign::Negative),
        }
    }

    pub(super) fn limit_value_expression(&mut self, value: LimitValue) -> ExprId {
        match value {
            LimitValue::Finite(value) => value,
            LimitValue::PositiveInfinity => self.infinity_expression(),
            LimitValue::NegativeInfinity => {
                let infinity = self.infinity_expression();
                self.arena.intern_call(self.heads.negate, &[infinity])
            }
        }
    }

    pub(super) fn infinity_expression(&mut self) -> ExprId {
        self.arena.builtin_head("inf")
    }

    pub(super) fn rat_expression(&mut self, value: &Rat) -> ExprId {
        if value.den().is_one() {
            self.arena.intern_int(value.num())
        } else {
            self.arena.intern_rat(value.clone())
        }
    }

    pub(super) fn expression_contains(&self, expression: ExprId, variable: ExprId) -> bool {
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

    pub(super) fn substitute_bound(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        value: ExprId,
    ) -> ExprId {
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

    pub(super) fn contains_error_or_infinite(&self, expression: ExprId) -> bool {
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

    pub(super) fn calculus_context(&self) -> crate::calculus::Context {
        crate::calculus::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        }
    }

    pub(super) fn algebra_context(&self) -> crate::algebra::Context {
        crate::algebra::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        }
    }

    pub(super) fn algebra_unary(
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
        let call = self.arena.intern_call(head, args);
        let hold = self.arena.builtin_head("unevaluated");
        let original = self.arena.intern_call(hold, &[call]);
        Ok(operation(self.arena, args[0], &context).unwrap_or(original))
    }
}
