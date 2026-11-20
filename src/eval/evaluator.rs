//! Evaluator state and rewrite methods.

#[allow(unused_imports)]
use crate::prelude::*;

use super::assumptions;
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Control {
    Return(ExprId),
    Break,
    Continue,
    Throw(ExprId, Option<ExprId>),
}

pub(crate) struct Evaluator<'a> {
    pub(super) arena: &'a mut Arena,
    pub(super) definitions: &'a mut Definitions,
    pub(super) heads: Heads,
    pub(super) locals: Vec<HashMap<SymbolId, ExprId>>,
    pub(super) assumptions: Vec<Assumption>,
    pub(super) steps: u64,
    pub(super) max_steps: u64,
    pub(super) depth: u32,
    pub(super) max_depth: u32,
    pub(super) approximation_digits: Option<u32>,
    pub(super) control: Option<Control>,
}

impl<'a> Evaluator<'a> {
    pub(super) fn new(arena: &'a mut Arena, definitions: &'a mut Definitions) -> Self {
        let heads = arena.heads();
        Evaluator {
            arena,
            definitions,
            heads,
            locals: Vec::new(),
            assumptions: Vec::new(),
            steps: 0,
            max_steps: 1_000_000,
            depth: 0,
            max_depth: 2048,
            approximation_digits: None,
            control: None,
        }
    }

