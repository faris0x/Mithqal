//! Exact integer polynomials, modular factorization, Hensel lifting, and
//! Zassenhaus factorization over the integers.
//!
//! This module replaces the machine word limit in the older Hensel path. It is
//! the polynomial substrate for number field integral bases, prime ideal
//! decomposition, and field discriminant computations.
//!
//! Modular factorization assumes a monic squarefree input. Equal degree
//! splitting uses a deterministic candidate sequence with a fixed attempt
//! bound, so a pathological input can fail instead of looping. Zassenhaus
//! factorization requires monic input above degree one.

#[allow(unused_imports)]
use crate::prelude::*;

use core::cmp::Ordering;

use crate::num::{Int, Rat};

use super::dense::{dense_div_rem, dense_gcd, lcm};
use super::intmatrix::{div_exact, extended_gcd};
use super::lattice::integer_sqrt;

/// A univariate polynomial with integer coefficients, lowest degree first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ZPoly {
    pub(crate) coeffs: Vec<Int>,
}

impl ZPoly {
    /// Build a polynomial, trimming high zero coefficients.
    pub(crate) fn new(coeffs: Vec<Int>) -> Self {
        let mut coeffs = coeffs;
        while coeffs.last().is_some_and(Int::is_zero) {
            coeffs.pop();
        }
        ZPoly { coeffs }
    }

    /// The zero polynomial.
    pub(crate) fn zero() -> Self {
        ZPoly { coeffs: Vec::new() }
    }

    /// The constant polynomial one.
    pub(crate) fn one() -> Self {
        ZPoly {
            coeffs: vec![Int::one()],
        }
    }

    /// The polynomial `x`.
    pub(crate) fn x() -> Self {
        ZPoly {
            coeffs: vec![Int::zero(), Int::one()],
        }
    }

    /// A constant polynomial.
    pub(crate) fn constant(value: Int) -> Self {
        ZPoly::new(vec![value])
    }

    /// True when the polynomial is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    /// True when the polynomial is one.
    pub(crate) fn is_one(&self) -> bool {
        self.coeffs.len() == 1 && self.coeffs[0].is_one()
    }

    /// The degree, or `None` for the zero polynomial.
    pub(crate) fn degree(&self) -> Option<usize> {
        self.coeffs.len().checked_sub(1)
    }

    /// The leading coefficient.
    pub(crate) fn leading(&self) -> Option<&Int> {
        self.coeffs.last()
    }

    /// True when the leading coefficient is one.
    pub(crate) fn is_monic(&self) -> bool {
        self.leading().is_some_and(Int::is_one)
    }

    /// One coefficient, or zero when out of range.
    pub(crate) fn coefficient(&self, index: usize) -> Int {
        self.coeffs.get(index).cloned().unwrap_or_else(Int::zero)
    }

    /// The sum of two polynomials.
    pub(crate) fn add(&self, other: &ZPoly) -> ZPoly {
        let length = self.coeffs.len().max(other.coeffs.len());
        let mut result = Vec::with_capacity(length);
        for index in 0..length {
            result.push(self.coefficient(index).add(&other.coefficient(index)));
        }
        ZPoly::new(result)
    }

    /// The difference of two polynomials.
    pub(crate) fn sub(&self, other: &ZPoly) -> ZPoly {
        let length = self.coeffs.len().max(other.coeffs.len());
        let mut result = Vec::with_capacity(length);
        for index in 0..length {
            result.push(self.coefficient(index).sub(&other.coefficient(index)));
        }
        ZPoly::new(result)
    }

    /// The negation.
    pub(crate) fn neg(&self) -> ZPoly {
        ZPoly::new(self.coeffs.iter().map(Int::neg).collect())
    }

    /// The product of two polynomials.
    pub(crate) fn multiply(&self, other: &ZPoly) -> ZPoly {
        if self.is_zero() || other.is_zero() {
            return ZPoly::zero();
        }
        let mut result = vec![Int::zero(); self.coeffs.len() + other.coeffs.len() - 1];
        for (left_index, left) in self.coeffs.iter().enumerate() {
            if left.is_zero() {
                continue;
            }
            for (right_index, right) in other.coeffs.iter().enumerate() {
                if right.is_zero() {
                    continue;
                }
                let index = left_index + right_index;
                result[index] = result[index].add(&left.mul(right));
            }
        }
        ZPoly::new(result)
    }

    /// Multiply every coefficient by a scalar.
    pub(crate) fn scalar_mul(&self, factor: &Int) -> ZPoly {
        ZPoly::new(self.coeffs.iter().map(|value| value.mul(factor)).collect())
    }

    /// The formal derivative.
    pub(crate) fn derivative(&self) -> ZPoly {
        if self.coeffs.len() < 2 {
            return ZPoly::zero();
        }
        let mut result = Vec::with_capacity(self.coeffs.len() - 1);
        for (index, value) in self.coeffs.iter().enumerate().skip(1) {
            result.push(value.mul(&Int::from_i64(index as i64)));
        }
        ZPoly::new(result)
    }

    /// Evaluate at an integer.
    pub(crate) fn evaluate(&self, value: &Int) -> Int {
        let mut result = Int::zero();
        for coefficient in self.coeffs.iter().rev() {
            result = result.mul(value).add(coefficient);
        }
        result
    }

    /// The gcd of all coefficients.
    pub(crate) fn content(&self) -> Int {
        let mut result = Int::zero();
        for value in &self.coeffs {
            result = result.gcd(&value.abs());
        }
        result
    }

    /// Divide out the content and make the leading coefficient positive.
    pub(crate) fn primitive_part(&self) -> ZPoly {
        if self.is_zero() {
            return ZPoly::zero();
        }
        let content = self.content();
        let mut result = if content.is_zero() {
            self.clone()
        } else {
            ZPoly::new(
                self.coeffs
                    .iter()
                    .map(|value| div_exact(value, &content))
                    .collect(),
            )
        };
        if result.leading().is_some_and(|value| value.sign() < 0) {
            result = result.neg();
        }
        result
    }

