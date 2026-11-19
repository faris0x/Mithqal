//! Exact integer number theory.
//!
//! Functions operate on machine sized integers for factorization and primality.
//! Modular arithmetic uses arbitrary precision integers.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::expr::{Arena, ExprId, Node};
use crate::num::Int;

fn as_int(arena: &Arena, expression: ExprId) -> Option<Int> {
    match arena.node(expression) {
        Node::Int(value) => Some(value.clone()),
        Node::Rat(value) if value.den().is_one() => Some(value.num()),
        _ => None,
    }
}

fn as_i64(arena: &Arena, expression: ExprId) -> Option<i64> {
    as_int(arena, expression)?.to_i64()
}

fn boolean(arena: &mut Arena, value: bool) -> ExprId {
    if value {
        arena.builtin_head("true")
    } else {
        arena.builtin_head("false")
    }
}

fn integer(arena: &mut Arena, value: Int) -> ExprId {
    arena.intern_int(value)
}

fn list(arena: &mut Arena, values: &[ExprId]) -> ExprId {
    let head = arena.builtin_head("list");
    arena.intern_call(head, values)
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn mul_mod(left: u64, right: u64, modulus: u64) -> u64 {
    ((left as u128 * right as u128) % modulus as u128) as u64
}

fn pow_mod(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    let mut result = 1u64 % modulus;
    base %= modulus;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = mul_mod(result, base, modulus);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = mul_mod(base, base, modulus);
        }
    }
    result
}

fn is_prime_u64(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    for prime in [
        2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83,
        89, 97,
    ] {
        if value == prime {
            return true;
        }
        if value.is_multiple_of(prime) {
            return false;
        }
    }
    let mut exponent = value - 1;
    let mut shifts = 0u32;
    while exponent & 1 == 0 {
        exponent >>= 1;
        shifts += 1;
    }
    if value < 3_215_031_751 {
        return miller_rabin(value, exponent, shifts, &[2, 3, 5, 7]);
    }
    if value < 3_474_749_660_383 {
        return miller_rabin(value, exponent, shifts, &[2, 3, 5, 7, 11, 13]);
    }
    miller_rabin(
        value,
        exponent,
        shifts,
        &[2, 325, 9_375, 28_178, 450_775, 9_780_504, 1_795_265_022],
    )
}

