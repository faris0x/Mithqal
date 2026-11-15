//! Limit, series, and convergence evaluation helpers.

use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn apply_limit(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
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

    pub(super) fn apply_converges(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
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

    pub(super) fn apply_series(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
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

    pub(super) fn series_term(
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

    pub(super) fn build_delta(&mut self, variable: ExprId, point: ExprId) -> ExprId {
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

    pub(super) fn limit_infinity(
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

    pub(super) fn decays_to_zero(
        &mut self,
        factors: &[ExprId],
        variable: ExprId,
        positive: bool,
    ) -> bool {
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

    pub(super) fn is_decaying_exponential(
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

    pub(super) fn exponential_slope(&mut self, argument: ExprId, variable: ExprId) -> Option<Rat> {
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

    pub(super) fn is_logarithm_expression(&self, expression: ExprId) -> bool {
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

    pub(super) fn logarithm_over_power_decays(
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

    pub(super) fn is_negative_variable_power(&self, expression: ExprId, variable: ExprId) -> bool {
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

    pub(super) fn is_polynomial_in(&mut self, expression: ExprId, variable: ExprId) -> bool {
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
        let symbol = self.arena.symbol_or_intern(name);
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
        let symbol = self.arena.symbol_or_intern("inf");
        self.arena.symbol_expr(symbol)
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
        let original = self.arena.intern_call(head, args);
        Ok(operation(self.arena, args[0], &context).unwrap_or(original))
    }
}
