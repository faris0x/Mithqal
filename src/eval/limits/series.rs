//! Series helpers.

use super::*;

impl<'a> Evaluator<'a> {
    pub(in crate::eval) fn apply_converges(
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

    pub(in crate::eval) fn apply_series(
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
        let evaluate_factorials = if args.len() == 5 {
            match self.boolean_value(args[4]) {
                Some(value) => value,
                None => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "series expects a boolean factorial flag",
                    ));
                }
            }
        } else {
            false
        };
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
            if !is_zero_value(self.arena, coefficient) {
                let term =
                    self.series_term(coefficient, delta, degree, &factorial, evaluate_factorials);
                terms.push(term);
            }
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
        if terms.is_empty() {
            return Ok(self.arena.intern_int(Int::zero()));
        }
        Ok(self.arena.intern_call(self.heads.series_sum, &terms))
    }

    pub(super) fn series_term(
        &mut self,
        coefficient: ExprId,
        delta: ExprId,
        degree: u32,
        factorial: &Int,
        evaluate_factorials: bool,
    ) -> ExprId {
        let negative = self.expression_sign(coefficient) == Some(Sign::Negative);
        let coefficient = if negative {
            let negated = self.arena.intern_call(self.heads.negate, &[coefficient]);
            self.eval(negated).unwrap_or(coefficient)
        } else {
            coefficient
        };
        let mut coefficient_factor = Some(coefficient);
        let mut include_factorial = false;
        if degree > 1 {
            if evaluate_factorials {
                coefficient_factor = None;
            } else if let Some(value) = exact_rational(self.arena, coefficient) {
                let factorial_value = Rat::from_ints(factorial.clone(), Int::one()).ok();
                if let Some(factorial_value) = factorial_value
                    && let Ok(ratio) = value.div(&factorial_value)
                    && ratio.den().is_one()
                {
                    coefficient_factor = Some(self.rat_expression(&ratio));
                } else {
                    include_factorial = true;
                }
            } else {
                include_factorial = true;
            }
        }
        let mut factors = Vec::with_capacity(3);
        if let Some(coefficient) = coefficient_factor
            && !is_one_value(self.arena, coefficient)
        {
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
        if evaluate_factorials && degree > 1 {
            let factorial = Rat::from_ints(factorial.clone(), Int::one()).ok();
            if let Some(factorial) = factorial
                && let Ok(reciprocal) = Rat::one().div(&factorial)
                && !reciprocal.is_one()
            {
                factors.push(self.rat_expression(&reciprocal));
            }
        } else if include_factorial {
            let value = self.arena.intern_int(Int::from_i64(degree as i64));
            let factorial = self.arena.intern_call(self.heads.factorial, &[value]);
            let exponent = self.arena.intern_int(Int::from_i64(-1));
            factors.push(
                self.arena
                    .intern_call(self.heads.power, &[factorial, exponent]),
            );
        }
        let term = match factors.len() {
            0 => self.arena.intern_int(Int::one()),
            1 => factors[0],
            _ => self.arena.intern_call(self.heads.times, &factors),
        };
        if negative {
            self.arena.intern_call(self.heads.negate, &[term])
        } else {
            term
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
}
