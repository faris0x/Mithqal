//! Canonical exact rationals backed by Mithqalite.

use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

use mithqalite::base::num::arithmetic::traits::{Abs, Ceiling, Floor, Sign};
use mithqalite::base::num::conversion::traits::RoundingFrom;
use mithqalite::base::rounding_modes::RoundingMode;
use mithqalite::{Integer, Rational};

use super::int::natural_bytes;
use super::{Int, NumError};

/// An exact rational number in canonical form.
///
/// The denominator is positive and the numerator and denominator are
/// coprime. Zero has denominator one.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rat {
    pub(crate) inner: Rational,
}

impl Rat {
    /// The rational zero.
    pub fn zero() -> Self {
        Rat {
            inner: Rational::from_unsigneds(0u32, 1u32),
        }
    }

    /// The rational one.
    pub fn one() -> Self {
        Rat {
            inner: Rational::from_unsigneds(1u32, 1u32),
        }
    }

    /// Build a rational from two `i64` values.
    pub fn from_i64(num: i64, den: i64) -> Result<Self, NumError> {
        if den == 0 {
            return Err(NumError::DivisionByZero);
        }
        Ok(Rat {
            inner: Rational::from_signeds(num, den),
        })
    }

    /// Build a rational from two exact integers.
    pub fn from_ints(num: Int, den: Int) -> Result<Self, NumError> {
        if den.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        Ok(Rat {
            inner: Rational::from_integers(num.inner, den.inner),
        })
    }

    /// Parse a rational in `num/den` or integer form.
    pub fn from_decimal(text: &str) -> Result<Self, NumError> {
        if text.is_empty() || text.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(NumError::Parse);
        }
        Rational::from_str(text)
            .map(|inner| Rat { inner })
            .map_err(|_| NumError::Parse)
    }

    /// Render the rational in `num/den` form.
    pub fn to_decimal(&self) -> String {
        self.inner.to_string()
    }

    pub(crate) fn num_sign(&self) -> i8 {
        match self.inner.sign() {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    pub(crate) fn num_limbs(&self) -> Vec<u8> {
        natural_bytes(self.inner.numerator_ref())
    }

    pub(crate) fn den_limbs(&self) -> Vec<u8> {
        natural_bytes(self.inner.denominator_ref())
    }

    /// The numerator as an exact integer.
    pub fn num(&self) -> Int {
        Int {
            inner: Integer::from_sign_and_abs(
                self.inner.sign() != Ordering::Less,
                self.inner.numerator_ref().clone(),
            ),
        }
    }

    /// The denominator as an exact integer.
    pub fn den(&self) -> Int {
        Int {
            inner: Integer::from(self.inner.denominator_ref().clone()),
        }
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.inner == 0u32
    }

    /// True when the value is one.
    pub fn is_one(&self) -> bool {
        self.inner == 1u32
    }

    /// The sign as `-1`, `0`, or `1`.
    pub fn sign(&self) -> i8 {
        self.num_sign()
    }

    /// Approximate the value as a double for heuristic paths.
    pub(crate) fn to_f64(&self) -> f64 {
        f64::rounding_from(&self.inner, RoundingMode::Nearest).0
    }

    /// Compare two rationals by value.
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(&self, other: &Rat) -> Ordering {
        self.inner.cmp(&other.inner)
    }

    /// Add two rationals.
    pub fn add(&self, other: &Rat) -> Rat {
        Rat {
            inner: &self.inner + &other.inner,
        }
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Rat) -> Rat {
        Rat {
            inner: &self.inner - &other.inner,
        }
    }

    /// Multiply two rationals.
    pub fn mul(&self, other: &Rat) -> Rat {
        Rat {
            inner: &self.inner * &other.inner,
        }
    }

    /// Divide `self` by `other`.
    pub fn div(&self, other: &Rat) -> Result<Rat, NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        Ok(Rat {
            inner: &self.inner / &other.inner,
        })
    }

    /// Negate the rational.
    pub fn neg(&self) -> Rat {
        Rat {
            inner: -self.inner.clone(),
        }
    }

    /// Absolute value.
    pub fn abs(&self) -> Rat {
        Rat {
            inner: self.inner.clone().abs(),
        }
    }

    /// The greatest integer not greater than the rational.
    pub fn floor(&self) -> Int {
        Int {
            inner: self.inner.clone().floor(),
        }
    }

    /// The least integer not less than the rational.
    pub fn ceil(&self) -> Int {
        Int {
            inner: self.inner.clone().ceiling(),
        }
    }

    /// The nearest integer, with halves rounded away from zero.
    pub fn round(&self) -> Int {
        let (quotient, remainder) = self
            .num()
            .divrem(&self.den())
            .expect("rational denominator is nonzero");
        if remainder.is_zero() {
            return quotient;
        }
        let twice = remainder.abs().mul(&Int::from_i64(2));
        if twice.cmp(&self.den()) != Ordering::Less {
            if self.sign() < 0 {
                quotient.sub(&Int::one())
            } else {
                quotient.add(&Int::one())
            }
        } else {
            quotient
        }
    }
}

impl fmt::Display for Rat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

impl fmt::Debug for Rat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rat({})", self.to_decimal())
    }
}
