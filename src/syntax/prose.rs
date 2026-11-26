//! Controlled mathematical prose parser.
//!
//! This module is the 0.0.3 surface language parser. It parses sentence
//! structure, modules, definitions and clauses, blocks, structures, instances,
//! derivative notation, and theorem skeletons into a prose AST, then lowers
//! definitions and statements into the evaluator AST.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode, Span};

use super::language::{Body, Case, ExprSource, PatternSource, Program, Stmt};

/// A parsed prose module.
#[derive(Clone, Debug)]
pub struct ProseProgram {
    /// Module name.
    pub name: String,
    /// Module statements.
    pub body: Vec<ProseStmt>,
}

/// One prose statement.
#[derive(Clone, Debug)]
pub enum ProseStmt {
    /// A module dependency.
    Use(String),
    /// An export list.
    Export(Vec<String>),
    /// A named definition.
    Define(ProseDefinition),
    /// A named assumption.
    Assume(String),
    /// A pattern match block.
    Match {
        /// Inspected expression text.
        scrutinee: String,
        /// Ordered cases.
        cases: Vec<(PatternSource, Vec<ProseStmt>)>,
        /// Fallback statements.
        else_body: Vec<ProseStmt>,
        /// One based source line.
        line: usize,
        /// One based source column.
        column: usize,
    },
    /// A conditional block.
    If {
        /// Condition text.
        condition: String,
        /// Statements taken when true.
        then_body: Vec<ProseStmt>,
        /// Statements taken when false.
        else_body: Vec<ProseStmt>,
        /// One based source line.
        line: usize,
        /// One based source column.
        column: usize,
    },
    /// A theorem and its proof skeleton.
    Theorem(ProseTheorem),
    /// A structure declaration.
    Structure {
        /// Structure name.
        name: String,
        /// Field names.
        fields: Vec<String>,
        /// Named laws.
        laws: Vec<(String, String)>,
    },
    /// A printed expression.
    Print {
        /// Printed expression.
        expression: String,
        /// One based source line.
        line: usize,
        /// One based source column.
        column: usize,
    },
    /// A module-level value binding.
    Let {
        /// Bound name.
        name: String,
        /// Bound expression.
        body: String,
        /// One based source line.
        line: usize,
        /// One based source column.
        column: usize,
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
        fields: Vec<(String, String)>,
    },
}

/// A definition written in controlled prose.
#[derive(Clone, Debug)]
pub struct ProseDefinition {
    /// The defined name, or `derivative` for derivative notation.
    pub name: String,
    /// Normalized left side.
    pub lhs: String,
    /// Normalized parameter texts.
    pub parameters: Vec<String>,
    /// True when the left side is a call, including a zero-argument call.
    pub callable: bool,
    /// Normalized right side.
    pub body: String,
    /// True when the definition targets the global notation binding.
    pub global: bool,
    /// Clauses attached to the definition.
    pub clauses: Vec<ProseClause>,
    /// One based source line.
    pub line: usize,
    /// One based source column.
    pub column: usize,
}

/// One clause attached to a statement.
#[derive(Clone, Debug)]
pub struct ProseClause {
    /// Clause keyword.
    pub kind: ProseClauseKind,
    /// Clause text after the keyword.
    pub text: String,
    /// Introduced or constrained variable, when the clause has the form
    /// `x is a ...`.
    pub variable: Option<String>,
    /// Domain or condition text, when recognized.
    pub domain: Option<String>,
}

/// Clause keyword.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProseClauseKind {
    /// `where`.
    Where,
    /// `assuming`.
    Assuming,
    /// `provided that`.
    ProvidedThat,
}

/// A theorem and its proof steps.
#[derive(Clone, Debug)]
pub struct ProseTheorem {
    /// Theorem name.
    pub name: String,
    /// Normalized claim text.
    pub claim: String,
    /// Normalized proof step texts.
    pub proof: Vec<String>,
    /// One based source line.
    pub line: usize,
    /// One based source column.
    pub column: usize,
}

struct IfFrame {
    condition: String,
    line: usize,
    column: usize,
    then_body: Vec<ProseStmt>,
    else_body: Vec<ProseStmt>,
    in_else: bool,
}

struct MatchFrame {
    scrutinee: String,
    line: usize,
    column: usize,
    cases: Vec<(PatternSource, Vec<ProseStmt>)>,
    else_body: Vec<ProseStmt>,
    in_else: bool,
}

enum Frame {
    If(IfFrame),
    Match(MatchFrame),
}

fn push_stmt(frames: &mut [Frame], body: &mut Vec<ProseStmt>, statement: ProseStmt) {
    if let Some(frame) = frames.last_mut() {
        match frame {
            Frame::If(frame) => {
                if frame.in_else {
                    frame.else_body.push(statement);
                } else {
                    frame.then_body.push(statement);
                }
            }
            Frame::Match(frame) => {
                if frame.in_else {
                    frame.else_body.push(statement);
                } else if let Some((_, body)) = frame.cases.last_mut() {
                    body.push(statement);
                }
            }
        }
    } else {
        body.push(statement);
    }
}

#[derive(Clone, Debug)]
struct Sentence {
    text: String,
    line: usize,
    column: usize,
}

/// True when the source looks like a controlled prose module.
pub fn looks_like_prose_program(source: &str) -> bool {
    first_word(source).is_some_and(|word| word == "module")
}

fn first_word(source: &str) -> Option<&str> {
    for line in source.lines() {
        let text = line
            .split("//")
            .next()
            .unwrap_or("")
            .split("(*")
            .next()
            .unwrap_or("")
            .trim();
        if text.is_empty() {
            continue;
        }
        return text.split_whitespace().next();
    }
    None
}

