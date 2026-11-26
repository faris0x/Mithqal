//! Source reader for statements, comments, and expressions.
//!
//! The surface grammar is described in `docs/syntax.md`.

#[allow(unused_imports)]
use crate::diag::Diagnostic;

mod language;
mod lexer;
mod parser;
mod prose;

pub use language::{Body, BodyStmt, Case, ExprSource, PatternSource, Program, Stmt};
pub use lexer::{Token, TokenKind, lex};
pub use parser::{parse_source, parse_statement};
pub use prose::{
    ProseClause, ProseClauseKind, ProseDefinition, ProseProgram, ProseStmt, ProseTheorem,
    looks_like_prose_program, lower_prose_program, normalize_formula, parse_prose_program,
};

/// Parse a complete controlled mathematical prose module.
pub fn parse_program(source: &str) -> Result<Program, Diagnostic> {
    let program = prose::parse_prose_program(source)?;
    prose::lower_prose_program(&program)
}
