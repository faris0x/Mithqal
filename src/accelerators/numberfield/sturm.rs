//! Exact real root counts by Sturm sequences.
//!
//! The signature of a number field only needs the number of real embeddings.
//! A Sturm sequence over the rationals gives that count without numeric root
//! finding or interval refinement.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::Rat;

use super::super::dense::trim;
use super::super::poly_z::ZPoly;

fn div_rem(dividend: &[Rat], divisor: &[Rat]) -> Option<(Vec<Rat>, Vec<Rat>)> {
    let divisor = trim(divisor.to_vec());
    if divisor.is_empty() {
        return None;
    }
    let mut remainder = trim(dividend.to_vec());
    let degree = divisor.len() - 1;
    if remainder.len() <= degree {
        return Some((Vec::new(), remainder));
    }
    let leading = divisor.last()?.clone();
    let mut quotient = vec![Rat::zero(); remainder.len() - degree];
    while remainder.len() > degree {
        let position = remainder.len() - 1 - degree;
        let coefficient = remainder.last()?.div(&leading).ok()?;
        quotient[position] = coefficient.clone();
        for (index, value) in divisor.iter().enumerate() {
            let target = position + index;
            let updated = remainder[target].sub(&coefficient.mul(value));
            remainder[target] = updated;
        }
        remainder = trim(remainder);
    }
    Some((trim(quotient), remainder))
}

/// The number of distinct real roots of an integer polynomial.
///
/// Returns `None` for the zero polynomial, a nonconstant polynomial with zero
/// derivative, or an inconsistent Sturm sequence.
pub(crate) fn real_root_count(polynomial: &ZPoly) -> Option<usize> {
    if polynomial.is_zero() {
        return None;
    }
    if polynomial.degree() == Some(0) {
        return Some(0);
    }
    let sequence = sturm_sequence(polynomial)?;
    let positive = sign_variations(&sequence, true);
    let negative = sign_variations(&sequence, false);
    if negative < positive {
        return None;
    }
    Some(negative - positive)
}

/// A Sturm sequence for the squarefree part of a polynomial.
fn sturm_sequence(polynomial: &ZPoly) -> Option<Vec<Vec<Rat>>> {
    if polynomial.is_zero() || polynomial.degree() == Some(0) {
        return None;
    }
    let derivative = polynomial.derivative();
    if derivative.is_zero() {
        return None;
    }
    let shared = polynomial.gcd(&derivative);
    let squarefree = polynomial.divide_exact(&shared)?;
    let mut sequence = vec![
        trim(squarefree.to_rat_coeffs()),
        trim(squarefree.derivative().to_rat_coeffs()),
    ];
    while sequence.len() >= 2 {
        let last = sequence.last()?.clone();
        if last.is_empty() {
            break;
        }
        let previous = &sequence[sequence.len() - 2];
        let (_, remainder) = div_rem(previous, &last)?;
        if remainder.is_empty() {
            break;
        }
        let next = trim(remainder.iter().map(Rat::neg).collect());
        sequence.push(next);
    }
    Some(sequence)
}

/// Evaluate an ascending rational polynomial at a rational point.
pub(crate) fn evaluate_at(polynomial: &[Rat], value: &Rat) -> Rat {
    let mut result = Rat::zero();
    for coefficient in polynomial.iter().rev() {
        result = result.mul(value).add(coefficient);
    }
    result
}

fn variations_at(sequence: &[Vec<Rat>], value: &Rat) -> usize {
    let mut previous = 0i8;
    let mut count = 0usize;
    for polynomial in sequence {
        let sign = evaluate_at(polynomial, value).num_sign();
        if sign == 0 {
            continue;
        }
        if previous != 0 && sign != previous {
            count += 1;
        }
        previous = sign;
    }
    count
}

/// The number of distinct real roots in `(lower, upper)`.
///
/// The endpoints must not be roots of the polynomial. Returns `None` when the
/// polynomial is degenerate or an endpoint is a root.
pub(crate) fn real_roots_between(polynomial: &ZPoly, lower: &Rat, upper: &Rat) -> Option<usize> {
    if lower.cmp(upper) != core::cmp::Ordering::Less {
        return None;
    }
    let sequence = sturm_sequence(polynomial)?;
    if evaluate_at(sequence.first()?, lower).is_zero()
        || evaluate_at(sequence.first()?, upper).is_zero()
    {
        return None;
    }
    let left = variations_at(&sequence, lower);
    let right = variations_at(&sequence, upper);
    if right > left {
        return None;
    }
    Some(left - right)
}

