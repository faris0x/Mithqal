//! Expression nodes, symbols, and attributes.

use crate::num::{Int, Rat};

/// A handle to an expression in the arena.
pub type ExprId = u32;

/// A handle to a symbol table entry.
pub type SymbolId = u32;

/// Structural attributes of a symbol.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Attributes(u16);

impl Attributes {
    /// Nested calls with the same head are spliced.
    pub const FLAT: u16 = 1 << 0;
    /// Arguments are sorted into canonical order.
    pub const ORDERLESS: u16 = 1 << 1;
    /// All arguments are held.
    pub const HOLD_ALL: u16 = 1 << 2;
    /// The first argument is held.
    pub const HOLD_FIRST: u16 = 1 << 3;
    /// All arguments after the first are held.
    pub const HOLD_REST: u16 = 1 << 4;
    /// The head threads over list arguments.
    pub const LISTABLE: u16 = 1 << 5;
    /// The symbol cannot be redefined.
    pub const PROTECTED: u16 = 1 << 6;
    /// The symbol is a constant for numeric evaluation.
    pub const CONSTANT: u16 = 1 << 7;

    /// An attribute set with no flags.
    pub const fn new() -> Self {
        Attributes(0)
    }

    /// Add a flag.
    pub const fn with(self, flag: u16) -> Self {
        Attributes(self.0 | flag)
    }

    /// Test a flag.
    pub const fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Flat and orderless, the common case for arithmetic heads.
    pub const fn arithmetic() -> Self {
        Attributes::new().with(Self::FLAT).with(Self::ORDERLESS)
    }
}

/// A node in the expression arena.
#[derive(Debug)]
pub enum Node {
    /// An exact integer.
    Int(Int),
    /// An exact rational with a denominator other than one.
    Rat(Rat),
    /// A symbol.
    Symbol(SymbolId),
    /// A call with a head and arguments.
    Call {
        /// The head expression.
        head: ExprId,
        /// The argument expressions.
        args: Box<[ExprId]>,
    },
}
