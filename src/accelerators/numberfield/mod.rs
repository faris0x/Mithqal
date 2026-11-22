//! Native algebraic number theory accelerator.
//!
//! This module implements number field construction, maximal orders, ideals,
//! decomposition, class groups, units, saturation, ray class orders, and
//! conductor machinery. It is a performance accelerator, not public policy.
//! The Mithqal library owns the user-facing algebraic number theory surface.

mod classgroup;
mod conductor;
mod decomposition;
mod element;
mod embedding;
mod field;
mod ideal;
mod integral_basis;
mod order;
mod rayclass;
mod sturm;
mod units;

#[allow(unused_imports)]
pub(crate) use classgroup::{ClassGroup, class_group};
pub(crate) use field::NumberField;
pub(crate) use integral_basis::maximal_order;
pub(crate) use units::unit_group;
