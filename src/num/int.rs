//! Arbitrary precision integers backed by GMP `mpz`.

use core::cmp::Ordering;
use core::fmt;
use std::ffi::CString;

use super::NumError;
use super::ffi::{self, Mpz};

/// An exact arbitrary precision integer.
pub struct Int {
    pub(crate) inner: Mpz,
}

impl Int {
    fn initialized() -> Self {
        let mut inner = Mpz {
            alloc: 0,
            size: 0,
            limbs: core::ptr::null_mut(),
        };
        // SAFETY: `inner` has room for the GMP structure and is initialized
        // exactly once before use.
        unsafe { ffi::__gmpz_init(&mut inner) };
        Int { inner }
    }

    /// The integer zero.
    pub fn zero() -> Self {
        Self::initialized()
    }

    /// The integer one.
    pub fn one() -> Self {
        let mut value = Self::initialized();
        // SAFETY: `value.inner` is initialized.
        unsafe { ffi::__gmpz_set_ui(&mut value.inner, 1) };
        value
    }

    /// Build an integer from an `i64`.
    pub fn from_i64(value: i64) -> Self {
        let mut out = Self::initialized();
        // SAFETY: `out.inner` is initialized.
        unsafe { ffi::__gmpz_set_si(&mut out.inner, value as core::ffi::c_long) };
        out
    }

    /// Build an integer from a `u64`.
    pub fn from_u64(value: u64) -> Self {
        let mut out = Self::initialized();
        // SAFETY: `out.inner` is initialized.
        unsafe { ffi::__gmpz_set_ui(&mut out.inner, value as core::ffi::c_ulong) };
        out
    }

    /// Parse a base ten integer.
    pub fn from_decimal(text: &str) -> Result<Self, NumError> {
        if text.is_empty() || text.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(NumError::Parse);
        }
        let c_text = CString::new(text).map_err(|_| NumError::Parse)?;
        let mut out = Self::initialized();
        // SAFETY: `out.inner` is initialized and `c_text` is NUL terminated.
        let status = unsafe { ffi::__gmpz_set_str(&mut out.inner, c_text.as_ptr(), 10) };
        if status == 0 {
            Ok(out)
        } else {
            Err(NumError::Parse)
        }
    }

    /// Render the integer in base ten.
    pub fn to_decimal(&self) -> String {
        // SAFETY: `self.inner` is initialized.
        let digits = unsafe { ffi::__gmpz_sizeinbase(&self.inner, 10) };
        let mut buffer = vec![0u8; digits + 2];
        // SAFETY: the buffer has room for the digits, a sign, and a NUL byte.
        unsafe {
            ffi::__gmpz_get_str(
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

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        // SAFETY: `self.inner` is initialized.
        unsafe { ffi::__gmpz_cmp_si(&self.inner, 0) == 0 }
    }

    /// True when the value is one.
    pub fn is_one(&self) -> bool {
        // SAFETY: `self.inner` is initialized.
        unsafe { ffi::__gmpz_cmp_ui(&self.inner, 1) == 0 }
    }

    /// The sign as `-1`, `0`, or `1`.
    pub fn sign(&self) -> i8 {
        // SAFETY: `self.inner` is initialized.
        let order = unsafe { ffi::__gmpz_cmp_si(&self.inner, 0) };
        match order.cmp(&0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Compare two integers.
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(&self, other: &Int) -> Ordering {
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpz_cmp(&self.inner, &other.inner) }.cmp(&0)
    }

    /// Add two integers.
    pub fn add(&self, other: &Int) -> Int {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized and GMP permits aliasing.
        unsafe { ffi::__gmpz_add(&mut out.inner, &self.inner, &other.inner) };
        out
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Int) -> Int {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized and GMP permits aliasing.
        unsafe { ffi::__gmpz_sub(&mut out.inner, &self.inner, &other.inner) };
        out
    }

    /// Multiply two integers.
    pub fn mul(&self, other: &Int) -> Int {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized and GMP permits aliasing.
        unsafe { ffi::__gmpz_mul(&mut out.inner, &self.inner, &other.inner) };
        out
    }

    /// Negate an integer.
    pub fn neg(&self) -> Int {
        let mut out = Self::initialized();
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpz_neg(&mut out.inner, &self.inner) };
        out
    }

    /// Absolute value.
    pub fn abs(&self) -> Int {
        let mut out = Self::initialized();
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpz_abs(&mut out.inner, &self.inner) };
        out
    }

    /// Truncated division and remainder.
    pub fn divrem(&self, other: &Int) -> Result<(Int, Int), NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let mut quotient = Self::initialized();
        let mut remainder = Self::initialized();
        // SAFETY: all values are initialized and the divisor is nonzero.
        unsafe {
            ffi::__gmpz_tdiv_qr(
                &mut quotient.inner,
                &mut remainder.inner,
                &self.inner,
                &other.inner,
            );
        }
        Ok((quotient, remainder))
    }

    /// Greatest common divisor of the absolute values.
    pub fn gcd(&self, other: &Int) -> Int {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe { ffi::__gmpz_gcd(&mut out.inner, &self.inner, &other.inner) };
        out
    }

    /// Raise to a non negative integer power.
    pub fn pow_u32(&self, exponent: u32) -> Int {
        let mut out = Self::initialized();
        // SAFETY: all values are initialized.
        unsafe {
            ffi::__gmpz_pow_ui(&mut out.inner, &self.inner, exponent as core::ffi::c_ulong);
        }
        out
    }

    /// Convert to `i64` when the value fits.
    pub fn to_i64(&self) -> Option<i64> {
        // SAFETY: `self.inner` is initialized.
        let fits = unsafe { ffi::__gmpz_fits_slong_p(&self.inner) };
        if fits == 0 {
            None
        } else {
            // SAFETY: the value fits a C long.
            Some(unsafe { ffi::__gmpz_get_si(&self.inner) } as i64)
        }
    }

    pub(crate) fn to_i128(&self) -> Option<i128> {
        if let Some(value) = self.to_i64() {
            return Some(value as i128);
        }
        self.to_decimal().parse::<i128>().ok()
    }

    pub(crate) fn magnitude(&self) -> &[u64] {
        let length = self.inner.size.unsigned_abs() as usize;
        if length == 0 {
            &[]
        } else {
            // SAFETY: GMP stores `length` initialized limbs at `limbs` for a
            // nonzero value. The slice borrows the integer for its lifetime.
            unsafe { core::slice::from_raw_parts(self.inner.limbs, length) }
        }
    }

    pub(crate) fn from_mpz_ref(source: &Mpz) -> Self {
        let mut out = Self::initialized();
        // SAFETY: both values are initialized.
        unsafe { ffi::__gmpz_set(&mut out.inner, source) };
        out
    }
}

impl Clone for Int {
    fn clone(&self) -> Self {
        Self::from_mpz_ref(&self.inner)
    }
}

impl Drop for Int {
    fn drop(&mut self) {
        // SAFETY: the value was initialized and is cleared exactly once.
        unsafe { ffi::__gmpz_clear(&mut self.inner) };
    }
}

impl PartialEq for Int {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Int {}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        Int::cmp(self, other)
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
