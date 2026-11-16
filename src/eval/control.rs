//! Definitions, closures, control flow, and list threading.

use super::*;

impl<'a> Evaluator<'a> {
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
        let Some(items) = self.list_items(args[0]).map(<[ExprId]>::to_vec) else {
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
        let Some(items) = self.list_items(args[1]).map(<[ExprId]>::to_vec) else {
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
        let Some(items) = self.list_items(args[2]).map(<[ExprId]>::to_vec) else {
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
        let Some(count) = nonnegative_integer(self.arena, args[2]) else {
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