/// Parse a complete controlled prose module.
pub fn parse_prose_program(source: &str) -> Result<ProseProgram, Diagnostic> {
    let sentences = split_sentences(source)?;
    let mut cursor = 0usize;
    let Some(header) = sentences.get(cursor) else {
        return Err(error("expected a module declaration", 1, 1));
    };
    let Some(rest) = strip_prefix_word(&header.text, "module") else {
        return Err(error(
            "a program must start with module",
            header.line,
            header.column,
        ));
    };
    let name = rest.trim();
    if !is_identifier(name) {
        return Err(error("invalid module name", header.line, header.column + 7));
    }
    cursor += 1;
    let mut body = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut closed = false;
    while let Some(sentence) = sentences.get(cursor) {
        cursor += 1;
        if sentence.text.eq_ignore_ascii_case("end module") {
            if !frames.is_empty() {
                return Err(error("expected end if", sentence.line, sentence.column));
            }
            closed = true;
            break;
        }
        if sentence.text.eq_ignore_ascii_case("else:")
            || sentence.text.eq_ignore_ascii_case("otherwise:")
        {
            let Some(frame) = frames.last_mut() else {
                return Err(error(
                    "else without a block",
                    sentence.line,
                    sentence.column,
                ));
            };
            match frame {
                Frame::If(frame) => {
                    if frame.in_else {
                        return Err(error("duplicate else", sentence.line, sentence.column));
                    }
                    frame.in_else = true;
                }
                Frame::Match(frame) => {
                    if frame.in_else {
                        return Err(error("duplicate otherwise", sentence.line, sentence.column));
                    }
                    frame.in_else = true;
                }
            }
            continue;
        }
        if let Some(pattern) = sentence
            .text
            .strip_prefix("case ")
            .and_then(|text| text.strip_suffix(':'))
        {
            let Some(Frame::Match(frame)) = frames.last_mut() else {
                return Err(error("case without match", sentence.line, sentence.column));
            };
            if frame.in_else {
                return Err(error(
                    "case after otherwise",
                    sentence.line,
                    sentence.column,
                ));
            }
            let pattern_text = normalize_formula(pattern.trim())?;
            let pattern = pattern_source(&pattern_text, sentence.line, sentence.column);
            frame.cases.push((pattern, Vec::new()));
            continue;
        }
        if let Some(scrutinee) = sentence
            .text
            .strip_prefix("match ")
            .and_then(|text| text.strip_suffix(" with:"))
        {
            frames.push(Frame::Match(MatchFrame {
                scrutinee: normalize_formula(scrutinee.trim())?,
                line: sentence.line,
                column: sentence.column,
                cases: Vec::new(),
                else_body: Vec::new(),
                in_else: false,
            }));
            continue;
        }
        if sentence.text.eq_ignore_ascii_case("end if") {
            let Some(Frame::If(frame)) = frames.pop() else {
                return Err(error("end if without if", sentence.line, sentence.column));
            };
            let statement = ProseStmt::If {
                condition: frame.condition,
                then_body: frame.then_body,
                else_body: frame.else_body,
                line: frame.line,
                column: frame.column,
            };
            push_stmt(&mut frames, &mut body, statement);
            continue;
        }
        if sentence.text.eq_ignore_ascii_case("end match") {
            let Some(Frame::Match(frame)) = frames.pop() else {
                return Err(error(
                    "end match without match",
                    sentence.line,
                    sentence.column,
                ));
            };
            let statement = ProseStmt::Match {
                scrutinee: frame.scrutinee,
                cases: frame.cases,
                else_body: frame.else_body,
                line: frame.line,
                column: frame.column,
            };
            push_stmt(&mut frames, &mut body, statement);
            continue;
        }
        if let Some(condition) = sentence
            .text
            .strip_prefix("if ")
            .and_then(|text| text.strip_suffix(" then:"))
        {
            frames.push(Frame::If(IfFrame {
                condition: normalize_formula(condition.trim())?,
                line: sentence.line,
                column: sentence.column,
                then_body: Vec::new(),
                else_body: Vec::new(),
                in_else: false,
            }));
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "use") {
            let module = rest.trim();
            if module.starts_with('"') && module.ends_with('"') && module.len() >= 2 {
                push_stmt(
                    &mut frames,
                    &mut body,
                    ProseStmt::Use(module[1..module.len() - 1].to_string()),
                );
                continue;
            }
            if !is_identifier(module) {
                return Err(error("invalid module name", sentence.line, sentence.column));
            }
            push_stmt(&mut frames, &mut body, ProseStmt::Use(module.to_string()));
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "export") {
            let names = rest
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if names.is_empty() {
                return Err(error(
                    "expected exported names",
                    sentence.line,
                    sentence.column,
                ));
            }
            push_stmt(&mut frames, &mut body, ProseStmt::Export(names));
            continue;
        }
        if let Some(definition) = definition_body(&sentence.text) {
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Define(parse_definition(&definition, sentence)?),
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "assume") {
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Assume(rest.trim().to_string()),
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "print") {
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Print {
                    expression: normalize_formula(rest.trim())?,
                    line: sentence.line,
                    column: sentence.column,
                },
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "show") {
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Print {
                    expression: normalize_formula(rest.trim())?,
                    line: sentence.line,
                    column: sentence.column,
                },
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "let") {
            let Some(position) = find_word_in(rest, "be") else {
                return Err(error(
                    "expected let name be expression",
                    sentence.line,
                    sentence.column,
                ));
            };
            let name = rest[..position].trim();
            if !is_identifier(name) {
                return Err(error(
                    "invalid binding name",
                    sentence.line,
                    sentence.column,
                ));
            }
            let value = normalize_formula(rest[position + 2..].trim())?;
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Let {
                    name: name.to_string(),
                    body: value,
                    line: sentence.line,
                    column: sentence.column,
                },
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "structure") {
            let structure_name = rest.trim();
            if !is_identifier(structure_name) {
                return Err(error(
                    "invalid structure name",
                    sentence.line,
                    sentence.column,
                ));
            }
            let mut fields = Vec::new();
            let mut laws = Vec::new();
            let mut found_end = false;
            while let Some(entry) = sentences.get(cursor) {
                cursor += 1;
                if entry.text.eq_ignore_ascii_case("end structure") {
                    found_end = true;
                    break;
                }
                if let Some(field) = strip_prefix_word(&entry.text, "field") {
                    let field = field.trim();
                    if !is_identifier(field) {
                        return Err(error("invalid field name", entry.line, entry.column));
                    }
                    fields.push(field.to_string());
                    continue;
                }
                if let Some(law) = strip_prefix_word(&entry.text, "law") {
                    let Some(colon) = law.find(':') else {
                        return Err(error("expected law name:", entry.line, entry.column));
                    };
                    let law_name = law[..colon].trim();
                    if !is_identifier(law_name) {
                        return Err(error("invalid law name", entry.line, entry.column));
                    }
                    let body = normalize_formula(law[colon + 1..].trim())?;
                    laws.push((law_name.to_string(), body));
                    continue;
                }
                return Err(error(
                    "expected field, law, or end structure",
                    entry.line,
                    entry.column,
                ));
            }
            if !found_end {
                return Err(error(
                    "expected end structure",
                    sentence.line,
                    sentence.column,
                ));
            }
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Structure {
                    name: structure_name.to_string(),
                    fields,
                    laws,
                },
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "instance") {
            let Some(colon) = rest.find(':') else {
                return Err(error(
                    "expected instance name : structure",
                    sentence.line,
                    sentence.column,
                ));
            };
            let instance_name = rest[..colon].trim();
            let mut structure_name = rest[colon + 1..].trim();
            let assumed = structure_name.ends_with(" assumed");
            if assumed {
                structure_name = structure_name.trim_end_matches(" assumed").trim();
            }
            if !is_identifier(instance_name) || !is_identifier(structure_name) {
                return Err(error(
                    "invalid instance declaration",
                    sentence.line,
                    sentence.column,
                ));
            }
            let mut fields = Vec::new();
            let mut found_end = false;
            while let Some(entry) = sentences.get(cursor) {
                cursor += 1;
                if entry.text.eq_ignore_ascii_case("end instance") {
                    found_end = true;
                    break;
                }
                let Some(equals) = find_top_level(&entry.text, '=') else {
                    return Err(error(
                        "expected field = expression",
                        entry.line,
                        entry.column,
                    ));
                };
                let field = entry.text[..equals].trim();
                if !is_identifier(field) {
                    return Err(error("invalid instance field", entry.line, entry.column));
                }
                let value = normalize_formula(entry.text[equals + 1..].trim())?;
                fields.push((field.to_string(), value));
            }
            if !found_end {
                return Err(error(
                    "expected end instance",
                    sentence.line,
                    sentence.column,
                ));
            }
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Instance {
                    name: instance_name.to_string(),
                    structure: structure_name.to_string(),
                    assumed,
                    fields,
                },
            );
            continue;
        }
        if let Some(rest) = strip_prefix_word(&sentence.text, "theorem") {
            let theorem_name = rest.trim();
            if !is_identifier(theorem_name) {
                return Err(error(
                    "invalid theorem name",
                    sentence.line,
                    sentence.column,
                ));
            }
            let Some(claim) = sentences.get(cursor) else {
                return Err(error(
                    "expected a theorem claim",
                    sentence.line,
                    sentence.column,
                ));
            };
            cursor += 1;
            let Some(proof_header) = sentences.get(cursor) else {
                return Err(error("expected proof:", claim.line, claim.column));
            };
            if !proof_header.text.eq_ignore_ascii_case("proof:") {
                return Err(error(
                    "expected proof:",
                    proof_header.line,
                    proof_header.column,
                ));
            }
            cursor += 1;
            let mut proof = Vec::new();
            let mut found_end = false;
            while let Some(step) = sentences.get(cursor) {
                cursor += 1;
                if step.text.eq_ignore_ascii_case("end proof") {
                    found_end = true;
                    break;
                }
                proof.push(step.text.trim().to_string());
            }
            if !found_end {
                return Err(error("expected end proof", claim.line, claim.column));
            }
            push_stmt(
                &mut frames,
                &mut body,
                ProseStmt::Theorem(ProseTheorem {
                    name: theorem_name.to_string(),
                    claim: claim.text.trim().to_string(),
                    proof,
                    line: sentence.line,
                    column: sentence.column,
                }),
            );
            continue;
        }
        return Err(error(
            "expected a module statement",
            sentence.line,
            sentence.column,
        ));
    }
    if !closed {
        return Err(error("expected end module", 1, 1));
    }
    if cursor < sentences.len() {
        let trailing = &sentences[cursor];
        return Err(error(
            "statements after end module",
            trailing.line,
            trailing.column,
        ));
    }
    Ok(ProseProgram {
        name: name.to_string(),
        body,
    })
}

