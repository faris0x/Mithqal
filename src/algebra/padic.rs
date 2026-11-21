//! Fixed-precision p-adic field arithmetic.
//!
//! A value is `p^valuation * unit` with the unit known modulo `p^precision`.
//! This is a working field implementation, not a general p-adic analysis layer.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::Int;

use super::intmatrix::extended_gcd;

/// A fixed-precision p-adic number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Padic {
    prime: Int,
    precision: u32,
    valuation: i64,
    unit: Int,
}

impl Padic {
    /// Build a p-adic value from a prime and precision.
    pub(crate) fn new(prime: Int, precision: u32) -> Option<Self> {
        if prime.sign() <= 0 || precision == 0 {
            return None;
        }
        Some(Padic {
            prime,
            precision,
            valuation: 0,
            unit: Int::zero(),
        })
    }

    /// The prime.
    pub(crate) fn prime(&self) -> &Int {
        &self.prime
    }

    /// The working precision.
    pub(crate) fn precision(&self) -> u32 {
        self.precision
    }

    /// The valuation, or `None` for zero.
    pub(crate) fn valuation(&self) -> Option<i64> {
        if self.unit.is_zero() {
            None
        } else {
            Some(self.valuation)
        }
    }

    /// True for zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.unit.is_zero()
    }

    /// The zero value.
    pub(crate) fn zero(prime: Int, precision: u32) -> Option<Self> {
        let mut value = Padic::new(prime, precision)?;
        value.unit = Int::zero();
        Some(value)
    }

    /// The one value.
    pub(crate) fn one(prime: Int, precision: u32) -> Option<Self> {
        let mut value = Padic::new(prime, precision)?;
        value.unit = Int::one();
        Some(value)
    }

    /// Build from an integer.
    pub(crate) fn from_integer(prime: Int, precision: u32, value: Int) -> Option<Self> {
        let mut result = Padic::zero(prime, precision)?;
        result.unit = result.reduce(&value);
        result.normalize()
    }

    /// Build from a rational whose denominator is a p-adic unit.
    pub(crate) fn from_rational(
        prime: Int,
        precision: u32,
        numerator: Int,
        denominator: Int,
    ) -> Option<Self> {
        let inverse = inverse_mod(&denominator, &prime, precision)?;
        let value = Padic::from_integer(prime, precision, numerator)?;
        value.multiply(&Padic::from_integer(
            value.prime.clone(),
            precision,
            inverse,
        )?)
    }

    /// Negation.
    pub(crate) fn negate(&self) -> Option<Self> {
        if self.is_zero() {
            return Some(self.clone());
        }
        let modulus = self.modulus();
        Some(Padic {
            prime: self.prime.clone(),
            precision: self.precision,
            valuation: self.valuation,
            unit: modulus.sub(&self.unit),
        })
    }

    /// Addition.
    pub(crate) fn add(&self, other: &Padic) -> Option<Self> {
        if self.prime != other.prime || self.precision != other.precision {
            return None;
        }
        if self.is_zero() {
            return Some(other.clone());
        }
        if other.is_zero() {
            return Some(self.clone());
        }
        let modulus = self.modulus();
        let valuation = self.valuation.min(other.valuation);
        let left_shift = self.valuation - valuation;
        let right_shift = other.valuation - valuation;
        let left = mul_power(&self.unit, &self.prime, left_shift, &modulus)?;
        let right = mul_power(&other.unit, &other.prime, right_shift, &modulus)?;
        let unit = left.add(&right).divrem(&modulus).ok()?.1;
        Padic {
            prime: self.prime.clone(),
            precision: self.precision,
            valuation,
            unit,
        }
        .normalize()
    }

    /// Subtraction.
    pub(crate) fn subtract(&self, other: &Padic) -> Option<Self> {
        self.add(&other.negate()?)
    }

    /// Multiplication.
    pub(crate) fn multiply(&self, other: &Padic) -> Option<Self> {
        if self.prime != other.prime || self.precision != other.precision {
            return None;
        }
        if self.is_zero() || other.is_zero() {
            return Padic::zero(self.prime.clone(), self.precision);
        }
        let modulus = self.modulus();
        let unit = self.unit.mul(&other.unit).divrem(&modulus).ok()?.1;
        Padic {
            prime: self.prime.clone(),
            precision: self.precision,
            valuation: self.valuation + other.valuation,
            unit,
        }
        .normalize()
    }

    /// Inverse, or `None` for zero.
    pub(crate) fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }
        let inverse = inverse_mod(&self.unit, &self.prime, self.precision)?;
        Padic {
            prime: self.prime.clone(),
            precision: self.precision,
            valuation: -self.valuation,
            unit: inverse,
        }
        .normalize()
    }

    fn modulus(&self) -> Int {
        self.prime.pow_u32(self.precision)
    }

    fn reduce(&self, value: &Int) -> Int {
        let modulus = self.modulus();
        value
            .divrem(&modulus)
            .map(|(_, remainder)| remainder)
            .unwrap_or_else(|_| Int::zero())
    }

    fn normalize(mut self) -> Option<Self> {
        if self.unit.is_zero() {
            self.valuation = 0;
            return Some(self);
        }
        let modulus = self.modulus();
        while self.unit.divrem(&self.prime).ok()?.1.is_zero() {
            self.unit = self.unit.divrem(&self.prime).ok()?.0;
            self.valuation += 1;
            if self.valuation > 64 {
                break;
            }
        }
        self.unit = self.unit.divrem(&modulus).ok()?.1;
        Some(self)
    }
}

