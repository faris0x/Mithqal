//! Expression arena, symbols, attributes, and canonical order.
//!
//! The term model is described in `docs/semantics.md`.

mod arena;
mod node;
mod order;

pub use arena::Arena;
pub use node::{Attributes, ExprId, Node, SymbolId};
