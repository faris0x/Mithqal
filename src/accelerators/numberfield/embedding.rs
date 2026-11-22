//! Exact and interval real embeddings of number field elements.
//!
//! Root isolation is exact, by Sturm sequences and rational bisection. Signs at
//! real embeddings are decided with rational intervals, so no floating point
//! enters the sign. Logarithmic enclosures for totally real fields use rational
//! interval arithmetic and only convert to `f64` at the end.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::super::dense::lcm;
use super::super::poly_z::ZPoly;
use super::element::FieldElement;
use super::field::NumberField;
use super::order::Order;
use super::sturm;

/// Isolating intervals for the real embeddings, in increasing order.
pub(crate) fn real_intervals(field: &NumberField) -> Option<Vec<(Rat, Rat)>> {
    sturm::real_root_intervals(field.defining_polynomial())
}

fn integer_polynomial(coefficients: &[Rat]) -> Option<ZPoly> {
    let mut denominator = Int::one();
    for value in coefficients {
        denominator = lcm(&denominator, &value.den());
    }
    let mut integers = Vec::with_capacity(coefficients.len());
    for value in coefficients {
        let scaled = value.mul(&Rat::from_ints(denominator.clone(), Int::one()).ok()?);
        if !scaled.den().is_one() {
            return None;
        }
        integers.push(scaled.num());
    }
    Some(ZPoly::new(integers))
}

/// Shrink an isolating interval toward the unique root it contains.
fn shrink(defining: &ZPoly, lower: &Rat, upper: &Rat) -> Option<(Rat, Rat)> {
    let coefficients = defining.to_rat_coeffs();
    let lower_sign = sturm::evaluate_at(&coefficients, lower).num_sign();
    let upper_sign = sturm::evaluate_at(&coefficients, upper).num_sign();
    if lower_sign == 0 || upper_sign == 0 || lower_sign == upper_sign {
        return None;
    }
    let middle = lower.add(upper).mul(&Rat::from_i64(1, 2).ok()?);
    let middle_sign = sturm::evaluate_at(&coefficients, &middle).num_sign();
    if middle_sign == 0 {
        return Some((middle.clone(), middle));
    }
    if middle_sign == lower_sign {
        Some((middle, upper.clone()))
    } else {
        Some((lower.clone(), middle))
    }
}

/// The sign of `element` at the real embedding with the given index.
///
/// The index follows the increasing order of the real roots of the defining
/// polynomial. Returns `None` when the embedding cannot be isolated or the
/// element vanishes there.
pub(crate) fn real_sign(order: &Order, element: &FieldElement, index: usize) -> Option<i8> {
    let field = order.field();
    let intervals = real_intervals(field)?;
    let (mut lower, mut upper) = intervals.get(index)?.clone();
    let element_coefficients = element.coefficients();
    let element_polynomial = integer_polynomial(element_coefficients)?;
    for _ in 0..512 {
        if lower.cmp(&upper) == core::cmp::Ordering::Equal {
            let value = sturm::evaluate_at(element_coefficients, &lower);
            return (value.num_sign() != 0).then(|| value.num_sign());
        }
        let at_lower = sturm::evaluate_at(element_coefficients, &lower);
        let at_upper = sturm::evaluate_at(element_coefficients, &upper);
        let lower_sign = at_lower.num_sign();
        let upper_sign = at_upper.num_sign();
        if lower_sign != 0 && lower_sign == upper_sign {
            let has_no_root = match element_polynomial.degree() {
                Some(0) => true,
                _ => matches!(
                    sturm::real_roots_between(&element_polynomial, &lower, &upper),
                    Some(0)
                ),
            };
            if has_no_root {
                return Some(lower_sign);
            }
        }
        let next = shrink(field.defining_polynomial(), &lower, &upper)?;
        lower = next.0;
        upper = next.1;
    }
    None
}

/// Interval arithmetic on closed rational intervals.
#[derive(Clone, Debug)]
pub(crate) struct Interval {
    /// Lower endpoint.
    pub(crate) lower: Rat,
    /// Upper endpoint.
    pub(crate) upper: Rat,
}

impl Interval {
    fn new(lower: Rat, upper: Rat) -> Self {
        if lower.cmp(&upper) == core::cmp::Ordering::Greater {
            Interval {
                lower: upper,
                upper: lower,
            }
        } else {
            Interval { lower, upper }
        }
    }

    fn add(&self, other: &Interval) -> Interval {
        Interval::new(self.lower.add(&other.lower), self.upper.add(&other.upper))
    }

