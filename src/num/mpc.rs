//! MPC complex arithmetic wrappers.
//!
//! Every unary operation propagates the input significance error through a
//! first order derivative magnitude estimate. The estimate is deliberately
//! conservative: when the derivative is singular the reported error becomes
//! infinite, and non finite results are rejected instead of entering the
//! expression arena.

use super::Real;
use super::ffi_mpc::{self, Mpc};
use super::ffi_mpfr::{self, MPFR_RNDN};

pub(crate) struct Complex {
    inner: Mpc,
    bits: u32,
    real_error: f64,
    imaginary_error: f64,
}

impl Complex {
    pub(crate) fn from_parts(real: &Real, imaginary: &Real) -> Option<Self> {
        let bits = real.bits.max(imaginary.bits);
        let mut inner = Mpc {
            real: ffi_mpfr::Mpfr {
                precision: 0,
                sign: 0,
                exponent: 0,
                limbs: core::ptr::null_mut(),
            },
            imaginary: ffi_mpfr::Mpfr {
                precision: 0,
                sign: 0,
                exponent: 0,
                limbs: core::ptr::null_mut(),
            },
        };
        // SAFETY: the structure has room for MPC state and is initialized once.
        unsafe { ffi_mpc::mpc_init2(&mut inner, bits as core::ffi::c_long) };
        // SAFETY: the components are initialized.
        unsafe {
            ffi_mpc::mpc_set_fr_fr(&mut inner, &real.inner, &imaginary.inner, MPFR_RNDN);
        }
        Some(Complex {
            inner,
            bits,
            real_error: real.error,
            imaginary_error: imaginary.error,
        })
    }

