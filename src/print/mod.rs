//! Canonical and infix printers.

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

/// Render an expression in the human surface form.
///
/// The output contains no whitespace and parses back into the same expression.
pub fn infix(arena: &Arena, id: ExprId) -> String {
    let mut out = String::new();
    write_infix(arena, id, 0, &mut out);
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

const P_SET: u8 = 1;
const P_RULE: u8 = 2;
const P_OR: u8 = 3;
const P_AND: u8 = 4;
const P_COMPARE: u8 = 5;
const P_ADD: u8 = 6;
const P_MUL: u8 = 7;
const P_UNARY: u8 = 8;
const P_POWER: u8 = 9;
const P_ATOM: u8 = 10;

fn head_name(arena: &Arena, id: ExprId) -> Option<&str> {
    match arena.node(id) {
        Node::Symbol(symbol) => Some(arena.symbol_name(*symbol)),
        Node::Call { head, .. } => match arena.node(*head) {
            Node::Symbol(symbol) => Some(arena.symbol_name(*symbol)),
            _ => None,
        },
        _ => None,
    }
}

fn precedence(arena: &Arena, id: ExprId) -> u8 {
    match arena.node(id) {
        Node::Int(value) => {
            if value.sign() < 0 {
                P_UNARY
            } else {
                P_ATOM
            }
        }
        Node::Rat(_) => P_MUL,
        Node::Symbol(_) => P_ATOM,
        Node::Call { .. } => match head_name(arena, id) {
            Some("set") | Some("setDelayed") => P_SET,
            Some("rule") | Some("ruleDelayed") => P_RULE,
            Some("or") => P_OR,
            Some("and") => P_AND,
            Some("equal") | Some("notEqual") | Some("same") | Some("less") | Some("lessEqual")
            | Some("greater") | Some("greaterEqual") => P_COMPARE,
            Some("plus") => P_ADD,
            Some("times") => P_MUL,
            Some("negate") | Some("not") => P_UNARY,
            Some("power") => P_POWER,
            _ => P_ATOM,
        },
    }
}

fn write_infix(arena: &Arena, id: ExprId, min: u8, out: &mut String) {
    let node_precedence = precedence(arena, id);
    if node_precedence < min {
        out.push('(');
        write_infix_inner(arena, id, out);
        out.push(')');
    } else {
        write_infix_inner(arena, id, out);
    }
}

fn write_infix_inner(arena: &Arena, id: ExprId, out: &mut String) {
    match arena.node(id) {
        Node::Int(value) => {
            let _ = out.write_str(&value.to_decimal());
        }
        Node::Rat(value) => {
            let _ = out.write_str(&value.to_decimal());
        }
        Node::Symbol(symbol) => {
            let _ = out.write_str(arena.symbol_name(*symbol));
        }
        Node::Call { args, .. } => match head_name(arena, id) {
            Some("plus") => write_join(arena, args, "+", P_ADD, P_ADD + 1, out),
            Some("times") => write_join(arena, args, "*", P_MUL, P_MUL + 1, out),
            Some("and") => write_join(arena, args, "&&", P_AND, P_AND + 1, out),
            Some("or") => write_join(arena, args, "||", P_OR, P_OR + 1, out),
            Some("set") => write_binary(arena, args, "=", P_SET, P_SET + 1, P_SET, out),
            Some("setDelayed") => write_binary(arena, args, ":=", P_SET, P_SET + 1, P_SET, out),
            Some("rule") => write_binary(arena, args, "->", P_RULE, P_RULE + 1, P_RULE, out),
            Some("ruleDelayed") => write_binary(arena, args, ":>", P_RULE, P_RULE + 1, P_RULE, out),
            Some("equal") => write_binary(
                arena,
                args,
                "===",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("notEqual") => write_binary(
                arena,
                args,
                "=/=",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("same") => write_binary(
                arena,
                args,
                "~",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("less") => write_binary(
                arena,
                args,
                "<",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("lessEqual") => write_binary(
                arena,
                args,
                "<=",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("greater") => write_binary(
                arena,
                args,
                ">",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("greaterEqual") => write_binary(
                arena,
                args,
                ">=",
                P_COMPARE,
                P_COMPARE + 1,
                P_COMPARE + 1,
                out,
            ),
            Some("negate") => {
                out.push('-');
                if let Some(arg) = args.first() {
                    write_infix(arena, *arg, P_UNARY, out);
                }
            }
            Some("not") => {
                out.push('!');
                if let Some(arg) = args.first() {
                    write_infix(arena, *arg, P_UNARY, out);
                }
            }
            Some("power") => {
                if args.len() == 2 {
                    write_infix(arena, args[0], P_POWER + 1, out);
                    out.push('^');
                    write_infix(arena, args[1], P_POWER, out);
                } else {
                    write_call(arena, id, args, out);
                }
            }
            Some("list") => {
                out.push('[');
                write_args(arena, args, out);
                out.push(']');
            }
            Some("pattern") => write_pattern(arena, args, out),
            Some("blank") => out.push('_'),
            Some("blankSequence") => out.push_str("__"),
            Some("blankNullSequence") => out.push_str("___"),
            Some("condition") => {
                if args.len() == 2 {
                    out.push_str("when(");
                    write_infix(arena, args[0], 0, out);
                    out.push(',');
                    write_infix(arena, args[1], 0, out);
                    out.push(')');
                } else {
                    write_call(arena, id, args, out);
                }
            }
            Some("function") => write_function(arena, args, out),
            _ => write_call(arena, id, args, out),
        },
    }
}

fn write_join(
    arena: &Arena,
    args: &[ExprId],
    separator: &str,
    prec: u8,
    child: u8,
    out: &mut String,
) {
    if args.is_empty() {
        out.push_str("()");
        return;
    }
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push_str(separator);
        }
        write_infix(arena, *arg, child, out);
    }
    let _ = prec;
}

fn write_binary(
    arena: &Arena,
    args: &[ExprId],
    separator: &str,
    _prec: u8,
    left: u8,
    right: u8,
    out: &mut String,
) {
    if args.len() == 2 {
        write_infix(arena, args[0], left, out);
        out.push_str(separator);
        write_infix(arena, args[1], right, out);
    } else {
        out.push_str("()");
    }
}

fn write_args(arena: &Arena, args: &[ExprId], out: &mut String) {
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_infix(arena, *arg, 0, out);
    }
}

fn write_call(arena: &Arena, id: ExprId, args: &[ExprId], out: &mut String) {
    if let Node::Call { head, .. } = arena.node(id) {
        match arena.node(*head) {
            Node::Symbol(symbol) => {
                let _ = out.write_str(arena.symbol_name(*symbol));
            }
            _ => {
                write_infix(arena, *head, P_ATOM, out);
            }
        }
    }
    out.push('(');
    write_args(arena, args, out);
    out.push(')');
}

fn write_pattern(arena: &Arena, args: &[ExprId], out: &mut String) {
    if args.len() != 2 {
        out.push_str("pattern(");
        write_args(arena, args, out);
        out.push(')');
        return;
    }
    if let Node::Symbol(symbol) = arena.node(args[0]) {
        let _ = out.write_str(arena.symbol_name(*symbol));
        match arena.node(args[1]) {
            Node::Call { head, .. } if head_name(arena, *head) == Some("blank") => {
                out.push('_');
                return;
            }
            Node::Call { head, .. } if head_name(arena, *head) == Some("blankSequence") => {
                out.push_str("__");
                return;
            }
            Node::Call { head, .. } if head_name(arena, *head) == Some("blankNullSequence") => {
                out.push_str("___");
                return;
            }
            _ => {}
        }
    }
    out.push_str("pattern(");
    write_args(arena, args, out);
    out.push(')');
}

fn write_function(arena: &Arena, args: &[ExprId], out: &mut String) {
    if args.len() != 2 {
        out.push_str("function(");
        write_args(arena, args, out);
        out.push(')');
        return;
    }
    let params = match arena.node(args[0]) {
        Node::Call { head, args: params } if head_name(arena, *head) == Some("list") => {
            Some(params.as_ref())
        }
        _ => None,
    };
    let Some(params) = params else {
        out.push_str("function(");
        write_args(arena, args, out);
        out.push(')');
        return;
    };
    let mut names = Vec::with_capacity(params.len());
    for param in params {
        match arena.node(*param) {
            Node::Symbol(symbol) => names.push(arena.symbol_name(*symbol)),
            _ => {
                out.push_str("function(");
                write_args(arena, args, out);
                out.push(')');
                return;
            }
        }
    }
    out.push_str("fn(");
    for (index, name) in names.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(name);
    }
    out.push_str(")=>");
    write_infix(arena, args[1], 0, out);
}
