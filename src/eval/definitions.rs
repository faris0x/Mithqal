//! Own values and down values.

use std::collections::HashMap;

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};

/// An own value attached to a symbol.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OwnValue {
    pub(crate) value: ExprId,
}

/// A down value rule.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rule {
    pub(crate) lhs: ExprId,
    pub(crate) rhs: ExprId,
    pub(crate) delayed: bool,
    pub(crate) specificity: u32,
    pub(crate) order: u64,
}

/// Session definitions.
#[derive(Default)]
pub(crate) struct Definitions {
    own: HashMap<SymbolId, OwnValue>,
    down: HashMap<SymbolId, Vec<Rule>>,
    next_order: u64,
    version: u64,
}

impl Definitions {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn own_value(&self, symbol: SymbolId) -> Option<OwnValue> {
        self.own.get(&symbol).copied()
    }

    pub(crate) fn rules(&self, symbol: SymbolId) -> &[Rule] {
        self.down.get(&symbol).map(Vec::as_slice).unwrap_or(&[])
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn roots(&self, roots: &mut Vec<ExprId>) {
        for own in self.own.values() {
            roots.push(own.value);
        }
        for rules in self.down.values() {
            for rule in rules {
                roots.push(rule.lhs);
                roots.push(rule.rhs);
            }
        }
    }

    pub(crate) fn define(
        &mut self,
        arena: &Arena,
        lhs: ExprId,
        rhs: ExprId,
        delayed: bool,
    ) -> Result<(), Diagnostic> {
        match arena.node(lhs) {
            Node::Symbol(symbol) => {
                self.check_protected(arena, *symbol)?;
                self.own.insert(*symbol, OwnValue { value: rhs });
                self.version = self.version.wrapping_add(1);
                Ok(())
            }
            Node::Call { head, .. } => {
                let symbol = arena.symbol_id(*head).ok_or_else(|| {
                    Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a definition head must be a symbol",
                    )
                })?;
                self.check_protected(arena, symbol)?;
                let rule = Rule {
                    lhs,
                    rhs,
                    delayed,
                    specificity: specificity(arena, lhs),
                    order: self.next_order,
                };
                self.next_order += 1;
                let rules = self.down.entry(symbol).or_default();
                rules.push(rule);
                rules.sort_by(|left, right| {
                    right
                        .specificity
                        .cmp(&left.specificity)
                        .then(left.order.cmp(&right.order))
                });
                self.version = self.version.wrapping_add(1);
                Ok(())
            }
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "a definition target must be a symbol or a call",
            )),
        }
    }

    fn check_protected(&self, arena: &Arena, symbol: SymbolId) -> Result<(), Diagnostic> {
        if arena.symbol_attributes(symbol).has(Attributes::PROTECTED) {
            Err(Diagnostic::eval(
                ErrorCode::EvalProtected,
                format!("symbol {} is protected", arena.symbol_name(symbol)),
            ))
        } else {
            Ok(())
        }
    }
}

fn specificity(arena: &Arena, pattern: ExprId) -> u32 {
    match arena.node(pattern) {
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => 1000,
        Node::Symbol(_) => 1000,
        Node::Call { head, args } => {
            let name = arena
                .symbol_id(*head)
                .map(|symbol| arena.symbol_name(symbol));
            match name {
                Some("blank") => 1,
                Some("blankSequence") | Some("blankNullSequence") => 0,
                Some("pattern") => {
                    let inner = args.get(1).copied().unwrap_or(pattern);
                    20 + specificity(arena, inner)
                }
                Some("condition") => {
                    let inner = args.first().copied().unwrap_or(pattern);
                    2 + specificity(arena, inner)
                }
                _ => {
                    let child: u32 = args.iter().map(|arg| specificity(arena, *arg)).sum();
                    100 + child
                }
            }
        }
    }
}
