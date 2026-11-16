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
}
