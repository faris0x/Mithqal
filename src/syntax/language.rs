//! Line based surface language with lowercase keywords.
//!
//! The parser handles module structure, definitions, bindings, conditionals,
//! and print statements. Expressions are parsed separately by the existing
//! expression reader and must contain no whitespace.

use crate::diag::{Diagnostic, ErrorCode, Span};

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

/// A source statement.
#[derive(Clone, Debug)]
pub enum Stmt {
    /// A definition with named parameters.
    Def {
        /// Function name.
        name: String,
        /// Parameter names.
        parameters: Vec<String>,
        /// Body expression.
        body: ExprSource,
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
    /// A print statement.
    Print {
        /// Printed expression.
        body: ExprSource,
    },
}

#[derive(Clone, Copy)]
struct Line<'a> {
    number: usize,
    column: usize,
    text: &'a str,
}

/// True when the source starts with a module declaration.
pub fn looks_like_program(source: &str) -> bool {
    first_word(source).is_some_and(|word| word == "mod")
}

/// True when the source starts with any surface language keyword.
pub fn looks_like_language(source: &str) -> bool {
    first_word(source).is_some_and(|word| {
        ["mod", "def", "let", "if", "prt", "end", "else"]
            .iter()
            .any(|keyword| word.eq_ignore_ascii_case(keyword))
    })
}

fn first_word(source: &str) -> Option<&str> {
    for line in source.lines() {
        let text = strip_comment(line).trim();
        if text.is_empty() {
            continue;
        }
        return text.split_ascii_whitespace().next();
    }
    None
}

/// Parse a complete module.
pub fn parse_program(source: &str) -> Result<Program, Diagnostic> {
    if !source.is_ascii() {
        return Err(Diagnostic::parse(
            ErrorCode::ParseUnexpectedCharacter,
            "language source must be ASCII",
            Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        ));
    }
    let mut lines = Vec::new();
    for (index, raw) in source.split_inclusive('\n').enumerate() {
        let without_newline = raw.strip_suffix('\n').unwrap_or(raw);
        let without_comment = strip_comment(without_newline);
        let trimmed = without_comment.trim_end();
        let leading = trimmed.len() - trimmed.trim_start().len();
        let text = trimmed.trim_start();
        if !text.is_empty() {
            lines.push(Line {
                number: index + 1,
                column: leading + 1,
                text,
            });
        }
    }
    let mut cursor = 0usize;
    let Some(first) = lines.get(cursor) else {
        return Err(parse_error(1, 1, "expected a module declaration"));
    };
    let (keyword, rest) = split_keyword(first.text);
    if keyword != "mod" {
        return Err(parse_error(
            first.number,
            first.column,
            "a program must start with mod",
        ));
    }
    let name = rest.trim();
    validate_identifier(name, first.number, first.column + 4)?;
    cursor += 1;
    let (body, terminator) = parse_block(&lines, &mut cursor)?;
    match terminator {
        Terminator::End => {}
        Terminator::Else => {
            return Err(parse_error(
                first.number,
                first.column,
                "unexpected else outside a conditional",
            ));
        }
    }
    if let Some(line) = lines.get(cursor) {
        return Err(parse_error(
            line.number,
            line.column,
            "statements after the module end",
        ));
    }
    Ok(Program {
        name: name.to_string(),
        body,
    })
}

#[derive(Clone, Copy)]
enum Terminator {
    End,
    Else,
}

fn parse_block<'a>(
    lines: &[Line<'a>],
    cursor: &mut usize,
) -> Result<(Vec<Stmt>, Terminator), Diagnostic> {
    let mut body = Vec::new();
    loop {
        let Some(line) = lines.get(*cursor) else {
            let (line, column) = lines
                .last()
                .map(|line| (line.number, line.column + line.text.len()))
                .unwrap_or((1, 1));
            return Err(parse_error(
                line,
                column,
                "expected end before end of source",
            ));
        };
        let (keyword, rest) = split_keyword(line.text);
        match keyword {
            "end" => {
                ensure_empty(rest, line)?;
                *cursor += 1;
                return Ok((body, Terminator::End));
            }
            "else" => {
                ensure_empty(rest, line)?;
                *cursor += 1;
                return Ok((body, Terminator::Else));
            }
            "def" => {
                let start = line.text.len() - rest.len();
                body.push(parse_definition(rest, line, start)?);
            }
            "let" => {
                let start = line.text.len() - rest.len();
                body.push(parse_let(rest, line, start)?);
            }
            "prt" => {
                let start = line.text.len() - rest.len();
                body.push(parse_print(rest, line, start)?);
            }
            "if" => {
                body.push(parse_if(lines, cursor)?);
                continue;
            }
            "mod" => {
                return Err(parse_error(
                    line.number,
                    line.column,
                    "nested modules are not supported",
                ));
            }
            _ => {
                return Err(parse_error(
                    line.number,
                    line.column,
                    "unknown statement keyword",
                ));
            }
        }
        *cursor += 1;
    }
}

fn parse_definition(rest: &str, line: &Line<'_>, base: usize) -> Result<Stmt, Diagnostic> {
    let offset = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let column = line.column + base + offset;
    let Some(equal) = find_assignment_equal(rest) else {
        return Err(parse_error(line.number, column, "a definition requires ="));
    };
    let signature = rest[..equal].trim();
    let body_text = rest[equal + 1..].trim();
    if let Some(index) = signature.find(char::is_whitespace) {
        return Err(parse_error(
            line.number,
            column + index,
            "whitespace is not allowed inside a definition signature",
        ));
    }
    let Some(open) = signature.find('(') else {
        return Err(parse_error(
            line.number,
            column,
            "a definition requires a parameter list",
        ));
    };
    if !signature.ends_with(')') {
        return Err(parse_error(
            line.number,
            column,
            "a definition requires a closing parenthesis",
        ));
    }
    let name = &signature[..open];
    validate_identifier(name, line.number, column)?;
    let parameters = parse_parameters(&signature[open + 1..signature.len() - 1], line)?;
    let body_column =
        column + equal + 1 + (rest[equal + 1..].len() - rest[equal + 1..].trim_start().len());
    let body = expression_source(body_text, line.number, body_column)?;
    Ok(Stmt::Def {
        name: name.to_string(),
        parameters,
        body,
    })
}