    pub(super) fn eval(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        if self.control.is_some() {
            return Ok(self.heads.null);
        }
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

    pub(super) fn eval_inner(&mut self, id: ExprId) -> Result<ExprId, Diagnostic> {
        let (head, args) = match self.arena.node(id) {
            Node::Int(_)
            | Node::Rat(_)
            | Node::Real(_)
            | Node::String(_)
            | Node::Bytes(_)
            | Node::Association(_) => return Ok(id),
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if let Some(value) = self.local_value(symbol) {
                    return Ok(value);
                }
                if let Some(own) = self.definitions.own_value(symbol) {
                    return self.eval(own.value);
                }
                if self.arena.symbol_name(symbol) == "$Context" {
                    let context = self.arena.current_context().to_string();
                    return Ok(self.arena.intern_string(&context));
                }
                if self.arena.symbol_name(symbol) == "$ContextPath" {
                    let path = self.arena.context_path().to_vec();
                    let mut values = Vec::with_capacity(path.len());
                    for context in path {
                        values.push(self.arena.intern_string(&context));
                    }
                    return Ok(self.arena.intern_call(self.heads.list, &values));
                }
                return Ok(id);
            }
            Node::Call { head, args } => (*head, SmallArgs::from_slice(args)),
        };
        let head = if let Node::Symbol(symbol) = self.arena.node(head) {
            let symbol = *symbol;
            if self.local_value(symbol).is_none() && self.definitions.own_value(symbol).is_none() {
                head
            } else {
                self.eval(head)?
            }
        } else {
            self.eval(head)?
        };
        let closure = match self.arena.node(head) {
            Node::Call {
                head: closure_head,
                args,
            } if *closure_head == self.heads.closure => Some(args.to_vec()),
            _ => None,
        };
        if let Some(closure) = closure {
            return self.apply_closure(&closure, args.as_slice());
        }
        if !matches!(self.arena.node(head), Node::Symbol(_)) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalNotCallable,
                "a call head must evaluate to a symbol",
            ));
        }

        if head == self.heads.list
            && args.as_slice().iter().all(|arg| {
                matches!(
                    self.arena.node(*arg),
                    Node::Int(_) | Node::Rat(_) | Node::Real(_)
                )
            })
        {
            return Ok(id);
        }
        if head == self.heads.begin {
            return self.eval_begin(args.as_slice());
        }
        if head == self.heads.end {
            return self.eval_end(args.as_slice());
        }
        if head == self.heads.context {
            return self.eval_context(args.as_slice());
        }
        if head == self.heads.contexts {
            return self.eval_contexts(args.as_slice());
        }
        if head == self.heads.hold {
            return self.eval_hold(args.as_slice());
        }
        if head == self.heads.unevaluated {
            if args.as_slice().len() != 1 {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalArity,
                    "unevaluated expects one argument",
                ));
            }
            return Ok(args.as_slice()[0]);
        }
        if head == self.heads.compound {
            return self.eval_compound(args.as_slice());
        }
        if head == self.heads.which {
            return self.eval_which(args.as_slice());
        }
        if head == self.heads.switch {
            return self.eval_switch(args.as_slice());
        }
        if head == self.heads.do_loop {
            return self.eval_do(args.as_slice());
        }
        if head == self.heads.while_loop {
            return self.eval_while(args.as_slice());
        }
        if head == self.heads.for_loop {
            return self.eval_for(args.as_slice());
        }
        if head == self.heads.return_form {
            return self.eval_return(args.as_slice());
        }
        if head == self.heads.break_form {
            return self.eval_break(args.as_slice());
        }
        if head == self.heads.continue_form {
            return self.eval_continue(args.as_slice());
        }
        if head == self.heads.throw_form {
            return self.eval_throw(args.as_slice());
        }
        if head == self.heads.catch_form {
            return self.eval_catch(args.as_slice());
        }
        if head == self.heads.unset {
            return self.eval_unset(args.as_slice());
        }
        if head == self.heads.clear || head == self.heads.clear_all || head == self.heads.remove {
            return self.eval_clear(head, args.as_slice());
        }
        if head == self.heads.message {
            return self.eval_message(args.as_slice());
        }
        if head == self.heads.check {
            return self.eval_check(args.as_slice());
        }
        if head == self.heads.quiet {
            return self.eval_quiet(args.as_slice());
        }
        if head == self.heads.off || head == self.heads.on {
            return self.eval_message_control(head == self.heads.on, args.as_slice());
        }
        if head == self.heads.up_set
            || head == self.heads.up_set_delayed
            || head == self.heads.tag_set
            || head == self.heads.tag_set_delayed
        {
            return self.eval_up_definition(head, args.as_slice());
        }
        if head == self.heads.set_attributes {
            return self.eval_set_attributes(args.as_slice(), false);
        }
        if head == self.heads.clear_attributes {
            return self.eval_set_attributes(args.as_slice(), true);
        }
        if head == self.heads.attributes {
            return self.eval_attributes(args.as_slice());
        }
        if head == self.heads.protect || head == self.heads.unprotect {
            return self.eval_protect(head, args.as_slice());
        }
        if head == self.heads.own_values
            || head == self.heads.down_values
            || head == self.heads.sub_values
            || head == self.heads.up_values
        {
            return self.eval_values(head, args.as_slice());
        }
        if head == self.heads.definition || head == self.heads.full_definition {
            return self.eval_definition_info(args.as_slice());
        }
        if self.is_mutation_head(head) {
            return self.eval_mutation(head, args.as_slice());
        }
        if head == self.heads.assume || head == self.heads.assuming {
            return self.eval_assume(args.as_slice());
        }
        if head == self.heads.assumptions_head {
            return self.eval_assumptions(args.as_slice());
        }
        if head == self.heads.function {
            return self.eval_function(args.as_slice());
        }
        if head == self.heads.approx {
            return self.eval_approx(args.as_slice());
        }
        if head == self.heads.interval {
            return self.eval_interval(args.as_slice());
        }
        if head == self.heads.complex {
            return self.eval_complex(args.as_slice());
        }
        if head == self.heads.map {
            return self.eval_map(head, args.as_slice());
        }
        if head == self.heads.apply {
            return self.eval_apply(head, args.as_slice());
        }
        if head == self.heads.fold {
            return self.eval_fold(head, args.as_slice());
        }
        if head == self.heads.nest {
            return self.eval_nest(head, args.as_slice());
        }
        if head == self.heads.set {
            return self.eval_definition(args.as_slice(), false);
        }
        if head == self.heads.set_delayed {
            return self.eval_definition(args.as_slice(), true);
        }
        if head == self.heads.let_head {
            return self.eval_let(args.as_slice());
        }
        if head == self.heads.local_defs {
            return self.eval_local_defs(args.as_slice());
        }
        if head == self.heads.forall || head == self.heads.exists || head == self.heads.set_builder
        {
            return self.eval_quantifier(head, args.as_slice());
        }
        if head == self.heads.if_head {
            return self.eval_if(args.as_slice());
        }
        if head == self.heads.and {
            return self.eval_and(args.as_slice());
        }
        if head == self.heads.or {
            return self.eval_or(args.as_slice());
        }
        if head == self.heads.not {
            return self.eval_not(args.as_slice());
        }

        let attrs = self.arena.head_attributes(head);
        if attrs.has(Attributes::HOLD_ALL)
            || attrs.has(Attributes::HOLD_FIRST)
            || attrs.has(Attributes::HOLD_REST)
        {
            return Ok(id);
        }

        let mut evaluated = SmallArgs::new();
        let sequence_hold = attrs.has(Attributes::SEQUENCE_HOLD);
        for arg in args.as_slice() {
            if let Node::Call { head, args: inner } = self.arena.node(*arg)
                && *head == self.heads.unevaluated
                && inner.len() == 1
            {
                evaluated.push(inner[0]);
                continue;
            }
            let arg = self.expand_series_sums(*arg);
            let value = self.eval(arg)?;
            if !sequence_hold
                && let Node::Call {
                    head,
                    args: sequence,
                } = self.arena.node(value)
                && *head == self.heads.sequence
            {
                let sequence = sequence.to_vec();
                for item in sequence {
                    evaluated.push(self.eval(item)?);
                }
            } else {
                evaluated.push(value);
            }
        }

        if head == self.heads.plus || head == self.heads.times || head == self.heads.negate {
            let candidate = self.arena.intern_call(head, evaluated.as_slice());
            if let Some((real, imaginary)) = self.gaussian(candidate) {
                return Ok(self.gaussian_node(&real, &imaginary));
            }
        }

        if attrs.has(Attributes::LISTABLE)
            && let Some(result) = self.thread_listable(head, evaluated.as_slice())?
        {
            return self.finish(id, result);
        }

        let result = self.apply(head, evaluated.as_slice())?;
        self.finish(id, result)
    }

    pub(super) fn eval_assume(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "assume expects an assumption and an expression",
            ));
        }
        let Some(formula) = assumptions::parse(self.arena, args[0]) else {
            return Ok(self.arena.intern_call(self.heads.assume, args));
        };
        let mut candidate = self.assumptions.clone();
        candidate.push(Assumption {
            formula,
            source: args[0],
        });
        if !assumptions::is_consistent(&candidate) {
            return Err(Diagnostic::eval(
                ErrorCode::EvalAssumption,
                "assumptions are inconsistent",
            ));
        }
        let length = self.assumptions.len();
        self.assumptions = candidate;
        let result = self.eval(args[1]);
        self.assumptions.truncate(length);
        result
    }

    pub(super) fn eval_assumptions(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() > 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "assumptions expects zero or one argument",
            ));
        }
        let symbol = if args.is_empty() {
            None
        } else {
            Some(self.arena.symbol_id(args[0]).ok_or_else(|| {
                Diagnostic::eval(ErrorCode::EvalDefinition, "assumptions expects a symbol")
            })?)
        };
        let sources = assumptions::sources(&self.assumptions, symbol);
        Ok(self.arena.intern_call(self.heads.list, &sources))
    }

    pub(super) fn expression_sign(&self, expression: ExprId) -> Option<Sign> {
        match self.arena.node(expression) {
            Node::Int(value) => Some(sign_of(value.sign())),
            Node::Rat(value) => Some(sign_of(value.sign())),
            Node::Real(value) => Some(sign_of(value.sign())),
            Node::String(_) | Node::Bytes(_) | Node::Association(_) => None,
            Node::Symbol(symbol) => {
                if let Some(sign) = assumptions::sign(&self.assumptions, *symbol) {
                    return Some(sign);
                }
                match self.arena.symbol_name(*symbol) {
                    "pi" | "e" => Some(Sign::Positive),
                    _ => None,
                }
            }
            Node::Call { head, args } => {
                let name = self
                    .arena
                    .symbol_id(*head)
                    .map(|symbol| self.arena.symbol_name(symbol))?;
                match name {
                    "negate" if args.len() == 1 => self.expression_sign(args[0]).map(flip_sign),
                    "abs" if args.len() == 1 => Some(Sign::NonNegative),
                    "sqrt" if args.len() == 1 => {
                        if self.expression_sign(args[0]) == Some(Sign::Positive) {
                            Some(Sign::Positive)
                        } else {
                            Some(Sign::NonNegative)
                        }
                    }
                    "exp" if args.len() == 1 => Some(Sign::Positive),
                    "times" => {
                        let mut exact = Sign::Positive;
                        for arg in args.iter() {
                            exact = multiply_signs(exact, self.expression_sign(*arg)?)?;
                        }
                        Some(exact)
                    }
                    "power" if args.len() == 2 => {
                        let exponent = match self.arena.node(args[1]) {
                            Node::Int(value) => value.to_i64()?,
                            _ => return None,
                        };
                        let base = self.expression_sign(args[0])?;
                        if exponent % 2 == 0 {
                            Some(match base {
                                Sign::Zero => Sign::Zero,
                                Sign::Negative | Sign::Positive => Sign::Positive,
                                Sign::NonNegative | Sign::NonPositive => Sign::NonNegative,
                            })
                        } else {
                            Some(base)
                        }
                    }
                    "plus" => {
                        let mut all_nonnegative = true;
                        let mut all_nonpositive = true;
                        for arg in args.iter() {
                            let sign = self.expression_sign(*arg)?;
                            all_nonnegative &=
                                matches!(sign, Sign::Positive | Sign::Zero | Sign::NonNegative);
                            all_nonpositive &=
                                matches!(sign, Sign::Negative | Sign::Zero | Sign::NonPositive);
                        }
                        if all_nonnegative {
                            Some(Sign::NonNegative)
                        } else if all_nonpositive {
                            Some(Sign::NonPositive)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        }
    }

    pub(super) fn exact_constant_value(&self, expression: ExprId) -> Option<Rat> {
        match self.arena.node(expression) {
            Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            Node::Rat(value) => Some(value.clone()),
            _ => None,
        }
    }

    pub(super) fn simplify_abs(&mut self, argument: ExprId) -> Option<ExprId> {
        match self.expression_sign(argument) {
            Some(Sign::Zero) => Some(self.arena.intern_int(Int::zero())),
            Some(Sign::Positive) | Some(Sign::NonNegative) => Some(argument),
            Some(Sign::Negative) | Some(Sign::NonPositive) => {
                Some(self.arena.intern_call(self.heads.negate, &[argument]))
            }
            None => None,
        }
    }

    pub(super) fn simplify_sqrt(&mut self, argument: ExprId) -> Option<ExprId> {
        if let Node::Call { head, args } = self.arena.node(argument)
            && args.len() == 2
            && self
                .arena
                .symbol_id(*head)
                .map(|symbol| self.arena.symbol_name(symbol))
                == Some("power")
            && let Node::Int(exponent) = self.arena.node(args[1])
            && exponent.to_i64() == Some(2)
        {
            let base = args[0];
            return match self.expression_sign(base) {
                Some(Sign::NonNegative) | Some(Sign::Positive) | Some(Sign::Zero) => Some(base),
                Some(Sign::NonPositive) | Some(Sign::Negative) => {
                    Some(self.arena.intern_call(self.heads.negate, &[base]))
                }
                None => Some(self.arena.intern_call(self.heads.abs, &[base])),
            };
        }
        if let Node::Int(value) = self.arena.node(argument)
            && let Some(root) = integer_sqrt(value)
        {
            return Some(self.arena.intern_int(root));
        }
        None
    }

    pub(super) fn expand_series_sums(&mut self, expression: ExprId) -> ExprId {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return expression;
        };
        if *head == self.heads.series_sum {
            let args = args.to_vec();
            return self.arena.intern_call(self.heads.plus, &args);
        }
        let head = *head;
        let args = args.to_vec();
        let mut changed = false;
        let normalized = args
            .into_iter()
            .map(|arg| {
                let value = self.expand_series_sums(arg);
                changed |= value != arg;
                value
            })
            .collect::<Vec<_>>();
        if changed {
            self.arena.intern_call(head, &normalized)
        } else {
            expression
        }
    }

    pub(super) fn finish(
        &mut self,
        original: ExprId,
        result: ExprId,
    ) -> Result<ExprId, Diagnostic> {
        if result == original {
            Ok(original)
        } else {
            self.eval(result)
        }
    }

    pub(super) fn local_value(&self, symbol: SymbolId) -> Option<ExprId> {
        for frame in self.locals.iter().rev() {
            if let Some(value) = frame.get(&symbol) {
                return Some(*value);
            }
        }
        None
    }

    pub(super) fn eval_function(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "fn expects parameters and a body",
            ));
        }
        self.parameter_list(args[0])?;
        let captured: Vec<(SymbolId, ExprId)> = self
            .locals
            .iter()
            .flat_map(|frame| frame.iter().map(|(symbol, value)| (*symbol, *value)))
            .collect();
        let mut bindings = Vec::with_capacity(captured.len());
        for (symbol, value) in captured {
            let name = self.arena.symbol_expr(symbol);
            bindings.push(self.arena.intern_call(self.heads.set, &[name, value]));
        }
        let environment = self.arena.intern_call(self.heads.list, &bindings);
        Ok(self
            .arena
            .intern_call(self.heads.closure, &[args[0], args[1], environment]))
    }

    pub(super) fn apply_closure(
        &mut self,
        closure: &[ExprId],
        call_args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        if closure.len() != 3 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "malformed closure value",
            ));
        }
        let params = self.parameter_list(closure[0])?;
        if params.len() != call_args.len() {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "closure called with the wrong number of arguments",
            ));
        }
        let mut frame = HashMap::new();
        self.read_environment(closure[2], &mut frame)?;
        for (symbol, argument) in params.into_iter().zip(call_args.iter()) {
            let value = self.eval(*argument)?;
            frame.insert(symbol, value);
        }
        let outer = core::mem::take(&mut self.locals);
        self.locals.push(frame);
        let result = self.eval(closure[1]);
        self.locals = outer;
        if let Some(value) = self.take_return() {
            return Ok(value);
        }
        result
    }

    pub(super) fn parameter_list(&self, expression: ExprId) -> Result<Vec<SymbolId>, Diagnostic> {
        let args = match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => args.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be a list",
                ));
            }
        };
        let mut symbols = Vec::with_capacity(args.len());
        let mut seen = HashSet::new();
        for arg in args {
            let Some(symbol) = self.arena.symbol_id(arg) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be symbols",
                ));
            };
            if !seen.insert(symbol) {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "function parameters must be distinct",
                ));
            }
            symbols.push(symbol);
        }
        Ok(symbols)
    }

    pub(super) fn read_environment(
        &self,
        expression: ExprId,
        frame: &mut HashMap<SymbolId, ExprId>,
    ) -> Result<(), Diagnostic> {
        let bindings = match self.arena.node(expression) {
            Node::Call { head, args } if *head == self.heads.list => args.to_vec(),
            _ => {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "closure environment must be a list",
                ));
            }
        };
        for binding in bindings {
            let (name, value) = match self.arena.node(binding) {
                Node::Call { head, args } if *head == self.heads.set && args.len() == 2 => {
                    (args[0], args[1])
                }
                _ => {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalDefinition,
                        "closure environment binding is malformed",
                    ));
                }
            };
            let Some(symbol) = self.arena.symbol_id(name) else {
                return Err(Diagnostic::eval(
                    ErrorCode::EvalDefinition,
                    "closure environment name must be a symbol",
                ));
            };
            frame.insert(symbol, value);
        }
        Ok(())
    }
}