fn definition_body(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let phrase = "we define";
    let mut depth = 0i32;
    let mut in_string = false;
    let mut index = 0usize;
    while index + phrase.len() <= text.len() {
        let character = text[index..].chars().next()?;
        if character == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if depth == 0
                && lower[index..].starts_with(phrase)
                && (index == 0
                    || !text[..index]
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_'))
            {
                let after = index + phrase.len();
                let before = text[..index].trim_end().trim_end_matches(',').trim_end();
                let tail = text[after..].trim_start();
                return Some(if before.is_empty() {
                    tail.to_string()
                } else {
                    format!("{before}, {tail}")
                });
            }
        }
        index += character.len_utf8();
    }
    None
}

/// Lower a prose module into the evaluator statement AST.
pub fn lower_prose_program(program: &ProseProgram) -> Result<Program, Diagnostic> {
    let mut body = Vec::with_capacity(program.body.len());
    lower_statements(&program.body, &mut body)?;
    Ok(Program {
        name: program.name.clone(),
        body,
    })
}

fn lower_statements(statements: &[ProseStmt], body: &mut Vec<Stmt>) -> Result<(), Diagnostic> {
    for statement in statements {
        match statement {
            ProseStmt::Use(module) => body.push(Stmt::Import {
                module: module.clone(),
            }),
            ProseStmt::Export(names) => body.push(Stmt::Export {
                names: names.clone(),
            }),
            ProseStmt::Assume(condition) => body.push(Stmt::Assume {
                condition: source(&lower_assumption(condition)?, 1, 1),
            }),
            ProseStmt::Define(definition) => body.push(lower_definition(definition)?),
            ProseStmt::Print {
                expression,
                line,
                column,
            } => body.push(Stmt::Print {
                body: source(expression, *line, *column),
            }),
            ProseStmt::Let {
                name,
                body: value,
                line,
                column,
            } => body.push(Stmt::Let {
                name: name.clone(),
                body: source(value, *line, *column),
            }),
            ProseStmt::Match {
                scrutinee,
                cases,
                else_body,
                line,
                column,
            } => {
                let mut lowered_cases = Vec::new();
                for (pattern, statements) in cases {
                    let mut case_body = Vec::new();
                    lower_statements(statements, &mut case_body)?;
                    lowered_cases.push(Case {
                        pattern: pattern.clone(),
                        body: case_body,
                    });
                }
                let mut lowered_else = Vec::new();
                lower_statements(else_body, &mut lowered_else)?;
                body.push(Stmt::Match {
                    scrutinee: source(scrutinee, *line, *column),
                    cases: lowered_cases,
                    else_body: lowered_else,
                });
            }
            ProseStmt::If {
                condition,
                then_body,
                else_body,
                line,
                column,
            } => {
                let mut then_statements = Vec::new();
                lower_statements(then_body, &mut then_statements)?;
                let mut else_statements = Vec::new();
                lower_statements(else_body, &mut else_statements)?;
                body.push(Stmt::If {
                    condition: source(condition, *line, *column),
                    then_body: then_statements,
                    else_body: else_statements,
                });
            }
            ProseStmt::Structure { name, fields, laws } => body.push(Stmt::Structure {
                name: name.clone(),
                fields: fields.clone(),
                laws: laws
                    .iter()
                    .map(|(law, body)| (law.clone(), source(body, 1, 1)))
                    .collect(),
            }),
            ProseStmt::Instance {
                name,
                structure,
                assumed,
                fields,
            } => body.push(Stmt::Instance {
                name: name.clone(),
                structure: structure.clone(),
                assumed: *assumed,
                fields: fields
                    .iter()
                    .map(|(field, value)| (field.clone(), source(value, 1, 1)))
                    .collect(),
            }),
            ProseStmt::Theorem(theorem) => body.push(Stmt::Theorem {
                name: theorem.name.clone(),
                claim: source(&theorem.claim, theorem.line, theorem.column),
                proof: theorem
                    .proof
                    .iter()
                    .map(|step| source(step, theorem.line, theorem.column))
                    .collect(),
            }),
        }
    }
    Ok(())
}

