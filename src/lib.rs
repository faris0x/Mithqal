//! A symbolic math engine.
//!
//! The current version defines the expression kernel, source reader, evaluator,
//! virtual machine, garbage collector, public library API, and the 0.0.2 data
//! and control layer.
//!
//! The library builds with `core` and `alloc` when the `std` feature is
//! disabled.
//!
//! The project is named Mithqal.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![forbid(unsafe_op_in_unsafe_fn)]

#[macro_use]
extern crate alloc;

mod algebra;
pub mod api;
pub mod builtins;
mod calculus;
pub mod catalog;
#[cfg(feature = "cli")]
pub mod cli;
mod collections;
pub mod config;
pub mod diag;
pub mod eval;
#[doc(hidden)]
pub mod expr;
mod hash;
#[cfg(feature = "std")]
pub mod io;
pub mod ir;
mod kernel;
mod linalg;
mod math;
pub mod num;
mod numbertheory;
mod prelude;
pub mod print;
pub mod runtime;
mod stdlib;
pub mod syntax;

pub use api::{ExprView, Session};
pub use config::Config;
pub use diag::{Diagnostic, ErrorCode, ErrorKind, Frame, Span};
pub use expr::ExprId;
pub use num::{Int, NumError, Rat, Real};
