//! Canonical and infix printers.

#[allow(unused_imports)]
use crate::prelude::*;

mod canonical;
mod infix;

pub use canonical::canonical;
pub use infix::infix;
