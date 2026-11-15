//! Dense polynomial coefficient helpers shared by algebra stages.

use crate::num::{Int, Rat};

pub(crate) fn trim(mut coefficients: Vec<Rat>) -> Vec<Rat> {
    while coefficients.last().is_some_and(Rat::is_zero) {
        coefficients.pop();
    }
    coefficients
}

pub(crate) fn dense_degree(coefficients: &[Rat]) -> usize {
    coefficients
        .iter()
        .rposition(|coefficient| !coefficient.is_zero())
        .map(|index| index + 1)
        .unwrap_or(0)
}

pub(crate) fn dense_div_rem(dividend: &[Rat], divisor: &[Rat]) -> Option<(Vec<Rat>, Vec<Rat>)> {
    let divisor = trim(divisor.to_vec());
    if divisor.is_empty() {
        return None;
    }
    let mut remainder = trim(dividend.to_vec());
    let divisor_degree = dense_degree(&divisor);
    if remainder.len() < divisor_degree {
        return Some((Vec::new(), remainder));
    }
    let mut quotient = vec![Rat::zero(); remainder.len() - divisor_degree + 1];
    let leading = divisor[divisor_degree - 1].clone();
    while remainder.len() >= divisor_degree {
        let degree = remainder.len() - divisor_degree;
        let coefficient = remainder[remainder.len() - 1].div(&leading).ok()?;
        quotient[degree] = coefficient.clone();
        for (index, divisor_coefficient) in divisor.iter().enumerate().take(divisor_degree) {
            let position = degree + index;
            let value = remainder[position].sub(&coefficient.mul(divisor_coefficient));
            remainder[position] = value;
        }
        remainder = trim(remainder);
    }
    Some((trim(quotient), remainder))
}

pub(crate) fn dense_gcd(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let mut left = trim(left.to_vec());
    let mut right = trim(right.to_vec());
    while !right.is_empty() {
        let remainder = match dense_div_rem(&left, &right) {
            Some((_, remainder)) => remainder,
            None => return vec![Rat::one()],
        };
        left = right;
        right = remainder;
    }
    if left.is_empty() {
        return vec![Rat::one()];
    }
    let leading = left.last().cloned().unwrap_or_else(Rat::one);
    left.iter()
        .map(|coefficient| coefficient.div(&leading).unwrap_or_else(|_| Rat::zero()))
        .collect()
}

pub(crate) fn dense_eval(coefficients: &[Rat], value: &Rat) -> Rat {
    let mut result = Rat::zero();
    for coefficient in coefficients.iter().rev() {
        result = result.mul(value).add(coefficient);
    }
    result
}

pub(crate) fn dense_derivative(coefficients: &[Rat]) -> Vec<Rat> {
    let mut derivative = Vec::with_capacity(coefficients.len().saturating_sub(1));
    for (index, coefficient) in coefficients.iter().enumerate().skip(1) {
        let factor = Rat::from_i64(index as i64, 1).unwrap_or_else(|_| Rat::one());
        derivative.push(coefficient.mul(&factor));
    }
    trim(derivative)
}

pub(crate) fn dense_multiply(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let mut product = vec![Rat::zero(); left.len() + right.len() - 1];
    for (left_index, left_coefficient) in left.iter().enumerate() {
        for (right_index, right_coefficient) in right.iter().enumerate() {
            let index = left_index + right_index;
            product[index] = product[index].add(&left_coefficient.mul(right_coefficient));
        }
    }
    trim(product)
}

pub(crate) fn dense_power(base: &[Rat], exponent: u32) -> Vec<Rat> {
    let mut result = vec![Rat::one()];
    for _ in 0..exponent {
        result = dense_multiply(&result, base);
    }
    result
}

pub(crate) fn dense_nth_root(coefficients: &[Rat], degree: u32) -> Option<Vec<Rat>> {
    if degree < 2 {
        return None;
    }
    let coefficients = trim(coefficients.to_vec());
    if coefficients.is_empty() {
        return None;
    }
    let degree_usize = degree as usize;
    let mut shift = 0usize;
    while coefficients.get(shift).is_some_and(Rat::is_zero) {
        shift += 1;
    }
    if shift >= coefficients.len() || !shift.is_multiple_of(degree_usize) {
        return None;
    }
    let core = &coefficients[shift..];
    let constant = rational_nth_root(core.first()?, degree)?;
    if constant.is_zero() {
        return None;
    }
    let core_degree = core.len().checked_sub(1)?;
    if !core_degree.is_multiple_of(degree_usize) {
        return None;
    }
    let root_degree = core_degree / degree_usize;
    let mut root = vec![Rat::zero(); root_degree + 1];
    root[0] = constant.clone();
    let denominator = Rat::from_i64(degree as i64, 1)
        .ok()?
        .mul(&rat_power(&constant, degree - 1));
    for power in 1..=root_degree {
        let expansion = dense_power(&root, degree);
        let target = core.get(power).cloned().unwrap_or_else(Rat::zero);
        let current = expansion.get(power).cloned().unwrap_or_else(Rat::zero);
        root[power] = target.sub(&current).div(&denominator).ok()?;
    }
    if dense_power(&root, degree) != core {
        return None;
    }
    let mut result = vec![Rat::zero(); shift / degree_usize];
    result.extend(root);
    Some(result)
}

pub(crate) fn rat_power(value: &Rat, exponent: u32) -> Rat {
    let mut result = Rat::one();
    for _ in 0..exponent {
        result = result.mul(value);
    }
    result
}

pub(crate) fn rational_nth_root(value: &Rat, degree: u32) -> Option<Rat> {
    if degree == 0 {
        return None;
    }
    let numerator = integer_nth_root(&value.num(), degree)?;
    let denominator = integer_nth_root(&value.den(), degree)?;
    if denominator.is_zero() {
        return None;
    }
    Rat::from_ints(numerator, denominator).ok()
}

pub(crate) fn integer_nth_root(value: &Int, degree: u32) -> Option<Int> {
    let value = value.to_i64()?;
    if value == 0 {
        return Some(Int::zero());
    }
    if value < 0 && degree.is_multiple_of(2) {
        return None;
    }
    let magnitude = value.unsigned_abs() as i128;
    let mut low = 0i128;
    let mut high = magnitude;
    let mut best = 0i128;
    while low <= high {
        let middle = low + (high - low) / 2;
        let mut power = 1i128;
        let mut overflow = false;
        for _ in 0..degree {
            match power.checked_mul(middle) {
                Some(next) => power = next,
                None => {
                    overflow = true;
                    break;
                }
            }
        }
        if overflow || power > magnitude {
            high = middle - 1;
        } else {
            best = middle;
            low = middle + 1;
        }
    }
    let root = Int::from_i64(i64::try_from(best).ok()?);
    Some(if value < 0 { root.neg() } else { root })
}

pub(crate) fn lcm(left: &Int, right: &Int) -> Int {
    if left.is_zero() || right.is_zero() {
        return Int::zero();
    }
    let gcd = left.gcd(right).abs();
    left.mul(right)
        .divrem(&gcd)
        .map(|(quotient, _)| quotient.abs())
        .unwrap_or_else(|_| Int::one())
}

pub(crate) fn integer_sqrt(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(middle);
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

pub(crate) fn rational_sqrt(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_sqrt(numerator)?;
    let denominator = integer_sqrt(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
}

pub(crate) fn integer_cbrt(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let cube = (middle as i128) * (middle as i128) * (middle as i128);
        if cube == value as i128 {
            return Some(middle);
        }
        if cube < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

pub(crate) fn rational_cbrt(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_cbrt(numerator)?;
    let denominator = integer_cbrt(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
}