    /// Exact division, or `None` when the remainder is nonzero or the quotient
    /// is not integral.
    pub(crate) fn divide_exact(&self, divisor: &ZPoly) -> Option<ZPoly> {
        let (quotient, remainder) = dense_div_rem(&self.to_rat_coeffs(), &divisor.to_rat_coeffs())?;
        if !remainder.is_empty() {
            return None;
        }
        ZPoly::from_rat_coeffs(&quotient)
    }

    /// The monic gcd over the rationals, returned with integer coefficients.
    pub(crate) fn gcd(&self, other: &ZPoly) -> ZPoly {
        let coefficients = dense_gcd(&self.to_rat_coeffs(), &other.to_rat_coeffs());
        let mut denominator = Int::one();
        for value in &coefficients {
            denominator = lcm(&denominator, &value.den());
        }
        let mut integers = Vec::with_capacity(coefficients.len());
        for value in &coefficients {
            let scale = div_exact(&denominator, &value.den());
            integers.push(value.num().mul(&scale));
        }
        ZPoly::new(integers).primitive_part()
    }

    /// The resultant over the integers.
    pub(crate) fn resultant(&self, other: &ZPoly) -> Option<Int> {
        let value = resultant_rat(self.to_rat_coeffs(), other.to_rat_coeffs())?;
        if value.den().is_one() {
            Some(value.num())
        } else {
            None
        }
    }

    /// Reduce every coefficient modulo a positive modulus.
    pub(crate) fn reduce_mod(&self, modulus: &Int) -> ZPoly {
        ZPoly::new(
            self.coeffs
                .iter()
                .map(|value| reduce_mod(value, modulus))
                .collect(),
        )
    }

    /// The full factorization into irreducible factors with multiplicities.
    ///
    /// Polynomials of degree greater than one must be monic. Non-monic higher
    /// degree input returns `None` until leading coefficient handling is added.
    pub(crate) fn factor(&self) -> Option<Vec<(ZPoly, u32)>> {
        if self.is_zero() {
            return None;
        }
        if self.degree() == Some(0) {
            return Some(Vec::new());
        }
        let normalized = self.primitive_part();
        let mut result = Vec::new();
        for (factor, multiplicity) in squarefree_decomposition(&normalized)? {
            let irreducible = factor_squarefree(&factor)?;
            for part in irreducible {
                result.push((part, multiplicity));
            }
        }
        let mut rebuilt = ZPoly::one();
        for (factor, multiplicity) in &result {
            if factor.degree().unwrap_or(0) == 0 {
                return None;
            }
            for _ in 0..*multiplicity {
                rebuilt = rebuilt.multiply(factor);
            }
        }
        if rebuilt != normalized {
            return None;
        }
        Some(result)
    }

    /// True when the primitive polynomial is irreducible over the integers.
    ///
    /// Non-monic polynomials above degree one are not decided and return
    /// `false`.
    pub(crate) fn irreducibility(&self) -> Option<bool> {
        match self.degree() {
            None => Some(false),
            Some(0) => Some(true),
            Some(1) => Some(true),
            Some(_) => self
                .factor()
                .map(|factors| factors.len() == 1 && factors[0].1 == 1),
        }
    }

    /// True when the primitive polynomial is irreducible over the integers.
    ///
    /// Non-monic polynomials above degree one and undecided factorizations
    /// return `false`. Use [`ZPoly::irreducibility`] when the distinction
    /// matters.
    pub(crate) fn is_irreducible(&self) -> bool {
        self.irreducibility().unwrap_or(false)
    }

    pub(crate) fn to_rat_coeffs(&self) -> Vec<Rat> {
        self.coeffs
            .iter()
            .map(|value| Rat::from_ints(value.clone(), Int::one()).unwrap_or_else(|_| Rat::zero()))
            .collect()
    }

    pub(crate) fn from_rat_coeffs(coefficients: &[Rat]) -> Option<ZPoly> {
        let mut integers = Vec::with_capacity(coefficients.len());
        for value in coefficients {
            if !value.den().is_one() {
                return None;
            }
            integers.push(value.num());
        }
        Some(ZPoly::new(integers))
    }
}

fn resultant_rat(mut left: Vec<Rat>, mut right: Vec<Rat>) -> Option<Rat> {
    left = trim_rat(left);
    right = trim_rat(right);
    if left.is_empty() || right.is_empty() {
        return Some(Rat::zero());
    }
    if left.len() < right.len() {
        let sign = if ((left.len() - 1) * (right.len() - 1)).is_multiple_of(2) {
            Rat::one()
        } else {
            Rat::from_i64(-1, 1).ok()?
        };
        return Some(sign.mul(&resultant_rat(right, left)?));
    }
    let mut result = Rat::one();
    loop {
        let left_degree = left.len() - 1;
        if right.is_empty() {
            return Some(Rat::zero());
        }
        let right_degree = right.len() - 1;
        if right_degree == 0 {
            return Some(result.mul(&rat_pow(&right[0], left_degree)));
        }
        let (_, remainder) = dense_div_rem(&left, &right)?;
        if remainder.is_empty() {
            return Some(Rat::zero());
        }
        if !(left_degree * right_degree).is_multiple_of(2) {
            result = result.neg();
        }
        let exponent = left_degree - (remainder.len() - 1);
        let leading = right.last()?.clone();
        result = result.mul(&rat_pow(&leading, exponent));
        left = right;
        right = remainder;
    }
}

fn rat_pow(base: &Rat, exponent: usize) -> Rat {
    let mut result = Rat::one();
    for _ in 0..exponent {
        result = result.mul(base);
    }
    result
}

fn trim_rat(mut values: Vec<Rat>) -> Vec<Rat> {
    while values.last().is_some_and(Rat::is_zero) {
        values.pop();
    }
    values
}

