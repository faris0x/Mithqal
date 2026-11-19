//! Rational roots and integer coefficient normalisation.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::BTreeSet;

use crate::num::{Int, Rat};

use super::dense::{dense_div_rem, dense_eval, lcm, rational_sqrt, trim};

pub(crate) fn primitive_normalize(coefficients: &[Rat]) -> Vec<Rat> {
    let Some((_, values)) = integerize(coefficients) else {
        return trim(coefficients.to_vec());
    };
    let mut values: Vec<Rat> = values
        .into_iter()
        .map(|value| Rat::from_ints(value, Int::one()).unwrap_or_else(|_| Rat::zero()))
        .collect();
    if values.last().is_some_and(|value| value.sign() < 0) {
        values = values.iter().map(Rat::neg).collect();
    }
    trim(values)
}

pub(crate) fn integerize(coefficients: &[Rat]) -> Option<(Rat, Vec<Int>)> {
    let mut denominator = Int::one();
    for coefficient in coefficients {
        denominator = lcm(&denominator, &coefficient.den());
    }
    let mut values = Vec::with_capacity(coefficients.len());
    let mut content = Int::zero();
    for coefficient in coefficients {
        let (quotient, _) = denominator.divrem(&coefficient.den()).ok()?;
        let value = coefficient.num().mul(&quotient);
        content = content.gcd(&value.abs());
        values.push(value);
    }
    if content.is_zero() {
        return None;
    }
    for value in &mut values {
        *value = value.divrem(&content).ok()?.0;
    }
    Some((Rat::from_ints(content, denominator).ok()?, values))
}

pub(crate) fn divisors(value: i64) -> Vec<i64> {
    let magnitude = value.unsigned_abs();
    if magnitude == 0 || magnitude > 1_000_000_000_000 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut divisor = 1i64;
    while (divisor as u128) * (divisor as u128) <= magnitude as u128 {
        if magnitude.is_multiple_of(divisor as u64) {
            let other = (magnitude / divisor as u64) as i64;
            out.push(divisor);
            if other != divisor {
                out.push(other);
            }
        }
        divisor += 1;
    }
    out
}

pub(crate) fn rational_roots(coefficients: &[Rat]) -> (Vec<(Rat, u32)>, Vec<Rat>) {
    if coefficients.len() == 2 && !coefficients[1].is_zero() {
        let root = coefficients[0]
            .neg()
            .div(&coefficients[1])
            .unwrap_or_else(|_| Rat::zero());
        return (vec![(root, 1)], vec![coefficients[1].clone()]);
    }
    if coefficients.len() == 3 && !coefficients[2].is_zero() {
        let constant = &coefficients[0];
        let linear = &coefficients[1];
        let quadratic = &coefficients[2];
        let four = Rat::from_i64(4, 1).unwrap_or_else(|_| Rat::zero());
        let discriminant = linear.mul(linear).sub(&four.mul(quadratic).mul(constant));
        let Some(root) = rational_sqrt(&discriminant) else {
            return (Vec::new(), coefficients.to_vec());
        };
        let denominator = quadratic.add(quadratic);
        let negative_linear = linear.neg();
        if root.is_zero() {
            let value = negative_linear
                .div(&denominator)
                .unwrap_or_else(|_| Rat::zero());
            return (vec![(value, 2)], vec![quadratic.clone()]);
        }
        let first = negative_linear
            .add(&root)
            .div(&denominator)
            .unwrap_or_else(|_| Rat::zero());
        let second = negative_linear
            .sub(&root)
            .div(&denominator)
            .unwrap_or_else(|_| Rat::zero());
        return (vec![(first, 1), (second, 1)], vec![quadratic.clone()]);
    }
    let Some((_, integer_coefficients)) = integerize(coefficients) else {
        return (Vec::new(), trim(coefficients.to_vec()));
    };
    let mut remaining: Vec<Rat> = integer_coefficients
        .iter()
        .map(|value| Rat::from_ints(value.clone(), Int::one()).unwrap_or_else(|_| Rat::zero()))
        .collect();
    remaining = trim(remaining);
    let mut roots = Vec::new();

    while remaining.len() > 1 && remaining[0].is_zero() {
        roots.push((Rat::zero(), 1));
        let Some((quotient, _)) = dense_div_rem(&remaining, &[Rat::zero(), Rat::one()]) else {
            break;
        };
        remaining = quotient;
    }
    if remaining.len() <= 1 {
        return (roots, remaining);
    }

    let (Some(constant), Some(leading)) = (
        remaining[0].num().to_i64(),
        remaining.last().and_then(|value| value.num().to_i64()),
    ) else {
        return (roots, remaining);
    };
    if leading == 0 {
        return (roots, remaining);
    }

    let mut candidates = BTreeSet::new();
    for numerator in divisors(constant.abs()) {
        for denominator in divisors(leading.abs()) {
            if denominator <= 0 {
                continue;
            }
            let gcd = gcd_i64(numerator, denominator);
            let numerator = numerator / gcd;
            let denominator = denominator / gcd;
            candidates.insert((numerator, denominator));
            candidates.insert((-numerator, denominator));
        }
    }

    for (numerator, denominator) in candidates {
        let candidate = match Rat::from_i64(numerator, denominator) {
            Ok(value) => value,
            Err(_) => continue,
        };
        loop {
            if remaining.len() <= 1 || !dense_eval(&remaining, &candidate).is_zero() {
                break;
            }
            let factor = [
                Rat::from_i64(-numerator, 1).unwrap_or_else(|_| Rat::zero()),
                Rat::from_i64(denominator, 1).unwrap_or_else(|_| Rat::one()),
            ];
            let Some((quotient, remainder)) = dense_div_rem(&remaining, &factor) else {
                break;
            };
            if !remainder.is_empty() {
                break;
            }
            remaining = quotient;
            push_root(&mut roots, candidate.clone());
        }
    }
    (roots, remaining)
}

pub(crate) fn gcd_i64(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs().max(1)
}

pub(crate) fn push_root(roots: &mut Vec<(Rat, u32)>, root: Rat) {
    for (candidate, multiplicity) in roots.iter_mut() {
        if *candidate == root {
            *multiplicity += 1;
            return;
        }
    }
    roots.push((root, 1));
}
