//! Definitions, closures, control flow, binders, local definitions, and list threading.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn eval_begin(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "begin expects one context name",
            ));
        }
        let context = self.context_argument(args[0])?;
        validate_context(&context)?;
        let context = self.arena.begin_context(&context);
        Ok(self.arena.intern_string(&context))
    }

    pub(super) fn eval_end(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if !args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "end expects no arguments",
            ));
        }
        let Some(ended) = self.arena.end_context() else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalContext,
                "end requires an enclosing begin",
            ));
        };
        Ok(self.arena.intern_string(&ended))
    }

    pub(super) fn eval_context(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "context expects one symbol",
            ));
        }
        let context = match self.arena.node(args[0]) {
            Node::Symbol(symbol) => self.arena.symbol_context(*symbol).to_string(),
            Node::String(value) => value.to_string(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalContext,
                    "context expects a symbol or a string",
                ));
            }
        };
        Ok(self.arena.intern_string(&context))
    }

    pub(super) fn eval_contexts(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if !args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "contexts expects no arguments",
            ));
        }
        let contexts = self.arena.contexts();
        let mut values = Vec::with_capacity(contexts.len());
        for context in contexts {
            values.push(self.arena.intern_string(&context));
        }
        Ok(self.arena.intern_call(self.heads.list, &values))
    }

    fn context_argument(&self, expression: ExprId) -> Result<String, Diagnostic> {
        match self.arena.node(expression) {
            Node::Symbol(symbol) => Ok(self.arena.symbol_display_name(*symbol)),
            Node::String(value) => Ok(value.to_string()),
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalContext,
                "a context name must be a symbol or a string",
            )),
        }
    }

    pub(super) fn eval_hold(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut values = Vec::with_capacity(args.len());
        let mut changed = false;
        for arg in args {
            if let Node::Call { head, args: inner } = self.arena.node(*arg)
                && *head == self.heads.evaluate
                && inner.len() == 1
            {
                changed = true;
                values.push(self.eval(inner[0])?);
            } else {
                values.push(*arg);
            }
        }
        if changed {
            Ok(self.arena.intern_call(self.heads.hold, &values))
        } else {
            Ok(self.arena.intern_call(self.heads.hold, args))
        }
    }

    pub(super) fn eval_compound(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut result = self.heads.null;
        for arg in args {
            result = self.eval(*arg)?;
            if self.control.is_some() {
                break;
            }
        }
        Ok(result)
    }

    pub(super) fn eval_which(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() & 1 == 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "which expects condition and value pairs",
            ));
        }
        let mut index = 0;
        while index < args.len() {
            let condition = self.eval(args[index])?;
            if condition == self.heads.true_value {
                return self.eval(args[index + 1]);
            }
            if condition != self.heads.false_value {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalContext,
                    "a which condition must be true or false",
                ));
            }
            index += 2;
        }
        Ok(self.heads.null)
    }

    pub(super) fn eval_switch(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "switch expects a scrutinee",
            ));
        }
        let scrutinee = self.eval(args[0])?;
        let mut index = 1;
        while index + 1 < args.len() {
            let pattern = args[index];
            let value = args[index + 1];
            let mut bindings = Bindings::new();
            let mut conditions = Vec::new();
            if match_pattern(
                self.arena,
                pattern,
                scrutinee,
                &mut bindings,
                &mut conditions,
            ) {
                let mut accepted = true;
                for condition in conditions {
                    let condition = substitute(self.arena, condition, &bindings);
                    if self.eval(condition)? != self.heads.true_value {
                        accepted = false;
                        break;
                    }
                }
                if accepted {
                    let value = substitute(self.arena, value, &bindings);
                    return self.eval(value);
                }
            }
            index += 2;
        }
        if index < args.len() {
            return self.eval(args[index]);
        }
        Ok(self.heads.null)
    }

    pub(super) fn eval_do(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "do expects a body and a count",
            ));
        }
        let count = self.eval(args[1])?;
        let Some(count) = nonnegative_integer(self.arena, count) else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalContext,
                "do expects a nonnegative integer count",
            ));
        };
        for _ in 0..count {
            self.eval(args[0])?;
            if self.loop_control() {
                break;
            }
        }
        Ok(self.heads.null)
    }

    pub(super) fn eval_while(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "while expects a condition and a body",
            ));
        }
        loop {
            let condition = self.eval(args[0])?;
            if condition == self.heads.false_value {
                break;
            }
            if condition != self.heads.true_value {
                if self.control.is_some() {
                    break;
                }
                return Err(Diagnostic::eval(
                    ErrorCode::EvalContext,
                    "a while condition must be true or false",
                ));
            }
            self.eval(args[1])?;
            if self.loop_control() {
                break;
            }
        }
        Ok(self.heads.null)
    }

    pub(super) fn eval_for(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 4 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "for expects initialization, condition, increment, and body",
            ));
        }
        self.eval(args[0])?;
        loop {
            if self.control.is_some() {
                break;
            }
            let condition = self.eval(args[1])?;
            if condition == self.heads.false_value {
                break;
            }
            if condition != self.heads.true_value {
                if self.control.is_some() {
                    break;
                }
                return Err(Diagnostic::eval(
                    ErrorCode::EvalContext,
                    "a for condition must be true or false",
                ));
            }
            self.eval(args[3])?;
            if self.loop_control() {
                break;
            }
            self.eval(args[2])?;
        }
        Ok(self.heads.null)
    }

    pub(super) fn eval_return(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() > 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "return expects zero or one argument",
            ));
        }
        let value = if args.is_empty() {
            self.heads.null
        } else {
            self.eval(args[0])?
        };
        if self.control.is_none() {
            self.control = Some(Control::Return(value));
        }
        Ok(value)
    }

    pub(super) fn eval_break(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if !args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "break expects no arguments",
            ));
        }
        self.control = Some(Control::Break);
        Ok(self.heads.null)
    }

    pub(super) fn eval_continue(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if !args.is_empty() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "continue expects no arguments",
            ));
        }
        self.control = Some(Control::Continue);
        Ok(self.heads.null)
    }

    pub(super) fn eval_throw(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "throw expects a value and an optional tag",
            ));
        }
        let value = self.eval(args[0])?;
        let tag = if args.len() == 2 {
            Some(self.eval(args[1])?)
        } else {
            None
        };
        if self.control.is_none() {
            self.control = Some(Control::Throw(value, tag));
        }
        Ok(value)
    }

    pub(super) fn eval_catch(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "catch expects a body and an optional tag",
            ));
        }
        let tag = if args.len() == 2 {
            Some(self.eval(args[1])?)
        } else {
            None
        };
        let result = self.eval(args[0])?;
        if let Some(Control::Throw(value, thrown_tag)) = self.control {
            if tag.is_none() || tag == thrown_tag {
                self.control = None;
                return Ok(value);
            }
            return Ok(result);
        }
        Ok(result)
    }

    pub(super) fn eval_unset(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "unset expects one target",
            ));
        }
        match self.arena.node(args[0]) {
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if self
                    .arena
                    .symbol_attributes(symbol)
                    .has(crate::expr::Attributes::PROTECTED)
                {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalProtected,
                        "a protected symbol cannot be unset",
                    ));
                }
                self.definitions.unset_symbol(symbol);
                Ok(self.heads.null)
            }
            Node::Call { head, .. } => {
                let Some(symbol) = self.arena.symbol_id(*head) else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalContext,
                        "unset expects a symbol or a function call",
                    ));
                };
                if self
                    .arena
                    .symbol_attributes(symbol)
                    .has(crate::expr::Attributes::PROTECTED)
                {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalProtected,
                        "a protected symbol cannot be unset",
                    ));
                }
                self.definitions.unset_rules(symbol);
                Ok(self.heads.null)
            }
            _ => Err(Diagnostic::eval(
                ErrorCode::EvalContext,
                "unset expects a symbol or a function call",
            )),
        }
    }

    pub(super) fn eval_clear(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        for arg in args {
            let symbol = match self.arena.node(*arg) {
                Node::Symbol(symbol) => *symbol,
                Node::String(name) => {
                    let name = name.to_string();
                    self.arena.resolve_symbol(&name)
                }
                _ => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalContext,
                        "clear expects symbols or strings",
                    ));
                }
            };
            if self
                .arena
                .symbol_attributes(symbol)
                .has(crate::expr::Attributes::PROTECTED)
            {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalProtected,
                    "a protected symbol cannot be cleared",
                ));
            }
            self.definitions.clear_symbol(symbol);
            if head == self.heads.clear_all
                && !crate::eval::is_kernel_head(self.arena.symbol_name(symbol))
            {
                self.arena
                    .set_symbol_attributes(symbol, crate::expr::Attributes::new());
            }
        }
        self.definitions.bump_version();
        Ok(self.heads.null)
    }

    pub(super) fn is_mutation_head(&self, head: ExprId) -> bool {
        head == self.heads.append_to
            || head == self.heads.prepend_to
            || head == self.heads.add_to
            || head == self.heads.subtract_from
            || head == self.heads.times_by
            || head == self.heads.divide_by
    }

    pub(super) fn eval_mutation(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "a mutation expects a target and a value",
            ));
        }
        let value = self.eval(args[1])?;
        let current = self.read_lvalue(args[0])?;
        let updated = if head == self.heads.append_to {
            let mut items = self.list_items(current).unwrap_or(&[]).to_vec();
            items.push(value);
            self.arena.intern_call(self.heads.list, &items)
        } else if head == self.heads.prepend_to {
            let items = self.list_items(current).unwrap_or(&[]).to_vec();
            let mut updated = Vec::with_capacity(items.len() + 1);
            updated.push(value);
            updated.extend(items);
            self.arena.intern_call(self.heads.list, &updated)
        } else if head == self.heads.add_to {
            let sum = self.arena.intern_call(self.heads.plus, &[current, value]);
            self.eval(sum)?
        } else if head == self.heads.subtract_from {
            let negated = self.arena.intern_call(self.heads.negate, &[value]);
            let sum = self.arena.intern_call(self.heads.plus, &[current, negated]);
            self.eval(sum)?
        } else if head == self.heads.times_by {
            let product = self.arena.intern_call(self.heads.times, &[current, value]);
            self.eval(product)?
        } else {
            let minus_one = self.arena.intern_int(crate::num::Int::from_i64(-1));
            let inverse = self
                .arena
                .intern_call(self.heads.power, &[value, minus_one]);
            let product = self
                .arena
                .intern_call(self.heads.times, &[current, inverse]);
            self.eval(product)?
        };
        self.write_lvalue(args[0], updated)?;
        Ok(updated)
    }

    fn loop_control(&mut self) -> bool {
        match self.control {
            Some(Control::Break) => {
                self.control = None;
                true
            }
            Some(Control::Continue) => {
                self.control = None;
                false
            }
            Some(_) => true,
            None => false,
        }
    }

    pub(super) fn take_return(&mut self) -> Option<ExprId> {
        if let Some(Control::Return(value)) = self.control {
            self.control = None;
            Some(value)
        } else {
            None
        }
    }

    pub(super) fn eval_definition(
        &mut self,
        args: &[ExprId],
        delayed: bool,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "a definition expects two arguments",
            ));
        }
        let rhs = if delayed {
            args[1]
        } else {
            self.eval(args[1])?
        };
        if !delayed && self.arena.symbol_id(args[0]).is_some() {
            self.write_lvalue(args[0], rhs)?;
            return Ok(rhs);
        }
        if let Node::Call { head, .. } = self.arena.node(args[0])
            && *head == self.heads.part
        {
            if delayed {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "setDelayed cannot assign a part",
                ));
            }
            self.write_lvalue(args[0], rhs)?;
            return Ok(rhs);
        }
        {
            let arena = &*self.arena;
            self.definitions.define(arena, args[0], rhs, delayed)?;
        }
        Ok(if delayed { self.heads.null } else { rhs })
    }

    pub(super) fn eval_let(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "let expects two arguments",
            ));
        }
        let bindings = match self.arena.node(args[0]) {
            Node::Call {
                head,
                args: bindings,
            } if *head == self.heads.list => bindings.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "let bindings must be a list",
                ));
            }
        };
        self.locals.push(HashMap::new());
        let result = (|| {
            for binding in bindings {
                let (lhs, rhs) = match self.arena.node(binding) {
                    Node::Call { head, args } if *head == self.heads.set && args.len() == 2 => {
                        (args[0], args[1])
                    }
                    _ => {
                        return Err(Diagnostic::eval(
                            ErrorCode::EvalDefinition,
                            "each let binding must use the form name=value",
                        ));
                    }
                };
                let Some(symbol) = self.arena.symbol_id(lhs) else {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "a let binding name must be a symbol",
                    ));
                };
                let value = self.eval(rhs)?;
                self.locals
                    .last_mut()
                    .expect("let frame exists")
                    .insert(symbol, value);
            }
            self.eval(args[1])
        })();
        self.locals.pop();
        result
    }

    pub(super) fn eval_local_defs(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "localDefs expects definitions and a body",
            ));
        }
        let Some(entries) = self.list_items(args[0]).map(<[ExprId]>::to_vec) else {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "localDefs expects a list of definitions",
            ));
        };
        let mut definitions = Vec::with_capacity(entries.len());
        for entry in entries {
            let Node::Call { head, args: parts } = self.arena.node(entry) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "each local definition must use the form setDelayed(lhs,rhs)",
                ));
            };
            if parts.len() != 2
                || self
                    .arena
                    .symbol_id(*head)
                    .map(|symbol| self.arena.symbol_name(symbol))
                    != Some("setDelayed")
            {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "each local definition must use the form setDelayed(lhs,rhs)",
                ));
            }
            definitions.push((parts[0], parts[1]));
        }
        self.definitions.push_scope();
        let result = (|| {
            for (lhs, rhs) in definitions {
                self.definitions.define_local(self.arena, lhs, rhs)?;
            }
            self.eval(args[1])
        })();
        self.definitions.pop_scope();
        result
    }

    pub(super) fn eval_quantifier(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "a binder expects a variable, a domain, and a body",
            ));
        }
        let Some((variable, domain, body, canonical)) = self.canonical_binder(head, args) else {
            return Ok(self.arena.intern_call(head, args));
        };
        let domain = self.eval(domain)?;
        let Some(items) = self.list_items(domain).map(<[ExprId]>::to_vec) else {
            return Ok(canonical);
        };
        let kind = if head == self.heads.forall {
            QuantifierKind::Forall
        } else if head == self.heads.exists {
            QuantifierKind::Exists
        } else {
            QuantifierKind::SetBuilder
        };
        let mut selected = Vec::new();
        let mut decided = true;
        self.locals.push(HashMap::new());
        let result = (|| -> Result<Option<bool>, Diagnostic> {
            for item in items {
                self.locals
                    .last_mut()
                    .expect("quantifier frame exists")
                    .insert(variable, item);
                let value = self.eval(body)?;
                match kind {
                    QuantifierKind::Forall => {
                        if value == self.heads.false_value {
                            return Ok(Some(false));
                        }
                        if value != self.heads.true_value {
                            decided = false;
                        }
                    }
                    QuantifierKind::Exists => {
                        if value == self.heads.true_value {
                            return Ok(Some(true));
                        }
                        if value != self.heads.false_value {
                            decided = false;
                        }
                    }
                    QuantifierKind::SetBuilder => {
                        if value == self.heads.true_value {
                            selected.push(item);
                        } else if value != self.heads.false_value {
                            decided = false;
                        }
                    }
                }
            }
            Ok(None)
        })();
        self.locals.pop();
        if let Some(value) = result? {
            return Ok(if value {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        if !decided {
            return Ok(canonical);
        }
        match kind {
            QuantifierKind::Forall => Ok(self.heads.true_value),
            QuantifierKind::Exists => Ok(self.heads.false_value),
            QuantifierKind::SetBuilder => {
                selected.sort_by(|left, right| self.arena.canonical_cmp(*left, *right));
                selected.dedup();
                Ok(self.arena.intern_call(self.heads.list, &selected))
            }
        }
    }

    fn canonical_binder(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Option<(SymbolId, ExprId, ExprId, ExprId)> {
        let variable = self.arena.symbol_id(args[0])?;
        let mut symbols = HashSet::new();
        all_symbols(self.arena, args[2], &mut symbols);
        let mut index = 0u32;
        let canonical = loop {
            let name = format!("v{index}");
            let symbol = self.arena.symbol_or_intern(&name);
            if !symbols.contains(&symbol) {
                break symbol;
            }
            index += 1;
        };
        let body = rename_expression(self.arena, args[2], variable, canonical);
        let variable = self.arena.symbol_expr(canonical);
        let call = self.arena.intern_call(head, &[variable, args[1], body]);
        Some((canonical, args[1], body, call))
    }

    pub(super) fn eval_if(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "if expects three arguments",
            ));
        }
        let condition = self.eval(args[0])?;
        if condition == self.heads.true_value {
            return self.eval(args[1]);
        }
        if condition == self.heads.false_value {
            return self.eval(args[2]);
        }
        if self.is_error(condition) {
            return Ok(condition);
        }
        Ok(self
            .arena
            .intern_call(self.heads.if_head, &[condition, args[1], args[2]]))
    }

    pub(super) fn eval_and(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut evaluated = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.eval(*arg)?;
            if value == self.heads.false_value {
                return Ok(self.heads.false_value);
            }
            if self.is_error(value) {
                return Ok(value);
            }
            evaluated.push(value);
        }
        evaluated.retain(|value| *value != self.heads.true_value);
        match evaluated.len() {
            0 => Ok(self.heads.true_value),
            1 => Ok(evaluated[0]),
            _ => Ok(self.arena.intern_call(self.heads.and, &evaluated)),
        }
    }

    pub(super) fn eval_or(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut evaluated = Vec::with_capacity(args.len());
        for arg in args {
            let value = self.eval(*arg)?;
            if value == self.heads.true_value {
                return Ok(self.heads.true_value);
            }
            if self.is_error(value) {
                return Ok(value);
            }
            evaluated.push(value);
        }
        evaluated.retain(|value| *value != self.heads.false_value);
        match evaluated.len() {
            0 => Ok(self.heads.false_value),
            1 => Ok(evaluated[0]),
            _ => Ok(self.arena.intern_call(self.heads.or, &evaluated)),
        }
    }

    pub(super) fn eval_not(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "not expects one argument",
            ));
        }
        let value = self.eval(args[0])?;
        if value == self.heads.true_value {
            return Ok(self.heads.false_value);
        }
        if value == self.heads.false_value {
            return Ok(self.heads.true_value);
        }
        if self.is_error(value) {
            return Ok(value);
        }
        Ok(self.arena.intern_call(self.heads.not, &[value]))
    }

    pub(super) fn thread_listable(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let mut length: Option<usize> = None;
        for arg in args {
            let Some(items) = self.list_items(*arg) else {
                continue;
            };
            match length {
                None => length = Some(items.len()),
                Some(existing) if existing == items.len() => {}
                Some(_) => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "listable arguments have different lengths",
                    ));
                }
            }
        }
        let Some(length) = length else {
            return Ok(None);
        };
        let mut results = Vec::with_capacity(length);
        for index in 0..length {
            let mut call_args = Vec::with_capacity(args.len());
            for arg in args {
                let value = self.list_items(*arg).map_or(*arg, |items| items[index]);
                call_args.push(value);
            }
            let call = self.arena.intern_call(head, &call_args);
            results.push(self.eval(call)?);
        }
        Ok(Some(self.arena.intern_call(self.heads.list, &results)))
    }

    pub(super) fn eval_map(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "map expects a list and a function",
            ));
        }
        let list = self.eval(args[0])?;
        let Some(items) = self.list_items(list).map(<[ExprId]>::to_vec) else {
            return Ok(self.arena.intern_call(head, args));
        };
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(self.call_function(args[1], &[item])?);
        }
        Ok(self.arena.intern_call(self.heads.list, &results))
    }

    pub(super) fn eval_apply(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "apply expects a function and a list",
            ));
        }
        let list = self.eval(args[1])?;
        let Some(items) = self.list_items(list).map(<[ExprId]>::to_vec) else {
            return Ok(self.arena.intern_call(head, args));
        };
        self.call_function(args[0], &items)
    }

    pub(super) fn eval_fold(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "fold expects a function, an initial value, and a list",
            ));
        }
        let list = self.eval(args[2])?;
        let Some(items) = self.list_items(list).map(<[ExprId]>::to_vec) else {
            return Ok(self.arena.intern_call(head, args));
        };
        let mut accumulator = self.eval(args[1])?;
        for item in items {
            accumulator = self.call_function(args[0], &[accumulator, item])?;
        }
        Ok(accumulator)
    }

    pub(super) fn eval_nest(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "nest expects a function, an initial value, and a count",
            ));
        }
        let count_value = self.eval(args[2])?;
        let Some(count) = nonnegative_integer(self.arena, count_value) else {
            return Ok(self.arena.intern_call(head, args));
        };
        if count > 10_000 {
            return Ok(self.arena.intern_call(head, args));
        }
        let mut value = self.eval(args[1])?;
        for _ in 0..count {
            value = self.call_function(args[0], &[value])?;
        }
        Ok(value)
    }

    pub(super) fn call_function(
        &mut self,
        function: ExprId,
        arguments: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let function = self.eval(function)?;
        if let Node::Call { head, args } = self.arena.node(function)
            && *head == self.heads.closure
        {
            let closure = args.to_vec();
            return self.apply_closure(&closure, arguments);
        }
        let call = self.arena.intern_call(function, arguments);
        self.eval(call)
    }

    pub(super) fn list_items(&self, expression: ExprId) -> Option<&[ExprId]> {
        match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => Some(args),
            _ => None,
        }
    }
}

#[derive(Copy, Clone)]
enum QuantifierKind {
    Forall,
    Exists,
    SetBuilder,
}

fn validate_context(context: &str) -> Result<(), Diagnostic> {
    let valid = !context.is_empty()
        && context.split(';').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
    if valid {
        Ok(())
    } else {
        Err(Diagnostic::eval(
            ErrorCode::EvalContext,
            "a context name must contain letters, digits, or underscores",
        ))
    }
}
