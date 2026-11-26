//! Prose module AST.
//!
//! This module defines the statement and expression-source types used by the
//! controlled mathematical prose parser and the evaluator.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::Span;

/// A parsed source expression with its original location.
#[derive(Clone, Debug)]
pub struct ExprSource {
    /// Whitespace free expression text.
    pub text: String,
    /// One based source line.
    pub line: usize,
    /// One based column of the first expression character.
    pub column: usize,
}

/// A parsed module.
#[derive(Clone, Debug)]
pub struct Program {
    /// Module name.
    pub name: String,
    /// Module body.
    pub body: Vec<Stmt>,
}

/// A parsed pattern source.
#[derive(Clone, Debug)]
pub enum PatternSource {
    /// A binding name.
    Name(String),
    /// A literal expression pattern.
    Literal(ExprSource),
}

/// One match case.
#[derive(Clone, Debug)]
pub struct Case {
    /// Case pattern.
    pub pattern: PatternSource,
    /// Case body.
    pub body: Vec<Stmt>,
}

/// A definition body.
#[derive(Clone, Debug)]
pub enum Body {
    /// A single expression on the definition line.
    Expression(ExprSource),
    /// A block of statements ending with one expression.
    Block(Vec<BodyStmt>),
}

/// A statement allowed inside a definition body.
#[derive(Clone, Debug)]
pub enum BodyStmt {
    /// A local function definition.
    Def {
        /// Function name.
        name: String,
        /// Parameter patterns.
        parameters: Vec<PatternSource>,
        /// Optional guard.
        guard: Option<ExprSource>,
        /// Body.
        body: Body,
    },
    /// A lexical binding.
    Let {
        /// Bound name.
        name: String,
        /// Bound expression.
        body: ExprSource,
    },
    /// A conditional.
    If {
        /// Condition expression.
        condition: ExprSource,
        /// Statements taken when true.
        then_body: Vec<BodyStmt>,
        /// Statements taken when false.
        else_body: Vec<BodyStmt>,
    },
    /// The final expression.
    Expr(ExprSource),
}

/// A parsed source statement.
#[derive(Clone, Debug)]
pub enum Stmt {
    /// A definition with named parameters.
    Def {
        /// Function name.
        name: String,
        /// Parameter patterns.
        parameters: Vec<PatternSource>,
        /// Optional guard.
        guard: Option<ExprSource>,
        /// Body.
        body: Body,
    },
    /// A value binding.
    Let {
        /// Bound name.
        name: String,
        /// Bound expression.
        body: ExprSource,
    },
    /// A conditional block.
    If {
        /// Condition expression.
        condition: ExprSource,
        /// Statements taken when true.
        then_body: Vec<Stmt>,
        /// Statements taken when false.
        else_body: Vec<Stmt>,
    },
    /// A pattern match block.
    Match {
        /// Inspected expression.
        scrutinee: ExprSource,
        /// Ordered cases.
        cases: Vec<Case>,
        /// Optional fallback body.
        else_body: Vec<Stmt>,
    },
    /// A print statement.
    Print {
        /// Printed expression.
        body: ExprSource,
    },
    /// A module export list.
    Export {
        /// Exported names.
        names: Vec<String>,
    },
    /// A module dependency.
    Import {
        /// Imported module name.
        module: String,
    },
    /// A module assumption.
    Assume {
        /// Assumed condition.
        condition: ExprSource,
    },
    /// A theorem declaration with a claim and proof skeleton.
    Theorem {
        /// Theorem name.
        name: String,
        /// Claim text.
        claim: ExprSource,
        /// Proof step texts.
        proof: Vec<ExprSource>,
    },
    /// A structure declaration.
    Structure {
        /// Structure name.
        name: String,
        /// Field names.
        fields: Vec<String>,
        /// Named laws.
        laws: Vec<(String, ExprSource)>,
    },
    /// A structure instance.
    Instance {
        /// Instance name.
        name: String,
        /// Structure name.
        structure: String,
        /// Skip law checking.
        assumed: bool,
        /// Field values.
        fields: Vec<(String, ExprSource)>,
    },
}

impl Stmt {
    /// The source location of a statement, when it has an expression.
    pub(crate) fn location(&self) -> Option<(Span, String, Option<String>)> {
        match self {
            Stmt::Def {
                name, guard, body, ..
            } => {
                let (span, expression) = body.location().or_else(|| {
                    guard
                        .as_ref()
                        .map(|guard| (span_of(guard), guard.text.clone()))
                })?;
                Some((span, expression, Some(format!("def {name}"))))
            }
            Stmt::Let { name, body } => Some((
                span_of(body),
                body.text.clone(),
                Some(format!("let {name}")),
            )),
            Stmt::If { condition, .. } => Some((
                span_of(condition),
                condition.text.clone(),
                Some("if".to_string()),
            )),
            Stmt::Match { scrutinee, .. } => Some((
                span_of(scrutinee),
                scrutinee.text.clone(),
                Some("match".to_string()),
            )),
            Stmt::Print { body } => {
                Some((span_of(body), body.text.clone(), Some("prt".to_string())))
            }
            Stmt::Assume { condition } => Some((
                span_of(condition),
                condition.text.clone(),
                Some("assume".to_string()),
            )),
            Stmt::Theorem { claim, .. } => Some((
                span_of(claim),
                claim.text.clone(),
                Some("theorem".to_string()),
            )),
            Stmt::Export { .. } | Stmt::Import { .. } => None,
            Stmt::Structure { laws, .. } => laws.first().map(|(_, body)| {
                (
                    span_of(body),
                    body.text.clone(),
                    Some("structure".to_string()),
                )
            }),
            Stmt::Instance { fields, .. } => fields.first().map(|(_, body)| {
                (
                    span_of(body),
                    body.text.clone(),
                    Some("instance".to_string()),
                )
            }),
        }
    }
}

impl Body {
    pub(crate) fn location(&self) -> Option<(Span, String)> {
        match self {
            Body::Expression(source) => Some((span_of(source), source.text.clone())),
            Body::Block(statements) => statements.first().and_then(BodyStmt::location),
        }
    }
}

impl BodyStmt {
    pub(crate) fn location(&self) -> Option<(Span, String)> {
        match self {
            BodyStmt::Def { guard, body, .. } => body.location().or_else(|| {
                guard
                    .as_ref()
                    .map(|guard| (span_of(guard), guard.text.clone()))
            }),
            BodyStmt::Let { body, .. } => Some((span_of(body), body.text.clone())),
            BodyStmt::If { condition, .. } => Some((span_of(condition), condition.text.clone())),
            BodyStmt::Expr(source) => Some((span_of(source), source.text.clone())),
        }
    }
}

fn span_of(source: &ExprSource) -> Span {
    Span {
        start: 0,
        end: source.text.len(),
        line: source.line,
        column: source.column,
    }
}
