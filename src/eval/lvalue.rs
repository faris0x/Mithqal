//! Part access, assignment, and mutation targets.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;
use crate::expr::AssociationEntry;

impl Evaluator<'_> {
    pub(super) fn apply_part(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() < 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "part expects an expression and at least one index",
            ));
        }
        let mut value = args[0];
        for index in &args[1..] {
            let Some(next) = self.part_one(value, *index) else {
                return Ok(self.arena.intern_call(self.heads.part, args));
            };
            value = next;
        }
        Ok(value)
    }

    fn part_one(&mut self, value: ExprId, index: ExprId) -> Option<ExprId> {
        if let Some((start, end)) = self.span_bounds(index) {
            return self.part_span(value, start, end);
        }
        match self.arena.node(value) {
            Node::Association(entries) => {
                let entries = entries.to_vec();
                for entry in entries {
                    if entry.key == index {
                        return Some(entry.value);
                    }
                }
                Some(self.heads.undefined)
            }
            Node::String(text) => {
                let index = self.index_value(index)?;
                let characters: Vec<char> = text.chars().collect();
                let position = index_position(index, characters.len())?;
                let character = characters.get(position)?;
                Some(self.arena.intern_string(&character.to_string()))
            }
            Node::Bytes(bytes) => {
                let index = self.index_value(index)?;
                let position = index_position(index, bytes.len())?;
                Some(
                    self.arena
                        .intern_int(Int::from_i64(i64::from(*bytes.get(position)?))),
                )
            }
            Node::Call { head, args } => {
                let index = self.index_value(index)?;
                if index == 0 {
                    return Some(*head);
                }
                let position = index_position(index, args.len())?;
                args.get(position).copied()
            }
            _ => None,
        }
    }

    fn span_bounds(&self, expression: ExprId) -> Option<(i64, i64)> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if *head != self.heads.span || args.len() != 2 {
            return None;
        }
        Some((self.index_value(args[0])?, self.index_value(args[1])?))
    }

    fn part_span(&mut self, value: ExprId, start: i64, end: i64) -> Option<ExprId> {
        match self.arena.node(value) {
            Node::String(text) => {
                let characters: Vec<char> = text.chars().collect();
                let (start, end) = span_positions(start, end, characters.len())?;
                let text: String = characters[start..=end].iter().collect();
                Some(self.arena.intern_string(&text))
            }
            Node::Bytes(bytes) => {
                let (start, end) = span_positions(start, end, bytes.len())?;
                let bytes = bytes[start..=end].to_vec();
                Some(self.arena.intern_bytes(&bytes))
            }
            Node::Call { head, args } => {
                let head = *head;
                let (start, end) = span_positions(start, end, args.len())?;
                let args = args[start..=end].to_vec();
                Some(self.arena.intern_call(head, &args))
            }
            _ => None,
        }
    }

    fn index_value(&self, expression: ExprId) -> Option<i64> {
        match self.arena.node(expression) {
            Node::Int(value) => value.to_i64(),
            Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
            _ => None,
        }
    }

    pub(super) fn read_lvalue(&mut self, lhs: ExprId) -> Result<ExprId, Diagnostic> {
        match self.arena.node(lhs) {
            Node::Symbol(symbol) => self.read_symbol(*symbol).ok_or_else(|| {
                Diagnostic::eval(ErrorCode::EvalPart, "a mutation target must have a value")
            }),
            Node::Call { head, .. } if *head == self.heads.part => self.eval(lhs),
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalPart,
                "a mutation target must be a symbol or a part",
            )),
        }
    }

    pub(super) fn write_lvalue(&mut self, lhs: ExprId, value: ExprId) -> Result<(), Diagnostic> {
        if let Some(symbol) = self.arena.symbol_id(lhs) {
            return self.write_symbol(symbol, value);
        }
        let Some((base, indices)) = self.flatten_part(lhs) else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalPart,
                "an assignment target must be a symbol or a part",
            ));
        };
        let Some(symbol) = self.arena.symbol_id(base) else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalPart,
                "a part assignment base must be a symbol",
            ));
        };
        let current = self.read_symbol(symbol).ok_or_else(|| {
            Diagnostic::eval(
                ErrorCode::EvalPart,
                "a part assignment target must have a value",
            )
        })?;
        let mut evaluated = Vec::with_capacity(indices.len());
        for index in indices {
            evaluated.push(self.eval(index)?);
        }
        let updated = self.update_path(current, &evaluated, value)?;
        self.write_symbol(symbol, updated)
    }

    fn flatten_part(&self, expression: ExprId) -> Option<(ExprId, Vec<ExprId>)> {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return None;
        };
        if *head != self.heads.part || args.len() < 2 {
            return None;
        }
        let (base, mut indices) = self.flatten_part(args[0]).unwrap_or((args[0], Vec::new()));
        indices.extend_from_slice(&args[1..]);
        Some((base, indices))
    }

    fn update_path(
        &mut self,
        container: ExprId,
        indices: &[ExprId],
        value: ExprId,
    ) -> Result<ExprId, Diagnostic> {
        let Some((index, rest)) = indices.split_first() else {
            return Ok(value);
        };
        match self.arena.node(container) {
            Node::Association(entries) => {
                let mut entries = entries.to_vec();
                if let Some(entry) = entries.iter_mut().find(|entry| entry.key == *index) {
                    entry.value = self.update_path(entry.value, rest, value)?;
                } else if rest.is_empty() {
                    entries.push(AssociationEntry { key: *index, value });
                } else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "association key is missing",
                    ));
                }
                self.arena.intern_association(entries)
            }
            Node::String(text) => {
                if !rest.is_empty() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a string part has one index",
                    ));
                }
                let index = self.index_value(*index).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "string index must be an integer")
                })?;
                let characters: Vec<char> = text.chars().collect();
                let position = index_position(index, characters.len()).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "string index is out of range")
                })?;
                let Node::String(replacement) = self.arena.node(value) else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a string part must be assigned a string",
                    ));
                };
                let mut characters = characters;
                let replacement: Vec<char> = replacement.chars().collect();
                if replacement.len() != 1 {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a string part must be assigned one character",
                    ));
                }
                characters[position] = replacement[0];
                Ok(self
                    .arena
                    .intern_string(&characters.into_iter().collect::<String>()))
            }
            Node::Bytes(bytes) => {
                if !rest.is_empty() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a byte array part has one index",
                    ));
                }
                let index = self.index_value(*index).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "byte index must be an integer")
                })?;
                let mut bytes = bytes.to_vec();
                let position = index_position(index, bytes.len()).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "byte index is out of range")
                })?;
                let Node::Int(byte) = self.arena.node(value) else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a byte part must be assigned an integer",
                    ));
                };
                let Some(byte) = byte.to_i64() else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a byte part must be assigned an integer",
                    ));
                };
                if !(0..=255).contains(&byte) {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalPart,
                        "a byte must be between 0 and 255",
                    ));
                }
                bytes[position] = byte as u8;
                Ok(self.arena.intern_bytes(&bytes))
            }
            Node::Call { head, args } => {
                let head = *head;
                let index = self.index_value(*index).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "part index must be an integer")
                })?;
                let mut args = args.to_vec();
                if index == 0 {
                    if !rest.is_empty() {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalPart,
                            "a call head cannot be indexed further",
                        ));
                    }
                    return Ok(self.arena.intern_call(value, &args));
                }
                let position = index_position(index, args.len()).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalPart, "part index is out of range")
                })?;
                args[position] = self.update_path(args[position], rest, value)?;
                Ok(self.arena.intern_call(head, &args))
            }
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalPart,
                "the part target does not support assignment",
            )),
        }
    }

    fn read_symbol(&mut self, symbol: SymbolId) -> Option<ExprId> {
        if let Some(value) = self.local_value(symbol) {
            return Some(value);
        }
        self.definitions.own_value(symbol).map(|own| own.value)
    }

    fn write_symbol(&mut self, symbol: SymbolId, value: ExprId) -> Result<(), Diagnostic> {
        for frame in self.locals.iter_mut().rev() {
            if let Some(slot) = frame.get_mut(&symbol) {
                *slot = value;
                return Ok(());
            }
        }
        let lhs = self.arena.symbol_expr(symbol);
        self.definitions.define(self.arena, lhs, value, false)
    }
}

fn span_positions(start: i64, end: i64, length: usize) -> Option<(usize, usize)> {
    let start = index_position(start, length)?;
    let end = index_position(end, length)?;
    if start > end {
        return None;
    }
    Some((start, end))
}

fn index_position(index: i64, length: usize) -> Option<usize> {
    if index > 0 {
        let position = index as usize - 1;
        (position < length).then_some(position)
    } else if index < 0 {
        let position = length as i64 + index;
        (position >= 0).then_some(position as usize)
    } else {
        None
    }
}
