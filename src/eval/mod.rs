//! Fixed point evaluator.
//!
//! Version 0.0.1 implements exact arithmetic, listable threading, comparisons,
//! logic, and the boundary for definitions. Definitions and pattern matching
//! are added in the next layer.

mod definitions;
mod pattern;

use core::cmp::Ordering;
use std::collections::HashMap;

use crate::builtins;
use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::{Arena, Attributes, ExprId, Node, SymbolId};
use crate::num::{Int, Rat};

use definitions::{Definitions, Rule};
use pattern::{Bindings, match_pattern, substitute};

pub(crate) use definitions::Definitions as SessionDefinitions;

struct Heads {
    plus: ExprId,
    times: ExprId,
    power: ExprId,
    negate: ExprId,
    list: ExprId,
    if_head: ExprId,
    and: ExprId,
    or: ExprId,
    not: ExprId,
    same: ExprId,
    equal: ExprId,
    not_equal: ExprId,
    less: ExprId,
    less_equal: ExprId,
    greater: ExprId,
    greater_equal: ExprId,
    true_value: ExprId,
    false_value: ExprId,
    undefined: ExprId,
    indeterminate: ExprId,
    failed: ExprId,
    imaginary: ExprId,
    set: ExprId,
    set_delayed: ExprId,
    let_head: ExprId,
    null: ExprId,
}

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
    let mut evaluator = Evaluator::new(arena, definitions);
    evaluator.max_steps = max_steps.max(1);
    evaluator.max_depth = max_depth.max(1);
    evaluator.eval(id)
}

struct Evaluator<'a> {
    arena: &'a mut Arena,
    definitions: &'a mut Definitions,
    heads: Heads,
    locals: Vec<HashMap<SymbolId, ExprId>>,
    steps: u64,
    max_steps: u64,
    depth: u32,
    max_depth: u32,
}

impl<'a> Evaluator<'a> {
    fn new(arena: &'a mut Arena, definitions: &'a mut Definitions) -> Self {
        let heads = Heads {
            plus: head(arena, "plus"),
            times: head(arena, "times"),
            power: head(arena, "power"),
            negate: head(arena, "negate"),
            list: head(arena, "list"),
            if_head: head(arena, "if"),
            and: head(arena, "and"),
            or: head(arena, "or"),
            not: head(arena, "not"),
            same: head(arena, "same"),
            equal: head(arena, "equal"),
            not_equal: head(arena, "notEqual"),
            less: head(arena, "less"),
            less_equal: head(arena, "lessEqual"),
            greater: head(arena, "greater"),
            greater_equal: head(arena, "greaterEqual"),
            true_value: head(arena, "true"),
            false_value: head(arena, "false"),
            undefined: head(arena, "undefined"),
            indeterminate: head(arena, "indeterminate"),
            failed: head(arena, "failed"),
            imaginary: head(arena, "i"),
            set: head(arena, "set"),
            set_delayed: head(arena, "setDelayed"),
            let_head: head(arena, "let"),
            null: head(arena, "null"),
        };
        Evaluator {
            arena,
            definitions,
            heads,
            locals: Vec::new(),
            steps: 0,
            max_steps: 1_000_000,
            depth: 0,
            max_depth: 2048,
        }
    }

