//! Exact polynomial algebra.
//!
//! Polynomials are sparse maps from canonical monomials to rational
//! coefficients. Monomials are sorted vectors of symbol and exponent pairs, so
//! equality and ordering need no expression hashing.
//!
//! Factoring combines numeric and monomial content extraction, univariate
//! rational roots and square-free decomposition, Hensel lifting, staged
//! multivariate interpolation, and grouping. Cancellation uses polynomial
//! gcds and monomial content.

#[allow(unused_imports)]
use crate::prelude::*;

mod dense;
mod factor;
mod hensel;
mod hensel_fast;
mod multivariate;
mod polynomial;
mod rational;
mod roots;
mod solve;

pub(crate) use dense::*;
pub(crate) use factor::{
    extract_common_factor, factor, factor_by_grouping, factor_dense_components, polynomial_gcd,
    resultant,
};
pub(crate) use polynomial::*;
pub(crate) use rational::{cancel, together};
pub(crate) use roots::*;
pub(crate) use solve::solve;

use rational::*;

use crate::collections::{BTreeMap, BTreeSet};

use crate::expr::{Arena, ExprId, Node, SymbolId};
use crate::num::{Int, Rat};

/// Expression heads owned by the evaluator and reused by the algebra layer.
#[derive(Copy, Clone)]
pub(crate) struct Context {
    pub(crate) plus: ExprId,
    pub(crate) times: ExprId,
    pub(crate) power: ExprId,
    pub(crate) negate: ExprId,
    pub(crate) list: ExprId,
    pub(crate) equal: ExprId,
}

fn variable_argument(arena: &Arena, expression: ExprId) -> Option<SymbolId> {
    arena.symbol_id(expression)
}

fn determinant(mut matrix: Vec<Vec<Rat>>) -> Rat {
    let mut result = Rat::one();
    let size = matrix.len();
    for column in 0..size {
        let Some(pivot_row) = (column..size).find(|row| !matrix[*row][column].is_zero()) else {
            return Rat::zero();
        };
        if pivot_row != column {
            matrix.swap(pivot_row, column);
            result = result.neg();
        }
        let pivot = matrix[column][column].clone();
        result = result.mul(&pivot);
        for row in (column + 1)..size {
            if matrix[row][column].is_zero() {
                continue;
            }
            let factor = match matrix[row][column].div(&pivot) {
                Ok(value) => value,
                Err(_) => return Rat::zero(),
            };
            let pivot = matrix[column].clone();
            for (index, value) in matrix[row].iter_mut().enumerate().skip(column) {
                *value = value.sub(&factor.mul(&pivot[index]));
            }
        }
    }
    result
}

pub(crate) fn degree(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let poly = poly_from_expr(arena, context, expression)?;
    if poly.is_zero() {
        return None;
    }
    let coefficients = poly.dense_in(variable)?;
    Some(arena.intern_int(Int::from_i64(coefficients.len() as i64 - 1)))
}

pub(crate) fn coefficient(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    index: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let index = integer_value(arena, index)?;
    if index < 0 {
        return None;
    }
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let value = coefficients
        .get(index as usize)
        .cloned()
        .unwrap_or_else(Rat::zero);
    Some(expression_from_rat(arena, &value))
}

pub(crate) fn polynomial_divide(
    arena: &mut Arena,
    dividend: ExprId,
    divisor: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let dividend = poly_from_expr(arena, context, dividend)?.dense_in(variable)?;
    let divisor = poly_from_expr(arena, context, divisor)?.dense_in(variable)?;
    if divisor.is_empty() {
        return None;
    }
    let (quotient, remainder) = dense_div_rem(&dividend, &divisor)?;
    let quotient = expression_from_poly(arena, &Poly::from_dense(variable, &quotient), context);
    let remainder = expression_from_poly(arena, &Poly::from_dense(variable, &remainder), context);
    let list = arena.builtin_head("list");
    Some(arena.intern_call(list, &[quotient, remainder]))
}

pub(crate) fn discriminant(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let degree = coefficients.len().saturating_sub(1);
    let value = match degree {
        0 | 1 => Rat::one(),
        2 => coefficients[1].mul(&coefficients[1]).sub(
            &Rat::from_i64(4, 1)
                .ok()?
                .mul(&coefficients[2])
                .mul(&coefficients[0]),
        ),
        3 => {
            let a = &coefficients[3];
            let b = &coefficients[2];
            let c = &coefficients[1];
            let d = &coefficients[0];
            let b2 = b.mul(b);
            let c2 = c.mul(c);
            let a2 = a.mul(a);
            let d2 = d.mul(d);
            let b3 = b2.mul(b);
            let c3 = c2.mul(c);
            let b2c2 = b2.mul(&c2);
            let ac3 = a.mul(&c3);
            let b3d = b3.mul(d);
            let a2d2 = a2.mul(&d2);
            let abcd = a.mul(b).mul(c).mul(d);
            b2c2.sub(&Rat::from_i64(4, 1).ok()?.mul(&ac3))
                .sub(&Rat::from_i64(4, 1).ok()?.mul(&b3d))
                .sub(&Rat::from_i64(27, 1).ok()?.mul(&a2d2))
                .add(&Rat::from_i64(18, 1).ok()?.mul(&abcd))
        }
        _ => return None,
    };
    Some(expression_from_rat(arena, &value))
}

/// The value of a rational function as its variable tends to infinity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RationalLimit {
    Finite(Rat),
    Infinite(i8),
}

/// Evaluate a rational expression at positive or negative infinity.
///
/// Returns `None` when the expression is not rational in the variable or the
/// denominator is identically zero.
pub(crate) fn rational_limit_at_infinity(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
    positive: bool,
) -> Option<RationalLimit> {
    let variable = variable_argument(arena, variable)?;
    let function = cancel_function(&rational_from_expr(arena, context, expression)?);
    if function.denominator.is_zero() {
        return None;
    }
    let numerator = function.numerator.dense_in(variable)?;
    let denominator = function.denominator.dense_in(variable)?;
    if numerator.is_empty() {
        return Some(RationalLimit::Finite(Rat::zero()));
    }
    if denominator.is_empty() {
        return None;
    }
    let numerator_degree = numerator.len() as i64 - 1;
    let denominator_degree = denominator.len() as i64 - 1;
    let numerator_leading = numerator.last()?;
    let denominator_leading = denominator.last()?;
    if numerator_degree < denominator_degree {
        return Some(RationalLimit::Finite(Rat::zero()));
    }
    if numerator_degree == denominator_degree {
        return numerator_leading
            .div(denominator_leading)
            .ok()
            .map(RationalLimit::Finite);
    }
    let ratio = numerator_leading.div(denominator_leading).ok()?;
    let difference = numerator_degree - denominator_degree;
    let sign = if !positive && difference % 2 != 0 {
        -ratio.sign()
    } else {
        ratio.sign()
    };
    Some(RationalLimit::Infinite(sign))
}

pub(crate) fn univariate_rational_root_multiplicities(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<Vec<(Rat, u32)>> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    Some(rational_roots(&coefficients).0)
}

pub(crate) fn univariate_rational_roots(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<(Vec<Rat>, Rat)> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let leading = coefficients.last()?.clone();
    let (roots, remaining) = rational_roots(&coefficients);
    if remaining.len() > 1 {
        return None;
    }
    if roots.iter().any(|(_, multiplicity)| *multiplicity != 1) {
        return None;
    }
    Some((roots.into_iter().map(|(root, _)| root).collect(), leading))
}
