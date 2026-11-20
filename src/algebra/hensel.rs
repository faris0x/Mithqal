//! Hensel lifting for univariate integer polynomials.
//!
//! The stage reduces a primitive integer polynomial modulo a small prime,
//! finds a nontrivial split into coprime monic factors, and lifts the pair to a
//! modulus larger than twice the coefficient bound. Lifted factors are
//! reconstructed over the integers and accepted only when exact division
//! succeeds.
//!
//! Coefficients and moduli use arbitrary precision integers, so the coefficient range is not
//! limited by machine word size.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

const MAX_DEGREE: usize = 8;
const PRIMES: [i64; 5] = [2, 3, 5, 7, 11];

/// Factor a primitive univariate integer polynomial through modular lifting.
pub(super) fn factor_integer_polynomial(coefficients: &[Rat]) -> Option<Vec<Vec<Rat>>> {
    if let Some(factors) = super::hensel_fast::factor_integer_polynomial(coefficients) {
        return Some(factors);
    }
    factor_integer_polynomial_big(coefficients)
}

fn factor_integer_polynomial_big(coefficients: &[Rat]) -> Option<Vec<Vec<Rat>>> {
    let coefficients = trim(coefficients.to_vec());
    let degree = coefficients.len().checked_sub(1)?;
    if !(4..=MAX_DEGREE).contains(&degree) {
        return None;
    }
    let (_, values) = integerize(&coefficients)?;
    let mut values: Vec<Int> = values;
    if values.last().is_some_and(|value| value.sign() < 0) {
        values = values.iter().map(Int::neg).collect();
    }
    let bound = coefficient_bound(&values)?;
    for prime in PRIMES {
        if int_rem_euclid(values.last()?, &Int::from_i64(prime))?.is_zero() {
            continue;
        }
        let reduced = mod_reduce(&values, prime);
        let derivative = mod_derivative(&reduced, prime);
        if !mod_is_one(&mod_gcd(&reduced, &derivative, prime), prime) {
            continue;
        }
        let (left, right) = find_modular_factor(&reduced, prime)?;
        let Some((left, right)) = lift_pair(&values, &left, &right, prime, &bound) else {
            continue;
        };
        let Some((factor, quotient)) =
            exact_split(&values, &left).or_else(|| exact_split(&values, &right))
        else {
            continue;
        };
        let mut factors = Vec::new();
        factors.extend(factor_integer_polynomial(&factor).unwrap_or_else(|| vec![factor]));
        factors.extend(factor_integer_polynomial(&quotient).unwrap_or_else(|| vec![quotient]));
        let mut product = vec![Rat::one()];
        for factor in &factors {
            product = dense_multiply(&product, factor);
        }
        if product == coefficients {
            return Some(factors);
        }
    }
    None
}

fn coefficient_bound(coefficients: &[Int]) -> Option<Int> {
    let mut maximum = Int::one();
    for value in coefficients {
        let magnitude = value.abs();
        if magnitude.cmp(&maximum) == core::cmp::Ordering::Greater {
            maximum = magnitude;
        }
    }
    let mut power = Int::one();
    for _ in 0..coefficients.len().saturating_sub(1) + 1 {
        power = power.mul(&Int::from_i64(2));
    }
    Some(power.mul(&maximum.add(&Int::one())))
}

fn exact_split(coefficients: &[Int], candidate: &[Int]) -> Option<(Vec<Rat>, Vec<Rat>)> {
    let candidate = primitive_integer(candidate);
    if candidate.len() < 2 || candidate.len() >= coefficients.len() {
        return None;
    }
    let dividend: Vec<Rat> = coefficients
        .iter()
        .map(|value| Rat::from_ints(value.clone(), Int::one()).ok())
        .collect::<Option<Vec<_>>>()?;
    let divisor: Vec<Rat> = candidate
        .iter()
        .map(|value| Rat::from_ints(value.clone(), Int::one()).ok())
        .collect::<Option<Vec<_>>>()?;
    let (quotient, remainder) = dense_div_rem(&dividend, &divisor)?;
    if !remainder.is_empty() {
        return None;
    }
    let quotient = primitive_rational(&quotient)?;
    Some((divisor, quotient))
}