fn lower_assumption(text: &str) -> Result<String, Diagnostic> {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    for (phrase, replacement) in [
        (" is positive", "{var}>0"),
        (" is negative", "{var}<0"),
        (" is nonzero", "{var}=/=0"),
        (" is an integer", "isInteger({var})"),
        (" is a real number", "isNumber({var})"),
        (" is a rational number", "isNumber({var})"),
        (" is a number", "isNumber({var})"),
    ] {
        if let Some(position) = lower.find(phrase) {
            let variable = trimmed[..position].trim();
            if is_identifier(variable) {
                return Ok(replacement.replace("{var}", variable));
            }
        }
    }
    normalize_formula(trimmed)
}

fn lower_definition(definition: &ProseDefinition) -> Result<Stmt, Diagnostic> {
    let body = Body::Expression(source(&definition.body, definition.line, definition.column));
    let guard = lower_guard(&definition.clauses, definition.line, definition.column)?;
    if definition.parameters.is_empty() && !definition.callable {
        if guard.is_some() {
            return Err(Diagnostic::parse(
                ErrorCode::ParseUnexpectedToken,
                "a value definition cannot have a guard",
                Span {
                    start: 0,
                    end: 0,
                    line: definition.line,
                    column: definition.column,
                },
            ));
        }
        if let Body::Expression(value) = body {
            return Ok(Stmt::Let {
                name: definition.name.clone(),
                body: value,
            });
        }
    }
    Ok(Stmt::Def {
        name: definition.name.clone(),
        parameters: definition
            .parameters
            .iter()
            .map(|parameter| pattern_source(parameter, definition.line, definition.column))
            .collect(),
        guard,
        body,
    })
}

fn pattern_source(text: &str, line: usize, column: usize) -> PatternSource {
    if is_identifier(text) {
        PatternSource::Name(text.to_string())
    } else {
        PatternSource::Literal(source(&elaborate_pattern(text), line, column))
    }
}

fn elaborate_local_lhs(text: &str) -> String {
    let Some(open) = text.find('(') else {
        return text.trim().to_string();
    };
    let name = text[..open].trim();
    if !text.ends_with(')') {
        return text.trim().to_string();
    }
    let arguments = &text[open + 1..text.len() - 1];
    let elaborated = split_top_level(arguments, ',')
        .iter()
        .map(|argument| elaborate_pattern(argument))
        .collect::<Vec<_>>()
        .join(",");
    format!("{name}({elaborated})")
}

fn elaborate_pattern(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < chars.len() {
        let character = chars[index];
        if character.is_alphabetic() || character == '_' {
            let start = index;
            while chars
                .get(index)
                .is_some_and(|value| value.is_alphanumeric() || *value == '_' || *value == '\'')
            {
                index += 1;
            }
            let word: String = chars[start..index].iter().collect();
            if chars.get(index) == Some(&'.')
                && chars.get(index + 1) == Some(&'.')
                && chars.get(index + 2) == Some(&'.')
            {
                result.push_str(&word);
                result.push_str("__");
                index += 3;
                continue;
            }
            if chars.get(index) == Some(&'…') {
                result.push_str(&word);
                result.push_str("__");
                index += 1;
                continue;
            }
            if is_pattern_constructor(&word)
                || word.ends_with('_')
                || chars.get(index) == Some(&'(')
            {
                result.push_str(&word);
            } else {
                result.push_str(&word);
                result.push('_');
            }
            continue;
        }
        result.push(character);
        index += 1;
    }
    result
}