    fn eval(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(Diagnostic::eval(
                ErrorCode::EvalBudget,
                "evaluation budget exhausted",
            ));
        }
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(Diagnostic::eval(
                ErrorCode::EvalBudget,
                "evaluation depth exhausted",
            ));
        }
        let result = self.eval_inner(id);
        self.depth -= 1;
        result
    }

    fn eval_inner(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        let (head, args) = match self.arena.node(id) {
            Node::Int(_) | Node::Rat(_) => return Ok(id),
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if let Some(value) = self.local_value(symbol) {
                    return Ok(value);
                }
                if let Some(own) = self.definitions.own_value(symbol) {
                    return self.eval(own.value);
                }
                return Ok(id);
            }
            Node::Call { head, args } => (*head, args.to_vec()),
        };
        let head = self.eval(head)?;
        if !matches!(self.arena.node(head), Node::Symbol(_)) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalNotCallable,
                "a call head must evaluate to a symbol",
            ));
        }

        if head == self.heads.set {
            return self.eval_definition(&args, false);
        }
        if head == self.heads.set_delayed {
            return self.eval_definition(&args, true);
        }
        if head == self.heads.let_head {
            return self.eval_let(&args);
        }
        if head == self.heads.if_head {
            return self.eval_if(&args);
        }
        if head == self.heads.and {
            return self.eval_and(&args);
        }
        if head == self.heads.or {
            return self.eval_or(&args);
        }
        if head == self.heads.not {
            return self.eval_not(&args);
        }

        let attrs = self.arena.head_attributes(head);
        if attrs.has(Attributes::HOLD_ALL)
            || attrs.has(Attributes::HOLD_FIRST)
            || attrs.has(Attributes::HOLD_REST)
        {
            return Ok(id);
        }

        let mut evaluated = Vec::with_capacity(args.len());
        for arg in &args {
            evaluated.push(self.eval(*arg)?);
        }

        if head == self.heads.plus || head == self.heads.times || head == self.heads.negate {
            let candidate = self.arena.intern_call(head, &evaluated);
            if let Some((real, imaginary)) = self.gaussian(candidate) {
                return Ok(self.gaussian_node(&real, &imaginary));
            }
        }

        if attrs.has(Attributes::LISTABLE) {
            if let Some(result) = self.thread_listable(head, &evaluated)? {
                return self.finish(id, result);
            }
        }

        let result = self.apply(head, &evaluated)?;
        self.finish(id, result)
    }

    fn finish(&mut self, original: ExprId, result: ExprId) -> Result<ExprId, Diagnostic> {
        if result == original {
            Ok(original)
        } else {
            self.eval(result)
        }
    }

    fn local_value(&self, symbol: SymbolId) -> Option<ExprId> {
        for frame in self.locals.iter().rev() {
            if let Some(value) = frame.get(&symbol) {
                return Some(*value);
            }
        }
        None
    }

    fn eval_definition(&mut self, args: &[ExprId], delayed: bool) -> Result<ExprId, Diagnostic> {
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

    fn eval_let(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
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
        let mut frame = HashMap::new();
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
            frame.insert(symbol, value);
        }
        self.locals.push(frame);
        let result = self.eval(args[1]);
        self.locals.pop();
        result
    }

    fn eval_if(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
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

    fn eval_and(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
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

    fn eval_or(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
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

    fn eval_not(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
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

    fn thread_listable(
        &mut self,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let mut length: Option<usize> = None;
        for arg in args {
            if let Node::Call {
                head: list_head,
                args: items,
            } = self.arena.node(*arg)
            {
                if *list_head == self.heads.list {
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
            }
        }
        let Some(length) = length else {
            return Ok(None);
        };
        let mut results = Vec::with_capacity(length);
        for index in 0..length {
            let mut call_args = Vec::with_capacity(args.len());
            for arg in args {
                if let Node::Call {
                    head: list_head,
                    args: items,
                } = self.arena.node(*arg)
                {
                    if *list_head == self.heads.list {
                        call_args.push(items[index]);
                        continue;
                    }
                }
                call_args.push(*arg);
            }
            let call = self.arena.intern_call(head, &call_args);
            results.push(self.eval(call)?);
        }
        Ok(Some(self.arena.intern_call(self.heads.list, &results)))
    }

    fn apply(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if head == self.heads.plus {
            return self.plus(args);
        }
        if head == self.heads.times {
            return self.times(args);
        }
        if head == self.heads.power {
            return self.power(args);
        }
        if head == self.heads.negate {
            return self.negate(args);
        }
        if head == self.heads.list {
            return Ok(self.arena.intern_call(self.heads.list, args));
        }
        if head == self.heads.same {
            return self.same(args);
        }
        if head == self.heads.equal {
            return self.equal(args);
        }
        if head == self.heads.not_equal {
            return self.not_equal(args);
        }
        if head == self.heads.less {
            return self.ordering(args, Ordering::Less, self.heads.less);
        }
        if head == self.heads.less_equal {
            return self.ordering(args, Ordering::Less, self.heads.less_equal);
        }
        if head == self.heads.greater {
            return self.ordering(args, Ordering::Greater, self.heads.greater);
        }
        if head == self.heads.greater_equal {
            return self.ordering(args, Ordering::Greater, self.heads.greater_equal);
        }
        if let Some(symbol) = self.arena.symbol_id(head) {
            if let Some(result) = self.apply_user(symbol, head, args)? {
                return Ok(result);
            }
        }
        Ok(self.arena.intern_call(head, args))
    }

    fn apply_user(
        &mut self,
        symbol: SymbolId,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let rules: Vec<Rule> = self.definitions.rules(symbol).to_vec();
        if rules.is_empty() {
            return Ok(None);
        }
        let call = self.arena.intern_call(head, args);
        for rule in rules {
            let mut bindings = Bindings::new();
            let mut conditions = Vec::new();
            if !match_pattern(&self.arena, rule.lhs, call, &mut bindings, &mut conditions) {
                continue;
            }
            let mut accepted = true;
            for condition in conditions {
                let condition = substitute(&mut self.arena, condition, &bindings);
                let value = self.eval(condition)?;
                if value != self.heads.true_value {
                    accepted = false;
                    break;
                }
            }
            if !accepted {
                continue;
            }
            let rhs = substitute(&mut self.arena, rule.rhs, &bindings);
            return if rule.delayed {
                self.eval(rhs).map(Some)
            } else {
                Ok(Some(rhs))
            };
        }
        Ok(None)
    }

    fn plus(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if let Some(value) = self.error_value(args) {
            return Ok(value);
        }
        let mut constant = Rat::zero();
        let mut terms: Vec<(ExprId, Rat)> = Vec::new();
        for arg in args {
            let (coefficient, body) = self.decompose_term(*arg)?;
            match body {
                None => constant = constant.add(&coefficient),
                Some(body) => {
                    let mut found = false;
                    for (existing, value) in &mut terms {
                        if *existing == body {
                            *value = value.add(&coefficient);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        terms.push((body, coefficient));
                    }
                }
            }
        }

        let mut out = Vec::new();
        if !constant.is_zero() {
            out.push(self.arena.intern_rat(constant));
        }
        for (body, coefficient) in terms {
            if coefficient.is_zero() {
                continue;
            }
            out.push(self.term(body, coefficient));
        }
        Ok(match out.len() {
            0 => self.arena.intern_int(Int::zero()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.plus, &out),
        })
    }

    fn times(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let has_zero = args
            .iter()
            .any(|arg| self.as_rat(*arg).is_some_and(|v| v.is_zero()));
        if args.iter().any(|arg| *arg == self.heads.indeterminate) {
            return Ok(self.heads.indeterminate);
        }
        if args.iter().any(|arg| *arg == self.heads.undefined) {
            return if has_zero {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.heads.undefined)
            };
        }
        if args.iter().any(|arg| *arg == self.heads.failed) {
            return Ok(self.heads.failed);
        }

        let mut coefficient = Rat::one();
        let mut bases: Vec<(ExprId, Rat)> = Vec::new();
        for arg in args {
            self.collect_factor(*arg, &mut coefficient, &mut bases)?;
        }
        if coefficient.is_zero() {
            return Ok(self.arena.intern_int(Int::zero()));
        }

        let mut factors = Vec::new();
        for (base, exponent) in bases {
            if exponent.is_zero() {
                continue;
            }
            if exponent.is_one() {
                factors.push(base);
            } else {
                let exponent_node = self.arena.intern_rat(exponent);
                factors.push(
                    self.arena
                        .intern_call(self.heads.power, &[base, exponent_node]),
                );
            }
        }

        let negative = coefficient.sign() < 0;
        let magnitude = coefficient.abs();
        let mut out = Vec::new();
        if !magnitude.is_one() || factors.is_empty() {
            out.push(self.arena.intern_rat(magnitude));
        }
        out.extend(factors);
        let product = match out.len() {
            0 => self.arena.intern_int(Int::one()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.times, &out),
        };
        if negative {
            Ok(self.arena.intern_call(self.heads.negate, &[product]))
        } else {
            Ok(product)
        }
    }

    fn power(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "power expects two arguments",
            ));
        }
        let base = args[0];
        let exponent = args[1];
        if self.is_error(base) {
            return Ok(base);
        }
        if self.is_error(exponent) {
            return Ok(exponent);
        }
        if let Some(exp) = self.as_rat(exponent) {
            if exp.is_zero() {
                return if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                    Ok(self.heads.indeterminate)
                } else {
                    Ok(self.arena.intern_int(Int::one()))
                };
            }
            if exp.is_one() {
                return Ok(base);
            }
            if exp.den().is_one() {
                if let Some(power) = exp.num().to_i64() {
                    if let Some(base_value) = self.as_rat(base) {
                        if base_value.is_zero() {
                            return if power < 0 {
                                Ok(self.heads.undefined)
                            } else {
                                Ok(self.arena.intern_int(Int::zero()))
                            };
                        }
                        let value = rat_pow(&base_value, power).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "numeric power failed")
                        })?;
                        return Ok(self.arena.intern_rat(value));
                    }
                    if base == self.heads.imaginary {
                        return Ok(self.imaginary_power(power));
                    }
                    if let Some((real, imaginary)) = self.gaussian(base) {
                        if real.is_zero() && imaginary.is_zero() {
                            return if power < 0 {
                                Ok(self.heads.undefined)
                            } else {
                                Ok(self.arena.intern_int(Int::zero()))
                            };
                        }
                        return match gaussian_pow(&real, &imaginary, power) {
                            Some((real, imaginary)) => Ok(self.gaussian_node(&real, &imaginary)),
                            None => Ok(self.heads.undefined),
                        };
                    }
                    let nested = match self.arena.node(base) {
                        Node::Call { head, args }
                            if *head == self.heads.power && args.len() == 2 =>
                        {
                            Some((args[0], args[1]))
                        }
                        _ => None,
                    };
                    if let Some((inner_base, inner_exponent)) = nested {
                        if let Some(inner) = self.as_rat(inner_exponent) {
                            let combined = inner.mul(&exp);
                            let combined_node = self.arena.intern_rat(combined);
                            return Ok(self
                                .arena
                                .intern_call(self.heads.power, &[inner_base, combined_node]));
                        }
                    }
                    let negated_base = match self.arena.node(base) {
                        Node::Call { head, args }
                            if *head == self.heads.negate && args.len() == 1 =>
                        {
                            Some(args[0])
                        }
                        _ => None,
                    };
                    if let Some(inner_base) = negated_base {
                        let inner = self
                            .arena
                            .intern_call(self.heads.power, &[inner_base, exponent]);
                        let value = self.eval(inner)?;
                        return if power % 2 == 0 {
                            Ok(value)
                        } else {
                            Ok(self.arena.intern_call(self.heads.negate, &[value]))
                        };
                    }
                }
            } else {
                if self.as_rat(base).is_some_and(|value| value.is_one()) {
                    return Ok(self.arena.intern_int(Int::one()));
                }
                if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                    return if exp.sign() < 0 {
                        Ok(self.heads.undefined)
                    } else {
                        Ok(self.arena.intern_int(Int::zero()))
                    };
                }
            }
        } else if self.as_rat(base).is_some_and(|value| value.is_one()) {
            return Ok(self.arena.intern_int(Int::one()));
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
    }

    fn negate(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "negate expects one argument",
            ));
        }
        let value = args[0];
        if self.is_error(value) {
            return Ok(value);
        }
        if let Some(number) = self.as_rat(value) {
            return Ok(self.arena.intern_rat(number.neg()));
        }
        let call = match self.arena.node(value) {
            Node::Call { head, args } => Some((*head, args.to_vec())),
            _ => None,
        };
        if let Some((head, args)) = call {
            if head == self.heads.negate && args.len() == 1 {
                return Ok(args[0]);
            }
            if head == self.heads.plus {
                let mut terms = Vec::with_capacity(args.len());
                for arg in args {
                    terms.push(self.arena.intern_call(self.heads.negate, &[arg]));
                }
                return Ok(self.arena.intern_call(self.heads.plus, &terms));
            }
        }
        Ok(self.arena.intern_call(self.heads.negate, &[value]))
    }

    fn same(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "same expects two arguments",
            ));
        }
        Ok(if args[0] == args[1] {
            self.heads.true_value
        } else {
            self.heads.false_value
        })
    }

    fn equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "equal expects two arguments",
            ));
        }
        if args[0] == args[1] {
            return Ok(self.heads.true_value);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            return Ok(if left == right {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(self.heads.equal, args))
    }

    fn not_equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "notEqual expects two arguments",
            ));
        }
        let equal = self.equal(args)?;
        if equal == self.heads.true_value {
            return Ok(self.heads.false_value);
        }
        if equal == self.heads.false_value {
            return Ok(self.heads.true_value);
        }
        Ok(self.arena.intern_call(self.heads.not_equal, args))
    }

    fn ordering(
        &mut self,
        args: &[ExprId],
        wanted: Ordering,
        head: ExprId,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "comparison expects two arguments",
            ));
        }
        if self.is_error(args[0]) {
            return Ok(args[0]);
        }
        if self.is_error(args[1]) {
            return Ok(args[1]);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            let order = left.cmp(&right);
            let result = match wanted {
                Ordering::Less => {
                    order == Ordering::Less
                        || (head == self.heads.less_equal && order == Ordering::Equal)
                }
                Ordering::Greater => {
                    order == Ordering::Greater
                        || (head == self.heads.greater_equal && order == Ordering::Equal)
                }
                Ordering::Equal => order == Ordering::Equal,
            };
            return Ok(if result {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(head, args))
    }

    fn decompose_term(&mut self, id: ExprId) -> Result<(Rat, Option<ExprId>), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => Ok((
                Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?,
                None,
            )),
            Node::Rat(value) => Ok((value.clone(), None)),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    let (coefficient, body) = self.decompose_term(args[0])?;
                    return Ok((coefficient.neg(), body));
                }
                if head == self.heads.times {
                    let mut coefficient = Rat::one();
                    let mut body = Vec::new();
                    for arg in args {
                        let (part, part_body) = self.decompose_term(arg)?;
                        coefficient = coefficient.mul(&part);
                        if let Some(part_body) = part_body {
                            body.push(part_body);
                        }
                    }
                    let body = match body.len() {
                        0 => None,
                        1 => Some(body[0]),
                        _ => Some(self.arena.intern_call(self.heads.times, &body)),
                    };
                    return Ok((coefficient, body));
                }
                Ok((Rat::one(), Some(id)))
            }
            Node::Symbol(_) => Ok((Rat::one(), Some(id))),
        }
    }

    fn collect_factor(
        &mut self,
        id: ExprId,
        coefficient: &mut Rat,
        bases: &mut Vec<(ExprId, Rat)>,
    ) -> Result<(), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let value = Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?;
                *coefficient = coefficient.mul(&value);
            }
            Node::Rat(value) => {
                *coefficient = coefficient.mul(value);
            }
            Node::Symbol(_) => self.add_base(bases, id, Rat::one()),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    *coefficient = coefficient.neg();
                    return self.collect_factor(args[0], coefficient, bases);
                }
                if head == self.heads.times {
                    for arg in args {
                        self.collect_factor(arg, coefficient, bases)?;
                    }
                    return Ok(());
                }
                if head == self.heads.power && args.len() == 2 {
                    if let Some(exponent) = self.as_rat(args[1]) {
                        if !exponent.is_zero() {
                            self.add_base(bases, args[0], exponent);
                        }
                        return Ok(());
                    }
                }
                self.add_base(bases, id, Rat::one());
            }
        }
        Ok(())
    }

    fn add_base(&self, bases: &mut Vec<(ExprId, Rat)>, base: ExprId, exponent: Rat) {
        for (existing, value) in bases.iter_mut() {
            if *existing == base {
                *value = value.add(&exponent);
                return;
            }
        }
        bases.push((base, exponent));
    }

    fn term(&mut self, body: ExprId, coefficient: Rat) -> ExprId {
        if coefficient.is_one() {
            return body;
        }
        if coefficient.sign() < 0 {
            let magnitude = coefficient.abs();
            if magnitude.is_one() {
                return self.arena.intern_call(self.heads.negate, &[body]);
            }
            let coefficient_node = self.arena.intern_rat(magnitude);
            let product = self
                .arena
                .intern_call(self.heads.times, &[coefficient_node, body]);
            return self.arena.intern_call(self.heads.negate, &[product]);
        }
        let coefficient_node = self.arena.intern_rat(coefficient);
        self.arena
            .intern_call(self.heads.times, &[coefficient_node, body])
    }

    fn imaginary_power(&mut self, power: i64) -> ExprId {
        let remainder = power.rem_euclid(4);
        match remainder {
            0 => self.arena.intern_int(Int::one()),
            1 => self.heads.imaginary,
            2 => self.arena.intern_int(Int::from_i64(-1)),
            _ => self
                .arena
                .intern_call(self.heads.negate, &[self.heads.imaginary]),
        }
    }

    fn gaussian(&self, id: ExprId) -> Option<(Rat, Rat)> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let real = Rat::from_ints(value.clone(), Int::one()).ok()?;
                Some((real, Rat::zero()))
            }
            Node::Rat(value) => Some((value.clone(), Rat::zero())),
            Node::Symbol(_) => {
                if id == self.heads.imaginary {
                    Some((Rat::zero(), Rat::one()))
                } else {
                    None
                }
            }
            Node::Call { head, args } => {
                let head = *head;
                if head == self.heads.plus {
                    let mut real = Rat::zero();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        real = real.add(&arg_real);
                        imaginary = imaginary.add(&arg_imaginary);
                    }
                    Some((real, imaginary))
                } else if head == self.heads.times {
                    let mut real = Rat::one();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        let next_real = real.mul(&arg_real).sub(&imaginary.mul(&arg_imaginary));
                        let next_imaginary =
                            real.mul(&arg_imaginary).add(&imaginary.mul(&arg_real));
                        real = next_real;
                        imaginary = next_imaginary;
                    }
                    Some((real, imaginary))
                } else if head == self.heads.negate && args.len() == 1 {
                    let (real, imaginary) = self.gaussian(args[0])?;
                    Some((real.neg(), imaginary.neg()))
                } else if head == self.heads.power && args.len() == 2 {
                    let exponent = match self.arena.node(args[1]) {
                        Node::Int(value) => value.to_i64()?,
                        Node::Rat(value) if value.den().is_one() => value.num().to_i64()?,
                        _ => return None,
                    };
                    let (real, imaginary) = self.gaussian(args[0])?;
                    gaussian_pow(&real, &imaginary, exponent)
                } else {
                    None
                }
            }
        }
    }

    fn gaussian_node(&mut self, real: &Rat, imaginary: &Rat) -> ExprId {
        if imaginary.is_zero() {
            return self.arena.intern_rat(real.clone());
        }
        let imaginary_part = self.term(self.heads.imaginary, imaginary.clone());
        if real.is_zero() {
            imaginary_part
        } else {
            let real_part = self.arena.intern_rat(real.clone());
            self.arena
                .intern_call(self.heads.plus, &[real_part, imaginary_part])
        }
    }

    fn as_rat(&self, id: ExprId) -> Option<Rat> {
        match self.arena.node(id) {
            Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            Node::Rat(value) => Some(value.clone()),
            _ => None,
        }
    }

    fn is_error(&self, id: ExprId) -> bool {
        id == self.heads.undefined || id == self.heads.indeterminate || id == self.heads.failed
    }

    fn error_value(&self, args: &[ExprId]) -> Option<ExprId> {
        if args.iter().any(|arg| *arg == self.heads.indeterminate) {
            return Some(self.heads.indeterminate);
        }
        if args.iter().any(|arg| *arg == self.heads.undefined) {
            return Some(self.heads.undefined);
        }
        if args.iter().any(|arg| *arg == self.heads.failed) {
            return Some(self.heads.failed);
        }
        None
    }
}

