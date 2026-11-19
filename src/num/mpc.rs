//! Complex arithmetic over dashu-backed reals.
//!
//! Every unary operation propagates the input significance error through a
//! first order derivative magnitude estimate. The estimate is deliberately
//! conservative: when the derivative is singular the reported error becomes
//! infinite, and non finite results are rejected instead of entering the
//! expression arena.

#[allow(unused_imports)]
use crate::prelude::*;

use super::Real;

#[derive(Clone)]
pub(crate) struct Complex {
    real: Real,
    imaginary: Real,
    bits: u32,
    real_error: f64,
    imaginary_error: f64,
}

impl Complex {
    pub(crate) fn from_parts(real: &Real, imaginary: &Real) -> Option<Self> {
        Some(Complex {
            real: real.clone(),
            imaginary: imaginary.clone(),
            bits: real.bits.max(imaginary.bits),
            real_error: real.error,
            imaginary_error: imaginary.error,
        })
    }

    pub(crate) fn exp(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let magnitude = value.real.exp().ok()?;
            let sine = value.imaginary.sin().ok()?;
            let cosine = value.imaginary.cos().ok()?;
            Some((magnitude.mul(&cosine).ok()?, magnitude.mul(&sine).ok()?))
        })
    }

    pub(crate) fn log(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let magnitude = value.abs()?.ln().ok()?;
            let angle = atan2(&value.imaginary, &value.real, digits_for_bits(value.bits))?;
            Some((magnitude, angle))
        })
    }

    pub(crate) fn sin(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let real = value
                .real
                .sin()
                .ok()?
                .mul(&value.imaginary.cosh().ok()?)
                .ok()?;
            let imaginary = value
                .real
                .cos()
                .ok()?
                .mul(&value.imaginary.sinh().ok()?)
                .ok()?;
            Some((real, imaginary))
        })
    }

    pub(crate) fn cos(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let real = value
                .real
                .cos()
                .ok()?
                .mul(&value.imaginary.cosh().ok()?)
                .ok()?;
            let imaginary = value
                .real
                .sin()
                .ok()?
                .mul(&value.imaginary.sinh().ok()?)
                .ok()?
                .neg()
                .ok()?;
            Some((real, imaginary))
        })
    }

    pub(crate) fn tan(&self) -> Option<(Real, Real)> {
        let sine = wrap_complex(self.sin()?)?;
        let cosine = wrap_complex(self.cos()?)?;
        sine.div(&cosine)?.parts()
    }

    pub(crate) fn sqrt(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let magnitude = value.abs()?;
            let two = Real::from_i64(2, digits_for_bits(value.bits)).ok()?;
            let real_part = magnitude.add(&value.real).ok()?.div(&two).ok()?;
            let imaginary_part = magnitude.sub(&value.real).ok()?.div(&two).ok()?;
            let real = real_part.sqrt().ok()?;
            let imaginary = imaginary_part.sqrt().ok()?;
            let imaginary = if value.imaginary.sign() < 0 {
                imaginary.neg().ok()?
            } else {
                imaginary
            };
            Some((real, imaginary))
        })
    }

    pub(crate) fn asin(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let square = value.mul(value)?;
            let one = Complex::from_real(&Real::one(digits_for_bits(value.bits)).ok()?)?;
            let one_minus = one.sub(&square)?;
            let root = wrap_complex(one_minus.sqrt()?)?;
            let rotated = Complex {
                real: value.imaginary.neg().ok()?,
                imaginary: value.real.clone(),
                bits: value.bits,
                real_error: value.imaginary_error,
                imaginary_error: value.real_error,
            };
            let logarithm = rotated.add(&root)?.log()?;
            Some((logarithm.1, logarithm.0.neg().ok()?))
        })
    }

    pub(crate) fn acos(&self) -> Option<(Real, Real)> {
        let digits = digits_for_bits(self.bits);
        let half_pi = Real::pi(digits)
            .ok()?
            .div(&Real::from_i64(2, digits).ok()?)
            .ok()?;
        let (real, imaginary) = self.asin()?;
        Some((half_pi.sub(&real).ok()?, imaginary.neg().ok()?))
    }

    pub(crate) fn atan(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let one = Real::one(digits_for_bits(value.bits)).ok()?;
            let rotated = Complex {
                real: value.imaginary.neg().ok()?,
                imaginary: value.real.clone(),
                bits: value.bits,
                real_error: value.imaginary_error,
                imaginary_error: value.real_error,
            };
            let left = one.sub(&rotated.real).ok()?;
            let right = one.add(&rotated.real).ok()?;
            let left = Complex {
                real: left,
                imaginary: rotated.imaginary.neg().ok()?,
                bits: value.bits,
                real_error: value.real_error,
                imaginary_error: value.imaginary_error,
            };
            let right = Complex {
                real: right,
                imaginary: rotated.imaginary.clone(),
                bits: value.bits,
                real_error: value.real_error,
                imaginary_error: value.imaginary_error,
            };
            let difference = left.log()?;
            let other = right.log()?;
            let real = difference.0.sub(&other.0).ok()?;
            let imaginary = difference.1.sub(&other.1).ok()?;
            let two = Real::from_i64(2, digits_for_bits(value.bits)).ok()?;
            Some((imaginary.neg().ok()?.div(&two).ok()?, real.div(&two).ok()?))
        })
    }

    pub(crate) fn sinh(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let real = value
                .real
                .sinh()
                .ok()?
                .mul(&value.imaginary.cos().ok()?)
                .ok()?;
            let imaginary = value
                .real
                .cosh()
                .ok()?
                .mul(&value.imaginary.sin().ok()?)
                .ok()?;
            Some((real, imaginary))
        })
    }

    pub(crate) fn cosh(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let real = value
                .real
                .cosh()
                .ok()?
                .mul(&value.imaginary.cos().ok()?)
                .ok()?;
            let imaginary = value
                .real
                .sinh()
                .ok()?
                .mul(&value.imaginary.sin().ok()?)
                .ok()?;
            Some((real, imaginary))
        })
    }

    pub(crate) fn tanh(&self) -> Option<(Real, Real)> {
        let sine = wrap_complex(self.sinh()?)?;
        let cosine = wrap_complex(self.cosh()?)?;
        sine.div(&cosine)?.parts()
    }

    pub(crate) fn asinh(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let square = value.mul(value)?;
            let one = Complex::from_real(&one_like(value)?)?;
            let plus_one = square.add(&one)?;
            let root = wrap_complex(plus_one.sqrt()?)?;
            let sum = value.add(&root)?;
            sum.log()
        })
    }

    pub(crate) fn acosh(&self) -> Option<(Real, Real)> {
        if self.imaginary.is_zero() && self.real.to_f64() < -1.0 {
            let digits = digits_for_bits(self.bits);
            let magnitude = self.real.neg().ok()?;
            let square = magnitude.mul(&magnitude).ok()?;
            let root = square.sub(&Real::one(digits).ok()?).ok()?.sqrt().ok()?;
            let real = magnitude.add(&root).ok()?.ln().ok()?;
            let imaginary = Real::pi(digits).ok()?;
            return Some((real, imaginary));
        }
        self.unary(|value| {
            let square = value.mul(value)?;
            let one = Complex::from_real(&one_like(value)?)?;
            let minus_one = square.sub(&one)?;
            let root = wrap_complex(minus_one.sqrt()?)?;
            let sum = value.add(&root)?;
            sum.log()
        })
    }

    #[allow(dead_code)]
    pub(crate) fn atanh(&self) -> Option<(Real, Real)> {
        self.unary(|value| {
            let one = one_like(value)?;
            let plus = one.add(&value.real).ok()?;
            let minus = one.sub(&value.real).ok()?;
            let left = Complex {
                real: plus,
                imaginary: value.imaginary.clone(),
                bits: value.bits,
                real_error: value.real_error,
                imaginary_error: value.imaginary_error,
            }
            .log()?;
            let right = Complex {
                real: minus,
                imaginary: value.imaginary.neg().ok()?,
                bits: value.bits,
                real_error: value.real_error,
                imaginary_error: value.imaginary_error,
            }
            .log()?;
            let two = Real::from_i64(2, digits_for_bits(value.bits)).ok()?;
            Some((
                left.0.sub(&right.0).ok()?.div(&two).ok()?,
                left.1.sub(&right.1).ok()?.div(&two).ok()?,
            ))
        })
    }

    pub(crate) fn abs(&self) -> Option<Real> {
        let square = self
            .real
            .mul(&self.real)
            .ok()?
            .add(&self.imaginary.mul(&self.imaginary).ok()?)
            .ok()?;
        let mut out = square.sqrt().ok()?;
        out.error = self.real_error.hypot(self.imaginary_error)
            + rounding(self.bits, out.to_f64().abs().max(1.0));
        Some(out)
    }

    pub(crate) fn from_real(value: &Real) -> Option<Self> {
        let digits = digits_for_bits(value.bits);
        let zero = Real::zero(digits).ok()?;
        Self::from_parts(value, &zero)
    }

    pub(crate) fn add(&self, other: &Complex) -> Option<Complex> {
        let real = self.real.add(&other.real).ok()?;
        let imaginary = self.imaginary.add(&other.imaginary).ok()?;
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = self.bits.max(other.bits);
        Some(value)
    }

    pub(crate) fn sub(&self, other: &Complex) -> Option<Complex> {
        let real = self.real.sub(&other.real).ok()?;
        let imaginary = self.imaginary.sub(&other.imaginary).ok()?;
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = self.bits.max(other.bits);
        Some(value)
    }

    pub(crate) fn mul(&self, other: &Complex) -> Option<Complex> {
        let real = self
            .real
            .mul(&other.real)
            .ok()?
            .sub(&self.imaginary.mul(&other.imaginary).ok()?)
            .ok()?;
        let imaginary = self
            .real
            .mul(&other.imaginary)
            .ok()?
            .add(&self.imaginary.mul(&other.real).ok()?)
            .ok()?;
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = self.bits.max(other.bits);
        Some(value)
    }

    pub(crate) fn div(&self, other: &Complex) -> Option<Complex> {
        let c_abs = other.real.abs().ok()?;
        let d_abs = other.imaginary.abs().ok()?;
        if other.real.is_zero() && other.imaginary.is_zero() {
            return None;
        }
        let (real, imaginary) = if c_abs.cmp(&d_abs) != core::cmp::Ordering::Less {
            let ratio = other.imaginary.div(&other.real).ok()?;
            let denominator = other.real.add(&other.imaginary.mul(&ratio).ok()?).ok()?;
            if denominator.is_zero() {
                return None;
            }
            let real_numerator = self.real.add(&self.imaginary.mul(&ratio).ok()?).ok()?;
            let imaginary_numerator = self.imaginary.sub(&self.real.mul(&ratio).ok()?).ok()?;
            (
                real_numerator.div(&denominator).ok()?,
                imaginary_numerator.div(&denominator).ok()?,
            )
        } else {
            let ratio = other.real.div(&other.imaginary).ok()?;
            let denominator = other.real.mul(&ratio).ok()?.add(&other.imaginary).ok()?;
            if denominator.is_zero() {
                return None;
            }
            let real_numerator = self.real.mul(&ratio).ok()?.add(&self.imaginary).ok()?;
            let imaginary_numerator = self.imaginary.mul(&ratio).ok()?.sub(&self.real).ok()?;
            (
                real_numerator.div(&denominator).ok()?,
                imaginary_numerator.div(&denominator).ok()?,
            )
        };
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = self.bits.max(other.bits);
        Some(value)
    }

    pub(crate) fn neg(&self) -> Option<Complex> {
        let real = self.real.neg().ok()?;
        let imaginary = self.imaginary.neg().ok()?;
        let mut value = Complex::from_parts(&real, &imaginary)?;
        value.bits = self.bits;
        Some(value)
    }

    pub(crate) fn parts(&self) -> Option<(Real, Real)> {
        Some((self.real.clone(), self.imaginary.clone()))
    }

    pub(crate) fn erf(&self) -> Option<(Real, Real)> {
        self.erf_series(true)
    }

    pub(crate) fn erfi(&self) -> Option<(Real, Real)> {
        self.erf_series(false)
    }

    pub(crate) fn erfc(&self) -> Option<(Real, Real)> {
        let (real, imaginary) = self.erf()?;
        let digits = digits_for_bits(self.bits);
        let one = Real::one(digits).ok()?;
        Some((one.sub(&real).ok()?, imaginary.neg().ok()?))
    }

    fn erf_series(&self, alternating: bool) -> Option<(Real, Real)> {
        let digits = digits_for_bits(self.bits);
        let square = self.mul(self)?;
        let mut term = self.clone();
        let mut sum = self.clone();
        let threshold = 10f64.powi(-(digits as i32 + 5));
        for n in 1..1024i64 {
            let denominator = Real::from_i64(n * (2 * n + 1), digits).ok()?;
            let numerator = Real::from_i64(2 * n - 1, digits).ok()?;
            let scale = denominator.div(&numerator).ok()?;
            term = term.mul(&square)?.scale_inv(&scale)?;
            if alternating && n % 2 == 1 {
                sum = sum.sub(&term)?;
            } else {
                sum = sum.add(&term)?;
            }
            if n > 1 && term.abs_value() <= threshold {
                break;
            }
        }
        let two = Real::from_i64(2, digits).ok()?;
        let sqrt_pi = Real::pi(digits).ok()?.sqrt().ok()?;
        let factor = two.div(&sqrt_pi).ok()?;
        sum = sum.scale(&factor)?;
        sum.parts()
    }

    fn scale(&self, factor: &Real) -> Option<Complex> {
        let real = self.real.mul(factor).ok()?;
        let imaginary = self.imaginary.mul(factor).ok()?;
        Complex::from_parts(&real, &imaginary)
    }

    fn scale_inv(&self, factor: &Real) -> Option<Complex> {
        let real = self.real.div(factor).ok()?;
        let imaginary = self.imaginary.div(factor).ok()?;
        Complex::from_parts(&real, &imaginary)
    }

    pub(crate) fn gamma(&self, digits: u32) -> Option<Complex> {
        self.gamma_value(digits)
    }

    pub(crate) fn lgamma(&self, digits: u32) -> Option<Complex> {
        let gamma = self.gamma_value(digits)?;
        Some(set_gamma_error(wrap_complex(gamma.log()?)?, digits))
    }

    fn gamma_value(&self, digits: u32) -> Option<Complex> {
        let digits = digits.max(16);
        let one = Complex::from_real(&Real::one(digits).ok()?)?;
        let w = self.sub(&one)?;
        if self.real.to_f64() < 0.5 {
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

    pub(crate) fn pow(&self, exponent: &Complex) -> Option<(Real, Real)> {
        if self.real.is_zero() && self.imaginary.is_zero() {
            if exponent.real.sign() > 0 {
                let digits = digits_for_bits(self.bits);
                return Some((Real::zero(digits).ok()?, Real::zero(digits).ok()?));
            }
            return None;
        }
        let logarithm = self.log()?;
        let product = Complex {
            real: logarithm.0,
            imaginary: logarithm.1,
            bits: self.bits,
            real_error: self.real_error + self.imaginary_error,
            imaginary_error: self.real_error + self.imaginary_error,
        };
        exponent.mul(&product)?.exp()
    }

    fn unary(&self, operation: impl Fn(&Complex) -> Option<(Real, Real)>) -> Option<(Real, Real)> {
        let (real, imaginary) = operation(self)?;
        if !real.is_finite() || !imaginary.is_finite() {
            return None;
        }
        let mut real = real;
        let mut imaginary = imaginary;
        let factor = derivative_scale(self, &real, &imaginary);
        apply_error(
            &mut real,
            &mut imaginary,
            self.bits,
            factor,
            self.real_error + self.imaginary_error,
        );
        Some((real, imaginary))
    }
}

fn digits_for_bits(bits: u32) -> u32 {
    ((bits as f64 * core::f64::consts::LOG10_2).ceil() as u32 + 4).max(16)
}

fn one_like(value: &Complex) -> Option<Real> {
    Real::one(digits_for_bits(value.bits)).ok()
}

fn atan2(y: &Real, x: &Real, digits: u32) -> Option<Real> {
    let zero = Real::zero(digits).ok()?;
    let pi = Real::pi(digits).ok()?;
    let half_pi = pi.div(&Real::from_i64(2, digits).ok()?).ok()?;
    if x.sign() > 0 {
        y.div(x).ok()?.atan().ok()
    } else if x.sign() < 0 {
        let base = y.div(x).ok()?.atan().ok()?;
        if y.sign() >= 0 {
            base.add(&pi).ok()
        } else {
            base.sub(&pi).ok()
        }
    } else if y.sign() > 0 {
        Some(half_pi)
    } else if y.sign() < 0 {
        half_pi.neg().ok()
    } else {
        Some(zero)
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
    let exponential = difference.exp().ok()?;
    let magnitude = power.mul(&exponential).ok()?.div(&factorial).ok()?;
    if k % 2 == 1 {
        Some(magnitude)
    } else {
        magnitude.neg().ok()
    }
}

impl Complex {
    fn abs_value(&self) -> f64 {
        (self.real.to_f64().powi(2) + self.imaginary.to_f64().powi(2)).sqrt()
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

fn derivative_scale(input: &Complex, real: &Real, imaginary: &Real) -> f64 {
    let input_scale = input.abs_value().max(1.0);
    let output_scale = (real.to_f64().abs() + imaginary.to_f64().abs()).max(1.0);
    (output_scale / input_scale).max(1.0) * 4.0
}

fn rounding(bits: u32, scale: f64) -> f64 {
    scale * 2f64.powi(-(bits as i32 - 1).max(1))
}
