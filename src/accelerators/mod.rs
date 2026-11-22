//! Native mathematical accelerators.
//!
//! These modules implement performance-critical mathematics behind narrow
//! interfaces. Public policy belongs in the Mithqal standard library. The
//! re-exports let the number field accelerator reuse the exact lattice and
//! polynomial kernels without importing the whole algebra layer.

#[allow(unused_imports)]
use crate::prelude::*;

pub(crate) use crate::algebra::{dense, intmatrix, lattice, poly_z, ratmatrix};

pub(crate) mod analytic;
#[cfg(feature = "numberfields")]
pub(crate) mod numberfield;
