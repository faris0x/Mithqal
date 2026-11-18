//! Exact and approximate numbers backed by Malachite.
//!
//! Integers, rationals, reals, and complex values are pure Rust values. No C
//! toolchain or foreign runtime is required.

mod int;
mod mpc;
mod rat;
mod real;

pub use int::Int;
pub use rat::Rat;
pub use real::Real;

pub(crate) const ROUND_DOWN: core::ffi::c_int = 3;
pub(crate) const ROUND_UP: core::ffi::c_int = 2;
pub(crate) use mpc::Complex as ComplexMpc;

use core::fmt;

/// Errors produced by exact numeric operations.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NumError {
    /// A decimal or rational literal could not be parsed.
    Parse,
    /// A divisor was zero.
    DivisionByZero,
    /// A value did not fit the requested Rust integer type.
    Overflow,
    /// A real operation left its domain.
    Domain,
}

impl fmt::Display for NumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumError::Parse => write!(f, "invalid numeric literal"),
            NumError::DivisionByZero => write!(f, "division by zero"),
            NumError::Overflow => write!(f, "numeric value does not fit"),
            NumError::Domain => write!(f, "value is outside the function domain"),
        }
    }
}

impl std::error::Error for NumError {}
