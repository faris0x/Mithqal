//! Exact numbers and GMP bindings.
//!
//! The current version uses GMP for integers and rationals. The FFI layer is private
//! and all public numeric values enforce GMP initialization and cleanup.

mod ffi;
mod ffi_mpc;
mod ffi_mpfr;
mod int;
mod mpc;
mod rat;
mod real;

pub use int::Int;
pub use rat::Rat;
pub use real::Real;

pub(crate) use ffi_mpfr::{MPFR_RNDD, MPFR_RNDU};
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