    fn mul(&self, other: &Interval) -> Interval {
        let products = [
            self.lower.mul(&other.lower),
            self.lower.mul(&other.upper),
            self.upper.mul(&other.lower),
            self.upper.mul(&other.upper),
        ];
        let mut lower = products[0].clone();
        let mut upper = products[0].clone();
        for value in &products[1..] {
            if value.cmp(&lower) == core::cmp::Ordering::Less {
                lower = value.clone();
            }
            if value.cmp(&upper) == core::cmp::Ordering::Greater {
                upper = value.clone();
            }
        }
        Interval::new(lower, upper)
    }

    fn contains_zero(&self) -> bool {
        self.lower.sign() <= 0 && self.upper.sign() >= 0
    }

    fn absolute(&self) -> Option<Interval> {
        if self.contains_zero() {
            return None;
        }
        let left = self.lower.abs();
        let right = self.upper.abs();
        Some(Interval::new(left.clone(), right.clone()).normalize())
    }

    fn normalize(self) -> Interval {
        if self.lower.cmp(&self.upper) == core::cmp::Ordering::Greater {
            Interval {
                lower: self.upper,
                upper: self.lower,
            }
        } else {
            self
        }
    }

    fn width(&self) -> Rat {
        self.upper.sub(&self.lower)
    }

    fn magnitude(&self) -> Rat {
        let left = self.lower.abs();
        let right = self.upper.abs();
        if left.cmp(&right) == core::cmp::Ordering::Greater {
            left
        } else {
            right
        }
    }
}

fn evaluate_interval(coefficients: &[Rat], interval: &Interval) -> Interval {
    let mut result = Interval::new(Rat::zero(), Rat::zero());
    for coefficient in coefficients.iter().rev() {
        result = result.mul(interval);
        result = result.add(&Interval::new(coefficient.clone(), coefficient.clone()));
    }
    result
}

/// A logarithmic enclosure `(lower, upper)` for one real embedding.
///
/// Only real embeddings are covered. Each bound is rigorous up to outward
/// floating point rounding of the rational endpoint logarithms.
pub(crate) fn real_log_intervals(
    order: &Order,
    element: &FieldElement,
    index: usize,
) -> Option<(f64, f64)> {
    let field = order.field();
    let intervals = real_intervals(field)?;
    let (mut lower, mut upper) = intervals.get(index)?.clone();
    let coefficients = element.coefficients();
    for _ in 0..512 {
        let value = evaluate_interval(coefficients, &Interval::new(lower.clone(), upper.clone()));
        if let Some(absolute) = value.absolute() {
            let scale = absolute.magnitude();
            let width = value.width();
            let scale_f64 = rational_to_f64(&scale)?;
            let width_f64 = rational_to_f64(&width)?;
            if width_f64 <= 1e-12 * (1.0 + scale_f64) {
                let low = rational_to_f64(&absolute.lower)?;
                let high = rational_to_f64(&absolute.upper)?;
                if low > 0.0 && high >= low {
                    let low_log = low.ln();
                    let high_log = high.ln();
                    let padding = 1e-12 * (1.0 + low_log.abs().max(high_log.abs()));
                    return Some((low_log - padding, high_log + padding));
                }
            }
        }
        let next = shrink(field.defining_polynomial(), &lower, &upper)?;
        if next.0.cmp(&next.1) == core::cmp::Ordering::Equal {
            return None;
        }
        lower = next.0;
        upper = next.1;
    }
    None
}

fn rational_to_f64(value: &Rat) -> Option<f64> {
    value.to_f64().is_finite().then(|| value.to_f64())
}

#[cfg(test)]
mod tests {
    use super::super::integral_basis::maximal_order;
    use super::*;
    use crate::num::Int;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| Int::from_i64(*value))
                .collect(),
        )
    }

    #[test]
    fn real_signs_are_exact() {
        let field = NumberField::new(poly(&[-2, 0, 1])).unwrap();
        let order = maximal_order(&field).unwrap();
        let generator = field.element(vec![Rat::zero(), Rat::one()]);
        assert_eq!(real_sign(&order, &generator, 0), Some(-1));
        assert_eq!(real_sign(&order, &generator, 1), Some(1));
        let one = field.one();
        assert_eq!(real_sign(&order, &one, 0), Some(1));
        assert_eq!(real_sign(&order, &one, 1), Some(1));
        let negative = field.negate(&one);
        assert_eq!(real_sign(&order, &negative, 0), Some(-1));
    }

    #[test]
    fn logarithm_intervals_bracket_numeric_values() {
        let field = NumberField::new(poly(&[-2, 0, 1])).unwrap();
        let order = maximal_order(&field).unwrap();
        let generator = field.element(vec![Rat::zero(), Rat::one()]);
        let (low, high) = real_log_intervals(&order, &generator, 1).unwrap();
        let expected = 2.0f64.sqrt().ln();
        assert!(low <= expected && expected <= high);
        let (low, high) = real_log_intervals(&order, &generator, 0).unwrap();
        let expected = 2.0f64.sqrt().ln();
        assert!(low <= expected && expected <= high);
    }
}