    pub(crate) fn exp(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_exp, |_, _, u, v| cabs(u, v))
    }

    pub(crate) fn log(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_log, |x, y, _, _| 1.0 / cabs(x, y))
    }

    pub(crate) fn sin(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_sin, |x, y, _, _| {
            cabs(x.cos() * y.cosh(), x.sin() * y.sinh())
        })
    }

    pub(crate) fn cos(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_cos, |x, y, _, _| {
            cabs(x.sin() * y.cosh(), x.cos() * y.sinh())
        })
    }

    pub(crate) fn tan(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_tan, |x, y, _, _| {
            let cosine = cabs(x.sin() * y.cosh(), x.cos() * y.sinh());
            1.0 / (cosine * cosine)
        })
    }

    pub(crate) fn sqrt(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_sqrt, |_, _, u, v| {
            let magnitude = cabs(u, v);
            if magnitude == 0.0 {
                0.0
            } else {
                0.5 / magnitude
            }
        })
    }

    pub(crate) fn asin(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_asin, |x, y, _, _| {
            inverse_sqrt(cabs(1.0 - (x * x - y * y), -2.0 * x * y))
        })
    }

    pub(crate) fn acos(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_acos, |x, y, _, _| {
            inverse_sqrt(cabs(1.0 - (x * x - y * y), -2.0 * x * y))
        })
    }

    pub(crate) fn atan(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_atan, |x, y, _, _| {
            1.0 / cabs(1.0 + (x * x - y * y), 2.0 * x * y)
        })
    }

    pub(crate) fn sinh(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_sinh, |x, y, _, _| {
            cabs(x.cosh() * y.cos(), x.sinh() * y.sin())
        })
    }

    pub(crate) fn cosh(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_cosh, |x, y, _, _| {
            cabs(x.sinh() * y.cos(), x.cosh() * y.sin())
        })
    }

    pub(crate) fn tanh(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_tanh, |x, y, _, _| {
            let cosine = cabs(x.cosh() * y.cos(), x.sinh() * y.sin());
            1.0 / (cosine * cosine)
        })
    }

    pub(crate) fn asinh(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_asinh, |x, y, _, _| {
            inverse_sqrt(cabs(1.0 + (x * x - y * y), 2.0 * x * y))
        })
    }

    pub(crate) fn acosh(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_acosh, |x, y, _, _| {
            inverse_sqrt(cabs((x * x - y * y) - 1.0, 2.0 * x * y))
        })
    }

    pub(crate) fn abs(&self) -> Option<Real> {
        let mut out = Real::blank(self.bits);
        // SAFETY: the output is initialized and the input is initialized.
        unsafe { ffi_mpc::mpc_abs(&mut out.inner, &self.inner, MPFR_RNDN) };
        if !out.is_finite() {
            return None;
        }
        out.error = self.real_error.hypot(self.imaginary_error)
            + rounding(self.bits, out.to_f64().abs().max(1.0));
        Some(out)
    }

    pub(crate) fn from_real(value: &Real) -> Option<Self> {
        let digits = ((value.bits as f64 * core::f64::consts::LOG10_2).ceil() as u32 + 4).max(16);
        let zero = Real::zero(digits).ok()?;
        Self::from_parts(value, &zero)
    }

    pub(crate) fn add(&self, other: &Complex) -> Option<Complex> {
        self.binary(other, ffi_mpc::mpc_add, 1.0)
    }

    pub(crate) fn sub(&self, other: &Complex) -> Option<Complex> {
        self.binary(other, ffi_mpc::mpc_sub, 1.0)
    }

    pub(crate) fn mul(&self, other: &Complex) -> Option<Complex> {
        let factor = self.abs_value() + other.abs_value();
        self.binary(other, ffi_mpc::mpc_mul, factor.max(1.0))
    }

    pub(crate) fn div(&self, other: &Complex) -> Option<Complex> {
        let denominator = other.abs_value();
        if denominator == 0.0 {
            return None;
        }
        self.binary(
            other,
            ffi_mpc::mpc_div,
            (self.abs_value() / denominator).max(1.0),
        )
    }

    pub(crate) fn neg(&self) -> Option<Complex> {
        let digits = ((self.bits as f64 * core::f64::consts::LOG10_2).ceil() as u32 + 4).max(16);
        let zero = Complex::from_real(&Real::zero(digits).ok()?)?;
        zero.sub(self)
    }

    pub(crate) fn parts(&self) -> Option<(Real, Real)> {
        Some((
            Real::from_mpfr_ref(&self.inner.real, self.bits, self.real_error),
            Real::from_mpfr_ref(&self.inner.imaginary, self.bits, self.imaginary_error),
        ))
    }

    pub(crate) fn gamma(&self, digits: u32) -> Option<Complex> {
        self.gamma_value(digits)
    }

    pub(crate) fn lgamma(&self, digits: u32) -> Option<Complex> {
        let gamma = self.gamma_value(digits)?;
        wrap_complex(gamma.log()?)
    }

    fn gamma_value(&self, digits: u32) -> Option<Complex> {
        let digits = digits.max(16);
        let one = Complex::from_real(&Real::one(digits).ok()?)?;
        let w = self.sub(&one)?;
        if self.real_value() < 0.5 {
            let reflected = one.sub(self)?.gamma_value(digits)?;
            let pi = Real::pi(digits).ok()?;
            let pi = Complex::from_real(&pi)?;
            let sine = wrap_complex(self.mul(&pi)?.sin()?)?;
            let denominator = sine.mul(&reflected)?;
            return Some(set_gamma_error(pi.div(&denominator)?, digits));
        }
        let a = ((digits as f64 * 2.0).ceil() as i64 + 10).clamp(24, 512);
        let offset = Complex::from_real(&Real::from_i64(a, digits).ok()?)?;
        let z = w.add(&offset)?;
        let half = Real::from_i64(1, digits)
            .ok()?
            .div(&Real::from_i64(2, digits).ok()?)
            .ok()?;
        let exponent = w.add(&Complex::from_real(&half)?)?;
        let power = wrap_complex(z.pow(&exponent)?)?;
        let exponential = wrap_complex(z.neg()?.exp()?)?;
        let mut sum = Complex::from_real(&spouge_c0(digits)?)?;
        for k in 1..a {
            let coefficient = spouge_ck(k, a, digits)?;
            let denominator = w.add(&Complex::from_real(&Real::from_i64(k, digits).ok()?)?)?;
            let term = Complex::from_real(&coefficient)?.div(&denominator)?;
            sum = sum.add(&term)?;
        }
        Some(set_gamma_error(power.mul(&exponential)?.mul(&sum)?, digits))
    }

    fn abs_value(&self) -> f64 {
        (self.real_value().powi(2) + self.imaginary_value().powi(2)).sqrt()
    }

    fn binary(
        &self,
        other: &Complex,
        operation: unsafe extern "C" fn(
            *mut Mpc,
            *const Mpc,
            *const Mpc,
            core::ffi::c_int,
        ) -> core::ffi::c_int,
        factor: f64,
    ) -> Option<Complex> {
        let bits = self.bits.max(other.bits);
        let mut output = empty_mpc();
        // SAFETY: the output structure is initialized.
        unsafe { ffi_mpc::mpc_init2(&mut output, bits as core::ffi::c_long) };
        // SAFETY: all structures are initialized.
        unsafe { operation(&mut output, &self.inner, &other.inner, MPFR_RNDN) };
        let mut real = Real::from_mpfr_ref(&output.real, bits, 0.0);
        let mut imaginary = Real::from_mpfr_ref(&output.imaginary, bits, 0.0);
        // SAFETY: the output is initialized and cleared once.
        unsafe { ffi_mpc::mpc_clear(&mut output) };
        if !real.is_finite() || !imaginary.is_finite() {
            return None;
        }
        apply_error(
            &mut real,
            &mut imaginary,
            bits,
            factor,
            self.real_error + self.imaginary_error + other.real_error + other.imaginary_error,
        );
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = bits;
        value.real_error = real.error;
        value.imaginary_error = imaginary.error;
        Some(value)
    }

    pub(crate) fn pow(&self, exponent: &Complex) -> Option<(Real, Real)> {
        let bits = self.bits.max(exponent.bits);
        let mut output = empty_mpc();
        // SAFETY: the output structure is initialized.
        unsafe { ffi_mpc::mpc_init2(&mut output, bits as core::ffi::c_long) };
        // SAFETY: all structures are initialized.
        unsafe { ffi_mpc::mpc_pow(&mut output, &self.inner, &exponent.inner, MPFR_RNDN) };
        let mut real = Real::from_mpfr_ref(&output.real, bits, 0.0);
        let mut imaginary = Real::from_mpfr_ref(&output.imaginary, bits, 0.0);
        // SAFETY: the output is initialized and cleared once.
        unsafe { ffi_mpc::mpc_clear(&mut output) };
        if !real.is_finite() || !imaginary.is_finite() {
            return None;
        }

        let base = (self.real_value(), self.imaginary_value());
        let exponent_value = (exponent.real_value(), exponent.imaginary_value());
        let output_value = (real.to_f64(), imaginary.to_f64());
        let factor = power_derivative(base, exponent_value, output_value);
        apply_error(
            &mut real,
            &mut imaginary,
            bits,
            factor,
            self.real_error + self.imaginary_error + exponent.real_error + exponent.imaginary_error,
        );
        Some((real, imaginary))
    }

    fn unary(
        &self,
        operation: unsafe extern "C" fn(*mut Mpc, *const Mpc, core::ffi::c_int) -> core::ffi::c_int,
        derivative: impl Fn(f64, f64, f64, f64) -> f64,
    ) -> Option<(Real, Real)> {
        let input = (self.real_value(), self.imaginary_value());
        let mut output = empty_mpc();
        // SAFETY: the output structure is initialized.
        unsafe { ffi_mpc::mpc_init2(&mut output, self.bits as core::ffi::c_long) };
        // SAFETY: both structures are initialized.
        unsafe { operation(&mut output, &self.inner, MPFR_RNDN) };
        let mut real = Real::from_mpfr_ref(&output.real, self.bits, 0.0);
        let mut imaginary = Real::from_mpfr_ref(&output.imaginary, self.bits, 0.0);
        // SAFETY: the output is initialized and cleared once.
        unsafe { ffi_mpc::mpc_clear(&mut output) };
        if !real.is_finite() || !imaginary.is_finite() {
            return None;
        }

        let factor = derivative(input.0, input.1, real.to_f64(), imaginary.to_f64());
        apply_error(
            &mut real,
            &mut imaginary,
            self.bits,
            factor,
            self.real_error + self.imaginary_error,
        );
        Some((real, imaginary))
    }

    fn real_value(&self) -> f64 {
        // SAFETY: the component is initialized.
        unsafe { ffi_mpfr::mpfr_get_d(&self.inner.real, MPFR_RNDN) }
    }

    fn imaginary_value(&self) -> f64 {
        // SAFETY: the component is initialized.
        unsafe { ffi_mpfr::mpfr_get_d(&self.inner.imaginary, MPFR_RNDN) }
    }
}

