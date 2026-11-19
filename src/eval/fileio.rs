//! File and data import/export functions.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::diag::{Diagnostic, ErrorCode};
use crate::expr::AssociationEntry;
use crate::io;
use crate::io::json::Json;
use crate::num::{Int, Rat};

use super::*;

impl Evaluator<'_> {
    pub(super) fn apply_io(
        &mut self,
        function: IoHead,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let value = match function {
            IoHead::ReadString | IoHead::ImportText => {
                let path = self.path(args[0])?;
                self.arena
                    .intern_string(&io::read_string(&path).map_err(io_error)?)
            }
            IoHead::WriteString | IoHead::ExportText => {
                let path = self.path(args[0])?;
                let text = self.text(args[1])?;
                io::write_string(&path, &text).map_err(io_error)?;
                self.heads.null
            }
            IoHead::FileExistsQ => {
                let path = self.path(args[0])?;
                self.boolean(io::exists(&path))
            }
            IoHead::FileNames => {
                let path = if args.is_empty() {
                    ".".to_string()
                } else {
                    self.path(args[0])?
                };
                let names = io::file_names(&path).map_err(io_error)?;
                let mut items = Vec::with_capacity(names.len());
                for name in names {
                    items.push(self.arena.intern_string(&name));
                }
                self.list(&items)
            }
            IoHead::DirectoryName => {
                let path = self.path(args[0])?;
                self.arena.intern_string(&io::directory_name(&path))
            }
            IoHead::FileNameJoin => {
                let items = self.list_items(args[0]).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::IoFailure, "fileNameJoin expects a list")
                })?;
                let mut parts = Vec::with_capacity(items.len());
                for item in items {
                    parts.push(self.text(*item)?);
                }
                self.arena.intern_string(&io::file_name_join(&parts))
            }
            IoHead::CreateDirectory => {
                let path = self.path(args[0])?;
                io::create_directory(&path).map_err(io_error)?;
                self.heads.null
            }
            IoHead::DeleteFile => {
                let path = self.path(args[0])?;
                io::delete_file(&path).map_err(io_error)?;
                self.heads.null
            }
            IoHead::ImportCsv | IoHead::ImportTsv => {
                let path = self.path(args[0])?;
                let text = io::read_string(&path).map_err(io_error)?;
                let delimiter = if matches!(function, IoHead::ImportCsv) {
                    ','
                } else {
                    '\t'
                };
                let rows = io::parse_delimited(&text, delimiter);
                let mut output = Vec::with_capacity(rows.len());
                for row in rows {
                    let mut fields = Vec::with_capacity(row.len());
                    for field in row {
                        fields.push(self.arena.intern_string(&field));
                    }
                    output.push(self.list(&fields));
                }
                self.list(&output)
            }
            IoHead::ExportCsv | IoHead::ExportTsv => {
                let path = self.path(args[0])?;
                let rows = self.rows(args[1])?;
                let delimiter = if matches!(function, IoHead::ExportCsv) {
                    ','
                } else {
                    '\t'
                };
                io::write_string(&path, &io::write_delimited(&rows, delimiter))
                    .map_err(io_error)?;
                self.heads.null
            }
            IoHead::ImportJson => {
                let path = self.path(args[0])?;
                let text = io::read_string(&path).map_err(io_error)?;
                let json = io::json::parse(&text).map_err(io_error)?;
                self.json_expression(&json).ok_or_else(|| {
                    Diagnostic::eval(ErrorCode::IoFailure, "cannot convert JSON to an expression")
                })?
            }
            IoHead::ExportJson => {
                let path = self.path(args[0])?;
                let json = self.expression_json(args[1])?;
                io::write_string(&path, &json_text(&json)).map_err(io_error)?;
                self.heads.null
            }
        };
        if matches!(
            function,
            IoHead::WriteString
                | IoHead::CreateDirectory
                | IoHead::DeleteFile
                | IoHead::ExportText
                | IoHead::ExportCsv
                | IoHead::ExportTsv
                | IoHead::ExportJson
        ) {
            self.definitions.bump_version();
        }
        Ok(Some(value))
    }

    fn path(&self, expression: ExprId) -> Result<String, Diagnostic> {
        self.text(expression)
    }

    fn text(&self, expression: ExprId) -> Result<String, Diagnostic> {
        match self.arena.node(expression) {
            Node::String(value) => Ok(value.to_string()),
            _ => Err(Diagnostic::eval(
                ErrorCode::IoFailure,
                "a file function expects a string",
            )),
        }
    }

    fn rows(&self, expression: ExprId) -> Result<Vec<Vec<String>>, Diagnostic> {
        let Some(rows) = self.list_items(expression) else {
            return Err(Diagnostic::eval(
                ErrorCode::IoFailure,
                "a table function expects a list of rows",
            ));
        };
        let mut output = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(fields) = self.list_items(*row) else {
                return Err(Diagnostic::eval(
                    ErrorCode::IoFailure,
                    "each table row must be a list",
                ));
            };
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                values.push(self.text(*field)?);
            }
            output.push(values);
        }
        Ok(output)
    }

    fn json_expression(&mut self, json: &Json) -> Option<ExprId> {
        Some(match json {
            Json::Null => self.heads.null,
            Json::Bool(value) => self.boolean(*value),
            Json::String(value) => self.arena.intern_string(value),
            Json::Number(text) => self.number_expression(text)?,
            Json::Array(values) => {
                let mut items = Vec::with_capacity(values.len());
                for value in values {
                    items.push(self.json_expression(value)?);
                }
                self.list(&items)
            }
            Json::Object(entries) => {
                let mut output = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    output.push(AssociationEntry {
                        key: self.arena.intern_string(key),
                        value: self.json_expression(value)?,
                    });
                }
                self.arena.intern_association(output).ok()?
            }
        })
    }

    fn expression_json(&self, expression: ExprId) -> Result<Json, Diagnostic> {
        Ok(match self.arena.node(expression) {
            Node::Int(value) => Json::Number(value.to_decimal()),
            Node::Rat(value) if value.den().is_one() => Json::Number(value.to_decimal()),
            Node::Rat(value) => Json::Number(format!("{}", value.to_f64())),
            Node::Real(value) => Json::Number(value.to_decimal()),
            Node::String(value) => Json::String(value.to_string()),
            Node::Bytes(values) => Json::Array(
                values
                    .iter()
                    .map(|byte| Json::Number(byte.to_string()))
                    .collect(),
            ),
            Node::Symbol(symbol) => match self.arena.symbol_name(*symbol) {
                "true" => Json::Bool(true),
                "false" => Json::Bool(false),
                "null" => Json::Null,
                _ => Json::String(self.arena.symbol_display_name(*symbol)),
            },
            Node::Call { head, args } if *head == self.heads.list => {
                let mut values = Vec::with_capacity(args.len());
                for argument in args.iter() {
                    values.push(self.expression_json(*argument)?);
                }
                Json::Array(values)
            }
            Node::Association(entries) => {
                let mut output = Vec::with_capacity(entries.len());
                for entry in entries.iter() {
                    let key = match self.arena.node(entry.key) {
                        Node::String(value) => value.to_string(),
                        _ => crate::print::infix(self.arena, entry.key),
                    };
                    output.push((key, self.expression_json(entry.value)?));
                }
                Json::Object(output)
            }
            _ => Json::String(crate::print::infix(self.arena, expression)),
        })
    }

    fn number_expression(&mut self, text: &str) -> Option<ExprId> {
        if !text.contains(['.', 'e', 'E']) {
            return Some(self.arena.intern_int(Int::from_decimal(text).ok()?));
        }
        let rational = decimal_rational(text)?;
        Some(self.arena.intern_rat(rational))
    }
}

