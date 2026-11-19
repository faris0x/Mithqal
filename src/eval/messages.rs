//! Non-fatal messages and message controls.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode, Message};

use super::*;

impl Evaluator<'_> {
    pub(super) fn eval_message(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "message expects a name",
            ));
        }
        let name = self.message_name(args[0]);
        let mut parts = Vec::with_capacity(args.len().saturating_sub(1));
        for argument in &args[1..] {
            parts.push(crate::print::infix(self.arena, *argument));
        }
        let text = if parts.is_empty() {
            name.clone()
        } else {
            format!("{name}: {}", parts.join(" "))
        };
        self.arena
            .push_message(Message::new("TGN-MSG-0001", name, text));
        self.definitions.bump_version();
        Ok(self.heads.null)
    }

    pub(super) fn eval_check(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "check expects an expression and a fallback",
            ));
        }
        let before = self.arena.message_count();
        let result = self.eval(args[0])?;
        if self.arena.message_count() > before {
            self.eval(args[1])
        } else {
            Ok(result)
        }
    }

    pub(super) fn eval_quiet(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "quiet expects one expression",
            ));
        }
        self.arena.suppress_messages();
        let result = self.eval(args[0]);
        self.arena.unsuppress_messages();
        result
    }

    pub(super) fn eval_message_control(
        &mut self,
        enable: bool,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        for argument in args {
            let name = self.message_name(*argument);
            if enable {
                self.arena.enable_message(&name);
            } else {
                self.arena.disable_message(&name);
            }
        }
        self.definitions.bump_version();
        Ok(self.heads.null)
    }

    fn message_name(&self, expression: ExprId) -> String {
        match self.arena.node(expression) {
            Node::Symbol(symbol) => self.arena.symbol_display_name(*symbol),
            Node::Call { head, args }
                if self
                    .arena
                    .symbol_id(*head)
                    .map(|symbol| self.arena.symbol_name(symbol))
                    == Some("messageName")
                    && args.len() == 2 =>
            {
                format!(
                    "{}::{}",
                    crate::print::infix(self.arena, args[0]),
                    crate::print::infix(self.arena, args[1])
                )
            }
            _ => crate::print::infix(self.arena, expression),
        }
    }
}