fn is_pattern_constructor(word: &str) -> bool {
    matches!(
        word,
        "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "asinh"
            | "acosh"
            | "atanh"
            | "exp"
            | "ln"
            | "sqrt"
            | "abs"
            | "floor"
            | "ceil"
            | "round"
            | "plus"
            | "times"
            | "power"
            | "negate"
            | "derivative"
            | "integral"
            | "list"
            | "true"
            | "false"
            | "undefined"
            | "pi"
            | "e"
            | "i"
    )
}

fn source(text: &str, line: usize, column: usize) -> ExprSource {
    ExprSource {
        text: text.to_string(),
        line,
        column,
    }
}

fn lower_guard(
    clauses: &[ProseClause],
    line: usize,
    column: usize,
) -> Result<Option<ExprSource>, Diagnostic> {
    let mut guards = Vec::new();
    for clause in clauses {
        if let (Some(variable), Some(domain)) = (&clause.variable, &clause.domain) {
            for part in split_word_and(domain) {
                if let Some(guard) = domain_guard(variable, &part) {
                    guards.push(format!("({guard})"));
                }
            }
        } else if let Some((left, right)) = split_phrase(&clause.text, "does not depend on") {
            let left = normalize_formula(left)?;
            let right = normalize_formula(right)?;
            guards.push(format!("(freeOf({left},{right}))"));
        } else {
            guards.push(format!("({})", normalize_formula(&clause.text)?));
        }
    }
    if guards.is_empty() {
        return Ok(None);
    }
    Ok(Some(source(&guards.join("&&"), line, column)))
}

fn domain_guard(variable: &str, domain: &str) -> Option<String> {
    let lower = domain.to_ascii_lowercase();
    if lower.contains("expression in") || lower.contains("polynomial in") {
        return None;
    }
    if lower.contains("nonzero integer") {
        return Some(format!("isInteger({variable})&&{variable}=/=0"));
    }
    if lower.contains("integer") {
        return Some(format!("isInteger({variable})"));
    }
    if lower.contains("number") || lower.contains("rational") || lower.contains("real") {
        if lower.contains("nonzero") {
            return Some(format!("isNumber({variable})&&{variable}=/=0"));
        }
        return Some(format!("isNumber({variable})"));
    }
    if lower.contains("prime") {
        return Some(format!("isPrime({variable})"));
    }
    if lower.contains("nonzero") {
        return Some(format!("{variable}=/=0"));
    }
    normalize_formula(domain)
        .ok()
        .filter(|text| !text.is_empty())
}

fn split_word_and(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if lower[index..].starts_with(" and ") {
            parts.push(current.trim().to_string());
            current.clear();
            index += 5;
            continue;
        }
        let character = text[index..].chars().next().unwrap_or(' ');
        current.push(character);
        index += character.len_utf8();
    }
    parts.push(current.trim().to_string());
    parts
}

fn split_phrase<'a>(text: &'a str, phrase: &str) -> Option<(&'a str, &'a str)> {
    let lower = text.to_ascii_lowercase();
    let position = lower.find(phrase)?;
    Some((
        text[..position].trim(),
        text[position + phrase.len()..].trim(),
    ))
}

fn parse_definition(text: &str, sentence: &Sentence) -> Result<ProseDefinition, Diagnostic> {
    let parts = split_top_level(text, ',');
    let mut clauses = Vec::new();
    let mut core_parts = Vec::new();
    for part in &parts {
        let trimmed = part.trim();
        if trimmed.to_ascii_lowercase().starts_with("where ")
            || trimmed.to_ascii_lowercase().starts_with("assuming ")
            || trimmed.to_ascii_lowercase().starts_with("provided that ")
        {
            clauses.push(parse_clause(trimmed, sentence)?);
        } else {
            core_parts.push(trimmed.to_string());
        }
    }
    let core = core_parts.join(",");
    let core = core.as_str();
    let equals = find_top_level(core, '=')
        .ok_or_else(|| error("a definition needs =", sentence.line, sentence.column))?;
    let lhs = normalize_formula(&core[..equals])?;
    let body = normalize_formula(&core[equals + 1..])?;
    let global = lhs.starts_with("global:");
    let callable = lhs.contains('(');
    let (name, parameters) = pattern_head(&lhs)?;
    Ok(ProseDefinition {
        name,
        global,
        callable,
        lhs,
        parameters,
        body,
        clauses,
        line: sentence.line,
        column: sentence.column,
    })
}

fn parse_clause(text: &str, sentence: &Sentence) -> Result<ProseClause, Diagnostic> {
    let trimmed = text.trim();
    let (kind, rest) = if let Some(rest) = strip_prefix_word(trimmed, "where") {
        (ProseClauseKind::Where, rest)
    } else if let Some(rest) = strip_prefix_word(trimmed, "assuming") {
        (ProseClauseKind::Assuming, rest)
    } else if let Some(rest) = strip_prefix_word(trimmed, "provided that") {
        (ProseClauseKind::ProvidedThat, rest)
    } else {
        return Err(error(
            "expected a where, assuming, or provided that clause",
            sentence.line,
            sentence.column,
        ));
    };
    let text = rest.trim().to_string();
    let (variable, domain) = split_is(&text);
    Ok(ProseClause {
        kind,
        text,
        variable,
        domain,
    })
}

fn split_is(text: &str) -> (Option<String>, Option<String>) {
    let lower = text.to_ascii_lowercase();
    if let Some(position) = lower.find(" is ") {
        let variable = text[..position].trim();
        let domain = text[position + 4..].trim();
        if is_identifier(variable) && !domain.is_empty() {
            return (Some(variable.to_string()), Some(domain.to_string()));
        }
    }
    (None, None)
}

