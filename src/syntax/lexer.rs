//! Expression lexer.
//!
//! The prose parser normalizes formulas before passing them here. The lexer
//! still rejects whitespace, which keeps expression tokens unambiguous.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode, Span};

/// A lexical token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    /// A statement separator.
    Newline,
    /// An integer or decimal literal.
    Number(String),
    /// A string literal.
    String(String),
    /// An identifier.
    Ident(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `<|`
    LessBar,
    /// `|>`
    BarGreater,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `_`
    Underscore,
    /// `__`
    Underscore2,
    /// `___`
    Underscore3,
    /// `=`
    Equal,
    /// `:=`
    SetDelayed,
    /// `->`
    Arrow,
    /// `:>`
    RuleDelayed,
    /// `=>`
    FatArrow,
    /// `===`
    Equal3,
    /// `==`
    Equal2,
    /// `=/=`
    NotEqual,
    /// `~`
    Tilde,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `!`
    Not,
}

/// A token with its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// The token kind.
    pub kind: TokenKind,
    /// The token span.
    pub span: Span,
}

/// Lex a complete source string.
pub fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut tokens = Vec::new();
    let mut line_start = 0usize;
    let mut comment_depth = 0usize;

    for (index, raw) in source.split_inclusive('\n').enumerate() {
        let line_number = index + 1;
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line_end = line_start + raw.len();

        if comment_depth > 0 {
            if let Some(after) = scan_comment(line, 0, &mut comment_depth) {
                check_comment_tail(line, after, line_number, line_start)?;
            }
        } else {
            let first = first_non_space(line);
            if first == line.len() {
                push_newline(&mut tokens, line_number, line_start, line, line_end);
                line_start = line_end;
                continue;
            }
            if line[first..].starts_with("(*") {
                let mut depth = 0usize;
                if let Some(after) = scan_comment(line, first, &mut depth) {
                    check_comment_tail(line, after, line_number, line_start)?;
                } else {
                    comment_depth = depth;
                }
                push_newline(&mut tokens, line_number, line_start, line, line_end);
                line_start = line_end;
                continue;
            }
            if first > 0 {
                return Err(Diagnostic::parse(
                    ErrorCode::ParseWhitespace,
                    "whitespace is not allowed inside a statement",
                    span_at(line_number, line_start, 0, 1),
                )
                .with_note("whitespace is allowed only on blank or comment lines"));
            }
            tokenize_line(line, line_number, line_start, &mut tokens)?;
        }

        push_newline(&mut tokens, line_number, line_start, line, line_end);
        line_start = line_end;
    }

    if comment_depth > 0 {
        let line_number = source.lines().count().max(1);
        return Err(Diagnostic::parse(
            ErrorCode::ParseUnclosedComment,
            "comment block is not closed",
            Span {
                start: source.len(),
                end: source.len(),
                line: line_number,
                column: 1,
            },
        ));
    }

    Ok(tokens)
}

fn first_non_space(line: &str) -> usize {
    line.bytes()
        .position(|byte| byte != b' ' && byte != b'\t')
        .unwrap_or(line.len())
}

fn push_newline(
    tokens: &mut Vec<Token>,
    line: usize,
    line_start: usize,
    text: &str,
    line_end: usize,
) {
    tokens.push(Token {
        kind: TokenKind::Newline,
        span: Span {
            start: line_start + text.len(),
            end: line_end,
            line,
            column: text.len() + 1,
        },
    });
}

fn check_comment_tail(
    line: &str,
    after: usize,
    line_number: usize,
    line_start: usize,
) -> Result<(), Diagnostic> {
    let rest = &line[after..];
    if rest.bytes().all(|byte| byte == b' ' || byte == b'\t') {
        return Ok(());
    }
    Err(Diagnostic::parse(
        ErrorCode::ParseInlineComment,
        "comments must occupy their own lines",
        span_at(line_number, line_start, after, 1),
    ))
}

fn scan_comment(line: &str, start: usize, depth: &mut usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut index = start;
    if *depth == 0 {
        if !line[start..].starts_with("(*") {
            return None;
        }
        *depth = 1;
        index = start + 2;
    }
    while index + 1 < bytes.len() {
        if bytes[index] == b'(' && bytes[index + 1] == b'*' {
            *depth += 1;
            index += 2;
            continue;
        }
        if bytes[index] == b'*' && bytes[index + 1] == b')' {
            *depth -= 1;
            index += 2;
            if *depth == 0 {
                return Some(index);
            }
            continue;
        }
        index += 1;
    }
    None
}

