//! Symbolic differentiation.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

pub(crate) fn differentiate(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    match arena.node(expression) {
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => Some(int_expression(arena, 0)),
        Node::Symbol(_) => Some(int_expression(
            arena,
            if expression == variable { 1 } else { 0 },
        )),
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            if head == context.plus {
                let mut derivatives = Vec::with_capacity(args.len());
                for arg in args {
                    derivatives.push(differentiate(arena, arg, variable, context)?);
                }
                return Some(sum(arena, context, &derivatives));
            }
            if head == context.negate {
                if args.len() != 1 {
                    return None;
                }
                let derivative = differentiate(arena, args[0], variable, context)?;
                return Some(negation(arena, context, derivative));
            }
            if head == context.times {
                let mut terms = Vec::with_capacity(args.len());
                for index in 0..args.len() {
                    let derivative = differentiate(arena, args[index], variable, context)?;
                    let mut factors = Vec::with_capacity(args.len());
                    for (other_index, arg) in args.iter().enumerate() {
                        if other_index != index {
                            factors.push(*arg);
                        }
                    }
                    factors.push(derivative);
                    terms.push(product(arena, context, &factors));
                }
                return Some(sum(arena, context, &terms));
            }
            if head == context.power {
                return differentiate_power(arena, &args, variable, context);
            }
            let name = function_name(arena, head)?.to_string();
            let argument = match args.as_slice() {
                [argument] => *argument,
                _ => return None,
            };
            let inner = differentiate(arena, argument, variable, context)?;
            let one = int_expression(arena, 1);
            let two = int_expression(arena, 2);
            let minus_one = int_expression(arena, -1);
            let minus_two = int_expression(arena, -2);
            let negative_half = rat_expression(arena, &Rat::from_i64(-1, 2).ok()?);
            match name.as_str() {
                "sin" => {
                    let cosine = call(arena, "cos", &[argument]);
                    Some(product(arena, context, &[cosine, inner]))
                }
                "cos" => {
                    let sine = call(arena, "sin", &[argument]);
                    let negative = negation(arena, context, sine);
                    Some(product(arena, context, &[negative, inner]))
                }
                "tan" => {
                    let cosine = call(arena, "cos", &[argument]);
                    let denominator = power(arena, context, cosine, minus_two);
                    Some(product(arena, context, &[denominator, inner]))
                }
                "exp" => {
                    let value = call(arena, "exp", &[argument]);
                    Some(product(arena, context, &[value, inner]))
                }
                "ln" => Some(divide(arena, context, inner, argument)),
                "sqrt" => {
                    let root = call(arena, "sqrt", &[argument]);
                    let denominator = product(arena, context, &[two, root]);
                    Some(divide(arena, context, inner, denominator))
                }
                "asin" | "acos" => {
                    let squared = power(arena, context, argument, two);
                    let negative_squared = negation(arena, context, squared);
                    let difference = sum(arena, context, &[one, negative_squared]);
                    let root = power(arena, context, difference, negative_half);
                    let derivative = product(arena, context, &[root, inner]);
                    if name == "acos" {
                        Some(negation(arena, context, derivative))
                    } else {
                        Some(derivative)
                    }
                }
                "atan" => {
                    let squared = power(arena, context, argument, two);
                    let denominator = sum(arena, context, &[one, squared]);
                    Some(divide(arena, context, inner, denominator))
                }
                "sinh" => {
                    let value = call(arena, "cosh", &[argument]);
                    Some(product(arena, context, &[value, inner]))
                }
                "cosh" => {
                    let value = call(arena, "sinh", &[argument]);
                    Some(product(arena, context, &[value, inner]))
                }
                "tanh" => {
                    let cosine = call(arena, "cosh", &[argument]);
                    let denominator = power(arena, context, cosine, minus_two);
                    Some(product(arena, context, &[denominator, inner]))
                }
                "asinh" => {
                    let squared = power(arena, context, argument, two);
                    let sum_value = sum(arena, context, &[one, squared]);
                    let root = power(arena, context, sum_value, negative_half);
                    Some(product(arena, context, &[root, inner]))
                }
                "acosh" => {
                    let squared = power(arena, context, argument, two);
                    let negative_one = negation(arena, context, one);
                    let difference = sum(arena, context, &[squared, negative_one]);
                    let root = power(arena, context, difference, negative_half);
                    Some(product(arena, context, &[root, inner]))
                }
                "atanh" => {
                    let squared = power(arena, context, argument, two);
                    let negative_squared = negation(arena, context, squared);
                    let difference = sum(arena, context, &[one, negative_squared]);
                    Some(divide(arena, context, inner, difference))
                }
                "abs" => {
                    let magnitude = call(arena, "abs", &[argument]);
                    let inverse = power(arena, context, magnitude, minus_one);
                    Some(product(arena, context, &[argument, inverse, inner]))
                }
                _ => {
                    if contains(arena, expression, variable) {
                        None
                    } else {
                        Some(int_expression(arena, 0))
                    }
                }
            }
        }
    }
}

fn differentiate_power(
    arena: &mut Arena,
    args: &[ExprId],
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    if args.len() != 2 {
        return None;
    }
    let base = args[0];
    let exponent = args[1];
    let base_has_variable = contains(arena, base, variable);
    let exponent_has_variable = contains(arena, exponent, variable);
    if !base_has_variable && !exponent_has_variable {
        return Some(int_expression(arena, 0));
    }
    let base_derivative = if base_has_variable {
        Some(differentiate(arena, base, variable, context)?)
    } else {
        None
    };
    let exponent_derivative = if exponent_has_variable {
        Some(differentiate(arena, exponent, variable, context)?)
    } else {
        None
    };
    if !base_has_variable {
        let value = power(arena, context, base, exponent);
        let logarithm = call(arena, "ln", &[base]);
        let exponent_derivative = exponent_derivative?;
        return Some(product(
            arena,
            context,
            &[value, logarithm, exponent_derivative],
        ));
    }
    if !exponent_has_variable {
        let value = if let (Some(value), Some(unit)) = (as_rat(arena, exponent), Some(Rat::one())) {
            let lowered = value.sub(&unit);
            let lowered_expression = rat_expression(arena, &lowered);
            power(arena, context, base, lowered_expression)
        } else {
            let one = int_expression(arena, 1);
            let negative_one = negation(arena, context, one);
            let lowered = sum(arena, context, &[exponent, negative_one]);
            power(arena, context, base, lowered)
        };
        let exponent_expression = if let Some(value) = as_rat(arena, exponent) {
            rat_expression(arena, &value)
        } else {
            exponent
        };
        let base_derivative = base_derivative?;
        return Some(product(
            arena,
            context,
            &[exponent_expression, value, base_derivative],
        ));
    }

    let value = power(arena, context, base, exponent);
    let logarithm = call(arena, "ln", &[base]);
    let base_derivative = base_derivative?;
    let exponent_derivative = exponent_derivative?;
    let exponent_term = product(arena, context, &[exponent_derivative, logarithm]);
    let divided = divide(arena, context, base_derivative, base);
    let base_term = product(arena, context, &[exponent, divided]);
    let inner = sum(arena, context, &[exponent_term, base_term]);
    Some(product(arena, context, &[value, inner]))
}
