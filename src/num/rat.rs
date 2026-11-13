//! Canonical exact rationals backed by GMP `mpq`.

use core::cmp::Ordering;
use core::fmt;
use std::ffi::CString;

use super::ffi::{self, Mpq};
use super::{Int, NumError};

/// An exact rational number in canonical form.
///
/// The denominator is positive and the numerator and denominator are
/// coprime. Zero has denominator one.
pub struct Rat {
    pub(crate) inner: Mpq,
}

impl Rat {
    fn initialized() -> Self {
        let mut inner = Mpq {
            num: ffi::Mpz {
                alloc: 0,
                size: 0,
                limbs: core::ptr::null_mut(),
            },
            den: ffi::Mpz {
                alloc: 0,
                size: 0,
                limbs: core::ptr::null_mut(),
            },
        };
        // SAFETY: `inner` has room for the GMP structure and is initialized
        // exactly once before use.
        unsafe { ffi::__gmpq_init(&mut inner) };
        Rat { inner }
    }

    /// The rational zero.
    pub fn zero() -> Self {
        Self::initialized()
    }

    /// The rational one.
    pub fn one() -> Self {
        let mut out = Self::initialized();
        // SAFETY: both integer fields are initialized by `mpq_init`.
        unsafe {
            ffi::__gmpz_set_ui(&mut out.inner.num, 1);
            ffi::__gmpz_set_ui(&mut out.inner.den, 1);
        }
        out
    }

    /// Build a rational from two `i64` values.
    pub fn from_i64(num: i64, den: i64) -> Result<Self, NumError> {
        if den == 0 {
            return Err(NumError::DivisionByZero);
        }
        let mut out = Self::initialized();
        // SAFETY: the fields are initialized by `mpq_init`.
        unsafe {
            ffi::__gmpz_set_si(&mut out.inner.num, num as core::ffi::c_long);
            ffi::__gmpz_set_ui(&mut out.inner.den, den.unsigned_abs() as core::ffi::c_ulong);
            if den < 0 {
                ffi::__gmpz_neg(&mut out.inner.num, &out.inner.num);
            }
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        Ok(out)
    }

    /// Build a rational from two exact integers.
    pub fn from_ints(num: Int, den: Int) -> Result<Self, NumError> {
        if den.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe {
            ffi::__gmpz_set(&mut out.inner.num, &num.inner);
            ffi::__gmpz_set(&mut out.inner.den, &den.inner);
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        Ok(out)
    }

    /// Parse a rational in `num/den` form.
    pub fn from_decimal(text: &str) -> Result<Self, NumError> {
        if text.is_empty() {
            return Err(NumError::Parse);
        }
        let c_text = CString::new(text).map_err(|_| NumError::Parse)?;
        let mut out = Self::initialized();
        // SAFETY: `out.inner` is initialized and `c_text` is NUL terminated.
        let status = unsafe { ffi::__gmpq_set_str(&mut out.inner, c_text.as_ptr(), 10) };
        if status != 0 {
            return Err(NumError::Parse);
        }
        // SAFETY: `out.inner` is initialized.
        unsafe { ffi::__gmpq_canonicalize(&mut out.inner) };
        Ok(out)
    }

    /// Render the rational in `num/den` form.
    pub fn to_decimal(&self) -> String {
        // SAFETY: the fields are initialized.
        let num_digits = unsafe { ffi::__gmpz_sizeinbase(&self.inner.num, 10) };
        // SAFETY: the fields are initialized.
        let den_digits = unsafe { ffi::__gmpz_sizeinbase(&self.inner.den, 10) };
        let mut buffer = vec![0u8; num_digits + den_digits + 3];
        // SAFETY: the buffer has room for both components and a NUL byte.
        unsafe {
            ffi::__gmpq_get_str(
                buffer.as_mut_ptr() as *mut core::ffi::c_char,
                10,
                &self.inner,
            );
        }
        let end = buffer
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(buffer.len());
        String::from_utf8(buffer[..end].to_vec()).expect("GMP produced non UTF-8 digits")
    }

    /// The numerator as an exact integer.
    pub fn num(&self) -> Int {
        Int::from_mpz_ref(&self.inner.num)
    }

    /// The denominator as an exact integer.
    pub fn den(&self) -> Int {
        Int::from_mpz_ref(&self.inner.den)
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        // SAFETY: the field is initialized.
        unsafe { ffi::__gmpz_cmp_si(&self.inner.num, 0) == 0 }
    }

    /// True when the value is one.
    pub fn is_one(&self) -> bool {
        // SAFETY: the fields are initialized.
        unsafe {
            ffi::__gmpz_cmp_ui(&self.inner.num, 1) == 0
                && ffi::__gmpz_cmp_ui(&self.inner.den, 1) == 0
        }
    }

    /// The sign as `-1`, `0`, or `1`.
    pub fn sign(&self) -> i8 {
        // SAFETY: the field is initialized.
        let order = unsafe { ffi::__gmpz_cmp_si(&self.inner.num, 0) };
        match order.cmp(&0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Compare two rationals by value.
    pub fn cmp(&self, other: &Rat) -> Ordering {
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpq_cmp(&self.inner, &other.inner) }.cmp(&0)
    }

    /// Add two rationals.
    pub fn add(&self, other: &Rat) -> Rat {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe {
            ffi::__gmpq_add(&mut out.inner, &self.inner, &other.inner);
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        out
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Rat) -> Rat {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe {
            ffi::__gmpq_sub(&mut out.inner, &self.inner, &other.inner);
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        out
    }

    /// Multiply two rationals.
    pub fn mul(&self, other: &Rat) -> Rat {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe {
            ffi::__gmpq_mul(&mut out.inner, &self.inner, &other.inner);
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        out
    }

    /// Divide `self` by `other`.
    pub fn div(&self, other: &Rat) -> Result<Rat, NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let mut out = Self::initialized();
        // SAFETY: all values are initialized and the divisor is nonzero.
        unsafe {
            ffi::__gmpq_div(&mut out.inner, &self.inner, &other.inner);
            ffi::__gmpq_canonicalize(&mut out.inner);
        }
        Ok(out)
    }

    /// Negate the rational.
    pub fn neg(&self) -> Rat {
        let mut out = Self::initialized();
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpq_neg(&mut out.inner, &self.inner) };
        out
    }

    /// Absolute value.
    pub fn abs(&self) -> Rat {
        if self.sign() < 0 {
            self.neg()
        } else {
            self.clone()
        }
    }
}

impl Clone for Rat {
    fn clone(&self) -> Self {
        let mut out = Self::initialized();
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpq_set(&mut out.inner, &self.inner) };
        out
    }
}

impl Drop for Rat {
    fn drop(&mut self) {
        // SAFETY: the value was initialized and is cleared exactly once.
        unsafe { ffi::__gmpq_clear(&mut self.inner) };
    }
}

impl PartialEq for Rat {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpq_equal(&self.inner, &other.inner) != 0 }
    }
}

impl Eq for Rat {}

impl PartialOrd for Rat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rat {
    fn cmp(&self, other: &Self) -> Ordering {
        Rat::cmp(self, other)
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