fn primitive_rational(coefficients: &[Rat]) -> Option<Vec<Rat>> {
    let mut rationals = trim(coefficients.to_vec());
    if rationals.last().is_some_and(|value| value.sign() < 0) {
        rationals = rationals.iter().map(Rat::neg).collect();
    }
    let (_, values) = integerize(&rationals)?;
    let mut values: Vec<Rat> = values
        .into_iter()
        .map(|value| Rat::from_ints(value, Int::one()).ok())
        .collect::<Option<Vec<_>>>()?;
    if values.last().is_some_and(|value| value.sign() < 0) {
        values = values.iter().map(Rat::neg).collect();
    }
    Some(trim(values))
}

fn primitive_integer(coefficients: &[Int]) -> Vec<Int> {
    let mut coefficients = coefficients.to_vec();
    while coefficients.last().is_some_and(Int::is_zero) {
        coefficients.pop();
    }
    if coefficients.is_empty() {
        return coefficients;
    }
    let mut divisor = Int::zero();
    for value in &coefficients {
        divisor = divisor.gcd(&value.abs());
    }
    if !divisor.is_one() && !divisor.is_zero() {
        for value in &mut coefficients {
            if let Ok((quotient, _)) = value.divrem(&divisor) {
                *value = quotient;
            }
        }
    }
    if coefficients.last().is_some_and(|value| value.sign() < 0) {
        coefficients = coefficients.iter().map(Int::neg).collect();
    }
    coefficients
}

fn lift_pair(
    coefficients: &[Int],
    left: &[i64],
    right: &[i64],
    prime: i64,
    bound: &Int,
) -> Option<(Vec<Int>, Vec<Int>)> {
    let mut modulus = Int::from_i64(prime);
    let limit = bound.mul(&Int::from_i64(2));
    let mut left: Vec<Int> = left.iter().map(|value| Int::from_i64(*value)).collect();
    let mut right: Vec<Int> = right.iter().map(|value| Int::from_i64(*value)).collect();
    let left_mod = mod_reduce(&left, prime);
    let right_mod = mod_reduce(&right, prime);
    let (gcd, mut bezout_left, mut bezout_right) = mod_extended_gcd(&left_mod, &right_mod, prime)?;
    if gcd.len() != 1 {
        return None;
    }
    let inverse = mod_inverse(gcd[0], prime)?;
    for value in &mut bezout_left {
        *value = mod_mul(*value, inverse, prime);
    }
    for value in &mut bezout_right {
        *value = mod_mul(*value, inverse, prime);
    }
    while modulus.cmp(&limit) != core::cmp::Ordering::Greater {
        let product = integer_multiply(&left, &right);
        let mut error = vec![0i64; coefficients.len()];
        for (index, value) in coefficients.iter().enumerate() {
            let product_value = product.get(index).cloned().unwrap_or_else(Int::zero);
            let difference = value.sub(&product_value);
            let (quotient, remainder) = difference.divrem(&modulus).ok()?;
            if !remainder.is_zero() {
                return None;
            }
            error[index] = int_rem_euclid(&quotient, &Int::from_i64(prime))?.to_i64()?;
        }
        let error = trim_mod(error, prime);
        let correction_left = mod_remainder(
            &mod_multiply(&bezout_right, &error, prime),
            &left_mod,
            prime,
        )?;
        let difference = mod_subtract(
            &error,
            &mod_multiply(&correction_left, &right_mod, prime),
            prime,
        );
        let (correction_right, remainder) = mod_divide(&difference, &left_mod, prime)?;
        if !remainder.is_empty() {
            return None;
        }
        let next_modulus = modulus.mul(&Int::from_i64(prime));
        for (index, value) in correction_left.iter().enumerate() {
            if index < left.len() {
                left[index] = left[index].add(&modulus.mul(&Int::from_i64(*value)));
            }
        }
        for (index, value) in correction_right.iter().enumerate() {
            if index < right.len() {
                right[index] = right[index].add(&modulus.mul(&Int::from_i64(*value)));
            }
        }
        for value in &mut left {
            *value = int_rem_euclid(value, &next_modulus)?;
        }
        for value in &mut right {
            *value = int_rem_euclid(value, &next_modulus)?;
        }
        modulus = next_modulus;
    }
    Some((
        symmetric_coefficients(&left, &modulus),
        symmetric_coefficients(&right, &modulus),
    ))
}