fn parse_let(rest: &str, line: &Line<'_>, base: usize) -> Result<Stmt, Diagnostic> {
    let offset = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let column = line.column + base + offset;
    let Some(equal) = find_assignment_equal(rest) else {
        return Err(parse_error(line.number, column, "a binding requires ="));
    };
    let name = rest[..equal].trim();
    validate_identifier(name, line.number, column)?;
    let body_column =
        column + equal + 1 + (rest[equal + 1..].len() - rest[equal + 1..].trim_start().len());
    let body = expression_source(rest[equal + 1..].trim(), line.number, body_column)?;
    Ok(Stmt::Let {
        name: name.to_string(),
        body,
    })
}

fn parse_print(rest: &str, line: &Line<'_>, base: usize) -> Result<Stmt, Diagnostic> {
    let offset = rest.len() - rest.trim_start().len();
    let body = expression_source(rest.trim(), line.number, line.column + base + offset)?;
    Ok(Stmt::Print { body })
}

fn parse_if<'a>(lines: &[Line<'a>], cursor: &mut usize) -> Result<Stmt, Diagnostic> {
    let line = lines[*cursor];
    let (_, rest) = split_keyword(line.text);
    let rest_start = line.text.len() - rest.len();
    let offset = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let Some((condition_text, condition_column)) = strip_trailing_keyword(rest, "then") else {
        return Err(parse_error(
            line.number,
            line.column,
            "a conditional requires then",
        ));
    };
    let condition = expression_source(
        condition_text,
        line.number,
        line.column + rest_start + offset,
    )?;
    let _ = condition_column;
    *cursor += 1;
    let (then_body, terminator) = parse_block(lines, cursor)?;
    let else_body = if matches!(terminator, Terminator::Else) {
        let (else_body, terminator) = parse_block(lines, cursor)?;
        if matches!(terminator, Terminator::Else) {
            let line = lines[*cursor - 1];
            return Err(parse_error(
                line.number,
                line.column,
                "unexpected second else",
            ));
        }
        else_body
    } else {
        Vec::new()
    };
    Ok(Stmt::If {
        condition,
        then_body,
        else_body,
    })
}

fn parse_parameters(source: &str, line: &Line<'_>) -> Result<Vec<String>, Diagnostic> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    if source.contains(char::is_whitespace) {
        return Err(parse_error(
            line.number,
            line.column,
            "whitespace is not allowed inside a parameter list",
        ));
    }
    let mut parameters = Vec::new();
    for parameter in source.split(',') {
        validate_identifier(parameter, line.number, line.column)?;
        parameters.push(parameter.to_string());
    }
    Ok(parameters)
}

fn expression_source(text: &str, line: usize, column: usize) -> Result<ExprSource, Diagnostic> {
    if text.is_empty() {
        return Err(parse_error(line, column, "expected an expression"));
    }
    if let Some((index, _)) = text.char_indices().find(|(_, ch)| ch.is_whitespace()) {
        return Err(parse_error(
            line,
            column + index,
            "whitespace is not allowed inside an expression",
        ));
    }
    Ok(ExprSource {
        text: text.to_string(),
        line,
        column,
    })
}

fn validate_identifier(source: &str, line: usize, column: usize) -> Result<(), Diagnostic> {
    if source.is_empty() {
        return Err(parse_error(line, column, "expected a name"));
    }
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return Err(parse_error(line, column, "expected a name"));
    };
    if !first.is_ascii_lowercase() {
        return Err(parse_error(
            line,
            column,
            "names must start with a lowercase ASCII letter",
        ));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(parse_error(
            line,
            column,
            "names must contain only ASCII letters and digits",
        ));
    }
    Ok(())
}

fn ensure_empty(rest: &str, line: &Line<'_>) -> Result<(), Diagnostic> {
    if rest.trim().is_empty() {
        Ok(())
    } else {
        Err(parse_error(
            line.number,
            line.column,
            "unexpected text after the keyword",
        ))
    }
}

fn split_keyword(text: &str) -> (&str, &str) {
    let end = text
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (&text[..end], &text[end..])
}

fn strip_trailing_keyword<'a>(text: &'a str, keyword: &str) -> Option<(&'a str, usize)> {
    let trimmed = text.trim_end();
    let start = trimmed.len().checked_sub(keyword.len())?;
    if &trimmed[start..] != keyword {
        return None;
    }
    if start > 0 {
        let before = &trimmed[..start];
        if !before.ends_with(char::is_whitespace) {
            return None;
        }
    }
    Some((trimmed[..start].trim_end(), start))
}

fn find_assignment_equal(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'=' {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|i| bytes.get(i)).copied();
        let next = bytes.get(index + 1).copied();
        if previous == Some(b'=') || next == Some(b'=') {
            continue;
        }
        if matches!(previous, Some(b'!') | Some(b'<') | Some(b'>')) {
            continue;
        }
        return Some(index);
    }
    None
}

fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(index) => &line[..index],
        None => line,
    }
}

fn parse_error(line: usize, column: usize, message: &str) -> Diagnostic {
    Diagnostic::parse(
        ErrorCode::ParseUnexpectedToken,
        message,
        Span {
            start: 0,
            end: 0,
            line,
            column,
        },
    )
}
