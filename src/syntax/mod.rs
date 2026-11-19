//! Source reader for statements, comments, and expressions.
//!
//! The surface grammar is described in `docs/syntax.md`.

#[allow(unused_imports)]
use crate::prelude::*;

mod language;
mod lexer;
mod parser;

pub use language::{
    Body, BodyStmt, Case, ExprSource, PatternSource, Program, Stmt, looks_like_language,
    looks_like_program, parse_program,
};
pub use lexer::{Token, TokenKind, lex};
pub use parser::{parse_source, parse_statement};
