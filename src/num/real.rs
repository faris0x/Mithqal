//! Arbitrary precision reals with significance tracking.

use core::cmp::Ordering;
use core::fmt;

use super::ffi_mpfr::{self, MPFR_RNDN, Mpfr};
use super::{NumError, Rat};

/// An approximate real value with an absolute error estimate.
pub struct Real {
    pub(crate) inner: Mpfr,
    pub(crate) bits: u32,
    pub(crate) error: f64,
}

impl Real {
    fn initialized(bits: u32) -> Self {
        let mut inner = Mpfr {
            precision: 0,
            sign: 0,
            exponent: 0,
            limbs: core::ptr::null_mut(),
        };
        // SAFETY: the structure has room for MPFR state and is initialized once.
        unsafe { ffi_mpfr::mpfr_init2(&mut inner, bits as core::ffi::c_long) };
        Real {
            inner,
            bits,
            error: 0.0,
        }
    }

    /// Convert an exact rational to a real with a requested decimal digit count.
    pub fn from_rat(value: &Rat, digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let mut out = Self::initialized(bits);
        // SAFETY: both values are initialized. `mpfr_set_q` reads the GMP
        // rational layout stored inside `Rat`.
        unsafe {
            ffi_mpfr::mpfr_set_q(&mut out.inner, &value.inner, MPFR_RNDN);
        }
        out.error = rounding_error(&out);
        Ok(out)
    }

    pub(crate) fn from_mpfr_ref(source: &Mpfr, bits: u32, error: f64) -> Self {
        let mut out = Self::initialized(bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_set(&mut out.inner, source, MPFR_RNDN) };
        out.error = error;
        out
    }

    /// Convert an `i64` to a real with a requested decimal digit count.
    pub fn from_i64(value: i64, digits: u32) -> Result<Self, NumError> {
        Self::from_i64_round(value, digits, MPFR_RNDN)
    }

    pub(crate) fn blank(bits: u32) -> Self {
        Self::initialized(bits)
    }