fn mul_power(value: &Int, prime: &Int, exponent: i64, modulus: &Int) -> Option<Int> {
    if exponent < 0 {
        return None;
    }
    let power = prime.pow_u32(exponent as u32);
    Some(value.mul(&power).divrem(modulus).ok()?.1)
}

fn inverse_mod(value: &Int, prime: &Int, precision: u32) -> Option<Int> {
    let modulus = prime.pow_u32(precision);
    let reduced = value.divrem(&modulus).ok()?.1;
    let (gcd, coefficient, _) = extended_gcd(&reduced, &modulus);
    if !gcd.is_one() {
        return None;
    }
    Some(coefficient.divrem(&modulus).ok()?.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn five() -> Int {
        Int::from_i64(5)
    }

    #[test]
    fn padic_addition_and_multiplication() {
        let third = Padic::from_rational(five(), 6, Int::one(), Int::from_i64(3)).unwrap();
        let two_thirds = third.add(&third).unwrap();
        let one = two_thirds.add(&third).unwrap();
        assert!(
            one.subtract(&Padic::one(five(), 6).unwrap())
                .unwrap()
                .is_zero()
        );
        assert_eq!(third.valuation(), Some(0));
    }

    #[test]
    fn padic_inverse_and_valuation() {
        let third = Padic::from_rational(five(), 6, Int::one(), Int::from_i64(3)).unwrap();
        let inverse = third.inverse().unwrap();
        let product = third.multiply(&inverse).unwrap();
        assert!(
            product
                .subtract(&Padic::one(five(), 6).unwrap())
                .unwrap()
                .is_zero()
        );
        let divisible = Padic::from_integer(five(), 6, Int::from_i64(25)).unwrap();
        assert_eq!(divisible.valuation(), Some(2));
        let reciprocal = divisible.inverse().unwrap();
        assert_eq!(reciprocal.valuation(), Some(-2));
    }

    #[test]
    fn padic_rejects_zero_divisors() {
        let zero = Padic::zero(five(), 4).unwrap();
        assert!(zero.inverse().is_none());
        assert!(Padic::from_rational(five(), 4, Int::one(), Int::from_i64(5)).is_none());
    }
}
