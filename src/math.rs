//! Floating-point functions available under both `std` and `core` plus `alloc`.

/// Floating-point functions that `core` does not provide for every supported target.
///
/// The inherent `std` methods take priority when the `std` feature is enabled. Without
/// `std`, the trait supplies the same operations through `libm`.
#[allow(dead_code)]
pub(crate) trait FloatMath {
    fn sqrt(self) -> f64;
    fn exp(self) -> f64;
    fn exp2(self) -> f64;
    fn ln(self) -> f64;
    fn log10(self) -> f64;
    fn sin(self) -> f64;
    fn cos(self) -> f64;
    fn tan(self) -> f64;
    fn asin(self) -> f64;
    fn acos(self) -> f64;
    fn atan(self) -> f64;
    fn sinh(self) -> f64;
    fn cosh(self) -> f64;
    fn tanh(self) -> f64;
    fn asinh(self) -> f64;
    fn acosh(self) -> f64;
    fn atanh(self) -> f64;
    fn floor(self) -> f64;
    fn ceil(self) -> f64;
    fn round(self) -> f64;
    fn hypot(self, other: f64) -> f64;
    fn powf(self, other: f64) -> f64;
    fn powi(self, exponent: i32) -> f64;
}

impl FloatMath for f64 {
    #[cfg(feature = "std")]
    fn sqrt(self) -> f64 {
        f64::sqrt(self)
    }
    #[cfg(not(feature = "std"))]
    fn sqrt(self) -> f64 {
        libm::sqrt(self)
    }

    #[cfg(feature = "std")]
    fn exp(self) -> f64 {
        f64::exp(self)
    }
    #[cfg(not(feature = "std"))]
    fn exp(self) -> f64 {
        libm::exp(self)
    }

    #[cfg(feature = "std")]
    fn exp2(self) -> f64 {
        f64::exp2(self)
    }
    #[cfg(not(feature = "std"))]
    fn exp2(self) -> f64 {
        libm::exp2(self)
    }

    #[cfg(feature = "std")]
    fn ln(self) -> f64 {
        f64::ln(self)
    }
    #[cfg(not(feature = "std"))]
    fn ln(self) -> f64 {
        libm::log(self)
    }

    #[cfg(feature = "std")]
    fn log10(self) -> f64 {
        f64::log10(self)
    }
    #[cfg(not(feature = "std"))]
    fn log10(self) -> f64 {
        libm::log10(self)
    }

    #[cfg(feature = "std")]
    fn sin(self) -> f64 {
        f64::sin(self)
    }
    #[cfg(not(feature = "std"))]
    fn sin(self) -> f64 {
        libm::sin(self)
    }

    #[cfg(feature = "std")]
    fn cos(self) -> f64 {
        f64::cos(self)
    }
    #[cfg(not(feature = "std"))]
    fn cos(self) -> f64 {
        libm::cos(self)
    }

    #[cfg(feature = "std")]
    fn tan(self) -> f64 {
        f64::tan(self)
    }
    #[cfg(not(feature = "std"))]
    fn tan(self) -> f64 {
        libm::tan(self)
    }

    #[cfg(feature = "std")]
    fn asin(self) -> f64 {
        f64::asin(self)
    }
    #[cfg(not(feature = "std"))]
    fn asin(self) -> f64 {
        libm::asin(self)
    }

    #[cfg(feature = "std")]
    fn acos(self) -> f64 {
        f64::acos(self)
    }
    #[cfg(not(feature = "std"))]
    fn acos(self) -> f64 {
        libm::acos(self)
    }

    #[cfg(feature = "std")]
    fn atan(self) -> f64 {
        f64::atan(self)
    }
    #[cfg(not(feature = "std"))]
    fn atan(self) -> f64 {
        libm::atan(self)
    }

    #[cfg(feature = "std")]
    fn sinh(self) -> f64 {
        f64::sinh(self)
    }
    #[cfg(not(feature = "std"))]
    fn sinh(self) -> f64 {
        libm::sinh(self)
    }

    #[cfg(feature = "std")]
    fn cosh(self) -> f64 {
        f64::cosh(self)
    }
    #[cfg(not(feature = "std"))]
    fn cosh(self) -> f64 {
        libm::cosh(self)
    }

    #[cfg(feature = "std")]
    fn tanh(self) -> f64 {
        f64::tanh(self)
    }
    #[cfg(not(feature = "std"))]
    fn tanh(self) -> f64 {
        libm::tanh(self)
    }

    #[cfg(feature = "std")]
    fn asinh(self) -> f64 {
        f64::asinh(self)
    }
    #[cfg(not(feature = "std"))]
    fn asinh(self) -> f64 {
        libm::asinh(self)
    }

    #[cfg(feature = "std")]
    fn acosh(self) -> f64 {
        f64::acosh(self)
    }
    #[cfg(not(feature = "std"))]
    fn acosh(self) -> f64 {
        libm::acosh(self)
    }

    #[cfg(feature = "std")]
    fn atanh(self) -> f64 {
        f64::atanh(self)
    }
    #[cfg(not(feature = "std"))]
    fn atanh(self) -> f64 {
        libm::atanh(self)
    }

    #[cfg(feature = "std")]
    fn floor(self) -> f64 {
        f64::floor(self)
    }
    #[cfg(not(feature = "std"))]
    fn floor(self) -> f64 {
        libm::floor(self)
    }

    #[cfg(feature = "std")]
    fn ceil(self) -> f64 {
        f64::ceil(self)
    }
    #[cfg(not(feature = "std"))]
    fn ceil(self) -> f64 {
        libm::ceil(self)
    }

    #[cfg(feature = "std")]
    fn round(self) -> f64 {
        f64::round(self)
    }
    #[cfg(not(feature = "std"))]
    fn round(self) -> f64 {
        libm::round(self)
    }

    #[cfg(feature = "std")]
    fn hypot(self, other: f64) -> f64 {
        f64::hypot(self, other)
    }
    #[cfg(not(feature = "std"))]
    fn hypot(self, other: f64) -> f64 {
        libm::hypot(self, other)
    }

    #[cfg(feature = "std")]
    fn powf(self, other: f64) -> f64 {
        f64::powf(self, other)
    }
    #[cfg(not(feature = "std"))]
    fn powf(self, other: f64) -> f64 {
        libm::pow(self, other)
    }

    #[cfg(feature = "std")]
    fn powi(self, exponent: i32) -> f64 {
        f64::powi(self, exponent)
    }
    #[cfg(not(feature = "std"))]
    fn powi(self, exponent: i32) -> f64 {
        libm::pow(self, exponent as f64)
    }
}