fn pattern_head(lhs: &str) -> Result<(String, Vec<String>), Diagnostic> {
    let lhs = lhs.strip_prefix("global:").unwrap_or(lhs);
    if let Some(inner) = lhs.strip_prefix("derivative(")
        && let Some(inner) = inner.strip_suffix(')')
    {
        return Ok(("derivative".to_string(), split_top_level(inner, ',')));
    }
    let Some(open) = lhs.find('(') else {
        if is_identifier(lhs) {
            return Ok((lhs.to_string(), Vec::new()));
        }
        return Err(error("invalid definition left side", 1, 1));
    };
    if !lhs.ends_with(')') {
        return Err(error("invalid definition left side", 1, 1));
    }
    let name = lhs[..open].trim();
    if !is_identifier(name) {
        return Err(error("invalid definition name", 1, 1));
    }
    let arguments = lhs[open + 1..lhs.len() - 1].trim();
    let parameters = if arguments.is_empty() {
        Vec::new()
    } else {
        split_top_level(arguments, ',')
    };
    Ok((name.to_string(), parameters))
}

fn split_sentences(source: &str) -> Result<Vec<Sentence>, Diagnostic> {
    let chars: Vec<char> = source.chars().collect();
    let mut sentences = Vec::new();
    let mut buffer = String::new();
    let mut line = 1usize;
    let mut column = 1usize;
    let mut start: Option<(usize, usize)> = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut index = 0usize;
    while index < chars.len() {
        let character = chars[index];
        if !in_string && character == '/' && chars.get(index + 1) == Some(&'/') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if !in_string && character == '(' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            let mut closed = false;
            while index + 1 < chars.len() {
                if chars[index] == '*' && chars[index + 1] == ')' {
                    index += 2;
                    closed = true;
                    break;
                }
                if chars[index] == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                index += 1;
            }
            if !closed {
                return Err(error("unclosed comment", line, column));
            }
            continue;
        }
        if character == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(error("unexpected closing delimiter", line, column));
                    }
                }
                _ => {}
            }
        }
        let trimmed = buffer.trim();
        let is_proof_colon =
            character == ':' && depth == 0 && !in_string && trimmed.eq_ignore_ascii_case("proof");
        let trimmed_lower = trimmed.to_ascii_lowercase();
        let is_block_colon = character == ':'
            && depth == 0
            && !in_string
            && (trimmed_lower == "else"
                || trimmed_lower == "otherwise"
                || (trimmed_lower.starts_with("if ") && trimmed_lower.ends_with(" then"))
                || (trimmed_lower.starts_with("match ") && trimmed_lower.ends_with(" with"))
                || trimmed_lower.starts_with("case "));
        let is_period = character == '.'
            && depth == 0
            && !in_string
            && !chars
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_digit());
        if is_period || is_proof_colon || is_block_colon {
            if !trimmed.is_empty() {
                let (start_line, start_column) = start.unwrap_or((line, column));
                let text = if is_proof_colon || is_block_colon {
                    format!("{trimmed}:")
                } else {
                    trimmed.to_string()
                };
                sentences.push(Sentence {
                    text,
                    line: start_line,
                    column: start_column,
                });
            }
            buffer.clear();
            start = None;
            index += 1;
            column += 1;
            continue;
        }
        if !character.is_whitespace() && start.is_none() {
            start = Some((line, column));
        }
        buffer.push(character);
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
        index += 1;
    }
    if !buffer.trim().is_empty() {
        return Err(error("a sentence must end with a period", line, column));
    }
    Ok(sentences)
}

/// Normalize a formula for the prose AST.
///
/// Whitespace outside string literals is removed and derivative notation is
/// lowered to `derivative(body, variable)`. A leading `*` on derivative
/// notation selects the global binding.
pub fn normalize_formula(text: &str) -> Result<String, Diagnostic> {
    let rewritten = rewrite_local_lets(text);
    let compact = compact_whitespace(&rewritten);
    Ok(rewrite_derivatives(&compact).replace("!=", "=/="))
}

fn rewrite_local_lets(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < chars.len() {
        if starts_word_at(&chars, index, "let") && prefix_position(&chars, index) {
            let after_let = index + 3;
            if let Some(equals) = find_char_top_level(&chars, after_let, '=') {
                let name: String = chars[after_let..equals].iter().collect();
                let name = name.trim();
                if (is_identifier(name) || name.contains('('))
                    && let Some(in_position) = find_word_top_level(&chars, equals + 1, "in")
                {
                    let value: String = chars[equals + 1..in_position].iter().collect();
                    let function_lhs = name.contains('(');
                    let value = if function_lhs {
                        format!("{name}={value}")
                    } else {
                        value
                    };
                    let body_start = in_position + 2;
                    let body_end = let_body_end(&chars, body_start);
                    let body: String = chars[body_start..body_end].iter().collect();
                    let definitions = split_top_level(value.trim(), ',');
                    let function_definitions = definitions.iter().any(|definition| {
                        find_top_level(definition, '=')
                            .is_some_and(|position| definition[..position].contains('('))
                    });
                    if function_definitions {
                        result.push_str("localDefs([");
                        for (position, definition) in definitions.iter().enumerate() {
                            if position > 0 {
                                result.push(',');
                            }
                            let Some(def_equals) = find_top_level(definition, '=') else {
                                result.push_str(definition);
                                continue;
                            };
                            let lhs = elaborate_local_lhs(definition[..def_equals].trim());
                            let rhs = rewrite_local_lets(definition[def_equals + 1..].trim());
                            result.push_str("setDelayed(");
                            result.push_str(&lhs);
                            result.push(',');
                            result.push_str(&rhs);
                            result.push(')');
                        }
                        result.push_str("],");
                        result.push_str(&rewrite_local_lets(body.trim()));
                        result.push(')');
                    } else {
                        result.push_str("let([");
                        result.push_str(name);
                        result.push('=');
                        result.push_str(&rewrite_local_lets(value.trim()));
                        result.push_str("],");
                        result.push_str(&rewrite_local_lets(body.trim()));
                        result.push(')');
                    }
                    index = body_end;
                    continue;
                }
            }
        }
        result.push(chars[index]);
        index += 1;
    }
    result
}

