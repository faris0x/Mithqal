//! Exact number field construction and arithmetic.
//!
//! The current layer covers defining polynomial validation, exact signatures,
//! power basis arithmetic, norms, traces, characteristic polynomials, minimal
//! polynomials, and integrality tests. Integral bases and ideals come later.

mod classgroup;
mod decomposition;
mod element;
mod field;
mod ideal;
mod integral_basis;
mod order;
mod sturm;
mod units;

#[allow(unused_imports)]
pub(crate) use classgroup::{ClassGroup, class_group};
pub(crate) use field::NumberField;
pub(crate) use integral_basis::maximal_order;
pub(crate) use units::unit_group;
