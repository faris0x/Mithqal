//! Public session and expression handle API.

use std::collections::HashMap;

use crate::config::Config;
use crate::diag::{Diagnostic, ErrorCode, Span};
use crate::eval::{SessionDefinitions, evaluate_with_definitions};
use crate::expr::{Arena, ExprId, Node};
use crate::hash::FnvBuildHasher;
use crate::num::{Int, Rat};
use crate::print;
use crate::runtime;
use crate::syntax;

/// A view of an expression.
#[derive(Clone, Copy, Debug)]
pub enum ExprView<'a> {
    /// An exact integer.
    Integer(&'a Int),
    /// An exact rational.
    Rational(&'a Rat),
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

    /// Parse and evaluate every statement, returning the final value.
    pub fn eval_source(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
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
