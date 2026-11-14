//! Public session and expression handle API.

use std::collections::HashMap;

use crate::config::Config;
use crate::diag::{Diagnostic, ErrorCode, Span};
use crate::eval::{SessionDefinitions, evaluate_with_definitions};
use crate::expr::{Arena, ExprId, Node, SymbolId};
use crate::hash::FnvBuildHasher;
use crate::num::{Int, Rat};
use crate::print;
use crate::runtime;
use crate::syntax::{self, Case, ExprSource, PatternSource, Stmt};

/// A view of an expression.
#[derive(Clone, Copy, Debug)]
pub enum ExprView<'a> {
    /// An exact integer.
    Integer(&'a Int),
    /// An exact rational.
    Rational(&'a Rat),
    /// An approximate real.
    Real(&'a crate::num::Real),
    /// A symbol.
    Symbol(&'a str),
    /// A call.
    Call {
        /// The head expression.
        head: ExprId,
        /// The argument expressions.
        args: &'a [ExprId],
    },
}

/// A symbolic math session.
pub struct Session {
    arena: Arena,
    definitions: SessionDefinitions,
    last_result: Option<ExprId>,
    parse_cache: HashMap<String, ExprId, FnvBuildHasher>,
    compiled_cache: HashMap<ExprId, runtime::Code, FnvBuildHasher>,
    eval_cache: HashMap<(ExprId, u64), ExprId, FnvBuildHasher>,
    compiled_results: HashMap<(ExprId, u64), ExprId, FnvBuildHasher>,
    output: Vec<String>,
    language_scopes: Vec<HashMap<SymbolId, ExprId, FnvBuildHasher>>,
    config: Config,
}

impl Session {
    /// Create a session with default limits.
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    /// Create a session with explicit limits.
    pub fn with_config(config: Config) -> Self {
        Session {
            arena: Arena::new(),
            definitions: SessionDefinitions::new(),
            last_result: None,
            parse_cache: HashMap::default(),
            compiled_cache: HashMap::default(),
            eval_cache: HashMap::default(),
            compiled_results: HashMap::default(),
            output: Vec::new(),
            language_scopes: Vec::new(),
            config,
        }
    }

    /// Parse a single statement.
    pub fn parse_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        if let Some(expression) = self.parse_cache.get(source) {
            return Ok(*expression);
        }
        let expression = syntax::parse_statement(&mut self.arena, source)?;
        self.parse_cache.insert(source.to_string(), expression);
        Ok(expression)
    }

    /// Parse one expression per statement.
    pub fn parse_source(&mut self, source: &str) -> Result<Vec<ExprId>, Diagnostic> {
        syntax::parse_source(&mut self.arena, source)
    }

    /// Parse and evaluate a single statement.
    pub fn eval_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let expression = self.parse_statement(source)?;
        self.eval(expression)
    }

    /// Parse and evaluate a complete surface language module.
    pub fn eval_language(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let program = syntax::parse_program(source)?;
        self.language_scopes.push(HashMap::default());
        let result = self.eval_language_statements(&program.body);
        self.language_scopes.pop();
        result
    }

    /// Printed output produced by `prt` statements.
    pub fn output(&self) -> &[String] {
        &self.output
    }

    /// Take the printed output produced by `prt` statements.
    pub fn take_output(&mut self) -> Vec<String> {
        core::mem::take(&mut self.output)
    }

    fn eval_language_statements(&mut self, statements: &[Stmt]) -> Result<ExprId, Diagnostic> {
        let mut last = None;
        for statement in statements {
            last = Some(self.eval_language_statement(statement)?);
        }
        Ok(last.unwrap_or_else(|| self.arena.builtin_head("null")))
    }

    fn eval_language_statement(&mut self, statement: &Stmt) -> Result<ExprId, Diagnostic> {
        match statement {
            Stmt::Def {
                name,
                parameters,
                guard,
                body,
            } => {
                let blank_head = self.arena.builtin_head("blank");
                let blank = self.arena.intern_call(blank_head, &[]);
                let pattern_head = self.arena.builtin_head("pattern");
                let mut patterns = Vec::with_capacity(parameters.len());
                let mut parameter_symbols = Vec::new();
                for parameter in parameters {
                    match parameter {
                        PatternSource::Name(parameter) => {
                            let symbol = self.arena.symbol_or_intern(parameter);
                            parameter_symbols.push(symbol);
                            let name = self.arena.symbol_expr(symbol);
                            patterns.push(self.arena.intern_call(pattern_head, &[name, blank]));
                        }
                        PatternSource::Literal(source) => {
                            patterns.push(self.language_expression(source)?);
                        }
                    }
                }
                let body = self.resolved_language_expression(body, &parameter_symbols)?;
                let guard = if let Some(guard) = guard {
                    Some(self.resolved_language_expression(guard, &parameter_symbols)?)
                } else {
                    None
                };
                let symbol = self.arena.symbol_or_intern(name);
                let head = self.arena.symbol_expr(symbol);
                let mut lhs = self.arena.intern_call(head, &patterns);
                if let Some(guard) = guard {
                    let condition = self.arena.builtin_head("condition");
                    lhs = self.arena.intern_call(condition, &[lhs, guard]);
                }
                self.definitions.define(&self.arena, lhs, body, true)?;
                Ok(self.arena.builtin_head("null"))
            }
            Stmt::Let { name, body } => {
                let value = self.resolved_language_expression(body, &[])?;
                let value = self.eval(value)?;
                let symbol = self.arena.symbol_or_intern(name);
                if let Some(scope) = self.language_scopes.last_mut() {
                    scope.insert(symbol, value);
                } else {
                    let mut scope = HashMap::default();
                    scope.insert(symbol, value);
                    self.language_scopes.push(scope);
                }
                Ok(value)
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition = self.resolved_language_expression(condition, &[])?;
                let condition = self.eval(condition)?;
                let truth = self.language_boolean(condition)?;
                self.language_scopes.push(HashMap::default());
                let result = if truth {
                    self.eval_language_statements(then_body)
                } else {
                    self.eval_language_statements(else_body)
                };
                self.language_scopes.pop();
                result
            }
            Stmt::Match {
                scrutinee,
                cases,
                else_body,
            } => {
                let scrutinee = self.resolved_language_expression(scrutinee, &[])?;
                let scrutinee = self.eval(scrutinee)?;
                for case in cases {
                    if self.eval_case(case, scrutinee)? {
                        let result = self.eval_language_statements(&case.body);
                        self.language_scopes.pop();
                        return result;
                    }
                }
                self.language_scopes.push(HashMap::default());
                let result = self.eval_language_statements(else_body);
                self.language_scopes.pop();
                result
            }
            Stmt::Print { body } => {
                let value = self.resolved_language_expression(body, &[])?;
                let value = self.eval(value)?;
                self.output.push(self.format_infix(value));
                Ok(value)
            }
        }
    }

    fn eval_case(&mut self, case: &Case, scrutinee: ExprId) -> Result<bool, Diagnostic> {
        self.language_scopes.push(HashMap::default());
        let matched = match &case.pattern {
            PatternSource::Name(name) => {
                let symbol = self.arena.symbol_or_intern(name);
                if let Some(scope) = self.language_scopes.last_mut() {
                    scope.insert(symbol, scrutinee);
                }
                true
            }
            PatternSource::Literal(source) => {
                let literal = self.language_expression(source)?;
                let literal = self.eval(literal)?;
                literal == scrutinee
            }
        };
        if !matched {
            self.language_scopes.pop();
        }
        Ok(matched)
    }

    fn language_expression(&mut self, source: &ExprSource) -> Result<ExprId, Diagnostic> {
        match syntax::parse_statement(&mut self.arena, &source.text) {
            Ok(expression) => Ok(expression),
            Err(mut diagnostic) => {
                if let Some(span) = diagnostic.span.as_mut() {
                    if span.line == 1 {
                        span.line = source.line;
                        span.column = span.column.saturating_add(source.column - 1);
                    }
                }
                Err(diagnostic)
            }
        }
    }

    fn resolved_language_expression(
        &mut self,
        source: &ExprSource,
        excluded: &[SymbolId],
    ) -> Result<ExprId, Diagnostic> {
        let expression = self.language_expression(source)?;
        Ok(self.resolve_expression(expression, excluded))
    }

    fn resolve_expression(&mut self, expression: ExprId, excluded: &[SymbolId]) -> ExprId {
        match self.arena.node(expression) {
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                if excluded.contains(&symbol) {
                    return expression;
                }
                for scope in self.language_scopes.iter().rev() {
                    if let Some(value) = scope.get(&symbol) {
                        return *value;
                    }
                }
                expression
            }
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                let head = self.resolve_expression(head, excluded);
                let mut resolved = Vec::with_capacity(args.len());
                for arg in args {
                    resolved.push(self.resolve_expression(arg, excluded));
                }
                self.arena.intern_call(head, &resolved)
            }
            _ => expression,
        }
    }

    fn language_boolean(&self, value: ExprId) -> Result<bool, Diagnostic> {
        if let Node::Symbol(symbol) = self.arena.node(value) {
            match self.arena.symbol_name(*symbol) {
                "true" => return Ok(true),
                "false" => return Ok(false),
                _ => {}
            }
        }
        Err(Diagnostic::eval(
            crate::diag::ErrorCode::EvalDefinition,
            "a conditional requires true or false",
        ))
    }

    /// Parse and evaluate every statement, returning the final value.
    pub fn eval_source(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        if syntax::looks_like_language(source) {
            return self.eval_language(source);
        }
        let statements = self.parse_source(source)?;
        if statements.is_empty() {
            return Err(Diagnostic::parse(
                ErrorCode::ParseUnexpectedToken,
                "expected a statement",
                Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            ));
        }
        let mut last = None;
        for statement in statements {
            last = Some(self.eval(statement)?);
        }
        Ok(last.expect("statements is not empty"))
    }

    /// Evaluate an expression to a normal form.
    pub fn eval(&mut self, expression: ExprId) -> Result<ExprId, Diagnostic> {
        let version = self.definitions.version();
        if let Some(value) = self.eval_cache.get(&(expression, version)) {
            self.last_result = Some(*value);
            return Ok(*value);
        }
        let value = evaluate_with_definitions(
            &mut self.arena,
            &mut self.definitions,
            expression,
            self.config.max_steps,
            self.config.max_depth,
        )?;
        self.eval_cache.insert((expression, version), value);
        self.last_result = Some(value);
        self.maybe_collect();
        Ok(value)
    }

    /// Evaluate an expression through the compiled instruction path.
    pub fn eval_compiled(&mut self, expression: ExprId) -> Result<ExprId, Diagnostic> {
        let version = self.definitions.version();
        if let Some(value) = self.compiled_results.get(&(expression, version)) {
            self.last_result = Some(*value);
            return Ok(*value);
        }
        let code = if let Some(code) = self.compiled_cache.get(&expression) {
            code.clone()
        } else {
            let code = runtime::compile(&mut self.arena, expression)?;
            self.compiled_cache.insert(expression, code.clone());
            code
        };
        let value = runtime::execute(
            &mut self.arena,
            &mut self.definitions,
            &code,
            self.config.max_steps,
            self.config.max_depth,
        )?;
        self.compiled_results.insert((expression, version), value);
        self.last_result = Some(value);
        self.maybe_collect();
        Ok(value)
    }

    fn maybe_collect(&mut self) {
        let threshold = (self.config.gc_threshold_bytes / 64).max(64);
        if self.arena.len() > threshold {
            self.collect_garbage(&[]);
        }
    }

    /// Parse and evaluate a single statement through the compiled path.
    pub fn eval_compiled_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        let expression = self.parse_statement(source)?;
        self.eval_compiled(expression)
    }

    /// Render an expression in canonical form.
    pub fn format(&self, expression: ExprId) -> String {
        print::canonical(&self.arena, expression)
    }

    /// Render an expression in human surface form.
    pub fn format_infix(&self, expression: ExprId) -> String {
        print::infix(&self.arena, expression)
    }

    /// Inspect an expression.
    pub fn view(&self, expression: ExprId) -> ExprView<'_> {
        match self.arena.node(expression) {
            Node::Int(value) => ExprView::Integer(value),
            Node::Rat(value) => ExprView::Rational(value),
            Node::Real(value) => ExprView::Real(value),
            Node::Symbol(symbol) => ExprView::Symbol(self.arena.symbol_name(*symbol)),
            Node::Call { head, args } => ExprView::Call {
                head: *head,
                args: args.as_ref(),
            },
        }
    }

    /// Reclaim expressions not reachable from definitions, the last result,
    /// or the supplied roots.
    pub fn collect_garbage(&mut self, roots: &[ExprId]) {
        let mut all = roots.to_vec();
        if let Some(last) = self.last_result {
            all.push(last);
        }
        self.definitions.roots(&mut all);
        self.arena.collect(&all);
        self.parse_cache.clear();
        self.compiled_cache.clear();
        self.eval_cache.clear();
        self.compiled_results.clear();
    }

    /// Number of live expression nodes in the session arena.
    pub fn live_expressions(&self) -> usize {
        self.arena.len()
    }

    /// The value of the most recent evaluation.
    pub fn last_result(&self) -> Option<ExprId> {
        self.last_result
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
