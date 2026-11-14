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
            (exponent as f64) * self.to_f64().abs().powi((exponent - 1) as i32)
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
        out.error = self.absolute_value().max(1.0) * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Cosine.
    pub fn cos(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_cos(&mut out.inner, &self.inner, MPFR_RNDN) };
        out.error = self.absolute_value().max(1.0) * self.error + rounding_error(&out);
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
        let derivative = 1.0 / (1.0 - self.to_f64().powi(2)).sqrt().max(f64::MIN_POSITIVE);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Arccosine.
    pub fn acos(&self) -> Result<Real, NumError> {
        let mut out = Self::initialized(self.bits);
        // SAFETY: both values are initialized.
        unsafe { ffi_mpfr::mpfr_acos(&mut out.inner, &self.inner, MPFR_RNDN) };
        let derivative = 1.0 / (1.0 - self.to_f64().powi(2)).sqrt().max(f64::MIN_POSITIVE);
        out.error = derivative * self.error + rounding_error(&out);
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

    /// A stable identity string for interning.
    pub(crate) fn key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.to_decimal_with_digits(40),
            self.bits,
            self.error.to_bits()
        )
    }

    fn to_f64(&self) -> f64 {
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
