//! Typed instruction stream.

use crate::expr::{ExprId, SymbolId};

/// A virtual machine instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instr {
    /// Push a constant expression.
    Const(u16),
    /// Evaluate the value on the top of the stack.
    Eval,
    /// Pop arguments and call a symbol head.
    CallSymbol(SymbolId, u16),
    /// Jump to an instruction index.
    Jump(u32),
    /// Pop a condition. Jump when it is `false`, otherwise push it back.
    JumpIfFalse(u32),
    /// Pop a condition. Jump when it is not `true`.
    JumpIfNotBoolean(u32),
    /// Return the value on the top of the stack.
    Return,
    /// A garbage collection safepoint.
    Safepoint,
    /// Add to the step count.
    Budget(u32),
}

/// A compiled instruction stream and its constant pool.
#[derive(Clone, Debug, Default)]
pub struct Code {
    /// The instructions.
    pub instrs: Vec<Instr>,
    /// The constant pool.
    pub constants: Vec<ExprId>,
    /// An optional name for diagnostics.
    pub name: Option<String>,
}