/// Squarefree decomposition by Yun's algorithm.
fn squarefree_decomposition(polynomial: &ZPoly) -> Option<Vec<(ZPoly, u32)>> {
    let derivative = polynomial.derivative();
    let shared = if derivative.is_zero() {
        polynomial.clone()
    } else {
        polynomial.gcd(&derivative)
    };
    let mut b = polynomial.divide_exact(&shared)?;
    let quotient = if derivative.is_zero() {
        ZPoly::zero()
    } else {
        derivative.divide_exact(&shared)?
    };
    let mut d = quotient.sub(&b.derivative());
    let mut result = Vec::new();
    let mut multiplicity = 1u32;
    while b.degree().unwrap_or(0) > 0 {
        let factor = b.gcd(&d);
        if factor.degree().unwrap_or(0) > 0 {
            result.push((factor.clone(), multiplicity));
        }
        let next_b = b.divide_exact(&factor)?;
        let next_c = d.divide_exact(&factor)?;
        let next_d = next_c.sub(&next_b.derivative());
        b = next_b;
        d = next_d;
        multiplicity += 1;
        if multiplicity > 128 {
            return None;
        }
    }
    Some(result)
}

/// Factor a squarefree primitive polynomial into irreducibles.
fn factor_squarefree(polynomial: &ZPoly) -> Option<Vec<ZPoly>> {
    let degree = polynomial.degree()?;
    if degree == 0 {
        return Some(Vec::new());
    }
    if degree == 1 {
        return Some(vec![polynomial.clone()]);
    }
    if !polynomial.is_monic() {
        return None;
    }
    for candidate in 2i64..512 {
        if !is_small_prime(candidate) {
            continue;
        }
        let prime = Int::from_i64(candidate);
        let reduced = polynomial.reduce_mod(&prime);
        if reduced.degree() != Some(degree) {
            continue;
        }
        if !is_squarefree_mod(&reduced.coeffs, &prime) {
            continue;
        }
        let Some(modular) = factor_mod_p_squarefree(&reduced, &prime) else {
            continue;
        };
        if modular.len() == 1 {
            return Some(vec![polynomial.clone()]);
        }
        let bound = coefficient_bound(polynomial);
        let target = bound.mul(&Int::from_i64(2)).add(&Int::one());
        let mut modulus = prime.clone();
        while modulus.cmp(&target) != Ordering::Greater {
            modulus = modulus.mul(&prime);
        }
        let Some(lifted) = hensel_lift_factors(&polynomial.coeffs, &modular, &prime, &modulus)
        else {
            continue;
        };
        if lifted.len() > 20 {
            continue;
        }
        // A complete modular factorization lifted beyond the coefficient bound
        // decides irreducibility: if no subset divides, no factor exists.
        return Some(
            recombine(polynomial, &lifted, &modulus).unwrap_or_else(|| vec![polynomial.clone()]),
        );
    }
    None
}

fn coefficient_bound(polynomial: &ZPoly) -> Int {
    let mut sum = Int::zero();
    for value in &polynomial.coeffs {
        sum = sum.add(&value.mul(value));
    }
    let norm = integer_sqrt(&sum).add(&Int::one());
    let degree = polynomial.degree().unwrap_or(0) as u32;
    norm.mul(&Int::from_i64(2).pow_u32(degree))
}

fn recombine(polynomial: &ZPoly, lifted: &[ZPoly], modulus: &Int) -> Option<Vec<ZPoly>> {
    let count = lifted.len();
    if count > 20 {
        return None;
    }
    let total = 1u64 << count;
    for size in 1..=count / 2 {
        for mask in 1u64..total {
            if mask.count_ones() as usize != size {
                continue;
            }
            let mut product = vec![Int::one()];
            for (index, factor) in lifted.iter().enumerate() {
                if mask >> index & 1 == 1 {
                    product = poly_mul_mod(&product, &factor.coeffs, modulus);
                }
            }
            let candidate = ZPoly::new(symmetric_reduce(&product, modulus)).primitive_part();
            let Some(degree) = candidate.degree() else {
                continue;
            };
            if degree == 0 || degree >= polynomial.degree().unwrap_or(0) {
                continue;
            }
            if let Some(quotient) = polynomial.divide_exact(&candidate) {
                let mut left = factor_squarefree(&candidate)?;
                let mut right = factor_squarefree(&quotient)?;
                left.append(&mut right);
                return Some(left);
            }
        }
    }
    None
}

fn is_small_prime(value: i64) -> bool {
    if value < 2 {
        return false;
    }
    let mut divisor = 2i64;
    while divisor * divisor <= value {
        if value % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

fn symmetric_reduce(coefficients: &[Int], modulus: &Int) -> Vec<Int> {
    let half = modulus
        .divrem(&Int::from_i64(2))
        .map(|(quotient, _)| quotient)
        .unwrap_or_else(|_| Int::zero());
    coefficients
        .iter()
        .map(|value| {
            let reduced = reduce_mod(value, modulus);
            if reduced.cmp(&half) == Ordering::Greater {
                reduced.sub(modulus)
            } else {
                reduced
            }
        })
        .collect()
}

/// Reduce a value into `[0, modulus)`.
pub(crate) fn reduce_mod(value: &Int, modulus: &Int) -> Int {
    if modulus.is_zero() {
        return value.clone();
    }
    match value.divrem(modulus) {
        Ok((_, remainder)) => {
            if remainder.sign() < 0 {
                remainder.add(modulus)
            } else {
                remainder
            }
        }
        Err(_) => Int::zero(),
    }
}

/// The inverse of a value modulo a positive modulus.
pub(crate) fn invert_mod(value: &Int, modulus: &Int) -> Option<Int> {
    let reduced = reduce_mod(value, modulus);
    let (gcd, coefficient, _) = extended_gcd(&reduced, modulus);
    if !gcd.is_one() {
        return None;
    }
    Some(reduce_mod(&coefficient, modulus))
}

fn poly_add_mod(left: &[Int], right: &[Int], modulus: &Int) -> Vec<Int> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        let left_value = left.get(index).cloned().unwrap_or_else(Int::zero);
        let right_value = right.get(index).cloned().unwrap_or_else(Int::zero);
        result.push(reduce_mod(&left_value.add(&right_value), modulus));
    }
    trim_mod(result, modulus)
}

fn poly_sub_mod(left: &[Int], right: &[Int], modulus: &Int) -> Vec<Int> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        let left_value = left.get(index).cloned().unwrap_or_else(Int::zero);
        let right_value = right.get(index).cloned().unwrap_or_else(Int::zero);
        result.push(reduce_mod(&left_value.sub(&right_value), modulus));
    }
    trim_mod(result, modulus)
}

