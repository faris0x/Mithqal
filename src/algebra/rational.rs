//! Rational function construction and cancellation.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

#[derive(Clone)]
pub(crate) struct RationalFunction {
    pub(super) numerator: Poly,
    pub(super) denominator: Poly,
}

impl RationalFunction {
    fn one() -> Self {
        RationalFunction {
            numerator: Poly::constant(Rat::one()),
            denominator: Poly::constant(Rat::one()),
        }
    }

    fn add(&self, other: &RationalFunction) -> RationalFunction {
        RationalFunction {
            numerator: self
                .numerator
                .mul(&other.denominator)
                .add(&other.numerator.mul(&self.denominator)),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    fn mul(&self, other: &RationalFunction) -> RationalFunction {
        RationalFunction {
            numerator: self.numerator.mul(&other.numerator),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    fn neg(&self) -> RationalFunction {
        RationalFunction {
            numerator: self.numerator.neg(),
            denominator: self.denominator.clone(),
        }
    }

    fn pow(&self, exponent: i64) -> Option<RationalFunction> {
        let power = exponent.unsigned_abs() as u32;
        let numerator = self.numerator.pow_u32(power);
        let denominator = self.denominator.pow_u32(power);
        if exponent >= 0 {
            Some(RationalFunction {
                numerator,
                denominator,
            })
        } else {
            Some(RationalFunction {
                numerator: denominator,
                denominator: numerator,
            })
        }
    }
}

pub(crate) fn rational_from_expr(
    arena: &Arena,
    context: &Context,
    expression: ExprId,
) -> Option<RationalFunction> {
    match arena.node(expression) {
        Node::Int(_) | Node::Rat(_) | Node::Symbol(_) => Some(RationalFunction {
            numerator: poly_from_expr(arena, context, expression)?,
            denominator: Poly::constant(Rat::one()),
        }),
        Node::Real(_) | Node::String(_) | Node::Bytes(_) | Node::Association(_) => None,
        Node::Call { head, args } => {
            if *head == context.plus {
                let mut sum = RationalFunction {
                    numerator: Poly::zero(),
                    denominator: Poly::constant(Rat::one()),
                };
                for arg in args.iter() {
                    sum = sum.add(&rational_from_expr(arena, context, *arg)?);
                }
                Some(sum)
            } else if *head == context.times {
                let mut product = RationalFunction::one();
                for arg in args.iter() {
                    product = product.mul(&rational_from_expr(arena, context, *arg)?);
                }
                Some(product)
            } else if *head == context.negate {
                if args.len() != 1 {
                    return None;
                }
                Some(rational_from_expr(arena, context, args[0])?.neg())
            } else if *head == context.power {
                if args.len() != 2 {
                    return None;
                }
                let exponent = integer_value(arena, args[1])?;
                Some(rational_from_expr(arena, context, args[0])?.pow(exponent)?)
            } else {
                None
            }
        }
    }
}

pub(crate) fn cancel_function(function: &RationalFunction) -> RationalFunction {
    let mut numerator = function.numerator.clone();
    let mut denominator = function.denominator.clone();
    if numerator.is_zero() || denominator.is_zero() {
        return RationalFunction {
            numerator: Poly::zero(),
            denominator: Poly::constant(Rat::one()),
        };
    }

    if let (Some(left), Some(right)) =
        (numerator.monomial_content(), denominator.monomial_content())
    {
        let common = common_monomial(&left, &right);
        if !common.is_one() {
            if let Some(reduced) = numerator.divide_monomial(&common) {
                numerator = reduced;
            }
            if let Some(reduced) = denominator.divide_monomial(&common) {
                denominator = reduced;
            }
        }
    }

    let numerator_variables = numerator.variables();
    let denominator_variables = denominator.variables();
    if numerator_variables.len() == 1 && numerator_variables == denominator_variables {
        let variable = *numerator_variables.iter().next().expect("one variable");
        if let (Some(numerator_dense), Some(denominator_dense)) =
            (numerator.dense_in(variable), denominator.dense_in(variable))
        {
            let gcd = dense_gcd(&numerator_dense, &denominator_dense);
            if gcd.len() > 1 {
                if let Some(value) = divide_by_gcd(&numerator_dense, &gcd, variable) {
                    numerator = value;
                }
                if let Some(value) = divide_by_gcd(&denominator_dense, &gcd, variable) {
                    denominator = value;
                }
            }
        }
    }

    if let Some((_, coefficient)) = denominator.leading_term() {
        let coefficient = coefficient.clone();
        if !coefficient.is_zero()
            && !coefficient.is_one()
            && let Ok(inverse) = Rat::one().div(&coefficient)
        {
            numerator = numerator.scale(&inverse);
            denominator = denominator.scale(&inverse);
        }
    }
    RationalFunction {
        numerator,
        denominator,
    }
}

pub(super) fn divide_by_gcd(coefficients: &[Rat], gcd: &[Rat], variable: SymbolId) -> Option<Poly> {
    let (quotient, remainder) = dense_div_rem(coefficients, gcd)?;
    if remainder.is_empty() {
        Some(Poly::from_dense(variable, &quotient))
    } else {
        None
    }
}

pub(super) fn common_monomial(left: &Monomial, right: &Monomial) -> Monomial {
    let mut powers = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.powers.len() && right_index < right.powers.len() {
        match left.powers[left_index].0.cmp(&right.powers[right_index].0) {
            core::cmp::Ordering::Less => left_index += 1,
            core::cmp::Ordering::Greater => right_index += 1,
            core::cmp::Ordering::Equal => {
                let exponent = left.powers[left_index].1.min(right.powers[right_index].1);
                if exponent != 0 {
                    powers.push((left.powers[left_index].0, exponent));
                }
                left_index += 1;
                right_index += 1;
            }
        }
    }
    Monomial { powers }
}

pub(crate) fn rational_expression(
    arena: &mut Arena,
    function: &RationalFunction,
    context: &Context,
) -> ExprId {
    let numerator = expression_from_poly(arena, &function.numerator, context);
    if let Some(value) = function.denominator.constant_value() {
        if value.is_one() {
            return numerator;
        }
        let inverse = Rat::one().div(&value).ok().unwrap_or_else(Rat::one);
        let scaled = function.numerator.scale(&inverse);
        return expression_from_poly(arena, &scaled, context);
    }
    let denominator = expression_from_poly(arena, &function.denominator, context);
    let exponent = arena.intern_int(Int::from_i64(-1));
    let reciprocal = arena.intern_call(context.power, &[denominator, exponent]);
    product(arena, &[numerator, reciprocal], context)
}

pub(crate) fn together(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    let function = rational_from_expr(arena, context, expression)?;
    let function = cancel_function(&function);
    Some(rational_expression(arena, &function, context))
}

pub(crate) fn cancel(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    together(arena, expression, context)
}