    pub(crate) fn from_i64_round(
        value: i64,
        digits: u32,
        rounding: core::ffi::c_int,
    ) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let mut out = Self::initialized(bits);
        // SAFETY: the value is initialized.
        unsafe {
            ffi_mpfr::mpfr_set_si(&mut out.inner, value as core::ffi::c_long, rounding);
        }
        out.error = if rounding == MPFR_RNDN {
            rounding_error(&out)
        } else {
            0.0
        };
        Ok(out)
    }

    pub(crate) fn add_round(
        &self,
        other: &Real,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_add(&mut out.inner, &self.inner, &other.inner, rounding);
        }
        Ok(out)
    }

    pub(crate) fn mul_round(
        &self,
        other: &Real,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_mul(&mut out.inner, &self.inner, &other.inner, rounding);
        }
        Ok(out)
    }

    pub(crate) fn div_round(
        &self,
        other: &Real,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized and the divisor is nonzero.
        unsafe {
            ffi_mpfr::mpfr_div(&mut out.inner, &self.inner, &other.inner, rounding);
        }
        Ok(out)
    }

    pub(crate) fn pow_si_round(
        &self,
        exponent: i64,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe {
            ffi_mpfr::mpfr_pow_si(
                &mut out.inner,
                &self.inner,
                exponent as core::ffi::c_long,
                rounding,
            );
        }
        Ok(out)
    }

    /// Convert an `f64` to a real with a requested decimal digit count.
    pub fn from_f64(value: f64, digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let mut out = Self::initialized(bits);
        // SAFETY: the value is initialized.
        unsafe { ffi_mpfr::mpfr_set_d(&mut out.inner, value, MPFR_RNDN) };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// A conservative lower endpoint from the stored error estimate.
    pub(crate) fn lower_bound(&self) -> Result<Real, NumError> {
        if !self.error.is_finite() || self.error == 0.0 {
            return Ok(self.clone());
        }
        let error = Self::from_f64(-self.error, 64)?;
        self.add_round(&error, ffi_mpfr::MPFR_RNDD)
    }

    /// A conservative upper endpoint from the stored error estimate.
    pub(crate) fn upper_bound(&self) -> Result<Real, NumError> {
        if !self.error.is_finite() || self.error == 0.0 {
            return Ok(self.clone());
        }
        let error = Self::from_f64(self.error, 64)?;
        self.add_round(&error, ffi_mpfr::MPFR_RNDU)
    }

    /// The approximate zero at a requested decimal digit count.
    pub fn zero(digits: u32) -> Result<Self, NumError> {
        Self::from_i64(0, digits)
    }

    /// The approximate one at a requested decimal digit count.
    pub fn one(digits: u32) -> Result<Self, NumError> {
        Self::from_i64(1, digits)
    }

    /// The constant pi with a requested decimal digit count.
    pub fn pi(digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let mut out = Self::initialized(bits);
        // SAFETY: the value is initialized.
        unsafe { ffi_mpfr::mpfr_const_pi(&mut out.inner, MPFR_RNDN) };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The constant e with a requested decimal digit count.
    pub fn e(digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let mut out = Self::initialized(bits);
        // SAFETY: the value is initialized.
        unsafe {
            ffi_mpfr::mpfr_set_ui(&mut out.inner, 1, MPFR_RNDN);
            ffi_mpfr::mpfr_exp(&mut out.inner, &out.inner, MPFR_RNDN);
        }
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The approximate decimal value.
    pub fn to_decimal(&self) -> String {
        let precision = self.precision_digits();
        let digits = if precision.is_finite() {
            precision.max(1.0).min(1000.0).floor() as usize
        } else {
            16
        };
        self.to_decimal_with_digits(digits.max(1))
    }

    fn to_decimal_with_digits(&self, digits: usize) -> String {
        // SAFETY: the value is initialized.
        if unsafe { ffi_mpfr::mpfr_cmp_ui(&self.inner, 0) } == 0 {
            return "0.0".to_string();
        }
        let mut buffer = vec![0u8; digits + 3];
        let mut exponent: core::ffi::c_long = 0;
        // SAFETY: the buffer is large enough for the requested digits, a sign,
        // and a NUL byte.
        let pointer = unsafe {
            ffi_mpfr::mpfr_get_str(
                buffer.as_mut_ptr() as *mut core::ffi::c_char,
                &mut exponent,
                10,
                digits,
                &self.inner,
                MPFR_RNDN,
            )
        };
        if pointer.is_null() {
            return "0.0".to_string();
        }
        let end = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        let text = String::from_utf8_lossy(&buffer[..end]).into_owned();
        format_decimal(&text, exponent)
    }

    /// Relative precision in decimal digits.
    pub fn precision_digits(&self) -> f64 {
        if self.error == 0.0 {
            return f64::INFINITY;
        }
        let value = self.absolute_value();
        if value == 0.0 {
            return self.accuracy_digits();
        }
        -(self.error / value).log10()
    }

    /// Absolute accuracy in decimal digits.
    pub fn accuracy_digits(&self) -> f64 {
        if self.error == 0.0 {
            f64::INFINITY
        } else {
            -self.error.log10()
        }
    }

    /// True when the value is exactly zero in the stored representation.
    pub fn is_zero(&self) -> bool {
        // SAFETY: the value is initialized.
        unsafe { ffi_mpfr::mpfr_cmp_ui(&self.inner, 0) == 0 }
    }

    /// The sign of the stored value.
    pub fn sign(&self) -> i8 {
        // SAFETY: the value is initialized.
        let order = unsafe { ffi_mpfr::mpfr_cmp_ui(&self.inner, 0) };
        match order.cmp(&0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Compare two approximate values by their stored approximations.
    pub fn cmp(&self, other: &Real) -> Ordering {
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_cmp(&self.inner, &other.inner) }.cmp(&0)
    }

    /// Add two approximate values.
    pub fn add(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_add(&mut out.inner, &self.inner, &other.inner, MPFR_RNDN);
        }
        out.error = self.error.hypot(other.error) + rounding_error(&out);
        Ok(out)
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_sub(&mut out.inner, &self.inner, &other.inner, MPFR_RNDN);
        }
        out.error = self.error.hypot(other.error) + rounding_error(&out);
        Ok(out)
    }

    /// Multiply two approximate values.
    pub fn mul(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_mul(&mut out.inner, &self.inner, &other.inner, MPFR_RNDN);
        }
        let left = self.absolute_value();
        let right = other.absolute_value();
        let relative = if left == 0.0 || right == 0.0 {
            0.0
        } else {
            (self.error / left).hypot(other.error / right)
        };
        out.error = out.absolute_value() * relative + rounding_error(&out);
        Ok(out)
    }

    /// Divide `self` by `other`.
    pub fn div(&self, other: &Real) -> Result<Real, NumError> {
        if other.is_zero() {
            return Err(NumError::DivisionByZero);
        }
        let bits = self.bits.max(other.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized and the divisor is nonzero.
        unsafe {
            ffi_mpfr::mpfr_div(&mut out.inner, &self.inner, &other.inner, MPFR_RNDN);
        }
        let left = self.absolute_value();
        let right = other.absolute_value();
        let relative = if left == 0.0 {
            0.0
        } else {
            (self.error / left).hypot(other.error / right)
        };
        out.error = out.absolute_value() * relative + rounding_error(&out);
        Ok(out)
    }

    /// Negate the value.
    pub fn neg(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_neg(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.error;
        Ok(out)
    }

    /// Absolute value.
    pub fn abs(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_abs(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.error;
        Ok(out)
    }

    /// Raise to a real power.
    pub fn pow(&self, exponent: &Real) -> Result<Real, NumError> {
        if self.sign() < 0 {
            return Err(NumError::Domain);
        }
        if self.is_zero() && exponent.sign() <= 0 {
            return Err(NumError::Domain);
        }
        let bits = self.bits.max(exponent.bits);
        let mut out = Self::initialized(bits);
        // SAFETY: all values are initialized.
        unsafe {
            ffi_mpfr::mpfr_pow(&mut out.inner, &self.inner, &exponent.inner, MPFR_RNDN);
        }
        let base = self.to_f64();
        let exponent_value = exponent.to_f64();
        let derivative_base = if base == 0.0 {
            0.0
        } else {
            exponent_value.abs() * base.abs().powf(exponent_value - 1.0)
        };
        let derivative_exponent = if base == 0.0 {
            0.0
        } else {
            out.absolute_value() * base.abs().ln().abs()
        };
        out.error = derivative_base * self.error
            + derivative_exponent * exponent.error
            + rounding_error(&out);
        Ok(out)
    }

    /// Raise to an integer power.
    pub fn pow_si(&self, exponent: i64) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe {
            ffi_mpfr::mpfr_pow_si(
                &mut out.inner,
                &self.inner,
                exponent as core::ffi::c_long,
                MPFR_RNDN,
            );
        }
        let derivative = if exponent == 0 {
            0.0
        } else {
            (exponent as f64).abs() * self.to_f64().abs().powf((exponent - 1) as f64)
        };
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Square root.
    pub fn sqrt(&self) -> Result<Real, NumError> {
        if self.sign() < 0 {
            return Err(NumError::Domain);
        }
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_sqrt(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = out.absolute_value();
        out.error = if derivative == 0.0 {
            rounding_error(&out)
        } else {
            self.error / (2.0 * derivative) + rounding_error(&out)
        };
        Ok(out)
    }

    /// Exponential function.
    pub fn exp(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_exp(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = out.absolute_value() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Natural logarithm.
    pub fn ln(&self) -> Result<Real, NumError> {
        if self.sign() <= 0 {
            return Err(NumError::Domain);
        }
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_log(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.error / self.absolute_value() + rounding_error(&out);
        Ok(out)
    }

    /// Sine.
    pub fn sin(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_sin(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.to_f64().cos().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Cosine.
    pub fn cos(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_cos(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.to_f64().sin().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Tangent.
    pub fn tan(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_tan(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = 1.0 + out.to_f64().powi(2);
        out.error = derivative.abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Arcsine.
    pub fn asin(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_asin(&mut out.inner, &self.inner, MPFR_RNDN) };
        let cosine = out.to_f64().cos().abs().max(f64::MIN_POSITIVE);
        out.error = self.error / cosine + rounding_error(&out);
        Ok(out)
    }

    /// Arccosine.
    pub fn acos(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_acos(&mut out.inner, &self.inner, MPFR_RNDN) };
        let cosine = out.to_f64().cos().abs().max(f64::MIN_POSITIVE);
        out.error = self.error / cosine + rounding_error(&out);
        Ok(out)
    }

    /// Arctangent.
    pub fn atan(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_atan(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = 1.0 / (1.0 + self.to_f64().powi(2));
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic sine.
    pub fn sinh(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_sinh(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.to_f64().cosh().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic cosine.
    pub fn cosh(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_cosh(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.to_f64().sinh().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic tangent.
    pub fn tanh(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_tanh(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = 1.0 - out.to_f64().powi(2);
        out.error = derivative.abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic sine.
    pub fn asinh(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_asinh(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = 1.0 / out.to_f64().cosh().abs().max(f64::MIN_POSITIVE);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic cosine.
    pub fn acosh(&self) -> Result<Real, NumError> {
        // SAFETY: the value is initialized.
        if unsafe { ffi_mpfr::mpfr_cmp_ui(&self.inner, 1) } < 0 {
            return Err(NumError::Domain);
        }
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_acosh(&mut out.inner, &self.inner, MPFR_RNDN) };
        let sinh = out.to_f64().sinh().abs();
        let derivative = if sinh == 0.0 { 0.0 } else { 1.0 / sinh };
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic tangent.
    pub fn atanh(&self) -> Result<Real, NumError> {
        // SAFETY: the value is initialized.
        if unsafe { ffi_mpfr::mpfr_cmpabs_ui(&self.inner, 1) } >= 0 {
            return Err(NumError::Domain);
        }
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_atanh(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = out.to_f64().cosh().powi(2);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Gamma function.
    pub fn gamma(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_gamma(&mut out.inner, &self.inner, MPFR_RNDN) };
        if !out.is_finite() {
            return Err(NumError::Domain);
        }
        let scale = out.to_f64().abs().max(1.0);
        let derivative = scale * self.to_f64().abs().ln().abs().max(1.0);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Logarithm of the absolute gamma function.
    pub fn lgamma(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_lngamma(&mut out.inner, &self.inner, MPFR_RNDN) };
        if !out.is_finite() {
            return Err(NumError::Domain);
        }
        let derivative = self.to_f64().abs().ln().abs().max(1.0);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Error function.
    pub fn erf(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_erf(&mut out.inner, &self.inner, MPFR_RNDN) };
        let value = self.to_f64();
        let derivative = 2.0 / core::f64::consts::PI.sqrt() * (-value * value).exp();
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Complementary error function.
    pub fn erfc(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_erfc(&mut out.inner, &self.inner, MPFR_RNDN) };
        let value = self.to_f64();
        let derivative = 2.0 / core::f64::consts::PI.sqrt() * (-value * value).exp();
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// True when the stored value is neither infinite nor NaN.
    pub(crate) fn is_finite(&self) -> bool {
        // SAFETY: the value is initialized.
        unsafe { ffi_mpfr::mpfr_nan_p(&self.inner) == 0 && ffi_mpfr::mpfr_inf_p(&self.inner) == 0 }
    }

    /// The greatest integer not greater than the value.
    pub fn floor(&self) -> Result<Real, NumError> {
        self.round_integral(ffi_mpfr::mpfr_floor)
    }

    /// The least integer not less than the value.
    pub fn ceil(&self) -> Result<Real, NumError> {
        self.round_integral(ffi_mpfr::mpfr_ceil)
    }

    /// The nearest integer, with halves rounded away from zero.
    pub fn round(&self) -> Result<Real, NumError> {
        self.round_integral(ffi_mpfr::mpfr_round)
    }

    fn round_integral(
        &self,
        operation: unsafe extern "C" fn(*mut Mpfr, *const Mpfr) -> core::ffi::c_int,
    ) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { operation(&mut out.inner, &self.inner) };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// A stable identity string for interning.
    pub(crate) fn key_parts(&self) -> (u32, i8, i64, Box<[u64]>, u64) {
        if self.is_zero() {
            return (self.bits, 0, 0, Box::new([]), self.error.to_bits());
        }
        let limbs = (self.bits as usize + 63) / 64;
        let digits = if limbs == 0 {
            Box::new([])
        } else {
            // SAFETY: MPFR stores `limbs` initialized limbs for a nonzero value.
            let slice = unsafe { core::slice::from_raw_parts(self.inner.limbs, limbs) };
            slice.to_vec().into_boxed_slice()
        };
        (
            self.bits,
            self.sign(),
            self.inner.exponent as i64,
            digits,
            self.error.to_bits(),
        )
    }

    /// Convert to a machine `f64` for diagnostics and external display.
    pub fn to_f64(&self) -> f64 {
        // SAFETY: the value is initialized.
        unsafe { ffi_mpfr::mpfr_get_d(&self.inner, MPFR_RNDN) }
    }

    fn absolute_value(&self) -> f64 {
        self.to_f64().abs()
    }
}

impl Clone for Real {
    fn clone(&self) -> Self {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_set(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.error;
        out
    }
}

impl Drop for Real {
    fn drop(&mut self) {
        // SAFETY: the value was initialized and is cleared exactly once.
        unsafe { ffi_mpfr::mpfr_clear(&mut self.inner) };
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_decimal())
    }
}

impl fmt::Debug for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Real({})", self.to_decimal())
    }
}

fn bits_for_digits(digits: u32) -> u32 {
    ((digits as f64 * 3.321_928_094_887_362) as u32)
        .saturating_add(32)
        .max(64)
}

fn rounding_error(value: &Real) -> f64 {
    let magnitude = value.absolute_value();
    if magnitude == 0.0 {
        0.0
    } else {
        magnitude * 2f64.powi(-(value.bits as i32 - 1).max(1))
    }
}

fn format_decimal(text: &str, exponent: core::ffi::c_long) -> String {
    let (sign, digits) = if let Some(rest) = text.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", text)
    };
    if digits.is_empty() {
        return "0.0".to_string();
    }
    let exponent = exponent as isize;
    let length = digits.len() as isize;
    let mut out = String::from(sign);
    if exponent <= 0 && exponent > -6 {
        out.push_str("0.");
        for _ in 0..(-exponent) {
            out.push('0');
        }
        out.push_str(digits);
    } else if exponent > 0 && exponent <= length {
        let split = exponent as usize;
        out.push_str(&digits[..split]);
        out.push('.');
        let rest = &digits[split..];
        if rest.is_empty() {
            out.push('0');
        } else {
            out.push_str(rest);
        }
    } else if exponent > length && exponent <= length + 6 {
        out.push_str(digits);
        for _ in 0..(exponent - length) {
            out.push('0');
        }
        out.push_str(".0");
    } else {
        out.push_str(&digits[..1]);
        out.push('.');
        let rest = &digits[1..];
        if rest.is_empty() {
            out.push('0');
        } else {
            out.push_str(rest);
        }
        out.push('e');
        out.push_str(&(exponent - 1).to_string());
    }
    out
}