/// Isolating intervals for the distinct real roots, in increasing order.
///
/// The intervals are disjoint, rational, and each contains exactly one root.
/// The construction uses exact Sturm counts and bisection only.
pub(crate) fn real_root_intervals(polynomial: &ZPoly) -> Option<Vec<(Rat, Rat)>> {
    if polynomial.is_zero() {
        return None;
    }
    if polynomial.degree() == Some(0) {
        return Some(Vec::new());
    }
    let sequence = sturm_sequence(polynomial)?;
    let total = real_root_count(polynomial)?;
    if total == 0 {
        return Some(Vec::new());
    }
    let bound = cauchy_bound(polynomial)?;
    let mut pending = vec![(bound.neg(), bound.clone())];
    let mut isolated: Vec<(Rat, Rat)> = Vec::new();
    for _ in 0..4096 {
        if pending.is_empty() {
            break;
        }
        let (lower, upper) = pending.pop()?;
        let count = interval_count(&sequence, &lower, &upper)?;
        if count == 0 {
            continue;
        }
        if count == 1 {
            isolated.push((lower, upper));
            continue;
        }
        let mut middle = lower.add(&upper).mul(&Rat::from_i64(1, 2).ok()?);
        let mut guard = 0;
        while evaluate_at(sequence.first()?, &middle).is_zero() {
            guard += 1;
            if guard > 32 {
                return None;
            }
            middle = middle.add(&Rat::from_i64(1, 1_000_003).ok()?);
        }
        pending.push((middle.clone(), upper));
        pending.push((lower, middle));
    }
    if !pending.is_empty() || isolated.len() != total {
        return None;
    }
    isolated.sort_by(|left, right| left.0.cmp(&right.0));
    Some(isolated)
}

fn interval_count(sequence: &[Vec<Rat>], lower: &Rat, upper: &Rat) -> Option<usize> {
    let left = variations_at(sequence, lower);
    let right = variations_at(sequence, upper);
    left.checked_sub(right)
}

fn cauchy_bound(polynomial: &ZPoly) -> Option<Rat> {
    let coefficients = polynomial.coeffs.clone();
    let leading = coefficients.last()?.abs();
    if leading.is_zero() {
        return None;
    }
    let mut maximum = Rat::zero();
    for coefficient in &coefficients[..coefficients.len() - 1] {
        let ratio = Rat::from_ints(coefficient.abs(), leading.clone()).ok()?;
        if ratio.cmp(&maximum) == core::cmp::Ordering::Greater {
            maximum = ratio;
        }
    }
    Some(maximum.add(&Rat::one()))
}

fn sign_variations(sequence: &[Vec<Rat>], positive: bool) -> usize {
    let mut previous = 0i8;
    let mut count = 0usize;
    for polynomial in sequence {
        let Some(leading) = polynomial.last() else {
            continue;
        };
        let mut sign = leading.sign();
        if !positive && (polynomial.len() - 1) % 2 == 1 {
            sign = -sign;
        }
        if sign == 0 {
            continue;
        }
        if previous != 0 && sign != previous {
            count += 1;
        }
        previous = sign;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| crate::num::Int::from_i64(*value))
                .collect(),
        )
    }

    #[test]
    fn quadratic_real_root_counts() {
        assert_eq!(real_root_count(&poly(&[1, 0, 1])), Some(0));
        assert_eq!(real_root_count(&poly(&[-2, 0, 1])), Some(2));
        assert_eq!(real_root_count(&poly(&[1, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[0, 1])), Some(1));
    }

    #[test]
    fn cubic_and_quartic_real_root_counts() {
        assert_eq!(real_root_count(&poly(&[-2, 0, 0, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[0, -1, 0, 1])), Some(3));
        assert_eq!(real_root_count(&poly(&[-1, -1, 0, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[1, 0, -10, 0, 1])), Some(4));
        assert_eq!(real_root_count(&poly(&[1, 0, 0, 0, 1])), Some(0));
    }

    #[test]
    fn cyclotomic_and_repeated_roots() {
        assert_eq!(real_root_count(&poly(&[1, 1, 1, 1, 1])), Some(0));
        assert_eq!(real_root_count(&poly(&[1, -2, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[1, 0, 0, 0, 1])), Some(0));
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert_eq!(real_root_count(&ZPoly::zero()), None);
        assert_eq!(
            real_root_count(&ZPoly::constant(crate::num::Int::from_i64(7))),
            Some(0)
        );
    }

    #[test]
    fn repeated_powers_have_one_distinct_real_root() {
        assert_eq!(real_root_count(&poly(&[0, 0, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[0, 0, 0, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[0, 0, 0, 0, 1])), Some(1));
        assert_eq!(real_root_count(&poly(&[1, 3, 3, 1])), Some(1));
    }

    #[test]
    fn even_powers_and_shifted_quartics() {
        assert_eq!(real_root_count(&poly(&[4, 0, 0, 0, 1])), Some(0));
        assert_eq!(real_root_count(&poly(&[-2, 0, 0, 0, 1])), Some(2));
        assert_eq!(real_root_count(&poly(&[-1, 0, 0, 0, 0, 0, 1])), Some(2));
    }

    #[test]
    fn negative_leading_coefficients_do_not_change_the_count() {
        assert_eq!(real_root_count(&poly(&[1, 0, -1])), Some(2));
        assert_eq!(real_root_count(&poly(&[1, -3, 3, -1])), Some(1));
        assert_eq!(real_root_count(&poly(&[-5])), Some(0));
    }
}
