//! Hensel lifting for univariate integer polynomials.
//!
//! The stage reduces a primitive integer polynomial modulo a small prime,
//! finds a nontrivial split into coprime monic factors, and lifts the pair to a
//! modulus larger than twice the coefficient bound. Lifted factors are
//! reconstructed over the integers and accepted only when exact division
//! succeeds.
//!
//! Coefficients are converted to signed 128 bit values. Inputs that exceed that
//! range fall back to leaving the polynomial unfactored.

use super::*;

const MAX_DEGREE: usize = 8;
const PRIMES: [i64; 5] = [2, 3, 5, 7, 11];

/// Factor a primitive univariate integer polynomial through modular lifting.
pub(super) fn factor_integer_polynomial(coefficients: &[Rat]) -> Option<Vec<Vec<Rat>>> {
    let coefficients = trim(coefficients.to_vec());
    let degree = coefficients.len().checked_sub(1)?;
    if !(4..=MAX_DEGREE).contains(&degree) {
        return None;
    }
    let (_, values) = integerize(&coefficients)?;
    let mut values: Vec<i128> = values
        .into_iter()
        .map(|value| value.to_i128())
        .collect::<Option<Vec<_>>>()?;
    if values.last().is_some_and(|value| *value < 0) {
        for value in &mut values {
            *value = -*value;
        }
    }
    let bound = coefficient_bound(&values)?;
    for prime in PRIMES {
        if values
            .last()
            .copied()
            .unwrap_or(0)
            .rem_euclid(prime as i128)
            == 0
        {
            continue;
        }
        let reduced = mod_reduce(&values, prime);
        let derivative = mod_derivative(&reduced, prime);
        if !mod_is_one(&mod_gcd(&reduced, &derivative, prime), prime) {
            continue;
        }
        let (left, right) = find_modular_factor(&reduced, prime)?;
        let Some((left, right)) = lift_pair(&values, &left, &right, prime, bound) else {
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

fn coefficient_bound(coefficients: &[i128]) -> Option<i128> {
    let mut maximum = 1i128;
    for value in coefficients {
        maximum = maximum.max(value.checked_abs()?);
    }
    let degree = coefficients.len().saturating_sub(1);
    (1i128.checked_shl(degree as u32 + 1)?).checked_mul(maximum.checked_add(1)?)
}

fn exact_split(coefficients: &[i128], candidate: &[i128]) -> Option<(Vec<Rat>, Vec<Rat>)> {
    let candidate = primitive_integer(candidate);
    if candidate.len() < 2 || candidate.len() >= coefficients.len() {
        return None;
    }
    let dividend: Vec<Rat> = coefficients
        .iter()
        .map(|value| rat_from_i128(*value))
        .collect::<Option<Vec<_>>>()?;
    let divisor: Vec<Rat> = candidate
        .iter()
        .map(|value| rat_from_i128(*value))
        .collect::<Option<Vec<_>>>()?;
    let (quotient, remainder) = dense_div_rem(&dividend, &divisor)?;
    if !remainder.is_empty() {
        return None;
    }
    let quotient = primitive_rational(&quotient)?;
    Some((divisor, quotient))
}

fn rat_from_i128(value: i128) -> Option<Rat> {
    let value = Int::from_decimal(&value.to_string()).ok()?;
    Rat::from_ints(value, Int::one()).ok()
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

fn primitive_integer(coefficients: &[i128]) -> Vec<i128> {
    let mut coefficients = coefficients.to_vec();
    while coefficients.last() == Some(&0) {
        coefficients.pop();
    }
    if coefficients.is_empty() {
        return coefficients;
    }
    let mut divisor = 0i128;
    for value in &coefficients {
        divisor = gcd_i128(divisor, value.checked_abs().unwrap_or(i128::MAX));
    }
    if divisor > 1 {
        for value in &mut coefficients {
            *value /= divisor;
        }
    }
    if coefficients.last().is_some_and(|value| *value < 0) {
        for value in &mut coefficients {
            *value = -*value;
        }
    }
    coefficients
}

fn gcd_i128(mut left: i128, mut right: i128) -> i128 {
    left = left.abs();
    right = right.abs();
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn lift_pair(
    coefficients: &[i128],
    left: &[i64],
    right: &[i64],
    prime: i64,
    bound: i128,
) -> Option<(Vec<i128>, Vec<i128>)> {
    let mut modulus = prime as i128;
    let limit = bound.checked_mul(2)?;
    let mut left: Vec<i128> = left.iter().map(|value| *value as i128).collect();
    let mut right: Vec<i128> = right.iter().map(|value| *value as i128).collect();
    let left_mod = mod_reduce_i128(&left, prime);
    let right_mod = mod_reduce_i128(&right, prime);
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
    while modulus <= limit {
        let product = integer_multiply(&left, &right)?;
        let mut error = vec![0i64; coefficients.len()];
        for (index, value) in coefficients.iter().enumerate() {
            let difference = value.checked_sub(product.get(index).copied().unwrap_or(0))?;
            if difference % modulus != 0 {
                return None;
            }
            error[index] = (difference / modulus).rem_euclid(prime as i128) as i64;
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
        let next_modulus = modulus.checked_mul(prime as i128)?;
        for (index, value) in correction_left.iter().enumerate() {
            if index < left.len() {
                left[index] = left[index].checked_add(modulus.checked_mul(*value as i128)?)?;
            }
        }
        for (index, value) in correction_right.iter().enumerate() {
            if index < right.len() {
                right[index] = right[index].checked_add(modulus.checked_mul(*value as i128)?)?;
            }
        }
        for value in &mut left {
            *value = value.rem_euclid(next_modulus);
        }
        for value in &mut right {
            *value = value.rem_euclid(next_modulus);
        }
        modulus = next_modulus;
    }
    Some((
        symmetric_coefficients(&left, modulus),
        symmetric_coefficients(&right, modulus),
    ))
}

fn symmetric_coefficients(coefficients: &[i128], modulus: i128) -> Vec<i128> {
    let half = modulus / 2;
    coefficients
        .iter()
        .map(|value| {
            let mut value = value.rem_euclid(modulus);
            if value > half {
                value -= modulus;
            }
            value
        })
        .collect()
}

fn integer_multiply(left: &[i128], right: &[i128]) -> Option<Vec<i128>> {
    let mut product = vec![0i128; left.len() + right.len() - 1];
    for (left_index, left_value) in left.iter().enumerate() {
        for (right_index, right_value) in right.iter().enumerate() {
            let index = left_index + right_index;
            product[index] = product[index].checked_add(left_value.checked_mul(*right_value)?)?;
        }
    }
    Some(product)
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

fn mod_reduce(coefficients: &[i128], prime: i64) -> Vec<i64> {
    trim_mod(
        coefficients
            .iter()
            .map(|value| value.rem_euclid(prime as i128) as i64)
            .collect(),
        prime,
    )
}

fn mod_reduce_i128(coefficients: &[i128], prime: i64) -> Vec<i64> {
    trim_mod(
        coefficients
            .iter()
            .map(|value| value.rem_euclid(prime as i128) as i64)
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
