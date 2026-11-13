//! Exact numbers and GMP bindings.
//!
//! Version 0.0.1 uses GMP for integers and rationals. The FFI layer is private
//! and all public numeric values enforce GMP initialization and cleanup.

mod ffi;
mod int;
mod rat;

pub use int::Int;
pub use rat::Rat;

use core::fmt;

/// Errors produced by exact numeric operations.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NumError {
    /// A decimal or rational literal could not be parsed.
    Parse,
    /// A divisor was zero.
    DivisionByZero,
    /// A value did not fit the requested Rust integer type.
    Overflow,
}

impl fmt::Display for NumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumError::Parse => write!(f, "invalid numeric literal"),
            NumError::DivisionByZero => write!(f, "division by zero"),
            NumError::Overflow => write!(f, "numeric value does not fit"),
        }
    }
}

impl std::error::Error for NumError {}