#[allow(clippy::needless_range_loop)]
fn let_body_end(chars: &[char], start: usize) -> usize {
    let mut depth = 0i32;
    let mut in_string = false;
    for index in start..chars.len() {
        let character = chars[index];
        if character == '"' {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            ',' if depth == 0 => return index,
            _ => {}
        }
    }
    chars.len()
}

fn starts_word_at(chars: &[char], index: usize, word: &str) -> bool {
    let mut position = index;
    for expected in word.chars() {
        if chars.get(position) != Some(&expected) {
            return false;
        }
        position += 1;
    }
    !chars
        .get(position)
        .is_some_and(|character| character.is_alphanumeric() || *character == '_')
}

fn find_char_top_level(chars: &[char], start: usize, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    for (index, character) in chars.iter().enumerate().skip(start) {
        if *character == '"' {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if *character == target && depth == 0 {
            return Some(index);
        }
    }
    None
}

fn find_word_in(text: &str, word: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    lower.find(word)
}

fn find_word_top_level(chars: &[char], start: usize, word: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut index = start;
    while index < chars.len() {
        let character = chars[index];
        if character == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if depth == 0
                && starts_word_at(chars, index, word)
                && (index == 0 || prefix_position(chars, index))
            {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn compact_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        if in_string {
            result.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            result.push(character);
            continue;
        }
        if character.is_whitespace() {
            continue;
        }
        match character {
            '×' | '·' => result.push('*'),
            '÷' => result.push('/'),
            '−' => result.push('-'),
            '≤' => result.push_str("<="),
            '≥' => result.push_str(">="),
            '≠' => result.push_str("=/="),
            '∧' => result.push_str("&&"),
            '∨' => result.push_str("||"),
            '¬' => result.push('!'),
            'π' => result.push_str("pi"),
            '∞' => result.push_str("inf"),
            _ => result.push(character),
        }
    }
    result
}

fn rewrite_derivatives(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '*'
            && prefix_position(&chars, index)
            && starts_derivative(&chars, index + 1)
        {
            index += 1;
            let (replacement, next) = derivative_at(&chars, index);
            result.push_str("global:");
            result.push_str(&replacement);
            index = next;
            continue;
        }
        if starts_derivative(&chars, index) {
            let (replacement, next) = derivative_at(&chars, index);
            result.push_str(&replacement);
            index = next;
            continue;
        }
        result.push(chars[index]);
        index += 1;
    }
    result
}

fn prefix_position(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let previous = chars[index - 1];
    !(previous.is_alphanumeric()
        || previous == '_'
        || previous == ')'
        || previous == ']'
        || previous == '}'
        || previous == '\''
        || previous == '"')
}

fn starts_derivative(chars: &[char], index: usize) -> bool {
    if index > 0 {
        let previous = chars[index - 1];
        if previous.is_alphanumeric() || previous == '_' {
            return false;
        }
    }
    if chars.get(index) == Some(&'d') && chars.get(index + 1) == Some(&'(') {
        return true;
    }
    chars.get(index) == Some(&'d')
        && chars.get(index + 1) == Some(&'/')
        && chars.get(index + 2) == Some(&'d')
}

fn derivative_at(chars: &[char], index: usize) -> (String, usize) {
    if chars.get(index) == Some(&'d')
        && chars.get(index + 1) == Some(&'/')
        && chars.get(index + 2) == Some(&'d')
    {
        let mut variable_end = index + 3;
        while chars
            .get(variable_end)
            .is_some_and(|character| character.is_alphanumeric() || *character == '_')
        {
            variable_end += 1;
        }
        let variable: String = chars[index + 3..variable_end].iter().collect();
        let mut open = variable_end;
        while chars
            .get(open)
            .is_some_and(|character| character.is_whitespace())
        {
            open += 1;
        }
        if chars.get(open) == Some(&'(')
            && let Some(close) = matching_paren(chars, open)
        {
            let inner: String = chars[open + 1..close].iter().collect();
            return (
                format!("derivative({},{})", rewrite_derivatives(&inner), variable),
                close + 1,
            );
        }
        return (chars[index..variable_end].iter().collect(), variable_end);
    }
    let open = index + 1;
    let Some(close) = matching_paren(chars, open) else {
        return (chars[index..].iter().collect(), chars.len());
    };
    let mut after = close + 1;
    if chars.get(after) != Some(&'/') || chars.get(after + 1) != Some(&'d') {
        return (chars[index..=close].iter().collect(), close + 1);
    }
    after += 2;
    let variable_start = after;
    while chars
        .get(after)
        .is_some_and(|character| character.is_alphanumeric() || *character == '_')
    {
        after += 1;
    }
    if variable_start == after {
        return (chars[index..=close].iter().collect(), close + 1);
    }
    let variable: String = chars[variable_start..after].iter().collect();
    let inner: String = chars[open + 1..close].iter().collect();
    (
        format!("derivative({},{})", rewrite_derivatives(&inner), variable),
        after,
    )
}

fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, character) in chars.iter().enumerate().skip(open) {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    for character in text.chars() {
        if character == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
            if character == separator && depth == 0 {
                parts.push(current.trim().to_string());
                current.clear();
                continue;
            }
        }
        current.push(character);
    }
    parts.push(current.trim().to_string());
    parts
}

fn find_top_level(text: &str, target: char) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        if character == '"' {
            in_string = !in_string;
        }
        if in_string {
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if character == target && depth == 0 {
            return Some(index);
        }
    }
    None
}

fn strip_prefix_word<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() < prefix.len() {
        return None;
    }
    let (head, rest) = text.split_at(prefix.len());
    if !head.eq_ignore_ascii_case(prefix) {
        return None;
    }
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest)
    } else {
        None
    }
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    characters.all(|character| {
        character.is_alphanumeric() || character == '_' || character == '\'' || character == '$'
    })
}