impl Drop for Complex {
    fn drop(&mut self) {
        // SAFETY: the value was initialized and is cleared exactly once.
        unsafe { ffi_mpc::mpc_clear(&mut self.inner) };
    }
}

fn set_gamma_error(mut value: Complex, digits: u32) -> Complex {
    let magnitude = value.abs_value().max(1.0);
    let error = magnitude * 10f64.powi(-(digits as i32 - 2));
    value.real_error = error;
    value.imaginary_error = error;
    value
}

fn wrap_complex(result: (Real, Real)) -> Option<Complex> {
    Complex::from_parts(&result.0, &result.1)
}

fn spouge_c0(digits: u32) -> Option<Real> {
    let two = Real::from_i64(2, digits).ok()?;
    let pi = Real::pi(digits).ok()?;
    two.mul(&pi).ok()?.sqrt().ok()
}

fn spouge_ck(k: i64, a: i64, digits: u32) -> Option<Real> {
    let mut factorial = Real::one(digits).ok()?;
    for value in 1..k {
        factorial = factorial.mul(&Real::from_i64(value, digits).ok()?).ok()?;
    }
    let difference = Real::from_i64(a - k, digits).ok()?;
    let exponent = Real::from_i64(2 * k - 1, digits)
        .ok()?
        .div(&Real::from_i64(2, digits).ok()?)
        .ok()?;
    let power = difference.pow(&exponent).ok()?;
    let exponential = Real::e(digits).ok()?.pow(&difference).ok()?;
    let magnitude = power.mul(&exponential).ok()?.div(&factorial).ok()?;
    if k % 2 == 1 {
        Some(magnitude)
    } else {
        magnitude.neg().ok()
    }
}

