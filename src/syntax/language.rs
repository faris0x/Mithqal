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
        Terminator::Case => {
            return Err(parse_error(
                first.number,
                first.column,
                "unexpected case outside a match",
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
    Case,
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
            "case" => return Ok((body, Terminator::Case)),
            "match" => {
                body.push(parse_match(lines, cursor)?);
                continue;
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
    let left = rest[..equal].trim();
    let body_text = rest[equal + 1..].trim();
    let Some(open) = left.find('(') else {
        return Err(parse_error(
            line.number,
            column,
            "a definition requires a parameter list",
        ));
    };
    let Some(close_offset) = left[open + 1..].find(')') else {
        return Err(parse_error(
            line.number,
            column,
            "a definition requires a closing parenthesis",
        ));
    };
    let close = open + 1 + close_offset;
    let signature = &left[..=close];
    if let Some(index) = signature.find(char::is_whitespace) {
        return Err(parse_error(
            line.number,
            column + index,
            "whitespace is not allowed inside a definition signature",
        ));
    }
    let name = &left[..open];
    validate_identifier(name, line.number, column)?;
    let parameters = parse_parameters(&left[open + 1..close], line, column + open + 1)?;
    let remainder = left[close + 1..].trim();
    let guard = if remainder.is_empty() {
        None
    } else {
        let Some(guard_text) = remainder.strip_prefix("when") else {
            return Err(parse_error(
                line.number,
                column + close + 1,
                "expected when before a definition guard",
            ));
        };
        if !guard_text.starts_with(char::is_whitespace) {
            return Err(parse_error(
                line.number,
                column + close + 1,
                "expected space after when",
            ));
        }
        let guard_text = guard_text.trim();
        Some(expression_source(
            guard_text,
            line.number,
            column + close + 1 + 5,
        )?)
    };
    let body_column =
        column + equal + 1 + (rest[equal + 1..].len() - rest[equal + 1..].trim_start().len());
    let body = expression_source(body_text, line.number, body_column)?;
    Ok(Stmt::Def {
        name: name.to_string(),
        parameters,
        guard,
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
    if matches!(terminator, Terminator::Case) {
        let line = lines[*cursor];
        return Err(parse_error(line.number, line.column, "unexpected case"));
    }
    let else_body = if matches!(terminator, Terminator::Else) {
        let (else_body, terminator) = parse_block(lines, cursor)?;
        if !matches!(terminator, Terminator::End) {
            let line = lines[*cursor - 1];
            return Err(parse_error(
                line.number,
                line.column,
                "expected end after else",
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

fn parse_match<'a>(lines: &[Line<'a>], cursor: &mut usize) -> Result<Stmt, Diagnostic> {
    let line = lines[*cursor];
    let (_, rest) = split_keyword(line.text);
    let rest_start = line.text.len() - rest.len();
    let offset = rest.len() - rest.trim_start().len();
    let scrutinee = expression_source(rest.trim(), line.number, line.column + rest_start + offset)?;
    *cursor += 1;
    let mut cases = Vec::new();
    let mut else_body = Vec::new();
    let (preamble, mut terminator) = parse_block(lines, cursor)?;
    if !preamble.is_empty() {
        return Err(parse_error(
            line.number,
            line.column,
            "a match requires a case before statements",
        ));
    }
    loop {
        match terminator {
            Terminator::Case => {
                let case_line = lines[*cursor];
                let (_, case_rest) = split_keyword(case_line.text);
                let case_start = case_line.text.len() - case_rest.len();
                let case_offset = case_rest.len() - case_rest.trim_start().len();
                let pattern_text = case_rest.trim();
                let pattern = if is_identifier(pattern_text) {
                    PatternSource::Name(pattern_text.to_string())
                } else {
                    PatternSource::Literal(expression_source(
                        pattern_text,
                        case_line.number,
                        case_line.column + case_start + case_offset,
                    )?)
                };
                *cursor += 1;
                let (body, next) = parse_block(lines, cursor)?;
                cases.push(Case { pattern, body });
                terminator = next;
            }
            Terminator::Else => {
                let (body, next) = parse_block(lines, cursor)?;
                if !matches!(next, Terminator::End) {
                    let line = lines[*cursor - 1];
                    return Err(parse_error(
                        line.number,
                        line.column,
                        "expected end after else",
                    ));
                }
                else_body = body;
                break;
            }
            Terminator::End => break,
        }
    }
    if cases.is_empty() && else_body.is_empty() {
        return Err(parse_error(
            line.number,
            line.column,
            "a match requires a case",
        ));
    }
    Ok(Stmt::Match {
        scrutinee,
        cases,
        else_body,
    })
}

fn parse_parameters(
    source: &str,
    line: &Line<'_>,
    base: usize,
) -> Result<Vec<PatternSource>, Diagnostic> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    if let Some((index, _)) = source.char_indices().find(|(_, ch)| ch.is_whitespace()) {
        return Err(parse_error(
            line.number,
            base + index,
            "whitespace is not allowed inside a parameter list",
        ));
    }
    let mut parameters = Vec::new();
    let mut start = 0usize;
    for parameter in source.split(',') {
        if parameter.is_empty() {
            return Err(parse_error(
                line.number,
                base + start,
                "expected a parameter",
            ));
        }
        if is_identifier(parameter) {
            parameters.push(PatternSource::Name(parameter.to_string()));
        } else {
            parameters.push(PatternSource::Literal(expression_source(
                parameter,
                line.number,
                base + start,
            )?));
        }
        start += parameter.len() + 1;
    }
    Ok(parameters)
}

fn is_identifier(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase() && chars.all(|ch| ch.is_ascii_alphanumeric())
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
