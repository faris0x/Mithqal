//! Exact real root counts by Sturm sequences.
//!
//! The signature of a number field only needs the number of real embeddings.
//! A Sturm sequence over the rationals gives that count without numeric root
//! finding or interval refinement.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::Rat;

use super::super::dense::{dense_div_rem, trim};
use super::super::poly_z::ZPoly;

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
        let (_, remainder) = dense_div_rem(previous, &last)?;
        if remainder.is_empty() {
            break;
        }
        let next = trim(remainder.iter().map(Rat::neg).collect());
        sequence.push(next);
    }
    let positive = sign_variations(&sequence, true);
    let negative = sign_variations(&sequence, false);
    if negative < positive {
        return None;
    }
    Some(negative - positive)
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
