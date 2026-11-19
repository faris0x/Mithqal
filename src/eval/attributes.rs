//! Symbol attributes and definition introspection.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Attributes, ExprId, SymbolId};

use super::*;

impl Evaluator<'_> {
    pub(super) fn eval_set_attributes(
        &mut self,
        args: &[ExprId],
        clear: bool,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() < 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "setAttributes expects a symbol and attributes",
            ));
        }
        let symbol = self.symbol_argument(args[0])?;
        if crate::builtins::attributes(self.arena.symbol_name(symbol)).is_some() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalProtected,
                "a built in symbol cannot change attributes",
            ));
        }
        let mut attributes = self.arena.symbol_attributes(symbol);
        if attributes.has(Attributes::PROTECTED) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalProtected,
                "an attribute change requires unprotect",
            ));
        }
        for argument in &args[1..] {
            let name = self
                .arena
                .symbol_id(*argument)
                .map(|id| self.arena.symbol_name(id).to_string())
                .ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::EvalDefinition, "an attribute must be a symbol")
                })?;
            let flag = attribute_flag(&name).ok_or_else(|| {
                Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    format!("unknown attribute {name}"),
                )
            })?;
            if clear {
                attributes = attributes.without(flag);
            } else {
                attributes = attributes.with(flag);
            }
        }
        self.arena.set_symbol_attributes(symbol, attributes);
        self.definitions.bump_version();
        Ok(self.heads.null)
    }

    pub(super) fn eval_attributes(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "attributes expects one symbol",
            ));
        }
        let symbol = self.symbol_argument(args[0])?;
        let attributes = self.attributes_list(symbol);
        Ok(attributes)
    }

    pub(super) fn eval_protect(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let protect = head == self.heads.protect;
        for argument in args {
            let symbol = self.symbol_argument(*argument)?;
            if !protect && crate::builtins::attributes(self.arena.symbol_name(symbol)).is_some() {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalProtected,
                    "a built in symbol cannot be unprotected",
                ));
            }
            let attributes = self.arena.symbol_attributes(symbol);
            self.arena.set_symbol_attributes(
                symbol,
                if protect {
                    attributes.with(Attributes::PROTECTED)
                } else {
                    attributes.without(Attributes::PROTECTED)
                },
            );
        }
        self.definitions.bump_version();
        Ok(self.heads.null)
    }

    pub(super) fn eval_values(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "a values function expects one symbol",
            ));
        }
        let symbol = self.symbol_argument(args[0])?;
        let values = if head == self.heads.own_values {
            self.own_values(symbol)
        } else if head == self.heads.down_values {
            self.down_values(symbol)
        } else if head == self.heads.up_values {
            self.up_values(symbol)
        } else {
            self.list(&[])
        };
        Ok(values)
    }

    pub(super) fn eval_definition_info(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "definition expects one symbol",
            ));
        }
        let symbol = self.symbol_argument(args[0])?;
        let attributes = self.attributes_list(symbol);
        let own = self.own_values(symbol);
        let down = self.down_values(symbol);
        let items = [attributes, own, down];
        Ok(self.list(&items))
    }

    fn own_values(&mut self, symbol: SymbolId) -> ExprId {
        let mut rules = Vec::new();
        if let Some(own) = self.definitions.own_value(symbol) {
            let name = self.arena.symbol_expr(symbol);
            let head = self.arena.builtin_head("ruleDelayed");
            rules.push(self.arena.intern_call(head, &[name, own.value]));
        }
        self.list(&rules)
    }

    fn up_values(&mut self, symbol: SymbolId) -> ExprId {
        let mut rules = Vec::new();
        for rule in self.definitions.up_rules(symbol) {
            let head = self.arena.builtin_head("ruleDelayed");
            rules.push(self.arena.intern_call(head, &[rule.lhs, rule.rhs]));
        }
        self.list(&rules)
    }

    fn down_values(&mut self, symbol: SymbolId) -> ExprId {
        let mut rules = Vec::new();
        for rule in self.definitions.rules(symbol) {
            let head = self.arena.builtin_head("ruleDelayed");
            rules.push(self.arena.intern_call(head, &[rule.lhs, rule.rhs]));
        }
        self.list(&rules)
    }

    fn attributes_list(&mut self, symbol: SymbolId) -> ExprId {
        let attributes = self.arena.symbol_attributes(symbol);
        let mut items = Vec::new();
        for (name, flag) in ATTRIBUTE_NAMES {
            if attributes.has(*flag) {
                let symbol = self.arena.symbol_or_intern(name);
                items.push(self.arena.symbol_expr(symbol));
            }
        }
        self.list(&items)
    }

    fn symbol_argument(&self, expression: ExprId) -> Result<SymbolId, Diagnostic> {
        self.arena
            .symbol_id(expression)
            .ok_or_else(|| Diagnostic::eval(ErrorCode::EvalDefinition, "expected a symbol"))
    }
}

const ATTRIBUTE_NAMES: &[(&str, u16)] = &[
    ("flat", Attributes::FLAT),
    ("orderless", Attributes::ORDERLESS),
    ("holdAll", Attributes::HOLD_ALL),
    ("holdFirst", Attributes::HOLD_FIRST),
    ("holdRest", Attributes::HOLD_REST),
    ("listable", Attributes::LISTABLE),
    ("protected", Attributes::PROTECTED),
    ("constant", Attributes::CONSTANT),
    ("oneIdentity", Attributes::ONE_IDENTITY),
    ("sequenceHold", Attributes::SEQUENCE_HOLD),
];

fn attribute_flag(name: &str) -> Option<u16> {
    ATTRIBUTE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, flag)| *flag)
}