fn poly_mul_mod(left: &[Int], right: &[Int], modulus: &Int) -> Vec<Int> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let mut result = vec![Int::zero(); left.len() + right.len() - 1];
    for (left_index, left_value) in left.iter().enumerate() {
        if left_value.is_zero() {
            continue;
        }
        for (right_index, right_value) in right.iter().enumerate() {
            if right_value.is_zero() {
                continue;
            }
            let index = left_index + right_index;
            result[index] = result[index].add(&left_value.mul(right_value));
        }
    }
    trim_mod(result, modulus)
}

fn trim_mod(mut coefficients: Vec<Int>, modulus: &Int) -> Vec<Int> {
    for value in &mut coefficients {
        *value = reduce_mod(value, modulus);
    }
    while coefficients.last().is_some_and(Int::is_zero) {
        coefficients.pop();
    }
    coefficients
}

fn poly_div_rem_mod(
    dividend: &[Int],
    divisor: &[Int],
    modulus: &Int,
) -> Option<(Vec<Int>, Vec<Int>)> {
    let divisor = trim_mod(divisor.to_vec(), modulus);
    if divisor.is_empty() {
        return None;
    }
    let mut remainder = trim_mod(dividend.to_vec(), modulus);
    let divisor_degree = divisor.len();
    if remainder.len() < divisor_degree {
        return Some((Vec::new(), remainder));
    }
    let leading_inverse = invert_mod(divisor.last()?, modulus)?;
    let mut quotient = vec![Int::zero(); remainder.len() - divisor_degree + 1];
    while remainder.len() >= divisor_degree {
        let shift = remainder.len() - divisor_degree;
        let coefficient = reduce_mod(
            &remainder[remainder.len() - 1].mul(&leading_inverse),
            modulus,
        );
        quotient[shift] = coefficient.clone();
        for (index, value) in divisor.iter().enumerate() {
            let position = shift + index;
            let product = coefficient.mul(value);
            remainder[position] = reduce_mod(&remainder[position].sub(&product), modulus);
        }
        remainder = trim_mod(remainder, modulus);
    }
    Some((trim_mod(quotient, modulus), remainder))
}

fn poly_gcd_mod(left: &[Int], right: &[Int], modulus: &Int) -> Option<Vec<Int>> {
    let mut left = trim_mod(left.to_vec(), modulus);
    let mut right = trim_mod(right.to_vec(), modulus);
    while !right.is_empty() {
        let (_, remainder) = poly_div_rem_mod(&left, &right, modulus)?;
        left = right;
        right = remainder;
    }
    monic_mod(&left, modulus)
}

fn poly_xgcd_mod(
    left: &[Int],
    right: &[Int],
    modulus: &Int,
) -> Option<(Vec<Int>, Vec<Int>, Vec<Int>)> {
    let mut old_r = trim_mod(left.to_vec(), modulus);
    let mut r = trim_mod(right.to_vec(), modulus);
    let mut old_s = vec![Int::one()];
    let mut s = Vec::new();
    let mut old_t = Vec::new();
    let mut t = vec![Int::one()];
    while !r.is_empty() {
        let (quotient, remainder) = poly_div_rem_mod(&old_r, &r, modulus)?;
        old_r = r;
        r = remainder;
        let next_s = poly_sub_mod(&old_s, &poly_mul_mod(&quotient, &s, modulus), modulus);
        old_s = s;
        s = next_s;
        let next_t = poly_sub_mod(&old_t, &poly_mul_mod(&quotient, &t, modulus), modulus);
        old_t = t;
        t = next_t;
    }
    if old_r.is_empty() {
        return None;
    }
    let leading_inverse = invert_mod(old_r.last()?, modulus)?;
    let monic = scale_mod(&old_r, &leading_inverse, modulus);
    Some((
        monic,
        scale_mod(&old_s, &leading_inverse, modulus),
        scale_mod(&old_t, &leading_inverse, modulus),
    ))
}

fn monic_mod(coefficients: &[Int], modulus: &Int) -> Option<Vec<Int>> {
    let coefficients = trim_mod(coefficients.to_vec(), modulus);
    let Some(leading) = coefficients.last() else {
        return Some(Vec::new());
    };
    let inverse = invert_mod(leading, modulus)?;
    Some(scale_mod(&coefficients, &inverse, modulus))
}

fn scale_mod(coefficients: &[Int], factor: &Int, modulus: &Int) -> Vec<Int> {
    trim_mod(
        coefficients
            .iter()
            .map(|value| reduce_mod(&value.mul(factor), modulus))
            .collect(),
        modulus,
    )
}

fn poly_pow_mod(base: &[Int], exponent: &Int, modulus: &[Int], prime: &Int) -> Option<Vec<Int>> {
    let mut result = vec![Int::one()];
    let mut factor = poly_div_rem_mod(base, modulus, prime)?.1;
    let mut exponent = exponent.clone();
    let two = Int::from_i64(2);
    while exponent.sign() > 0 {
        let (quotient, remainder) = exponent.divrem(&two).ok()?;
        if !remainder.is_zero() {
            result = poly_mul_mod(&result, &factor, prime);
            result = poly_div_rem_mod(&result, modulus, prime)?.1;
        }
        exponent = quotient;
        if exponent.is_zero() {
            break;
        }
        factor = poly_mul_mod(&factor, &factor, prime);
        factor = poly_div_rem_mod(&factor, modulus, prime)?.1;
    }
    Some(result)
}