fn symmetric_coefficients(coefficients: &[Int], modulus: &Int) -> Vec<Int> {
    coefficients
        .iter()
        .map(|value| {
            let remainder = int_rem_euclid(value, modulus).unwrap_or_else(Int::zero);
            let doubled = remainder.mul(&Int::from_i64(2));
            if doubled.cmp(modulus) == core::cmp::Ordering::Greater {
                remainder.sub(modulus)
            } else {
                remainder
            }
        })
        .collect()
}

fn integer_multiply(left: &[Int], right: &[Int]) -> Vec<Int> {
    let mut product = vec![Int::zero(); left.len() + right.len() - 1];
    for (left_index, left_value) in left.iter().enumerate() {
        for (right_index, right_value) in right.iter().enumerate() {
            let index = left_index + right_index;
            product[index] = product[index].add(&left_value.mul(right_value));
        }
    }
    product
}

fn int_rem_euclid(value: &Int, modulus: &Int) -> Option<Int> {
    let (_, remainder) = value.divrem(modulus).ok()?;
    if remainder.sign() < 0 {
        Some(remainder.add(modulus))
    } else {
        Some(remainder)
    }
}

fn find_modular_factor(coefficients: &[i64], prime: i64) -> Option<(Vec<i64>, Vec<i64>)> {
    let degree = coefficients.len().checked_sub(1)?;
    for candidate_degree in 2..=degree / 2 {
        let count = (prime as u64).checked_pow(candidate_degree as u32)?;
        for code in 0..count {
            let mut candidate = vec![0i64; candidate_degree + 1];
            candidate[candidate_degree] = 1;
            let mut value = code;
            for slot in candidate.iter_mut().take(candidate_degree) {
                *slot = (value % prime as u64) as i64;
                value /= prime as u64;
            }
            let (quotient, remainder) = mod_divide(coefficients, &candidate, prime)?;
            if remainder.is_empty() && quotient.len() > 1 {
                return Some((candidate, quotient));
            }
        }
    }
    None
}

fn mod_reduce(coefficients: &[Int], prime: i64) -> Vec<i64> {
    trim_mod(
        coefficients
            .iter()
            .map(|value| {
                int_rem_euclid(value, &Int::from_i64(prime))
                    .and_then(|value| value.to_i64())
                    .unwrap_or(0)
            })
            .collect(),
        prime,
    )
}

fn trim_mod(mut coefficients: Vec<i64>, prime: i64) -> Vec<i64> {
    for value in &mut coefficients {
        *value = value.rem_euclid(prime);
    }
    while coefficients.last() == Some(&0) {
        coefficients.pop();
    }
    coefficients
}

fn mod_is_one(coefficients: &[i64], prime: i64) -> bool {
    coefficients.len() == 1 && coefficients[0].rem_euclid(prime) == 1
}

fn mod_subtract(left: &[i64], right: &[i64], prime: i64) -> Vec<i64> {
    let length = left.len().max(right.len());
    let mut difference = vec![0i64; length];
    for (index, slot) in difference.iter_mut().enumerate() {
        let value = left.get(index).copied().unwrap_or(0) - right.get(index).copied().unwrap_or(0);
        *slot = value.rem_euclid(prime);
    }
    trim_mod(difference, prime)
}

fn mod_multiply(left: &[i64], right: &[i64], prime: i64) -> Vec<i64> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let mut product = vec![0i64; left.len() + right.len() - 1];
    for (left_index, left_value) in left.iter().enumerate() {
        for (right_index, right_value) in right.iter().enumerate() {
            let index = left_index + right_index;
            product[index] =
                (product[index] + mod_mul(*left_value, *right_value, prime)).rem_euclid(prime);
        }
    }
    trim_mod(product, prime)
}

