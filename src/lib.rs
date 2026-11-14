//! A symbolic math engine.
//!
//! The current version defines the expression kernel, source reader, evaluator,
//! virtual machine, garbage collector, and public library API.
//!
//! The crate name `cas` is a placeholder.

#![warn(missing_docs)]
#![forbid(unsafe_op_in_unsafe_fn)]

pub mod api;
pub mod builtins;
pub mod config;
pub mod diag;
pub mod eval;
pub mod expr;
mod hash;
pub mod num;
pub mod print;
pub mod runtime;
pub mod syntax;

pub use api::{ExprView, Session};
pub use config::Config;
pub use diag::{Diagnostic, ErrorCode, ErrorKind, Frame, Span};
pub use expr::ExprId;
pub use num::{Int, NumError, Rat};