fn poly_derivative_mod(coefficients: &[Int], modulus: &Int) -> Vec<Int> {
    if coefficients.len() < 2 {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(coefficients.len() - 1);
    for (index, value) in coefficients.iter().enumerate().skip(1) {
        result.push(reduce_mod(
            &value.mul(&Int::from_i64(index as i64)),
            modulus,
        ));
    }
    trim_mod(result, modulus)
}

fn is_squarefree_mod(coefficients: &[Int], prime: &Int) -> bool {
    let derivative = poly_derivative_mod(coefficients, prime);
    if derivative.is_empty() {
        return coefficients.len() <= 1;
    }
    match poly_gcd_mod(coefficients, &derivative, prime) {
        Some(gcd) => gcd.len() <= 1,
        None => false,
    }
}

/// Factor a monic squarefree polynomial modulo a prime.
pub(crate) fn factor_mod_p_squarefree(polynomial: &ZPoly, prime: &Int) -> Option<Vec<ZPoly>> {
    let normalized = monic_mod(&polynomial.reduce_mod(prime).coeffs, prime)?;
    if normalized.is_empty() || !is_squarefree_mod(&normalized, prime) {
        return None;
    }
    let mut remaining = normalized.clone();
    let mut factors = Vec::new();
    let x_poly = vec![Int::zero(), Int::one()];
    let mut degree = 1usize;
    while remaining.len() > 1 && 2 * degree < remaining.len() {
        let exponent = prime.pow_u32(degree as u32);
        let power = poly_pow_mod(&x_poly, &exponent, &remaining, prime)?;
        let difference = poly_sub_mod(&power, &x_poly, prime);
        let group = poly_gcd_mod(&remaining, &difference, prime)?;
        if group.len() > 1 {
            equal_degree_factor(&group, degree, prime, &mut factors)?;
            let (quotient, remainder) = poly_div_rem_mod(&remaining, &group, prime)?;
            if !remainder.is_empty() {
                return None;
            }
            remaining = monic_mod(&quotient, prime)?;
        }
        degree += 1;
    }
    if remaining.len() > 1 {
        factors.push(ZPoly::new(remaining));
    }
    let mut product = vec![Int::one()];
    for factor in &factors {
        product = poly_mul_mod(&product, &factor.coeffs, prime);
    }
    if product != normalized {
        return None;
    }
    Some(factors)
}

fn equal_degree_factor(
    polynomial: &[Int],
    degree: usize,
    prime: &Int,
    output: &mut Vec<ZPoly>,
) -> Option<()> {
    let polynomial_degree = polynomial.len() - 1;
    if polynomial_degree <= degree {
        output.push(ZPoly::new(polynomial.to_vec()));
        return Some(());
    }
    let two = Int::from_i64(2);
    let exponent = if prime.cmp(&two) == Ordering::Equal {
        None
    } else {
        Some(
            prime
                .pow_u32(degree as u32)
                .sub(&Int::one())
                .divrem(&two)
                .ok()?
                .0,
        )
    };
    for attempt in 0..512usize {
        let candidate = candidate_polynomial(attempt);
        let candidate = poly_div_rem_mod(&candidate, polynomial, prime)?.1;
        if candidate.is_empty() {
            continue;
        }
        let map = if let Some(exponent) = &exponent {
            let value = poly_pow_mod(&candidate, exponent, polynomial, prime)?;
            poly_sub_mod(&value, &[Int::one()], prime)
        } else {
            trace_map(&candidate, degree, polynomial, prime)?
        };
        let factor = poly_gcd_mod(polynomial, &map, prime)?;
        if factor.len() > 1 && factor.len() < polynomial.len() {
            equal_degree_factor(&factor, degree, prime, output)?;
            let (quotient, remainder) = poly_div_rem_mod(polynomial, &factor, prime)?;
            if !remainder.is_empty() {
                return None;
            }
            equal_degree_factor(&quotient, degree, prime, output)?;
            return Some(());
        }
    }
    None
}

fn candidate_polynomial(index: usize) -> Vec<Int> {
    let power = index / 8;
    let constant = index % 8;
    let mut coefficients = vec![Int::zero(); power + 1];
    coefficients[power] = Int::one();
    coefficients[0] = Int::from_i64(constant as i64);
    coefficients
}

fn trace_map(
    candidate: &[Int],
    degree: usize,
    polynomial: &[Int],
    prime: &Int,
) -> Option<Vec<Int>> {
    let mut total = Vec::new();
    let mut term = candidate.to_vec();
    for _ in 0..degree {
        total = poly_add_mod(&total, &term, prime);
        term = poly_mul_mod(&term, &term, prime);
        term = poly_div_rem_mod(&term, polynomial, prime)?.1;
    }
    Some(total)
}

/// Lift a factorization of a monic polynomial modulo a prime to a higher power
/// of that prime.
pub(crate) fn hensel_lift_factors(
    polynomial: &[Int],
    factors: &[ZPoly],
    prime: &Int,
    target: &Int,
) -> Option<Vec<ZPoly>> {
    if factors.is_empty() {
        return None;
    }
    let lifted = lift_tree(polynomial, factors, prime, target)?;
    let mut product = vec![Int::one()];
    for factor in &lifted {
        product = poly_mul_mod(&product, &factor.coeffs, target);
    }
    if ZPoly::new(product) != ZPoly::new(polynomial.to_vec()).reduce_mod(target) {
        return None;
    }
    Some(lifted)
}

fn lift_tree(
    polynomial: &[Int],
    factors: &[ZPoly],
    prime: &Int,
    target: &Int,
) -> Option<Vec<ZPoly>> {
    if factors.len() == 1 {
        return Some(vec![ZPoly::new(polynomial.to_vec())]);
    }
    let middle = factors.len() / 2;
    let left_mod = product_mod(
        &factors[..middle]
            .iter()
            .map(|factor| factor.coeffs.clone())
            .collect::<Vec<_>>(),
        prime,
    );
    let right_mod = product_mod(
        &factors[middle..]
            .iter()
            .map(|factor| factor.coeffs.clone())
            .collect::<Vec<_>>(),
        prime,
    );
    let (left, right) = hensel_lift_two(
        &ZPoly::new(polynomial.to_vec()),
        &ZPoly::new(left_mod),
        &ZPoly::new(right_mod),
        prime,
        target,
    )?;
    let mut output = lift_tree(&left.coeffs, &factors[..middle], prime, target)?;
    output.extend(lift_tree(&right.coeffs, &factors[middle..], prime, target)?);
    Some(output)
}

fn product_mod(factors: &[Vec<Int>], modulus: &Int) -> Vec<Int> {
    let mut product = vec![Int::one()];
    for factor in factors {
        product = poly_mul_mod(&product, factor, modulus);
    }
    product
}

/// Lift a two factor decomposition `f = g h` from modulo `p` to modulo
/// `target`.
pub(crate) fn hensel_lift_two(
    polynomial: &ZPoly,
    left: &ZPoly,
    right: &ZPoly,
    prime: &Int,
    target: &Int,
) -> Option<(ZPoly, ZPoly)> {
    if !polynomial.is_monic() || !is_power_of(target, prime) {
        return None;
    }
    let mut left_coeffs = monic_mod(&left.reduce_mod(prime).coeffs, prime)?;
    let mut right_coeffs = monic_mod(&right.reduce_mod(prime).coeffs, prime)?;
    let polynomial_coeffs = polynomial.coeffs.clone();
    let (gcd, mutual_left, mutual_right) = poly_xgcd_mod(&left_coeffs, &right_coeffs, prime)?;
    if gcd.len() != 1 {
        return None;
    }
    let mut s = mutual_left;
    let mut t = mutual_right;
    let mut modulus = prime.clone();
    while modulus.cmp(target) == Ordering::Less {
        let next = modulus.mul(&modulus);
        let product = poly_mul_mod(&left_coeffs, &right_coeffs, &next);
        let error = poly_sub_mod(&polynomial_coeffs, &product, &next);
        let mut divided = Vec::with_capacity(error.len());
        for value in &error {
            let (quotient, remainder) = value.divrem(&modulus).ok()?;
            if !remainder.is_zero() {
                return None;
            }
            divided.push(quotient);
        }
        let divided = trim_mod(divided, &modulus);
        // u solves H u = e modulo G, with G the left factor.
        let scaled = poly_mul_mod(&t, &divided, &modulus);
        let (_, u) = poly_div_rem_mod(&scaled, &left_coeffs, &modulus)?;
        // v is obtained by exact division of the residual by the left factor.
        let correction = poly_mul_mod(&right_coeffs, &u, &modulus);
        let residual = poly_sub_mod(&divided, &correction, &modulus);
        let (v, remainder) = poly_div_rem_mod(&residual, &left_coeffs, &modulus)?;
        if !remainder.is_empty() {
            return None;
        }
        let left_step = scale_mod(&u, &modulus, &next);
        let right_step = scale_mod(&v, &modulus, &next);
        let new_left = poly_add_mod(&left_coeffs, &left_step, &next);
        let new_right = poly_add_mod(&right_coeffs, &right_step, &next);
        let check = poly_add_mod(
            &poly_mul_mod(&new_left, &s, &next),
            &poly_mul_mod(&new_right, &t, &next),
            &next,
        );
        let delta = poly_sub_mod(&[Int::one()], &check, &next);
        let s_correction = poly_mul_mod(&s, &delta, &next);
        s = poly_add_mod(&s, &s_correction, &next);
        let t_correction = poly_mul_mod(&t, &delta, &next);
        t = poly_add_mod(&t, &t_correction, &next);
        left_coeffs = new_left;
        right_coeffs = new_right;
        modulus = next;
    }
    let left = ZPoly::new(trim_mod(left_coeffs, target));
    let right = ZPoly::new(trim_mod(right_coeffs, target));
    if left.multiply(&right).reduce_mod(target) != polynomial.reduce_mod(target) {
        return None;
    }
    Some((left, right))
}

fn is_power_of(value: &Int, base: &Int) -> bool {
    if value.is_one() || value.sign() <= 0 || base.sign() <= 0 || base.is_one() {
        return false;
    }
    let mut current = Int::one();
    while current.cmp(value) == Ordering::Less {
        current = current.mul(base);
    }
    current == *value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::intmatrix::IntMatrix;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| Int::from_i64(*value))
                .collect(),
        )
    }

    fn multiply_all(factors: &[ZPoly]) -> ZPoly {
        let mut product = ZPoly::one();
        for factor in factors {
            product = product.multiply(factor);
        }
        product
    }

    #[test]
    fn arithmetic_matches_expansion() {
        let left = poly(&[1, 2, 3]);
        let right = poly(&[4, 5]);
        assert_eq!(left.multiply(&right), poly(&[4, 13, 22, 15]));
        assert_eq!(left.derivative(), poly(&[2, 6]));
        assert_eq!(left.evaluate(&Int::from_i64(2)), Int::from_i64(17));
        assert_eq!(left.gcd(&left), left.primitive_part());
    }

    #[test]
    fn resultant_matches_special_values() {
        let first = poly(&[-2, 0, 1]);
        let second = poly(&[-1, 1]);
        assert_eq!(first.resultant(&second), Some(Int::from_i64(-1)));
        assert_eq!(
            poly(&[1, 0, 1]).resultant(&poly(&[2, 0, 1])),
            Some(Int::from_i64(1))
        );
    }

    #[test]
    fn modular_factorization_returns_irreducible_factors() {
        let prime = Int::from_i64(5);
        let factors = factor_mod_p_squarefree(&poly(&[1, 0, 1]), &prime).unwrap();
        assert_eq!(factors.len(), 2);
        let mut product = vec![Int::one()];
        for factor in &factors {
            product = poly_mul_mod(&product, &factor.coeffs, &prime);
        }
        assert_eq!(ZPoly::new(product), poly(&[1, 0, 1]).reduce_mod(&prime));

        let prime = Int::from_i64(3);
        let factors = factor_mod_p_squarefree(&poly(&[1, 0, 0, 0, 1]), &prime).unwrap();
        assert_eq!(factors.len(), 2);
        let mut product = vec![Int::one()];
        for factor in &factors {
            product = poly_mul_mod(&product, &factor.coeffs, &prime);
        }
        assert_eq!(
            ZPoly::new(product),
            poly(&[1, 0, 0, 0, 1]).reduce_mod(&prime)
        );
    }

    #[test]
    fn hensel_lifting_reconstructs_the_polynomial() {
        let prime = Int::from_i64(5);
        let target = Int::from_i64(25);
        let source = poly(&[1, 0, 1]);
        let left = poly(&[3, 1]);
        let right = poly(&[2, 1]);
        let (lifted_left, lifted_right) =
            hensel_lift_two(&source, &left, &right, &prime, &target).unwrap();
        let product = lifted_left.multiply(&lifted_right).reduce_mod(&target);
        assert_eq!(product, source.reduce_mod(&target));
    }

    #[test]
    fn zassenhaus_handles_three_irreducible_quadratics() {
        let product = poly(&[6, 0, 11, 0, 6, 0, 1]);
        let mut factors = product.factor().unwrap();
        factors.sort_by_key(|(factor, _)| factor.degree());
        assert_eq!(factors.len(), 3);
        assert_eq!(
            multiply_all(
                &factors
                    .iter()
                    .map(|(factor, _)| factor.clone())
                    .collect::<Vec<_>>()
            ),
            product.primitive_part()
        );
        for (factor, multiplicity) in factors {
            assert_eq!(multiplicity, 1);
            assert_eq!(factor.degree(), Some(2));
        }
    }

    #[test]
    fn hensel_lifts_three_factors_together() {
        let prime = Int::from_i64(5);
        let target = Int::from_i64(125);
        let source = poly(&[-6, 11, -6, 1]);
        let factors = vec![poly(&[-1, 1]), poly(&[-2, 1]), poly(&[-3, 1])];
        let lifted = hensel_lift_factors(&source.coeffs, &factors, &prime, &target).unwrap();
        assert_eq!(lifted.len(), 3);
        let mut product = vec![Int::one()];
        for factor in &lifted {
            product = poly_mul_mod(&product, &factor.coeffs, &target);
        }
        assert_eq!(ZPoly::new(product), source.reduce_mod(&target));
    }

    #[test]
    fn modular_factorization_handles_characteristic_two() {
        let prime = Int::from_i64(2);
        let factors = factor_mod_p_squarefree(&poly(&[1, 1, 1]), &prime).unwrap();
        assert_eq!(factors.len(), 1);
        let mut product = vec![Int::one()];
        for factor in &factors {
            product = poly_mul_mod(&product, &factor.coeffs, &prime);
        }
        assert_eq!(ZPoly::new(product), poly(&[1, 1, 1]).reduce_mod(&prime));
    }

    fn sylvester_determinant(left: &ZPoly, right: &ZPoly) -> Int {
        let left_degree = left.degree().unwrap();
        let right_degree = right.degree().unwrap();
        let size = left_degree + right_degree;
        let mut rows = vec![vec![Int::zero(); size]; size];
        let left_coefficients: Vec<Int> = left.coeffs.iter().rev().cloned().collect();
        let right_coefficients: Vec<Int> = right.coeffs.iter().rev().cloned().collect();
        for row in 0..right_degree {
            for (offset, value) in left_coefficients.iter().enumerate() {
                rows[row][row + offset] = value.clone();
            }
        }
        for row in 0..left_degree {
            for (offset, value) in right_coefficients.iter().enumerate() {
                rows[right_degree + row][row + offset] = value.clone();
            }
        }
        IntMatrix::from_rows(rows).unwrap().determinant().unwrap()
    }

    #[test]
    fn resultant_agrees_with_the_sylvester_determinant() {
        let mut state = 0x0bad_f00d_dead_beefu64;
        for _ in 0..24 {
            let mut left = ZPoly::one();
            let mut right = ZPoly::one();
            for _ in 0..2 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let root = ((state >> 33) % 7) as i64 - 3;
                left = left.multiply(&poly(&[-root, 1]));
            }
            for _ in 0..3 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let root = ((state >> 33) % 7) as i64 - 3;
                right = right.multiply(&poly(&[-root, 1]));
            }
            assert_eq!(
                left.resultant(&right),
                Some(sylvester_determinant(&left, &right))
            );
        }
    }

    #[test]
    fn gcd_and_resultant_detect_shared_factors() {
        let left = poly(&[-3, 3, -1, 1]);
        let right = poly(&[-3, 2, 1]);
        let common = left.gcd(&right);
        assert_eq!(common, poly(&[-1, 1]).primitive_part());
        let shared = poly(&[-1, 1]);
        assert_eq!(left.resultant(&shared), Some(Int::zero()));
        assert_ne!(left.resultant(&poly(&[1, 1, 1])), Some(Int::zero()));
    }

    #[test]
    fn zassenhaus_factors_four_quadratics() {
        let factors = vec![
            poly(&[1, 0, 1]),
            poly(&[2, 0, 1]),
            poly(&[3, 0, 1]),
            poly(&[5, 0, 1]),
        ];
        let mut product = ZPoly::one();
        for factor in &factors {
            product = product.multiply(factor);
        }
        let recovered = product.factor().unwrap();
        assert_eq!(recovered.len(), factors.len());
        assert_eq!(
            multiply_all(
                &recovered
                    .iter()
                    .map(|(factor, _)| factor.clone())
                    .collect::<Vec<_>>()
            ),
            product
        );
    }

    #[test]
    fn hensel_lifts_with_prime_three_to_high_precision() {
        let prime = Int::from_i64(3);
        let target = Int::from_i64(243);
        let source = poly(&[0, 2, -3, 1]);
        let factors = vec![poly(&[0, 1]), poly(&[-1, 1]), poly(&[-2, 1])];
        let lifted = hensel_lift_factors(&source.coeffs, &factors, &prime, &target).unwrap();
        let mut product = vec![Int::one()];
        for factor in &lifted {
            product = poly_mul_mod(&product, &factor.coeffs, &target);
        }
        assert_eq!(ZPoly::new(product), source.reduce_mod(&target));
    }

    #[test]
    fn factorization_edge_cases_are_explicit() {
        assert_eq!(ZPoly::zero().factor(), None);
        assert_eq!(ZPoly::constant(Int::from_i64(7)).factor(), Some(Vec::new()));
        assert_eq!(poly(&[1, 1, 2]).factor(), None);
        assert_eq!(poly(&[2, 1]).factor().unwrap().len(), 1);
    }

    #[test]
    fn modular_factorization_rejects_repeated_input() {
        let prime = Int::from_i64(2);
        assert_eq!(factor_mod_p_squarefree(&poly(&[0, 0, 1]), &prime), None);
        assert_eq!(factor_mod_p_squarefree(&poly(&[1, 0, 1]), &prime), None);
    }

    #[test]
    fn hensel_rejects_a_target_that_is_not_a_prime_power() {
        let source = poly(&[1, 0, 1]);
        let left = poly(&[3, 1]);
        let right = poly(&[2, 1]);
        assert_eq!(
            hensel_lift_two(
                &source,
                &left,
                &right,
                &Int::from_i64(5),
                &Int::from_i64(10)
            ),
            None
        );
    }

    #[test]
    fn hensel_lifts_in_characteristic_two() {
        let prime = Int::from_i64(2);
        let target = Int::from_i64(8);
        let source = poly(&[1, 0, 0, 1]);
        let factors = vec![poly(&[1, 1]), poly(&[1, 1, 1])];
        let lifted = hensel_lift_factors(&source.coeffs, &factors, &prime, &target).unwrap();
        let mut product = vec![Int::one()];
        for factor in &lifted {
            product = poly_mul_mod(&product, &factor.coeffs, &target);
        }
        assert_eq!(ZPoly::new(product), source.reduce_mod(&target));
    }

    #[test]
    fn resultant_is_symmetric_and_multiplicative_over_roots() {
        let left_roots = [-2i64, 0, 3];
        let right_roots = [-1i64, 1, 4];
        let mut left = ZPoly::one();
        for root in left_roots {
            left = left.multiply(&poly(&[-root, 1]));
        }
        let mut right = ZPoly::one();
        for root in right_roots {
            right = right.multiply(&poly(&[-root, 1]));
        }
        let mut expected = Int::one();
        for a in left_roots {
            for b in right_roots {
                expected = expected.mul(&Int::from_i64(a - b));
            }
        }
        assert_eq!(left.resultant(&right), Some(expected));
        let symmetric = right.resultant(&left).unwrap().neg();
        assert_eq!(left.resultant(&right).unwrap(), symmetric);
        assert_eq!(
            left.resultant(&poly(&[0, 1])),
            Some(left.evaluate(&Int::zero()))
        );
    }

    #[test]
    fn irreducibility_handles_a_quartic_with_no_irreducible_modular_reduction() {
        let quartic = poly(&[1, 0, -10, 0, 1]);
        assert_eq!(quartic.irreducibility(), Some(true));
        assert_eq!(quartic.factor().unwrap().len(), 1);
        assert!(quartic.is_irreducible());
        assert_eq!(poly(&[-1, 0, 1]).irreducibility(), Some(false));
        assert_eq!(poly(&[-2, 0, 0, 1]).factor().unwrap().len(), 1);
    }

    #[test]
    fn zassenhaus_random_monic_polynomials_rebuild_and_factor_irreducibly() {
        let mut state = 0xfeed_face_cafe_beefu64;
        for _ in 0..60 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let degree = 1 + (state % 6) as usize;
            let mut coefficients = Vec::with_capacity(degree + 1);
            for _ in 0..degree {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                coefficients.push(Int::from_i64(((state >> 33) % 11) as i64 - 5));
            }
            coefficients.push(Int::one());
            let polynomial = ZPoly::new(coefficients);
            let factors = polynomial.factor().unwrap();
            let rebuilt = multiply_all(
                &factors
                    .iter()
                    .flat_map(|(factor, multiplicity)| {
                        core::iter::repeat_n(factor.clone(), *multiplicity as usize)
                    })
                    .collect::<Vec<_>>(),
            );
            assert_eq!(rebuilt, polynomial.primitive_part());
            for (factor, _) in factors {
                assert_eq!(factor.irreducibility(), Some(true));
            }
        }
    }

    #[test]
    fn zassenhaus_recovers_random_linear_products() {
        let mut state = 0x1234_5678_9abc_def0u64;
        for _ in 0..24 {
            let mut product = ZPoly::one();
            for _ in 0..5 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let root = ((state >> 35) % 9) as i64 - 4;
                product = product.multiply(&poly(&[-root, 1]));
            }
            let factors = product.factor().unwrap();
            let total: u32 = factors.iter().map(|(_, multiplicity)| *multiplicity).sum();
            assert_eq!(total, 5);
            let rebuilt = multiply_all(
                &factors
                    .iter()
                    .flat_map(|(factor, multiplicity)| {
                        core::iter::repeat_n(factor.clone(), *multiplicity as usize)
                    })
                    .collect::<Vec<_>>(),
            );
            assert_eq!(rebuilt, product.primitive_part());
        }
    }

    #[test]
    fn zassenhaus_recovers_known_irreducible_products() {
        let factors = vec![poly(&[1, 0, 1]), poly(&[2, 0, 1]), poly(&[5, 0, 1])];
        let mut product = ZPoly::one();
        for factor in &factors {
            product = product.multiply(factor);
        }
        let mut recovered = product.factor().unwrap();
        recovered.sort_by_key(|(factor, _)| factor.coeffs.clone());
        let mut expected = factors.clone();
        expected.sort_by_key(|factor| factor.coeffs.clone());
        assert_eq!(recovered.len(), expected.len());
        for ((found, multiplicity), wanted) in recovered.iter().zip(&expected) {
            assert_eq!(found, wanted);
            assert_eq!(*multiplicity, 1);
        }
    }

    #[test]
    fn zassenhaus_factors_composite_and_irreducible_polynomials() {
        let product = poly(&[2, 0, 3, 0, 1]);
        let mut factors = product.factor().unwrap();
        factors.sort_by_key(|(factor, multiplicity)| (factor.degree(), *multiplicity));
        assert_eq!(factors.len(), 2);
        assert_eq!(
            multiply_all(
                &factors
                    .iter()
                    .map(|(factor, _)| factor.clone())
                    .collect::<Vec<_>>()
            ),
            product.primitive_part()
        );

        assert!(poly(&[1, 1, 1, 1, 1]).is_irreducible());
        assert!(poly(&[-8, -2, -1, 1]).is_irreducible());

        let repeated = poly(&[2, -3, 0, 1]);
        let factors = repeated.factor().unwrap();
        let total: u32 = factors.iter().map(|(_, multiplicity)| *multiplicity).sum();
        assert_eq!(total, 3);
        let rebuilt = multiply_all(
            &factors
                .iter()
                .flat_map(|(factor, multiplicity)| {
                    core::iter::repeat_n(factor.clone(), *multiplicity as usize)
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(rebuilt, repeated.primitive_part());
    }
}