fn mod_divide(dividend: &[i64], divisor: &[i64], prime: i64) -> Option<(Vec<i64>, Vec<i64>)> {
    let divisor = trim_mod(divisor.to_vec(), prime);
    if divisor.is_empty() {
        return None;
    }
    let mut remainder = trim_mod(dividend.to_vec(), prime);
    let divisor_degree = divisor.len();
    if remainder.len() < divisor_degree {
        return Some((Vec::new(), remainder));
    }
    let inverse = mod_inverse(*divisor.last()?, prime)?;
    let mut quotient = vec![0i64; remainder.len() - divisor_degree + 1];
    while remainder.len() >= divisor_degree {
        let degree = remainder.len() - divisor_degree;
        let coefficient = mod_mul(*remainder.last()?, inverse, prime);
        quotient[degree] = coefficient;
        for (index, divisor_coefficient) in divisor.iter().enumerate() {
            let position = degree + index;
            remainder[position] = (remainder[position]
                - mod_mul(coefficient, *divisor_coefficient, prime))
            .rem_euclid(prime);
        }
        remainder = trim_mod(remainder, prime);
    }
    Some((trim_mod(quotient, prime), remainder))
}

fn mod_remainder(dividend: &[i64], divisor: &[i64], prime: i64) -> Option<Vec<i64>> {
    mod_divide(dividend, divisor, prime).map(|(_, remainder)| remainder)
}

fn mod_gcd(left: &[i64], right: &[i64], prime: i64) -> Vec<i64> {
    let mut left = trim_mod(left.to_vec(), prime);
    let mut right = trim_mod(right.to_vec(), prime);
    while !right.is_empty() {
        let remainder = match mod_remainder(&left, &right, prime) {
            Some(remainder) => remainder,
            None => return vec![1],
        };
        left = right;
        right = remainder;
    }
    if left.is_empty() {
        return vec![1];
    }
    let Some(inverse) = mod_inverse(*left.last().unwrap_or(&1), prime) else {
        return vec![1];
    };
    left.iter()
        .map(|value| mod_mul(*value, inverse, prime))
        .collect()
}

fn mod_extended_gcd(
    left: &[i64],
    right: &[i64],
    prime: i64,
) -> Option<(Vec<i64>, Vec<i64>, Vec<i64>)> {
    let mut old_remainder = trim_mod(left.to_vec(), prime);
    let mut remainder = trim_mod(right.to_vec(), prime);
    let mut old_left = vec![1i64];
    let mut left_coefficient = Vec::new();
    let mut old_right = Vec::new();
    let mut right_coefficient = vec![1i64];
    while !remainder.is_empty() {
        let (quotient, next_remainder) = mod_divide(&old_remainder, &remainder, prime)?;
        old_remainder = remainder;
        remainder = next_remainder;
        let next_left = mod_subtract(
            &old_left,
            &mod_multiply(&quotient, &left_coefficient, prime),
            prime,
        );
        old_left = left_coefficient;
        left_coefficient = next_left;
        let next_right = mod_subtract(
            &old_right,
            &mod_multiply(&quotient, &right_coefficient, prime),
            prime,
        );
        old_right = right_coefficient;
        right_coefficient = next_right;
    }
    Some((old_remainder, old_left, old_right))
}

fn mod_derivative(coefficients: &[i64], prime: i64) -> Vec<i64> {
    let derivative: Vec<i64> = coefficients
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, value)| mod_mul(*value, index as i64, prime))
        .collect();
    trim_mod(derivative, prime)
}

fn mod_inverse(value: i64, prime: i64) -> Option<i64> {
    let value = value.rem_euclid(prime);
    if value == 0 {
        return None;
    }
    let mut exponent = prime - 2;
    let mut result = 1i64;
    let mut base = value;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mod_mul(result, base, prime);
        }
        base = mod_mul(base, base, prime);
        exponent >>= 1;
    }
    Some(result)
}

fn mod_mul(left: i64, right: i64, prime: i64) -> i64 {
    (left.rem_euclid(prime) * right.rem_euclid(prime)).rem_euclid(prime)
}