fn error(message: &str, line: usize, column: usize) -> Diagnostic {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_derivative_definitions_and_clauses() {
        let source = r#"
module calculus.

use algebra.

export derive, antiderivative.

we define d(sin(u))/dx = cos(u) * d(u)/dx,
where u is an expression in x.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        assert_eq!(program.name, "calculus");
        assert!(matches!(program.body[0], ProseStmt::Use(ref name) if name == "algebra"));
        let ProseStmt::Define(definition) = &program.body[2] else {
            panic!("expected a definition");
        };
        assert_eq!(definition.name, "derivative");
        assert!(!definition.global);
        assert_eq!(definition.parameters, vec!["sin(u)", "x"]);
        assert_eq!(definition.body, "cos(u)*derivative(u,x)");
        assert_eq!(definition.clauses[0].variable.as_deref(), Some("u"));
        assert_eq!(
            definition.clauses[0].domain.as_deref(),
            Some("an expression in x")
        );
    }

    #[test]
    fn clauses_can_precede_the_definition() {
        let source = r#"
module calculus.

where f does not depend on x,
we define d(f)/dx = 0.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        let ProseStmt::Define(definition) = &program.body[0] else {
            panic!("expected a definition");
        };
        assert_eq!(definition.name, "derivative");
        assert_eq!(definition.body, "0");
        assert_eq!(definition.clauses.len(), 1);
    }

    #[test]
    fn global_asterisk_and_explicit_variable_derivative() {
        let source = r#"
module calculus.

we define *d(u^2)/dx = 2 * u * d(u)/dx.

we define d/dx (u^3) = 3 * u^2 * d(u)/dx.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        let ProseStmt::Define(first) = &program.body[0] else {
            panic!("expected a definition");
        };
        assert!(first.global);
        assert_eq!(first.body, "2*u*derivative(u,x)");
        let ProseStmt::Define(second) = &program.body[1] else {
            panic!("expected a definition");
        };
        assert_eq!(second.body, "3*u^2*derivative(u,x)");
    }

    #[test]
    fn structured_patterns_bind_variables() {
        let source = r#"
module patterns.

we define f(sin(u)) = cos(u).

end module.
"#;
        let program = lower_prose_program(&parse_prose_program(source).unwrap()).unwrap();
        let Stmt::Def { parameters, .. } = &program.body[0] else {
            panic!("expected a definition");
        };
        match &parameters[0] {
            PatternSource::Literal(source) => assert_eq!(source.text, "sin(u_)"),
            _ => panic!("expected a literal pattern"),
        }
    }

    #[test]
    fn variadic_patterns_use_double_underscore() {
        let source = r#"
module patterns.

we define firstOf(plus(a, rest...)) = a.

end module.
"#;
        let program = lower_prose_program(&parse_prose_program(source).unwrap()).unwrap();
        let Stmt::Def { parameters, .. } = &program.body[0] else {
            panic!("expected a definition");
        };
        match &parameters[0] {
            PatternSource::Literal(source) => assert_eq!(source.text, "plus(a_,rest__)"),
            _ => panic!("expected a literal pattern"),
        }
    }

    #[test]
    fn global_marker_is_preserved_on_the_right_side() {
        let source = r#"
module calculus.

we define f(x) = *d(u)/dx + d(u)/dx.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        let ProseStmt::Define(definition) = &program.body[0] else {
            panic!("expected a definition");
        };
        assert_eq!(definition.body, "global:derivative(u,x)+derivative(u,x)");
    }

    #[test]
    fn parses_theorem_and_proof_skeleton() {
        let source = r#"
module calculus.

theorem derivative_of_sine.
for every expression u in x, d(sin(u))/dx = cos(u) * d(u)/dx.
proof:
by the definition of d.
end proof.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        let ProseStmt::Theorem(theorem) = &program.body[0] else {
            panic!("expected a theorem");
        };
        assert_eq!(theorem.name, "derivative_of_sine");
        assert_eq!(theorem.proof.len(), 1);
    }

    #[test]
    fn unicode_and_ascii_are_accepted() {
        let source = r#"
module Unicode.

we define π = 3 + 1/7.

we define f(x) = x ≤ 2 ∧ x ≠ 1.

end module.
"#;
        let program = parse_prose_program(source).unwrap();
        assert_eq!(program.name, "Unicode");
        assert_eq!(program.body.len(), 2);
    }

    #[test]
    fn local_let_syntax_lowers() {
        assert_eq!(
            normalize_formula("let y = x + 1 in y^2").unwrap(),
            "let([y=x+1],y^2)"
        );
        assert_eq!(
            normalize_formula("let y = x + 1 in let z = y * 2 in z").unwrap(),
            "let([y=x+1],let([z=y*2],z))"
        );
    }

    #[test]
    fn local_function_definitions_lower() {
        assert_eq!(
            normalize_formula(
                "let loop(0, acc) = acc, loop(k, acc) = loop(k - 1, acc * k) in loop(n, 1)"
            )
            .unwrap(),
            "localDefs([setDelayed(loop(0,acc_),acc),setDelayed(loop(k_,acc_),loop(k-1,acc*k))],loop(n,1))"
        );
    }

    #[test]
    fn unicode_operators_normalize_for_evaluation() {
        assert_eq!(normalize_formula("x ≤ 2 ∧ x ≠ 1").unwrap(), "x<=2&&x=/=1");
        assert_eq!(normalize_formula("2 × 3 − 1").unwrap(), "2*3-1");
    }

    #[test]
    fn missing_period_is_rejected() {
        let source = "module calculus\nwe define f(x) = x\n";
        assert!(parse_prose_program(source).is_err());
    }

    #[test]
    fn malformed_blocks_are_rejected() {
        for source in [
            "module m.\nif 1 > 0 then:\nprint 1.\nend module.",
            "module m.\nelse:\nend module.",
            "module m.\nend if.\nend module.",
            "module m.\ncase 1:\nend module.",
            "module m.\nmatch 1 with:\ncase 1:\nprint 1.\nend module.",
            "module m.\ntheorem t.\n1 = 1.\nend module.",
        ] {
            assert!(parse_prose_program(source).is_err(), "{source}");
        }
    }

    #[test]
    fn detects_prose_modules() {
        assert!(looks_like_prose_program("module calculus.\n"));
        assert!(!looks_like_prose_program("mod calculus\n"));
    }
}
