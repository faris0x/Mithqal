//! Arbitrary precision reals with significance tracking.
//!
//! Values are stored as Malachite binary floats. The wrapper keeps the
//! engine's decimal significance model and directed rounding interface.

use core::cmp::Ordering;
use core::fmt;

use malachite::Float;
use malachite::base::num::arithmetic::traits::{Abs, Sign};
use malachite::base::num::conversion::string::options::ToSciOptions;
use malachite::base::num::conversion::traits::{RoundingFrom, ToSci};
use malachite::base::rounding_modes::RoundingMode;

use super::int::natural_bytes;
use super::{NumError, Rat};

/// An approximate real value with an absolute error estimate.
#[derive(Clone)]
pub struct Real {
    pub(crate) inner: Float,
    pub(crate) bits: u32,
    pub(crate) error: f64,
}

impl Real {
    fn exact_bits(value: Float, bits: u32) -> Self {
        let mut out = Real {
            inner: value,
            bits,
            error: 0.0,
        };
        out.error = rounding_error(&out);
        out
    }

    /// Convert an exact rational to a real with a requested decimal digit count.
    pub fn from_rat(value: &Rat, digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let inner = Float::from_rational_prec(value.inner.clone(), u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// Convert an `i64` to a real with a requested decimal digit count.
    pub fn from_i64(value: i64, digits: u32) -> Result<Self, NumError> {
        Self::from_i64_round(value, digits, 0)
    }

    pub(crate) fn one_bits(bits: u32) -> Self {
        Self::from_i64_bits(1, bits).expect("one is always representable")
    }

    pub(crate) fn from_i64_bits(value: i64, bits: u32) -> Result<Self, NumError> {
        Ok(Self::exact_bits(
            Float::from_signed_prec(value, u64::from(bits)).0,
            bits,
        ))
    }

    pub(crate) fn from_i64_round(
        value: i64,
        digits: u32,
        rounding: core::ffi::c_int,
    ) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let inner =
            Float::from_signed_prec_round(value, u64::from(bits), rounding_mode(rounding)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = if rounding == 0 {
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
        let inner = self
            .inner
            .add_prec_round_ref_ref(&other.inner, u64::from(bits), rounding_mode(rounding))
            .0;
        Ok(Real {
            inner,
            bits,
            error: 0.0,
        })
    }

    pub(crate) fn mul_round(
        &self,
        other: &Real,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let inner = self
            .inner
            .mul_prec_round_ref_ref(&other.inner, u64::from(bits), rounding_mode(rounding))
            .0;
        Ok(Real {
            inner,
            bits,
            error: 0.0,
        })
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
        let inner = self
            .inner
            .div_prec_round_ref_ref(&other.inner, u64::from(bits), rounding_mode(rounding))
            .0;
        Ok(Real {
            inner,
            bits,
            error: 0.0,
        })
    }

    pub(crate) fn pow_si_round(
        &self,
        exponent: i64,
        rounding: core::ffi::c_int,
    ) -> Result<Real, NumError> {
        if self.is_zero() && exponent < 0 {
            return Err(NumError::Domain);
        }
        let inner = self
            .inner
            .pow_s_prec_round_ref(exponent, u64::from(self.bits), rounding_mode(rounding))
            .0;
        Ok(Real {
            inner,
            bits: self.bits,
            error: 0.0,
        })
    }

    pub(crate) fn sqrt_round(&self, rounding: core::ffi::c_int) -> Result<Real, NumError> {
        if self.sign() < 0 {
            return Err(NumError::Domain);
        }
        Ok(Real {
            inner: self
                .inner
                .sqrt_prec_round_ref(u64::from(self.bits), rounding_mode(rounding))
                .0,
            bits: self.bits,
            error: 0.0,
        })
    }

    pub(crate) fn exp_round(&self, rounding: core::ffi::c_int) -> Result<Real, NumError> {
        Ok(Real {
            inner: self
                .inner
                .exp_prec_round_ref(u64::from(self.bits), rounding_mode(rounding))
                .0,
            bits: self.bits,
            error: 0.0,
        })
    }

    pub(crate) fn ln_round(&self, rounding: core::ffi::c_int) -> Result<Real, NumError> {
        if self.sign() <= 0 {
            return Err(NumError::Domain);
        }
        Ok(Real {
            inner: self
                .inner
                .ln_prec_round_ref(u64::from(self.bits), rounding_mode(rounding))
                .0,
            bits: self.bits,
            error: 0.0,
        })
    }

    pub(crate) fn sin_round(&self, rounding: core::ffi::c_int) -> Result<Real, NumError> {
        Ok(Real {
            inner: self
                .inner
                .sin_prec_round_ref(u64::from(self.bits), rounding_mode(rounding))
                .0,
            bits: self.bits,
            error: 0.0,
        })
    }

    pub(crate) fn cos_round(&self, rounding: core::ffi::c_int) -> Result<Real, NumError> {
        Ok(Real {
            inner: self
                .inner
                .cos_prec_round_ref(u64::from(self.bits), rounding_mode(rounding))
                .0,
            bits: self.bits,
            error: 0.0,
        })
    }

    /// Convert an `f64` to a real with a requested decimal digit count.
    pub fn from_f64(value: f64, digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let inner =
            Float::from_primitive_float_prec_round(value, u64::from(bits), RoundingMode::Nearest).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// A conservative lower endpoint from the stored error estimate.
    pub(crate) fn lower_bound(&self) -> Result<Real, NumError> {
        if !self.error.is_finite() || self.error == 0.0 {
            return Ok(self.clone());
        }
        let error = Self::from_f64(-self.error, 64)?;
        self.add_round(&error, super::ROUND_DOWN)
    }

    /// A conservative upper endpoint from the stored error estimate.
    pub(crate) fn upper_bound(&self) -> Result<Real, NumError> {
        if !self.error.is_finite() || self.error == 0.0 {
            return Ok(self.clone());
        }
        let error = Self::from_f64(self.error, 64)?;
        self.add_round(&error, super::ROUND_UP)
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
        let inner = Float::pi_prec(u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The constant e with a requested decimal digit count.
    pub fn e(digits: u32) -> Result<Self, NumError> {
        let bits = bits_for_digits(digits);
        let inner = Float::e_prec(u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The approximate decimal value.
    pub fn to_decimal(&self) -> String {
        let precision = self.precision_digits();
        let digits = if precision.is_finite() {
            precision.clamp(1.0, 10000.0).floor() as usize
        } else {
            16
        };
        self.to_decimal_with_digits(digits.max(1))
    }

    fn to_decimal_with_digits(&self, digits: usize) -> String {
        if self.is_zero() {
            return "0.0".to_string();
        }
        let mut options = ToSciOptions::default();
        options.set_precision(digits as u64);
        options.set_include_trailing_zeros(true);
        self.inner.to_sci_with_options(options).to_string()
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
        self.inner.is_zero()
    }

    /// The sign of the stored value.
    pub fn sign(&self) -> i8 {
        if self.is_zero() {
            0
        } else if self.inner.sign() == Ordering::Less {
            -1
        } else {
            1
        }
    }

    /// Compare two approximate values by their stored approximations.
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(&self, other: &Real) -> Ordering {
        self.inner
            .partial_cmp(&other.inner)
            .unwrap_or(Ordering::Equal)
    }

    /// Add two approximate values.
    pub fn add(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let inner = self.inner.add_prec_ref_ref(&other.inner, u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = self.error.hypot(other.error) + rounding_error(&out);
        Ok(out)
    }

    /// Subtract `other` from `self`.
    pub fn sub(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let inner = self.inner.sub_prec_ref_ref(&other.inner, u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
        out.error = self.error.hypot(other.error) + rounding_error(&out);
        Ok(out)
    }

    /// Multiply two approximate values.
    pub fn mul(&self, other: &Real) -> Result<Real, NumError> {
        let bits = self.bits.max(other.bits);
        let inner = self.inner.mul_prec_ref_ref(&other.inner, u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
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
        let inner = self.inner.div_prec_ref_ref(&other.inner, u64::from(bits)).0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
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
        let mut out = self.clone();
        out.inner = -out.inner;
        Ok(out)
    }

    /// Absolute value.
    pub fn abs(&self) -> Result<Real, NumError> {
        let mut out = self.clone();
        out.inner = out.inner.abs();
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
        let inner = self
            .inner
            .pow_prec_ref_ref(&exponent.inner, u64::from(bits))
            .0;
        let mut out = Real {
            inner,
            bits,
            error: 0.0,
        };
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
        if self.is_zero() && exponent < 0 {
            return Err(NumError::Domain);
        }
        let inner = self.inner.pow_s_prec_ref(exponent, u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
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
        let inner = self.inner.sqrt_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
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
        let inner = self.inner.exp_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = out.absolute_value() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Natural logarithm.
    pub fn ln(&self) -> Result<Real, NumError> {
        if self.sign() <= 0 {
            return Err(NumError::Domain);
        }
        let inner = self.inner.ln_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = self.error / self.absolute_value() + rounding_error(&out);
        Ok(out)
    }

    /// Sine.
    pub fn sin(&self) -> Result<Real, NumError> {
        let inner = self.inner.sin_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = self.to_f64().cos().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Cosine.
    pub fn cos(&self) -> Result<Real, NumError> {
        let inner = self.inner.cos_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = self.to_f64().sin().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Tangent.
    pub fn tan(&self) -> Result<Real, NumError> {
        let inner = self.inner.tan_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let derivative = 1.0 + out.to_f64().powi(2);
        out.error = derivative.abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Arcsine.
    pub fn asin(&self) -> Result<Real, NumError> {
        let one = Real::one_bits(self.bits);
        let minus_one = one.neg()?;
        if self.cmp(&one) == Ordering::Greater || self.cmp(&minus_one) == Ordering::Less {
            return Err(NumError::Domain);
        }
        let inner = self.inner.asin_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let cosine = out.to_f64().cos().abs().max(f64::MIN_POSITIVE);
        out.error = self.error / cosine + rounding_error(&out);
        Ok(out)
    }

    /// Arccosine.
    pub fn acos(&self) -> Result<Real, NumError> {
        let one = Real::one_bits(self.bits);
        let minus_one = one.neg()?;
        if self.cmp(&one) == Ordering::Greater || self.cmp(&minus_one) == Ordering::Less {
            return Err(NumError::Domain);
        }
        let inner = self.inner.acos_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let cosine = out.to_f64().cos().abs().max(f64::MIN_POSITIVE);
        out.error = self.error / cosine + rounding_error(&out);
        Ok(out)
    }

    /// Arctangent.
    pub fn atan(&self) -> Result<Real, NumError> {
        let inner = self.inner.atan_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let derivative = 1.0 / (1.0 + self.to_f64().powi(2));
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic sine.
    pub fn sinh(&self) -> Result<Real, NumError> {
        let inner = self.inner.sinh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = self.to_f64().cosh().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic cosine.
    pub fn cosh(&self) -> Result<Real, NumError> {
        let inner = self.inner.cosh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        out.error = self.to_f64().sinh().abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Hyperbolic tangent.
    pub fn tanh(&self) -> Result<Real, NumError> {
        let inner = self.inner.tanh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let derivative = 1.0 - out.to_f64().powi(2);
        out.error = derivative.abs() * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic sine.
    pub fn asinh(&self) -> Result<Real, NumError> {
        let inner = self.inner.asinh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let derivative = 1.0 / out.to_f64().cosh().abs().max(f64::MIN_POSITIVE);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic cosine.
    pub fn acosh(&self) -> Result<Real, NumError> {
        if self.cmp(&Real::one_bits(self.bits)) == Ordering::Less {
            return Err(NumError::Domain);
        }
        let inner = self.inner.acosh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let sinh = out.to_f64().sinh().abs();
        let derivative = if sinh == 0.0 { 0.0 } else { 1.0 / sinh };
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Inverse hyperbolic tangent.
    pub fn atanh(&self) -> Result<Real, NumError> {
        let one = Real::one_bits(self.bits);
        let minus_one = one.neg()?;
        if self.cmp(&one) != Ordering::Less || self.cmp(&minus_one) != Ordering::Greater {
            return Err(NumError::Domain);
        }
        let inner = self.inner.atanh_prec_ref(u64::from(self.bits)).0;
        let mut out = Real {
            inner,
            bits: self.bits,
            error: 0.0,
        };
        let derivative = out.to_f64().cosh().powi(2);
        out.error = derivative * self.error + rounding_error(&out);
        Ok(out)
    }

    /// Gamma function.
    pub fn gamma(&self) -> Result<Real, NumError> {
        if !self.is_finite() || self.is_zero() {
            return Err(NumError::Domain);
        }
        if self.sign() < 0 && self.inner.round_to_integer_ref().0.is_zero() {
            return Err(NumError::Domain);
        }
        let digits = digits_for_bits(self.bits);
        if self.to_f64() < 0.5 {
            let one = Real::one(digits)?;
            let reflected = one.sub(self)?.gamma()?;
            let pi = Real::pi(digits)?;
            let sine = self.mul(&pi)?.sin()?;
            let denominator = sine.mul(&reflected)?;
            let result = pi.div(&denominator)?;
            if !result.is_finite() {
                return Err(NumError::Domain);
            }
            return Ok(result);
        }
        let result = spouge_gamma(self, digits)?;
        if !result.is_finite() {
            return Err(NumError::Domain);
        }
        Ok(result)
    }

    /// Logarithm of the absolute gamma function.
    pub fn lgamma(&self) -> Result<Real, NumError> {
        let gamma = self.gamma()?;
        let result = gamma.abs()?.ln()?;
        if !result.is_finite() {
            return Err(NumError::Domain);
        }
        Ok(result)
    }

    /// Error function.
    pub fn erf(&self) -> Result<Real, NumError> {
        if self.is_zero() {
            return Ok(self.clone());
        }
        let digits = digits_for_bits(self.bits);
        let magnitude = self.to_f64().abs();
        if magnitude > 6.0 {
            let result = Real::one(digits)?;
            return if self.sign() < 0 {
                result.neg()
            } else {
                Ok(result)
            };
        }
        let x2 = self.mul(self)?;
        let mut term = self.clone();
        let mut sum = term.clone();
        let threshold = 10f64.powi(-(digits as i32 + 5));
        for n in 0..512i64 {
            let numerator = Real::from_i64(-(2 * n + 1), digits)?;
            let denominator = Real::from_i64((n + 1) * (2 * n + 3), digits)?;
            let next = term.mul(&x2)?.mul(&numerator)?.div(&denominator)?;
            term = next;
            sum = sum.add(&term)?;
            if term.abs()?.to_f64() <= threshold {
                break;
            }
        }
        let two = Real::from_i64(2, digits)?;
        let sqrt_pi = Real::pi(digits)?.sqrt()?;
        sum.mul(&two)?.div(&sqrt_pi)
    }

    /// Complementary error function.
    pub fn erfc(&self) -> Result<Real, NumError> {
        let one = Real::one(digits_for_bits(self.bits))?;
        one.sub(&self.erf()?)
    }

    /// True when the stored value is neither infinite nor NaN.
    pub(crate) fn is_finite(&self) -> bool {
        self.inner.is_finite()
    }

    /// The greatest integer not greater than the value.
    pub fn floor(&self) -> Result<Real, NumError> {
        let mut out = self.clone();
        out.inner = self.inner.round_to_integer_round_ref(RoundingMode::Floor).0;
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The least integer not less than the value.
    pub fn ceil(&self) -> Result<Real, NumError> {
        let mut out = self.clone();
        out.inner = self
            .inner
            .round_to_integer_round_ref(RoundingMode::Ceiling)
            .0;
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// The nearest integer, with halves rounded away from zero.
    pub fn round(&self) -> Result<Real, NumError> {
        let mut out = self.clone();
        out.inner = self.inner.round_to_integer_ties_away_ref().0;
        out.error = rounding_error(&out);
        Ok(out)
    }

    /// A stable identity string for interning.
    pub(crate) fn key_parts(&self) -> (u32, i8, i64, Box<[u8]>, u64) {
        if self.is_zero() {
            return (self.bits, 0, 0, Box::new([]), self.error.to_bits());
        }
        let exponent = i64::from(self.inner.get_exponent().unwrap_or(0));
        let significand = self.inner.to_significand().unwrap_or_default();
        (
            self.bits,
            self.sign(),
            exponent,
            natural_bytes(&significand).into_boxed_slice(),
            self.error.to_bits(),
        )
    }

    /// Convert to a machine `f64` for diagnostics and external display.
    pub fn to_f64(&self) -> f64 {
        f64::rounding_from(&self.inner, RoundingMode::Nearest).0
    }

    fn absolute_value(&self) -> f64 {
        self.to_f64().abs()
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
    ((digits as f64 * std::f64::consts::LOG2_10) as u32)
        .saturating_add(32)
        .max(64)
}

fn digits_for_bits(bits: u32) -> u32 {
    ((bits as f64 * std::f64::consts::LOG10_2).ceil() as u32).max(1)
}

fn rounding_error(value: &Real) -> f64 {
    let magnitude = value.absolute_value();
    if magnitude == 0.0 {
        0.0
    } else {
        magnitude * 2f64.powi(-(value.bits as i32 - 1).max(1))
    }
}

fn rounding_mode(rounding: core::ffi::c_int) -> RoundingMode {
    match rounding {
        super::ROUND_DOWN => RoundingMode::Floor,
        super::ROUND_UP => RoundingMode::Ceiling,
        _ => RoundingMode::Nearest,
    }
}

fn spouge_gamma(value: &Real, digits: u32) -> Result<Real, NumError> {
    let a = ((digits as f64 * 2.0).ceil() as i64 + 10).clamp(24, 512);
    let one = Real::one(digits)?;
    let two = Real::from_i64(2, digits)?;
    let half = one.div(&two)?;
    let w = value.sub(&one)?;
    let offset = Real::from_i64(a, digits)?;
    let base = w.add(&offset)?;
    let exponent = w.add(&half)?;
    let power = base.pow(&exponent)?;
    let exponential = base.neg()?.exp()?;
    let mut sum = Real::pi(digits)?.mul(&two)?.sqrt()?;
    for k in 1..a {
        let coefficient = spouge_coefficient(k, a, digits)?;
        let denominator = w.add(&Real::from_i64(k, digits)?)?;
        let term = coefficient.div(&denominator)?;
        sum = sum.add(&term)?;
    }
    power.mul(&exponential)?.mul(&sum)
}

fn spouge_coefficient(k: i64, a: i64, digits: u32) -> Result<Real, NumError> {
    let mut factorial = Real::one(digits)?;
    for value in 1..k {
        factorial = factorial.mul(&Real::from_i64(value, digits)?)?;
    }
    let difference = Real::from_i64(a - k, digits)?;
    let exponent = Real::from_i64(2 * k - 1, digits)?.div(&Real::from_i64(2, digits)?)?;
    let power = difference.pow(&exponent)?;
    let exponential = difference.exp()?;
    let magnitude = power.mul(&exponential)?.div(&factorial)?;
    if k % 2 == 1 {
        Ok(magnitude)
    } else {
        magnitude.neg()
    }
}
