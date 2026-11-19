//! Canonical printer.

#[allow(unused_imports)]
use crate::prelude::*;

use core::fmt::Write;

use crate::expr::{Arena, ExprId, Node};

/// Render an expression in canonical form.
///
/// The output uses full head names, contains no whitespace, and parses back
/// into the same expression.
pub fn canonical(arena: &Arena, id: ExprId) -> String {
    let mut out = String::new();
    write_canonical(arena, id, &mut out);
    out
}

fn write_canonical(arena: &Arena, id: ExprId, out: &mut String) {
    match arena.node(id) {
        Node::Int(value) => {
            let _ = out.write_str(&value.to_decimal());
        }
        Node::Rat(value) => {
            let _ = out.write_str(&value.to_decimal());
        }
        Node::Real(value) => {
            let _ = out.write_str(&value.to_decimal());
        }
        Node::String(value) => write_string(value, out),
        Node::Bytes(value) => {
            out.push_str("byteArray(");
            for (index, byte) in value.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{byte}");
            }
            out.push(')');
        }
        Node::Symbol(symbol) => {
            let _ = out.write_str(&arena.symbol_full_name(*symbol));
        }
        Node::Call { head, args } => {
            write_canonical(arena, *head, out);
            out.push('(');
            for (index, arg) in args.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(arena, *arg, out);
            }
            out.push(')');
        }
        Node::Association(entries) => {
            out.push_str("<|");
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(arena, entry.key, out);
                out.push_str("->");
                write_canonical(arena, entry.value, out);
            }
            out.push_str("|>");
        }
    }
}

fn write_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            other => out.push(other),
        }
    }
    out.push('"');
}