fn empty_mpc() -> Mpc {
    Mpc {
        real: ffi_mpfr::Mpfr {
            precision: 0,
            sign: 0,
            exponent: 0,
            limbs: core::ptr::null_mut(),
        },
        imaginary: ffi_mpfr::Mpfr {
            precision: 0,
            sign: 0,
            exponent: 0,
            limbs: core::ptr::null_mut(),
        },
    }
}

fn apply_error(real: &mut Real, imaginary: &mut Real, bits: u32, factor: f64, input_error: f64) {
    let scale = (real.to_f64().abs() + imaginary.to_f64().abs()).max(1.0);
    let error = if factor.is_finite() && input_error.is_finite() {
        factor * input_error + factor.max(1.0) * rounding(bits, scale)
    } else {
        f64::INFINITY
    };
    real.error = error;
    imaginary.error = error;
}

fn power_derivative(base: (f64, f64), exponent: (f64, f64), output: (f64, f64)) -> f64 {
    let magnitude = cabs(base.0, base.1);
    let output_magnitude = cabs(output.0, output.1);
    let exponent_magnitude = cabs(exponent.0, exponent.1);
    let base_term = if magnitude == 0.0 {
        0.0
    } else {
        exponent_magnitude * output_magnitude / magnitude
    };
    let log_magnitude = if magnitude == 0.0 {
        0.0
    } else {
        cabs(magnitude.ln(), base.1.atan2(base.0))
    };
    let exponent_term = exponent_magnitude * output_magnitude * log_magnitude;
    4.0 * (base_term + exponent_term)
}

fn inverse_sqrt(value: f64) -> f64 {
    if value <= 0.0 {
        f64::INFINITY
    } else {
        1.0 / value.sqrt()
    }
}

fn cabs(real: f64, imaginary: f64) -> f64 {
    real.hypot(imaginary)
}

fn rounding(bits: u32, scale: f64) -> f64 {
    scale * 2f64.powi(-(bits as i32 - 1).max(1))
}
