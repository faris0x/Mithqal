//! Exact polynomial solving.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

pub(crate) fn solve(
    arena: &mut Arena,
    equation: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let polynomial = if let Node::Call { head, args } = arena.node(equation) {
        if *head == context.equal && args.len() == 2 {
            poly_from_expr(arena, context, args[0])?.sub(&poly_from_expr(arena, context, args[1])?)
        } else {
            poly_from_expr(arena, context, equation)?
        }
    } else {
        poly_from_expr(arena, context, equation)?
    };
    let coefficients = polynomial.dense_in(variable)?;
    if coefficients.len() <= 1 {
        if coefficients.first().is_some_and(Rat::is_zero) || coefficients.is_empty() {
            return None;
        }
        return Some(arena.intern_call(context.list, &[]));
    }

    let (roots, remaining) = rational_roots(&coefficients);
    let mut solutions = Vec::new();
    for (root, _) in roots {
        solutions.push(expression_from_rat(arena, &root));
    }
    let degree = remaining.len().saturating_sub(1);
    match degree {
        0 => {}
        1 => {
            let root = Rat::zero().sub(&remaining[0]).div(&remaining[1]).ok()?;
            solutions.push(expression_from_rat(arena, &root));
        }
        2 => {
            quadratic_solutions(arena, &remaining, context, &mut solutions)?;
        }
        _ => return None,
    }

    solutions.sort_by(|left, right| arena.canonical_cmp(*left, *right));
    solutions.dedup();
    Some(arena.intern_call(context.list, &solutions))
}

fn quadratic_solutions(
    arena: &mut Arena,
    coefficients: &[Rat],
    context: &Context,
    solutions: &mut Vec<ExprId>,
) -> Option<()> {
    let a = coefficients[2].clone();
    let b = coefficients[1].clone();
    let c = coefficients[0].clone();
    let four = Rat::from_i64(4, 1).ok()?;
    let discriminant = b.mul(&b).sub(&four.mul(&a).mul(&c));
    let denominator = Rat::from_i64(2, 1).ok()?.mul(&a);
    if discriminant.sign() >= 0 {
        if let Some(root) = rational_sqrt(&discriminant) {
            let first = Rat::zero().sub(&b).add(&root).div(&denominator).ok()?;
            let second = Rat::zero().sub(&b).sub(&root).div(&denominator).ok()?;
            solutions.push(expression_from_rat(arena, &first));
            if first != second {
                solutions.push(expression_from_rat(arena, &second));
            }
            return Some(());
        }
        let square_root = sqrt_value_expression(arena, &discriminant, context)?;
        push_quadratic_root(
            arena,
            &b,
            &denominator,
            square_root,
            false,
            context,
            solutions,
        );
        push_quadratic_root(
            arena,
            &b,
            &denominator,
            square_root,
            true,
            context,
            solutions,
        );
        return Some(());
    }

    let magnitude = discriminant.neg();
    let square_root = sqrt_value_expression(arena, &magnitude, context)?;
    let symbol = arena.symbol_or_intern("i");
    let imaginary_unit = arena.symbol_expr(symbol);
    let imaginary = arena.intern_call(context.times, &[imaginary_unit, square_root]);
    push_quadratic_root(
        arena,
        &b,
        &denominator,
        imaginary,
        false,
        context,
        solutions,
    );
    push_quadratic_root(arena, &b, &denominator, imaginary, true, context, solutions);
    Some(())
}

fn sqrt_value_expression(arena: &mut Arena, value: &Rat, context: &Context) -> Option<ExprId> {
    if let Some(root) = rational_sqrt(value) {
        return Some(expression_from_rat(arena, &root));
    }
    let (factor, radicand) = sqrt_decompose(value)?;
    if radicand.is_one() {
        return Some(expression_from_rat(arena, &factor));
    }
    let root = sqrt_expression(arena, &radicand);
    if factor.is_one() {
        Some(root)
    } else {
        let factor = expression_from_rat(arena, &factor);
        Some(product(arena, &[factor, root], context))
    }
}

fn sqrt_decompose(value: &Rat) -> Option<(Rat, Rat)> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    if numerator < 0 || denominator <= 0 {
        return None;
    }
    if numerator > 1_000_000_000_000 || denominator > 1_000_000_000_000 {
        return None;
    }
    let (numerator_square, numerator_rest) = square_part(numerator);
    let (denominator_square, denominator_rest) = square_part(denominator);
    Some((
        Rat::from_i64(numerator_square, denominator_square).ok()?,
        Rat::from_i64(numerator_rest, denominator_rest).ok()?,
    ))
}

fn square_part(mut value: i64) -> (i64, i64) {
    let mut square = 1i64;
    let mut rest = 1i64;
    let mut factor = 2i64;
    while factor * factor <= value {
        let mut exponent = 0;
        while value % factor == 0 {
            value /= factor;
            exponent += 1;
        }
        if exponent > 0 {
            let half = exponent / 2;
            for _ in 0..half {
                square = square.saturating_mul(factor);
            }
            if exponent % 2 == 1 {
                rest = rest.saturating_mul(factor);
            }
        }
        factor += 1;
    }
    rest = rest.saturating_mul(value);
    (square, rest)
}

fn push_quadratic_root(
    arena: &mut Arena,
    b: &Rat,
    denominator: &Rat,
    radical: ExprId,
    subtract: bool,
    context: &Context,
    solutions: &mut Vec<ExprId>,
) {
    let negated = expression_from_rat(arena, &b.neg());
    let radical = if subtract {
        negate(arena, radical, context)
    } else {
        radical
    };
    let numerator = arena.intern_call(context.plus, &[negated, radical]);
    let denominator = expression_from_rat(arena, denominator);
    let exponent = arena.intern_int(Int::from_i64(-1));
    let inverse = arena.intern_call(context.power, &[denominator, exponent]);
    solutions.push(product(arena, &[numerator, inverse], context));
}

fn sqrt_expression(arena: &mut Arena, value: &Rat) -> ExprId {
    let value = expression_from_rat(arena, value);
    let symbol = arena.symbol_or_intern("sqrt");
    let head = arena.symbol_expr(symbol);
    arena.intern_call(head, &[value])
}