fn io_error(message: String) -> Diagnostic {
    Diagnostic::eval(ErrorCode::IoFailure, message)
}

fn decimal_rational(text: &str) -> Option<Rat> {
    let (negative, text) = match text.strip_prefix('-') {
        Some(text) => (true, text),
        None => (false, text),
    };
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(index) => (&text[..index], text[index + 1..].parse::<i32>().ok()?),
        None => (text, 0),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (mantissa, ""),
    };
    let digits = if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}{fraction}")
    };
    let mut numerator = Int::from_decimal(&digits).ok()?;
    let exponent = exponent - fraction.len() as i32;
    if negative {
        numerator = numerator.neg();
    }
    let mut denominator = Int::one();
    if exponent > 0 {
        numerator = numerator.mul(&Int::from_i64(10).pow_u32(exponent as u32));
    } else if exponent < 0 {
        denominator = Int::from_i64(10).pow_u32((-exponent) as u32);
    }
    Rat::from_ints(numerator, denominator).ok()
}

fn json_text(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(true) => "true".to_string(),
        Json::Bool(false) => "false".to_string(),
        Json::Number(text) => text.clone(),
        Json::String(text) => {
            let mut out = String::from("\"");
            for character in text.chars() {
                match character {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    character if character.is_control() => {
                        let _ = core::fmt::Write::write_fmt(
                            &mut out,
                            format_args!("\\u{:04x}", character as u32),
                        );
                    }
                    character => out.push(character),
                }
            }
            out.push('"');
            out
        }
        Json::Array(values) => {
            let body = values.iter().map(json_text).collect::<Vec<_>>().join(",");
            format!("[{body}]")
        }
        Json::Object(entries) => {
            let body = entries
                .iter()
                .map(|(key, value)| {
                    let key = json_text(&Json::String(key.clone()));
                    format!("{key}:{}", json_text(value))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
    }
}
