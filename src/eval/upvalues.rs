//! Up values and tagged definitions.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

impl Evaluator<'_> {
    pub(super) fn eval_up_definition(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let delayed = head == self.heads.up_set_delayed || head == self.heads.tag_set_delayed;
        let tagged = head == self.heads.tag_set || head == self.heads.tag_set_delayed;
        let (tag, lhs, rhs) = if tagged {
            if args.len() != 3 {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalArity,
                    "tagSet expects a tag, a left hand side, and a right hand side",
                ));
            }
            let Some(tag) = self.arena.symbol_id(args[0]) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "a tag must be a symbol",
                ));
            };
            (tag, args[1], args[2])
        } else {
            if args.len() != 2 {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalArity,
                    "upSet expects a left hand side and a right hand side",
                ));
            }
            let Node::Call { head: lhs_head, .. } = self.arena.node(args[0]) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "an up value left hand side must be a call",
                ));
            };
            let Some(tag) = self.arena.symbol_id(*lhs_head) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "an up value left hand side must have a symbol head",
                ));
            };
            (tag, args[0], args[1])
        };
        let rhs = if delayed { rhs } else { self.eval(rhs)? };
        self.definitions
            .define_up(self.arena, tag, lhs, rhs, delayed)?;
        Ok(if delayed { self.heads.null } else { rhs })
    }

    pub(super) fn apply_upvalues(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let mut tags = Vec::new();
        if let Some(symbol) = self.arena.symbol_id(head) {
            tags.push(symbol);
        }
        for argument in args {
            match self.arena.node(*argument) {
                Node::Symbol(symbol) => {
                    if !tags.contains(symbol) {
                        tags.push(*symbol);
                    }
                }
                Node::Call { head, .. } => {
                    if let Some(symbol) = self.arena.symbol_id(*head)
                        && !tags.contains(&symbol)
                    {
                        tags.push(symbol);
                    }
                }
                _ => {}
            }
        }
        let call = self.arena.intern_call(head, args);
        for tag in tags {
            for rule in self.definitions.up_rules(tag) {
                let mut bindings = Bindings::new();
                let mut conditions = Vec::new();
                if !match_pattern(self.arena, rule.lhs, call, &mut bindings, &mut conditions) {
                    continue;
                }
                let mut accepted = true;
                for condition in conditions {
                    let condition = substitute(self.arena, condition, &bindings);
                    if self.eval(condition)? != self.heads.true_value {
                        accepted = false;
                        break;
                    }
                }
                if !accepted {
                    continue;
                }
                let rhs = substitute(self.arena, rule.rhs, &bindings);
                if !rule.delayed {
                    return Ok(Some(rhs));
                }
                let value = self.eval(rhs)?;
                if let Some(value) = self.take_return() {
                    return Ok(Some(value));
                }
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
}
