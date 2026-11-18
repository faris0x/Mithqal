//! Arbitrary precision integers backed by Mithqalite.

use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

use mithqalite::Integer;
use mithqalite::Natural;
use mithqalite::base::num::arithmetic::traits::{Abs, DivRem, Gcd, Pow, Sign};

use super::NumError;

/// An exact arbitrary precision integer.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Int {
    pub(crate) inner: Integer,
}

impl Int {
    /// The integer zero.
    pub fn zero() -> Self {
        Int {
            inner: Integer::from(0u32),
        }
    }

    /// The integer one.
    pub fn one() -> Self {
        Int {
            inner: Integer::from(1u32),
        }
    }

    /// Build an integer from an `i64`.
    pub fn from_i64(value: i64) -> Self {
        Int {
            inner: Integer::from(value),
        }
    }

    /// Build an integer from a `u64`.
    pub fn from_u64(value: u64) -> Self {
        Int {
            inner: Integer::from(value),
        }
    }

    /// Parse a base ten integer.
    pub fn from_decimal(text: &str) -> Result<Self, NumError> {
        if text.is_empty() || text.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(NumError::Parse);
        }
        Integer::from_str(text)
            .map(|inner| Int { inner })
            .map_err(|_| NumError::Parse)
    }

    /// Render the integer in base ten.
    pub fn to_decimal(&self) -> String {
        self.inner.to_string()
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
        match self.inner.sign() {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Compare two integers.
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(&self, other: &Int) -> Ordering {
        self.inner.cmp(&other.inner)
    }

    /// Add two integers.
    pub fn add(&self, other: &Int) -> Int {
        Int {
            inner: &self.inner + &other.inner,
        }
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Int) -> Int {
        Int {
            inner: &self.inner - &other.inner,
        }
    }

    /// Multiply two integers.
    pub fn mul(&self, other: &Int) -> Int {
        Int {
            inner: &self.inner * &other.inner,
        }
    }

    /// Negate an integer.
    pub fn neg(&self) -> Int {
        Int {
            inner: -self.inner.clone(),
        }
    }

    /// Absolute value.
    pub fn abs(&self) -> Int {
        Int {
            inner: self.inner.clone().abs(),
        }
    }

    /// Truncated division and remainder.
    pub fn divrem(&self, other: &Int) -> Result<(Int, Int), NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let (quotient, remainder) = (&self.inner).div_rem(&other.inner);
        Ok((Int { inner: quotient }, Int { inner: remainder }))
    }

    /// Greatest common divisor of the absolute values.
    pub fn gcd(&self, other: &Int) -> Int {
        if self.is_zero() && other.is_zero() {
            return Int::zero();
        }
        Int {
            inner: Integer::from((&self.inner).gcd(&other.inner)),
        }
    }

    /// Raise to a non negative integer power.
    pub fn pow_u32(&self, exponent: u32) -> Int {
        Int {
            inner: self.inner.clone().pow(u64::from(exponent)),
        }
    }

    /// Convert to `i64` when the value fits.
    pub fn to_i64(&self) -> Option<i64> {
        i64::try_from(&self.inner).ok()
    }

    pub(crate) fn to_i128(&self) -> Option<i128> {
        i128::try_from(&self.inner).ok()
    }

    pub(crate) fn magnitude(&self) -> Vec<u8> {
        natural_bytes(self.inner.unsigned_abs_ref())
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

impl fmt::Debug for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Int({})", self.to_decimal())
    }
}

pub(crate) fn natural_bytes(value: &Natural) -> Vec<u8> {
    let mut bytes = Vec::new();
    for limb in value.as_limbs_asc() {
        bytes.extend_from_slice(&limb.to_le_bytes());
    }
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    bytes
}
