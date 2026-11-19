//! Expression arena, symbols, attributes, and canonical order.
//!
//! The term model is described in `docs/semantics.md`.
//!
//! This module is an unstable internal surface. Library users should inspect
//! expressions through [`crate::Session`].

#[allow(unused_imports)]
use crate::prelude::*;

mod arena;
mod node;
mod order;

pub use arena::Arena;
pub use node::{AssociationEntry, Attributes, ExprId, Node, SymbolId};
