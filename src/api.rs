//! Public session and expression handle API.

use crate::config::Config;
use crate::diag::{Diagnostic, ErrorCode, Span};
use crate::eval::{SessionDefinitions, evaluate_with_definitions};
use crate::expr::{Arena, ExprId, Node};
use crate::num::{Int, Rat};
use crate::print;
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
    /// An opaque closure value.
    Closure,
}

/// A symbolic math session.
pub struct Session {
    arena: Arena,
    definitions: SessionDefinitions,
    last_result: Option<ExprId>,
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
            config,
        }
    }

    /// Parse a single statement.
    pub fn parse_statement(&mut self, source: &str) -> Result<ExprId, Diagnostic> {
        syntax::parse_statement(&mut self.arena, source)
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
        let value = evaluate_with_definitions(
            &mut self.arena,
            &mut self.definitions,
            expression,
            self.config.max_steps,
            self.config.max_depth,
        )?;
        self.last_result = Some(value);
        Ok(value)
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

    /// Remove unreachable expressions. Version 0.0.1 has no collector yet.
    pub fn collect_garbage(&mut self, _roots: &[ExprId]) {}

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