fn tokenize_line(
    line: &str,
    line_number: usize,
    line_start: usize,
    tokens: &mut Vec<Token>,
) -> Result<(), Diagnostic> {
    let bytes = line.as_bytes();
    let mut pos = 0usize;

    while pos < bytes.len() {
        let byte = bytes[pos];
        match byte {
            b' ' | b'\t' | b'\r' => {
                return Err(Diagnostic::parse(
                    ErrorCode::ParseWhitespace,
                    "whitespace is not allowed inside a statement",
                    span_at(line_number, line_start, pos, 1),
                ));
            }
            b'0'..=b'9' => {
                let start = pos;
                while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                    pos += 1;
                }
                if pos < bytes.len() && bytes[pos] == b'.' {
                    pos += 1;
                    let fraction_start = pos;
                    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    if fraction_start == pos {
                        return Err(Diagnostic::parse(
                            ErrorCode::ParseInvalidNumber,
                            "a decimal point must be followed by a digit",
                            span_at(line_number, line_start, start, pos - start + 1),
                        ));
                    }
                    if pos < bytes.len() && bytes[pos] == b'.' {
                        return Err(Diagnostic::parse(
                            ErrorCode::ParseInvalidNumber,
                            "a number cannot contain two decimal points",
                            span_at(line_number, line_start, start, pos - start + 1),
                        ));
                    }
                }
                push(
                    tokens,
                    TokenKind::Number(line[start..pos].to_string()),
                    line_number,
                    line_start,
                    start,
                    pos,
                );
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'$' => {
                let start = pos;
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric()
                        || bytes[pos] == b'_'
                        || bytes[pos] == b'$')
                {
                    if bytes[pos] == b'_' {
                        break;
                    }
                    pos += 1;
                }
                if bytes[start] == b'$' && pos == start + 1 {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "a dollar sign must begin a name",
                        span_at(line_number, line_start, start, 1),
                    ));
                }
                push(
                    tokens,
                    TokenKind::Ident(line[start..pos].to_string()),
                    line_number,
                    line_start,
                    start,
                    pos,
                );
            }
            b'_' => {
                let start = pos;
                while pos < bytes.len() && bytes[pos] == b'_' {
                    pos += 1;
                }
                let count = pos - start;
                let kind = match count {
                    1 => TokenKind::Underscore,
                    2 => TokenKind::Underscore2,
                    3 => TokenKind::Underscore3,
                    _ => {
                        return Err(Diagnostic::parse(
                            ErrorCode::ParseUnexpectedCharacter,
                            "a pattern suffix has at most three underscores",
                            span_at(line_number, line_start, start, count),
                        ));
                    }
                };
                push(tokens, kind, line_number, line_start, start, pos);
            }
            b'+' => single(tokens, TokenKind::Plus, line_number, line_start, &mut pos),
            b'*' => single(tokens, TokenKind::Star, line_number, line_start, &mut pos),
            b'/' => single(tokens, TokenKind::Slash, line_number, line_start, &mut pos),
            b'^' => single(tokens, TokenKind::Caret, line_number, line_start, &mut pos),
            b',' => single(tokens, TokenKind::Comma, line_number, line_start, &mut pos),
            b';' => single(
                tokens,
                TokenKind::Semicolon,
                line_number,
                line_start,
                &mut pos,
            ),
            b'~' => single(tokens, TokenKind::Tilde, line_number, line_start, &mut pos),
            b'(' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'*' {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseInlineComment,
                        "comments must occupy their own lines",
                        span_at(line_number, line_start, pos, 2),
                    ));
                }
                single(tokens, TokenKind::LParen, line_number, line_start, &mut pos);
            }
            b')' => single(tokens, TokenKind::RParen, line_number, line_start, &mut pos),
            b'[' => single(
                tokens,
                TokenKind::LBracket,
                line_number,
                line_start,
                &mut pos,
            ),
            b']' => single(
                tokens,
                TokenKind::RBracket,
                line_number,
                line_start,
                &mut pos,
            ),
            b'!' => single(tokens, TokenKind::Not, line_number, line_start, &mut pos),
            b'"' => {
                let (value, end) = scan_string(line, pos, line_number, line_start)?;
                push(
                    tokens,
                    TokenKind::String(value),
                    line_number,
                    line_start,
                    pos,
                    end,
                );
                pos = end;
            }
            b'<' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'|' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::LessBar,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if pos + 1 < bytes.len() && bytes[pos + 1] == b'=' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::LessEqual,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else {
                    single(tokens, TokenKind::Less, line_number, line_start, &mut pos);
                }
            }
            b'>' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'=' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::GreaterEqual,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else {
                    single(
                        tokens,
                        TokenKind::Greater,
                        line_number,
                        line_start,
                        &mut pos,
                    );
                }
            }
            b'&' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'&' {
                    let start = pos;
                    pos += 2;
                    push(tokens, TokenKind::And, line_number, line_start, start, pos);
                } else {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "expected &&",
                        span_at(line_number, line_start, pos, 1),
                    ));
                }
            }
            b'|' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'>' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::BarGreater,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if pos + 1 < bytes.len() && bytes[pos + 1] == b'|' {
                    let start = pos;
                    pos += 2;
                    push(tokens, TokenKind::Or, line_number, line_start, start, pos);
                } else {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "expected ||",
                        span_at(line_number, line_start, pos, 1),
                    ));
                }
            }
            b'-' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'>' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::Arrow,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else {
                    single(tokens, TokenKind::Minus, line_number, line_start, &mut pos);
                }
            }
            b':' => {
                if pos + 1 < bytes.len() && bytes[pos + 1] == b'=' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::SetDelayed,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if pos + 1 < bytes.len() && bytes[pos + 1] == b'>' {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::RuleDelayed,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "expected := or :>",
                        span_at(line_number, line_start, pos, 1),
                    ));
                }
            }
            b'=' => {
                if line[pos..].starts_with("===") {
                    let start = pos;
                    pos += 3;
                    push(
                        tokens,
                        TokenKind::Equal3,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if line[pos..].starts_with("=/=") {
                    let start = pos;
                    pos += 3;
                    push(
                        tokens,
                        TokenKind::NotEqual,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if line[pos..].starts_with("=>") {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::FatArrow,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else if line[pos..].starts_with("==") {
                    let start = pos;
                    pos += 2;
                    push(
                        tokens,
                        TokenKind::Equal2,
                        line_number,
                        line_start,
                        start,
                        pos,
                    );
                } else {
                    single(tokens, TokenKind::Equal, line_number, line_start, &mut pos);
                }
            }
            _ => {
                return Err(Diagnostic::parse(
                    ErrorCode::ParseUnexpectedCharacter,
                    format!("unexpected character {:?}", byte as char),
                    span_at(line_number, line_start, pos, 1),
                ));
            }
        }
    }
    Ok(())
}

fn single(
    tokens: &mut Vec<Token>,
    kind: TokenKind,
    line: usize,
    line_start: usize,
    pos: &mut usize,
) {
    let start = *pos;
    *pos += 1;
    push(tokens, kind, line, line_start, start, *pos);
}

fn push(
    tokens: &mut Vec<Token>,
    kind: TokenKind,
    line: usize,
    line_start: usize,
    start: usize,
    end: usize,
) {
    tokens.push(Token {
        kind,
        span: Span {
            start: line_start + start,
            end: line_start + end,
            line,
            column: start + 1,
        },
    });
}

fn span_at(line: usize, line_start: usize, column: usize, len: usize) -> Span {
    Span {
        start: line_start + column,
        end: line_start + column + len,
        line,
        column: column + 1,
    }
}

fn scan_string(
    line: &str,
    start: usize,
    line_number: usize,
    line_start: usize,
) -> Result<(String, usize), Diagnostic> {
    let bytes = line.as_bytes();
    let mut position = start + 1;
    let mut value = String::new();
    while position < bytes.len() {
        match bytes[position] {
            b'"' => return Ok((value, position + 1)),
            b'\\' => {
                let Some(escaped) = bytes.get(position + 1).copied() else {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "a string escape is incomplete",
                        span_at(line_number, line_start, position, 1),
                    ));
                };
                match escaped {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'0' => value.push('\0'),
                    _ => {
                        return Err(Diagnostic::parse(
                            ErrorCode::ParseUnexpectedCharacter,
                            "unknown string escape",
                            span_at(line_number, line_start, position, 2),
                        ));
                    }
                }
                position += 2;
            }
            _ => {
                let Some(character) = line[position..].chars().next() else {
                    return Err(Diagnostic::parse(
                        ErrorCode::ParseUnexpectedCharacter,
                        "a string contains invalid text",
                        span_at(line_number, line_start, position, 1),
                    ));
                };
                value.push(character);
                position += character.len_utf8();
            }
        }
    }
    Err(Diagnostic::parse(
        ErrorCode::ParseUnclosedDelimiter,
        "a string is not closed",
        span_at(line_number, line_start, start, 1),
    ))
}
