//! MPC complex arithmetic wrappers.

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
        self.unary(ffi_mpc::mpc_exp, 1.0)
    }

    pub(crate) fn log(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_log, 0.1)
    }

    pub(crate) fn sin(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_sin, 2.0)
    }

    pub(crate) fn cos(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_cos, 2.0)
    }

    pub(crate) fn tan(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_tan, 4.0)
    }

    pub(crate) fn sqrt(&self) -> Option<(Real, Real)> {
        self.unary(ffi_mpc::mpc_sqrt, 0.5)
    }

    pub(crate) fn pow(&self, exponent: &Complex) -> Option<(Real, Real)> {
        let bits = self.bits.max(exponent.bits);
        let mut output = Mpc {
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
        // SAFETY: the output structure is initialized.
        unsafe { ffi_mpc::mpc_init2(&mut output, bits as core::ffi::c_long) };
        // SAFETY: all structures are initialized.
        unsafe { ffi_mpc::mpc_pow(&mut output, &self.inner, &exponent.inner, MPFR_RNDN) };
        let mut real = Real::from_mpfr_ref(&output.real, bits, 0.0);
        let mut imaginary = Real::from_mpfr_ref(&output.imaginary, bits, 0.0);
        // SAFETY: the output is initialized and cleared once.
        unsafe { ffi_mpc::mpc_clear(&mut output) };
        let scale = (real.to_f64().abs() + imaginary.to_f64().abs()).max(1.0);
        let error = (self.real_error
            + self.imaginary_error
            + exponent.real_error
            + exponent.imaginary_error
            + rounding(bits, scale))
            * 4.0;
        real.error = error;
        imaginary.error = error;
        Some((real, imaginary))
    }

    fn unary(
        &self,
        operation: unsafe extern "C" fn(*mut Mpc, *const Mpc, core::ffi::c_int) -> core::ffi::c_int,
        factor: f64,
    ) -> Option<(Real, Real)> {
        let mut output = Mpc {
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
        // SAFETY: the output structure is initialized.
        unsafe { ffi_mpc::mpc_init2(&mut output, self.bits as core::ffi::c_long) };
        // SAFETY: both structures are initialized.
        unsafe { operation(&mut output, &self.inner, MPFR_RNDN) };
        let mut real = Real::from_mpfr_ref(&output.real, self.bits, 0.0);
        let mut imaginary = Real::from_mpfr_ref(&output.imaginary, self.bits, 0.0);
        // SAFETY: the output is initialized and cleared once.
        unsafe { ffi_mpc::mpc_clear(&mut output) };

        let scale = (real.to_f64().abs() + imaginary.to_f64().abs()).max(1.0);
        let error = (self.real_error + self.imaginary_error + rounding(self.bits, scale)) * factor;
        real.error = error;
        imaginary.error = error;
        Some((real, imaginary))
    }
}

impl Drop for Complex {
    fn drop(&mut self) {
        // SAFETY: the value was initialized and is cleared exactly once.
        unsafe { ffi_mpc::mpc_clear(&mut self.inner) };
    }
}

fn rounding(bits: u32, scale: f64) -> f64 {
    scale * 2f64.powi(-(bits as i32 - 1).max(1))
}
