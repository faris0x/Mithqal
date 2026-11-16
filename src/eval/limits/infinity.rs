//! Infinity helpers.

use super::*;

impl<'a> Evaluator<'a> {
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
}
