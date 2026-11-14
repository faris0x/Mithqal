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
