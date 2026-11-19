//! Compiler from expressions to typed instructions.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node};

use super::instr::{Code, Instr};

/// Compile an expression into an instruction stream.
pub fn compile(arena: &mut Arena, expression: ExprId) -> Result<Code, Diagnostic> {
    let mut compiler = Compiler {
        arena,
        code: Code::default(),
    };
    compiler.compile_value(expression)?;
    compiler.emit(Instr::Return)?;
    Ok(compiler.code)
}

struct Compiler<'a> {
    arena: &'a mut Arena,
    code: Code,
}

impl Compiler<'_> {
    fn compile_value(&mut self, expression: ExprId) -> Result<(), Diagnostic> {
        let (head, args) = match self.arena.node(expression) {
            Node::Int(_) | Node::Rat(_) | Node::Real(_) => {
                let constant = self.constant(expression)?;
                self.emit(Instr::Const(constant))?;
                return Ok(());
            }
            Node::Symbol(_) => {
                let constant = self.constant(expression)?;
                self.emit(Instr::Const(constant))?;
                self.emit(Instr::Eval)?;
                return Ok(());
            }
            Node::Call { head, args } => (*head, args.to_vec()),
        };
        let Some(symbol) = self.arena.symbol_id(head) else {
            return self.fallback(expression);
        };
        let name = self.arena.symbol_name(symbol).to_string();
        if name == "if" && args.len() == 3 {
            return self.compile_if(expression, &args);
        }
        let attrs = self.arena.head_attributes(head);
        if attrs.has(Attributes::HOLD_ALL)
            || attrs.has(Attributes::HOLD_FIRST)
            || attrs.has(Attributes::HOLD_REST)
        {
            return self.fallback(expression);
        }
        for arg in &args {
            self.compile_value(*arg)?;
        }
        self.emit(Instr::Safepoint)?;
        self.emit(Instr::Budget(1))?;
        self.emit(Instr::CallSymbol(symbol, args.len() as u16))?;
        Ok(())
    }

    fn compile_if(&mut self, expression: ExprId, args: &[ExprId]) -> Result<(), Diagnostic> {
        self.compile_value(args[0])?;
        let jump_false = self.emit(Instr::JumpIfFalse(u32::MAX))?;
        let jump_fallback = self.emit(Instr::JumpIfNotBoolean(u32::MAX))?;
        self.compile_value(args[1])?;
        let jump_end = self.emit(Instr::Jump(u32::MAX))?;
        let else_start = self.code.instrs.len() as u32;
        self.patch(jump_false, else_start);
        self.compile_value(args[2])?;
        let jump_end_two = self.emit(Instr::Jump(u32::MAX))?;
        let fallback_start = self.code.instrs.len() as u32;
        self.patch(jump_fallback, fallback_start);
        let constant = self.constant(expression)?;
        self.emit(Instr::Const(constant))?;
        self.emit(Instr::Eval)?;
        let end = self.code.instrs.len() as u32;
        self.patch(jump_end, end);
        self.patch(jump_end_two, end);
        Ok(())
    }

    fn fallback(&mut self, expression: ExprId) -> Result<(), Diagnostic> {
        let constant = self.constant(expression)?;
        self.emit(Instr::Const(constant))?;
        self.emit(Instr::Eval)?;
        Ok(())
    }

    fn constant(&mut self, expression: ExprId) -> Result<u16, Diagnostic> {
        if self.code.constants.len() > u16::MAX as usize {
            return Err(Diagnostic::eval(
                ErrorCode::InternalInvariant,
                "constant pool exhausted",
            ));
        }
        let index = self.code.constants.len() as u16;
        self.code.constants.push(expression);
        Ok(index)
    }

    fn emit(&mut self, instr: Instr) -> Result<usize, Diagnostic> {
        if self.code.instrs.len() > u32::MAX as usize {
            return Err(Diagnostic::eval(
                ErrorCode::InternalInvariant,
                "instruction stream exhausted",
            ));
        }
        let index = self.code.instrs.len();
        self.code.instrs.push(instr);
        Ok(index)
    }

    fn patch(&mut self, index: usize, target: u32) {
        match &mut self.code.instrs[index] {
            Instr::Jump(value) | Instr::JumpIfFalse(value) | Instr::JumpIfNotBoolean(value) => {
                *value = target
            }
            _ => {}
        }
    }
}
