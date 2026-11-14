//! Typed instruction virtual machine.

use crate::builtins;
use crate::diag::{Diagnostic, ErrorCode};
use crate::eval::{SessionDefinitions, evaluate_with_definitions};
use crate::expr::{Arena, ExprId};

use super::instr::{Code, Instr};

/// Execute a compiled instruction stream.
pub(crate) fn execute(
    arena: &mut Arena,
    definitions: &mut SessionDefinitions,
    code: &Code,
    max_steps: u64,
    max_depth: u32,
) -> Result<ExprId, Diagnostic> {
    let _name = code.name.as_deref();
    let true_symbol = arena.intern_symbol("true", builtins::attributes("true").unwrap_or_default());
    let false_symbol =
        arena.intern_symbol("false", builtins::attributes("false").unwrap_or_default());
    let true_value = arena.symbol_expr(true_symbol);
    let false_value = arena.symbol_expr(false_symbol);
    let mut machine = Machine {
        arena,
        definitions,
        stack: Vec::new(),
        steps: 0,
        max_steps,
        max_depth,
        true_value,
        false_value,
    };
    machine.run(code)
}

struct Machine<'a> {
    arena: &'a mut Arena,
    definitions: &'a mut SessionDefinitions,
    stack: Vec<ExprId>,
    steps: u64,
    max_steps: u64,
    max_depth: u32,
    true_value: ExprId,
    false_value: ExprId,
}

impl Machine<'_> {
    fn run(&mut self, code: &Code) -> Result<ExprId, Diagnostic> {
        let mut pc = 0usize;
        loop {
            self.steps += 1;
            if self.steps > self.max_steps {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalBudget,
                    "virtual machine budget exhausted",
                ));
            }
            let Some(instr) = code.instrs.get(pc).cloned() else {
                return Err(Diagnostic::eval(
                    ErrorCode::InternalInvariant,
                    "instruction pointer left the code",
                ));
            };
            pc += 1;
            match instr {
                Instr::Const(index) => {
                    let Some(value) = code.constants.get(index as usize) else {
                        return Err(Diagnostic::eval(
                            ErrorCode::InternalInvariant,
                            "constant index out of range",
                        ));
                    };
                    self.stack.push(*value);
                }
                Instr::Eval => {
                    let value = self.pop()?;
                    let value = evaluate_with_definitions(
                        self.arena,
                        self.definitions,
                        value,
                        self.max_steps,
                        self.max_depth,
                    )?;
                    self.stack.push(value);
                }
                Instr::CallSymbol(symbol, argc) => {
                    let args = self.pop_args(argc)?;
                    let head = self.arena.symbol_expr(symbol);
                    let call = self.arena.intern_call(head, &args);
                    let value = evaluate_with_definitions(
                        self.arena,
                        self.definitions,
                        call,
                        self.max_steps,
                        self.max_depth,
                    )?;
                    self.stack.push(value);
                }
                Instr::Jump(target) => {
                    pc = target as usize;
                }
                Instr::JumpIfFalse(target) => {
                    let condition = self.pop()?;
                    if condition == self.false_value {
                        pc = target as usize;
                    } else {
                        self.stack.push(condition);
                    }
                }
                Instr::JumpIfNotBoolean(target) => {
                    let condition = self.pop()?;
                    if condition != self.true_value {
                        pc = target as usize;
                    }
                }
                Instr::Return => {
                    return self.pop();
                }
                Instr::Safepoint => {}
                Instr::Budget(amount) => {
                    self.steps = self.steps.saturating_add(amount as u64);
                }
            }
        }
    }

    fn pop(&mut self) -> Result<ExprId, Diagnostic> {
        self.stack.pop().ok_or_else(|| {
            Diagnostic::eval(
                ErrorCode::InternalInvariant,
                "virtual machine stack underflow",
            )
        })
    }

    fn pop_args(&mut self, count: u16) -> Result<Vec<ExprId>, Diagnostic> {
        let count = count as usize;
        if self.stack.len() < count {
            return Err(Diagnostic::eval(
                ErrorCode::InternalInvariant,
                "virtual machine stack underflow in call",
            ));
        }
        let start = self.stack.len() - count;
        let args = self.stack.split_off(start);
        Ok(args)
    }
}