fn head(arena: &mut Arena, name: &str) -> ExprId {
    let attrs = builtins::attributes(name).unwrap_or_else(Attributes::new);
    let symbol = arena.intern_symbol(name, attrs);
    arena.symbol_expr(symbol)
}

fn gaussian_pow(real: &Rat, imaginary: &Rat, power: i64) -> Option<(Rat, Rat)> {
    if power == 0 {
        return Some((Rat::one(), Rat::zero()));
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut base = (real.clone(), imaginary.clone());
    let mut result = (Rat::one(), Rat::zero());
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = gaussian_mul(&result, &base);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = gaussian_mul(&base, &base);
        }
    }
    if negative {
        let norm = result.0.mul(&result.0).add(&result.1.mul(&result.1));
        if norm.is_zero() {
            return None;
        }
        let inverse = Rat::one().div(&norm).ok()?;
        Some((result.0.mul(&inverse), result.1.neg().mul(&inverse)))
    } else {
        Some(result)
    }
}

fn gaussian_mul(left: &(Rat, Rat), right: &(Rat, Rat)) -> (Rat, Rat) {
    let real = left.0.mul(&right.0).sub(&left.1.mul(&right.1));
    let imaginary = left.0.mul(&right.1).add(&left.1.mul(&right.0));
    (real, imaginary)
}

fn rat_pow(base: &Rat, power: i64) -> Result<Rat, ()> {
    if power == 0 {
        return Ok(Rat::one());
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut result = Rat::one();
    let mut factor = base.clone();
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.mul(&factor);
        }
        exponent >>= 1;
        if exponent > 0 {
            factor = factor.mul(&factor);
        }
    }
    if negative {
        if result.is_zero() {
            return Err(());
        }
        Rat::one().div(&result).map_err(|_| ())
    } else {
        Ok(result)
    }
}
