//! Exact symbolic calculus.
//!
//! Differentiation and integration work directly on canonical expressions.
//! Unsupported forms return `None`, which leaves the original calculus call
//! unevaluated.

use crate::expr::{Arena, ExprId, Node};
use crate::num::{Int, Rat};

/// Expression heads shared with the evaluator.
#[derive(Copy, Clone)]
pub(crate) struct Context {
    pub(crate) plus: ExprId,
    pub(crate) times: ExprId,
    pub(crate) power: ExprId,
    pub(crate) negate: ExprId,
    pub(crate) list: ExprId,
    pub(crate) equal: ExprId,
}

fn as_rat(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

fn rat_expression(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

fn int_expression(arena: &mut Arena, value: i64) -> ExprId {
    arena.intern_int(Int::from_i64(value))
}

fn call(arena: &mut Arena, name: &str, args: &[ExprId]) -> ExprId {
    let symbol = arena.symbol_or_intern(name);
    let head = arena.symbol_expr(symbol);
    arena.intern_call(head, args)
}

fn sum(arena: &mut Arena, context: &Context, terms: &[ExprId]) -> ExprId {
    let mut constant = Rat::zero();
    let mut collected = Vec::new();
    for term in terms {
        collect_sum(arena, context, *term, &mut constant, &mut collected);
    }
    if !constant.is_zero() {
        collected.insert(0, rat_expression(arena, &constant));
    }
    match collected.len() {
        0 => int_expression(arena, 0),
        1 => collected[0],
        _ => arena.intern_call(context.plus, &collected),
    }
}

fn collect_sum(
    arena: &Arena,
    context: &Context,
    expression: ExprId,
    constant: &mut Rat,
    terms: &mut Vec<ExprId>,
) {
    if let Some(value) = as_rat(arena, expression) {
        *constant = constant.add(&value);
        return;
    }
    match arena.node(expression) {
        Node::Call { head, args } if *head == context.plus => {
            for arg in args.iter() {
                collect_sum(arena, context, *arg, constant, terms);
            }
        }
        _ => terms.push(expression),
    }
}

fn product(arena: &mut Arena, context: &Context, factors: &[ExprId]) -> ExprId {
    let mut constant = Rat::one();
    let mut collected = Vec::new();
    for factor in factors {
        collect_product(arena, context, *factor, &mut constant, &mut collected);
    }
    if constant.is_zero() {
        return int_expression(arena, 0);
    }
    if !constant.is_one() {
        collected.insert(0, rat_expression(arena, &constant));
    }
    match collected.len() {
        0 => int_expression(arena, 1),
        1 => collected[0],
        _ => arena.intern_call(context.times, &collected),
    }
}

fn collect_product(
    arena: &Arena,
    context: &Context,
    expression: ExprId,
    constant: &mut Rat,
    factors: &mut Vec<ExprId>,
) {
    if let Some(value) = as_rat(arena, expression) {
        *constant = constant.mul(&value);
        return;
    }
    if constant.is_zero() {
        return;
    }
    match arena.node(expression) {
        Node::Call { head, args } if *head == context.times => {
            for arg in args.iter() {
                collect_product(arena, context, *arg, constant, factors);
            }
        }
        _ => factors.push(expression),
    }
}

fn negation(arena: &mut Arena, context: &Context, value: ExprId) -> ExprId {
    if let Some(rational) = as_rat(arena, value) {
        return rat_expression(arena, &rational.neg());
    }
    if let Node::Call { head, args } = arena.node(value)
        && *head == context.negate
        && args.len() == 1
    {
        return args[0];
    }
    arena.intern_call(context.negate, &[value])
}

fn power(arena: &mut Arena, context: &Context, base: ExprId, exponent: ExprId) -> ExprId {
    if let Some(value) = as_rat(arena, exponent) {
        if value.is_zero() {
            return int_expression(arena, 1);
        }
        if value.is_one() {
            return base;
        }
    }
    arena.intern_call(context.power, &[base, exponent])
}

fn divide(arena: &mut Arena, context: &Context, numerator: ExprId, denominator: ExprId) -> ExprId {
    let exponent = int_expression(arena, -1);
    let inverse = power(arena, context, denominator, exponent);
    product(arena, context, &[numerator, inverse])
}

fn contains(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    if expression == variable {
        return true;
    }
    match arena.node(expression) {
        Node::Call { args, .. } => args.iter().any(|arg| contains(arena, *arg, variable)),
        _ => false,
    }
}

fn function_name(arena: &Arena, head: ExprId) -> Option<&str> {
    arena
        .symbol_id(head)
        .map(|symbol| arena.symbol_name(symbol))
}

mod differentiate;
mod integrate;

pub(crate) use differentiate::differentiate;
pub(crate) use integrate::integrate;
