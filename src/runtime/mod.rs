//! Typed instruction compiler and virtual machine.
//!
//! The evaluator owns the rewrite semantics. The virtual machine executes
//! compiled control flow and delegates symbolic calls to the evaluator.

mod compiler;
mod instr;
mod vm;

pub(crate) use compiler::compile;
pub(crate) use vm::execute;
