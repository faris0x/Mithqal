//! Source reader for statements, comments, and expressions.
//!
//! The surface grammar is described in `docs/syntax.md`.

mod lexer;
mod parser;

pub use lexer::{Token, TokenKind, lex};
pub use parser::{parse_source, parse_statement};
