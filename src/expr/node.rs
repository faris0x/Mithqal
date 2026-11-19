//! Expression nodes, symbols, and attributes.

#[allow(unused_imports)]
use crate::prelude::*;

use core::fmt;

use crate::num::{Int, Rat, Real};

/// A generational handle to an expression in the arena.
#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ExprId(u64);

impl ExprId {
    const INDEX_BITS: u32 = 32;

    pub(crate) fn from_parts(index: u32, generation: u32) -> Self {
        ExprId(((generation as u64) << Self::INDEX_BITS) | index as u64)
    }

    pub(crate) fn index(self) -> usize {
        (self.0 & ((1u64 << Self::INDEX_BITS) - 1)) as usize
    }

    pub(crate) fn generation(self) -> u32 {
        (self.0 >> Self::INDEX_BITS) as u32
    }
}

impl fmt::Debug for ExprId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ExprId({}, {})", self.index(), self.generation())
    }
}

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
    /// A one argument pattern can match a bare value.
    pub const ONE_IDENTITY: u16 = 1 << 8;
    /// Sequence arguments are held at this head.
    pub const SEQUENCE_HOLD: u16 = 1 << 9;

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

    /// Remove a flag.
    pub const fn without(self, flag: u16) -> Self {
        Attributes(self.0 & !flag)
    }

    /// Flat and orderless, the common case for arithmetic heads.
    pub const fn arithmetic() -> Self {
        Attributes::new().with(Self::FLAT).with(Self::ORDERLESS)
    }
}

/// One key and value pair in an association.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AssociationEntry {
    /// The association key.
    pub key: ExprId,
    /// The value associated with the key.
    pub value: ExprId,
}

/// A node in the expression arena.
#[derive(Debug)]
pub enum Node {
    /// An exact integer.
    Int(Int),
    /// An exact rational with a denominator other than one.
    Rat(Rat),
    /// An approximate real with a significance estimate.
    Real(Real),
    /// A string value.
    String(Box<str>),
    /// An immutable byte array.
    Bytes(Box<[u8]>),
    /// A symbol.
    Symbol(SymbolId),
    /// A call with a head and arguments.
    Call {
        /// The head expression.
        head: ExprId,
        /// The argument expressions.
        args: Box<[ExprId]>,
    },
    /// An ordered key and value mapping with unique canonical keys.
    Association(Box<[AssociationEntry]>),
}
