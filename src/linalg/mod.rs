//! Exact rational linear algebra.

use crate::expr::{Arena, ExprId, Node};
use crate::num::{Int, Rat};

/// A dense matrix of exact rationals in row major order.
mod decompose;
mod matrix;
mod symbolic;

pub(crate) use decompose::*;
pub(crate) use matrix::*;
pub(crate) use symbolic::*;