pub(crate) fn fold_numeric_call(arena: &mut Arena, id: ExprId) -> Option<ExprId> {
    match arena.node(id) {
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => Some(id),
        Node::Call { head, args } => {
            let symbol = arena.symbol_id(*head)?;
            let name = arena.symbol_name(symbol);
            if name != "plus" && name != "times" && name != "negate" {
                return None;
            }
            let args = args.as_ref();
            if args.is_empty() {
                return None;
            }
            if name == "negate" {
                if args.len() != 1 {
                    return None;
                }
                return match arena.node(args[0]) {
                    Node::Int(value) => Some(arena.intern_int(value.neg())),
                    Node::Rat(value) => Some(arena.intern_rat(value.neg())),
                    _ => None,
                };
            }
            if args
                .iter()
                .all(|arg| matches!(arena.node(*arg), Node::Int(_)))
            {
                let mut value = if name == "plus" {
                    Int::zero()
                } else {
                    Int::one()
                };
                for arg in args {
                    let Node::Int(number) = arena.node(*arg) else {
                        return None;
                    };
                    value = if name == "plus" {
                        value.add(number)
                    } else {
                        value.mul(number)
                    };
                }
                return Some(arena.intern_int(value));
            }
            let mut value = if name == "plus" {
                Rat::zero()
            } else {
                Rat::one()
            };
            for arg in args {
                let number = match arena.node(*arg) {
                    Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok()?,
                    Node::Rat(value) => value.clone(),
                    _ => return None,
                };
                value = if name == "plus" {
                    value.add(&number)
                } else {
                    value.mul(&number)
                };
            }
            Some(arena.intern_rat(value))
        }
        _ => None,
    }
}