fn miller_rabin(value: u64, exponent: u64, shifts: u32, bases: &[u64]) -> bool {
    'witness: for base in bases {
        let base = base % value;
        if base == 0 {
            continue;
        }
        let mut power = pow_mod(base, exponent, value);
        if power == 1 || power == value - 1 {
            continue;
        }
        for _ in 1..shifts {
            power = mul_mod(power, power, value);
            if power == value - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn pollard_rho(value: u64) -> u64 {
    if value.is_multiple_of(2) {
        return 2;
    }
    let mut constant = 1u64;
    loop {
        let mut slow = 2u64;
        let mut fast = 2u64;
        let mut divisor = 1u64;
        while divisor == 1 {
            slow = ((mul_mod(slow, slow, value) as u128 + constant as u128) % value as u128) as u64;
            fast = ((mul_mod(fast, fast, value) as u128 + constant as u128) % value as u128) as u64;
            fast = ((mul_mod(fast, fast, value) as u128 + constant as u128) % value as u128) as u64;
            divisor = gcd_u64(slow.abs_diff(fast), value);
        }
        if divisor != value {
            return divisor;
        }
        constant += 1;
    }
}

fn factor_recursive(value: u64, factors: &mut Vec<u64>) {
    if value == 1 {
        return;
    }
    if is_prime_u64(value) {
        factors.push(value);
        return;
    }
    let divisor = pollard_rho(value);
    factor_recursive(divisor, factors);
    factor_recursive(value / divisor, factors);
}

fn factor_u64(value: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    factor_recursive(value, &mut factors);
    factors.sort_unstable();
    let mut grouped = Vec::new();
    for factor in factors {
        match grouped.last_mut() {
            Some((previous, count)) if *previous == factor => *count += 1,
            _ => grouped.push((factor, 1)),
        }
    }
    grouped
}

pub(crate) fn is_prime(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    Some(boolean(arena, value > 1 && is_prime_u64(value as u64)))
}

pub(crate) fn next_prime(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    let mut candidate = if value < 2 { 2 } else { value + 1 };
    if candidate < 2 {
        candidate = 2;
    }
    while !is_prime_u64(candidate as u64) {
        candidate = candidate.checked_add(1)?;
    }
    Some(integer(arena, Int::from_i64(candidate)))
}

pub(crate) fn factor_integer(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value == 0 {
        return None;
    }
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let mut entries = Vec::new();
    if negative {
        let minus = integer(arena, Int::from_i64(-1));
        let one = integer(arena, Int::from_i64(1));
        entries.push(list(arena, &[minus, one]));
    }
    if magnitude > 1 {
        for (prime, exponent) in factor_u64(magnitude) {
            let prime = integer(arena, Int::from_i64(prime as i64));
            let exponent = integer(arena, Int::from_i64(exponent as i64));
            entries.push(list(arena, &[prime, exponent]));
        }
    }
    Some(list(arena, &entries))
}

pub(crate) fn divisors(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value == 0 {
        return None;
    }
    let magnitude = value.unsigned_abs();
    if magnitude == 1 {
        let one = integer(arena, Int::from_i64(1));
        return Some(list(arena, &[one]));
    }
    let mut values = vec![1u64];
    for (prime, exponent) in factor_u64(magnitude) {
        let mut next = Vec::with_capacity(values.len() * (exponent as usize + 1));
        let mut power = 1u64;
        for _ in 0..=exponent {
            for value in &values {
                next.push(value * power);
            }
            power = power.saturating_mul(prime);
        }
        values = next;
    }
    values.sort_unstable();
    let entries = values
        .into_iter()
        .map(|value| integer(arena, Int::from_i64(value as i64)))
        .collect::<Vec<_>>();
    Some(list(arena, &entries))
}

pub(crate) fn totient(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value <= 0 {
        return None;
    }
    let magnitude = value as u64;
    let mut result = magnitude;
    for (prime, _) in factor_u64(magnitude) {
        result = result / prime * (prime - 1);
    }
    Some(integer(arena, Int::from_i64(result as i64)))
}

pub(crate) fn sigma(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value <= 0 {
        return None;
    }
    let magnitude = value as u64;
    let mut result = Int::one();
    for (prime, exponent) in factor_u64(magnitude) {
        let prime = Int::from_i64(prime as i64);
        let mut power = Int::one();
        let mut sum = Int::zero();
        for _ in 0..=exponent {
            sum = sum.add(&power);
            power = power.mul(&prime);
        }
        result = result.mul(&sum);
    }
    Some(integer(arena, result))
}

pub(crate) fn is_even(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_int(arena, expression)?;
    let two = Int::from_i64(2);
    let (_, remainder) = value.divrem(&two).ok()?;
    Some(boolean(arena, remainder.is_zero()))
}

pub(crate) fn is_odd(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_int(arena, expression)?;
    let two = Int::from_i64(2);
    let (_, remainder) = value.divrem(&two).ok()?;
    Some(boolean(arena, !remainder.is_zero()))
}

pub(crate) fn divides(arena: &mut Arena, left: ExprId, right: ExprId) -> Option<ExprId> {
    let left = as_int(arena, left)?;
    let right = as_int(arena, right)?;
    if left.is_zero() {
        return Some(boolean(arena, right.is_zero()));
    }
    let (_, remainder) = right.divrem(&left).ok()?;
    Some(boolean(arena, remainder.is_zero()))
}

pub(crate) fn modulo(arena: &mut Arena, left: ExprId, modulus: ExprId) -> Option<ExprId> {
    let left = as_int(arena, left)?;
    let modulus = as_int(arena, modulus)?;
    if modulus.is_zero() {
        return None;
    }
    let modulus = modulus.abs();
    let (_, mut remainder) = left.divrem(&modulus).ok()?;
    if remainder.sign() < 0 {
        remainder = remainder.add(&modulus);
    }
    Some(integer(arena, remainder))
}

pub(crate) fn power_mod(
    arena: &mut Arena,
    base: ExprId,
    exponent: ExprId,
    modulus: ExprId,
) -> Option<ExprId> {
    let base = as_int(arena, base)?;
    let exponent = as_int(arena, exponent)?;
    let modulus = as_int(arena, modulus)?;
    if modulus.is_zero() || exponent.sign() < 0 {
        return None;
    }
    let modulus = modulus.abs();
    let two = Int::from_i64(2);
    let mut base = modulo_int(&base, &modulus)?;
    let mut exponent = exponent;
    let mut result = modulo_int(&Int::one(), &modulus)?;
    while exponent.sign() > 0 {
        let (half, remainder) = exponent.divrem(&two).ok()?;
        if !remainder.is_zero() {
            result = modulo_int(&result.mul(&base), &modulus)?;
        }
        exponent = half;
        if exponent.sign() > 0 {
            base = modulo_int(&base.mul(&base), &modulus)?;
        }
    }
    Some(integer(arena, result))
}

fn modulo_int(value: &Int, modulus: &Int) -> Option<Int> {
    let (_, mut remainder) = value.divrem(modulus).ok()?;
    if remainder.sign() < 0 {
        remainder = remainder.add(modulus);
    }
    Some(remainder)
}

pub(crate) fn previous_prime(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value <= 2 {
        return None;
    }
    let mut candidate = value - 1;
    while candidate >= 2 {
        if is_prime_u64(candidate as u64) {
            return Some(integer(arena, Int::from_i64(candidate)));
        }
        candidate -= 1;
    }
    None
}

pub(crate) fn divisor_count(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value == 0 {
        return None;
    }
    let mut count = 1u64;
    for (_, exponent) in factor_u64(value.unsigned_abs()) {
        count = count.saturating_mul(exponent as u64 + 1);
    }
    Some(integer(arena, Int::from_i64(count as i64)))
}

pub(crate) fn moebius(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value <= 0 {
        return None;
    }
    let factors = factor_u64(value as u64);
    if factors.iter().any(|(_, exponent)| *exponent > 1) {
        return Some(integer(arena, Int::zero()));
    }
    let sign = if factors.len().is_multiple_of(2) {
        1
    } else {
        -1
    };
    Some(integer(arena, Int::from_i64(sign)))
}

pub(crate) fn radical(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value == 0 {
        return None;
    }
    let mut result = Int::one();
    for (prime, _) in factor_u64(value.unsigned_abs()) {
        result = result.mul(&Int::from_i64(prime as i64));
    }
    Some(integer(arena, result))
}

pub(crate) fn is_square_free(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value <= 0 {
        return None;
    }
    let factors = factor_u64(value as u64);
    Some(boolean(
        arena,
        factors.iter().all(|(_, exponent)| *exponent == 1),
    ))
}

pub(crate) fn prime_count(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value < 2 {
        return Some(integer(arena, Int::zero()));
    }
    if value > 5_000_000 {
        return None;
    }
    let limit = value as usize;
    let mut sieve = vec![true; limit + 1];
    sieve[0] = false;
    sieve[1] = false;
    let mut divisor = 2usize;
    while divisor * divisor <= limit {
        if sieve[divisor] {
            let mut multiple = divisor * divisor;
            while multiple <= limit {
                sieve[multiple] = false;
                multiple += divisor;
            }
        }
        divisor += 1;
    }
    let count = sieve.into_iter().filter(|prime| *prime).count();
    Some(integer(arena, Int::from_i64(count as i64)))
}

pub(crate) fn fibonacci(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if !(0..=10_000).contains(&value) {
        return None;
    }
    if value <= 92 {
        let mut previous = 0u64;
        let mut current = 1u64;
        for _ in 0..value {
            let next = previous + current;
            previous = current;
            current = next;
        }
        return Some(integer(arena, Int::from_u64(previous)));
    }
    let mut previous = Int::zero();
    let mut current = Int::one();
    for _ in 0..value {
        let next = previous.add(&current);
        previous = current;
        current = next;
    }
    Some(integer(arena, previous))
}

pub(crate) fn is_perfect_square(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, expression)?;
    if value < 0 {
        return Some(boolean(arena, false));
    }
    Some(boolean(arena, integer_sqrt_u64(value as u64).is_some()))
}

fn integer_sqrt_u64(value: u64) -> Option<u64> {
    let mut low = 0u64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as u128) * (middle as u128);
        if square == value as u128 {
            return Some(middle);
        }
        if square < value as u128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

pub(crate) fn mod_inverse(arena: &mut Arena, value: ExprId, modulus: ExprId) -> Option<ExprId> {
    let value = as_i64(arena, value)?;
    let modulus = as_i64(arena, modulus)?.abs();
    if modulus <= 1 {
        return None;
    }
    let mut old_r = value.rem_euclid(modulus);
    let mut r = modulus;
    let mut old_s = 1i64;
    let mut s = 0i64;
    while r != 0 {
        let quotient = old_r / r;
        let next_r = old_r - quotient * r;
        old_r = r;
        r = next_r;
        let next_s = old_s - quotient * s;
        old_s = s;
        s = next_s;
    }
    if old_r != 1 {
        return None;
    }
    Some(integer(arena, Int::from_i64(old_s.rem_euclid(modulus))))
}

pub(crate) fn jacobi(arena: &mut Arena, value: ExprId, modulus: ExprId) -> Option<ExprId> {
    let mut value = as_i64(arena, value)?;
    let modulus = as_i64(arena, modulus)?;
    if modulus <= 0 || modulus % 2 == 0 {
        return None;
    }
    value = value.rem_euclid(modulus);
    let mut modulus = modulus;
    let mut result = 1i64;
    while value != 0 {
        while value % 2 == 0 {
            value /= 2;
            let remainder = modulus % 8;
            if remainder == 3 || remainder == 5 {
                result = -result;
            }
        }
        core::mem::swap(&mut value, &mut modulus);
        if value % 4 == 3 && modulus % 4 == 3 {
            result = -result;
        }
        value %= modulus;
    }
    if modulus == 1 {
        Some(integer(arena, Int::from_i64(result)))
    } else {
        Some(integer(arena, Int::zero()))
    }
}
