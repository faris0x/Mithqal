//! Fixed point evaluator.
//!
//! Evaluates canonical expressions through exact arithmetic, listable
//! threading, comparisons, logic, definitions, closures, control flow,
//! numeric boundaries, and limit and series dispatch.

mod arithmetic;
mod control;
mod definitions;
mod dispatch;
mod evaluator;
mod heads;
mod limits;
mod numeric;
mod pattern;
mod support;

use core::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
use crate::kernel::Heads;
use crate::num::{ComplexMpc, Int, MPFR_RNDD, MPFR_RNDU, Rat, Real};

use definitions::{Definitions, Rule};
pub(crate) use evaluator::{Evaluator, fold_numeric_call};
pub(crate) use heads::*;
use pattern::{Bindings, match_pattern, substitute};
use support::*;

pub(crate) use definitions::Definitions as SessionDefinitions;

/// Evaluate an expression to a normal form.
pub fn evaluate(arena: &mut Arena, id: ExprId) -> Result<ExprId, Diagnostic> {
    let mut definitions = Definitions::new();
    evaluate_with_definitions(arena, &mut definitions, id, 1_000_000, 2048)
}

/// Evaluate an expression with explicit budget and depth limits.
pub fn evaluate_with_limits(
    arena: &mut Arena,
    id: ExprId,
    max_steps: u64,
    max_depth: u32,
) -> Result<ExprId, Diagnostic> {
    let mut definitions = Definitions::new();
    evaluate_with_definitions(arena, &mut definitions, id, max_steps, max_depth)
}

pub(crate) fn evaluate_with_definitions(
    arena: &mut Arena,
    definitions: &mut Definitions,
    id: ExprId,
    max_steps: u64,
    max_depth: u32,
) -> Result<ExprId, Diagnostic> {
    if max_steps >= 16
        && max_depth >= 4
        && let Some(value) = fold_numeric_call(arena, id)
    {
        return Ok(value);
    }
    let mut evaluator = Evaluator::new(arena, definitions);
    evaluator.max_steps = max_steps.max(1);
    evaluator.max_depth = max_depth.max(1);
    evaluator.eval(id)
}

struct SmallArgs {
    inline: [ExprId; 6],
    len: usize,
    heap: Vec<ExprId>,
}

impl SmallArgs {
    fn new() -> Self {
        SmallArgs {
            inline: [ExprId::default(); 6],
            len: 0,
            heap: Vec::new(),
        }
    }

    fn from_slice(args: &[ExprId]) -> Self {
        if args.len() <= 6 {
            let mut inline = [ExprId::default(); 6];
            inline[..args.len()].copy_from_slice(args);
            SmallArgs {
                inline,
                len: args.len(),
                heap: Vec::new(),
            }
        } else {
            SmallArgs {
                inline: [ExprId::default(); 6],
                len: 0,
                heap: args.to_vec(),
            }
        }
    }

    fn push(&mut self, value: ExprId) {
        if self.heap.is_empty() && self.len < 6 {
            self.inline[self.len] = value;
            self.len += 1;
        } else {
            if self.heap.is_empty() {
                self.heap.extend_from_slice(&self.inline[..self.len]);
            }
            self.heap.push(value);
        }
    }

    fn as_slice(&self) -> &[ExprId] {
        if self.heap.is_empty() {
            &self.inline[..self.len]
        } else {
            &self.heap
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Sign {
    Negative,
    Zero,
    Positive,
    NonNegative,
    NonPositive,
}

#[derive(Copy, Clone)]
struct Assumption {
    symbol: SymbolId,
    sign: Sign,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitSide {
    Both,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitTarget {
    Finite(ExprId),
    PositiveInfinity,
    NegativeInfinity,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum LimitValue {
    Finite(ExprId),
    PositiveInfinity,
    NegativeInfinity,
}

impl LimitValue {
    fn sign(self) -> Option<Sign> {
        match self {
            LimitValue::PositiveInfinity => Some(Sign::Positive),
            LimitValue::NegativeInfinity => Some(Sign::Negative),
            LimitValue::Finite(_) => None,
        }
    }

    fn with_sign(self, sign: Sign) -> LimitValue {
        if !matches!(
            self,
            LimitValue::PositiveInfinity | LimitValue::NegativeInfinity
        ) {
            return self;
        }
        let current = match self {
            LimitValue::NegativeInfinity => Sign::Negative,
            _ => Sign::Positive,
        };
        match sign_product(current, sign) {
            Some(Sign::Negative) => LimitValue::NegativeInfinity,
            _ => LimitValue::PositiveInfinity,
        }
    }
}
