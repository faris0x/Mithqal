//! Canonical printer.

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
        Node::Symbol(symbol) => {
            let _ = out.write_str(arena.symbol_name(*symbol));
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
    }
}
