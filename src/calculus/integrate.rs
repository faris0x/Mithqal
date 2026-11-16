//! Symbolic integration.

use super::*;

pub(crate) fn integrate(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    if let Some(result) = integrate_raw(arena, expression, variable, context) {
        return Some(result);
    }
    let algebra = crate::algebra::Context {
        plus: context.plus,
        times: context.times,
        power: context.power,
        negate: context.negate,
        list: context.list,
        equal: context.equal,
    };
    let expanded = crate::algebra::expand(arena, expression, &algebra)?;
    if expanded == expression {
        return None;
    }
    integrate_raw(arena, expanded, variable, context)
}

fn integrate_raw(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    if !contains(arena, expression, variable) {
        return Some(product(arena, context, &[expression, variable]));
    }
    if expression == variable {
        let two = int_expression(arena, 2);
        let squared = power(arena, context, variable, two);
        return Some(divide(arena, context, squared, two));
    }
    match arena.node(expression) {
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            if head == context.plus {
                let mut integrals = Vec::with_capacity(args.len());
                for arg in args {
                    integrals.push(integrate_raw(arena, arg, variable, context)?);
                }
                return Some(sum(arena, context, &integrals));
            }
            if head == context.negate {
                if args.len() != 1 {
                    return None;
                }
                let integral = integrate_raw(arena, args[0], variable, context)?;
                return Some(negation(arena, context, integral));
            }
            if head == context.times {
                let mut constants = Vec::new();
                let mut dependent = Vec::new();
                for arg in args {
                    if contains(arena, arg, variable) {
                        dependent.push(arg);
                    } else {
                        constants.push(arg);
                    }
                }
                if dependent.len() == 1 {
                    let integral = integrate_raw(arena, dependent[0], variable, context)?;
                    constants.push(integral);
                    return Some(product(arena, context, &constants));
                }
                if dependent.len() == 2
                    && let Some(integral) =
                        integrate_by_parts(arena, dependent[0], dependent[1], variable, context)?
                {
                    constants.push(integral);
                    return Some(product(arena, context, &constants));
                }
                let combined = product(arena, context, &dependent);
                let algebra = crate::algebra::Context {
                    plus: context.plus,
                    times: context.times,
                    power: context.power,
                    negate: context.negate,
                    list: context.list,
                    equal: context.equal,
                };
                let expanded = crate::algebra::expand(arena, combined, &algebra)?;
                if expanded == combined {
                    return None;
                }
                let integral = integrate_raw(arena, expanded, variable, context)?;
                constants.push(integral);
                return Some(product(arena, context, &constants));
            }
            if head == context.power {
                return integrate_power(arena, &args, variable, context);
            }
            let name = function_name(arena, head)?.to_string();
            let argument = match args.as_slice() {
                [argument] => *argument,
                _ => return None,
            };
            let slope = differentiate(arena, argument, variable, context)?;
            if contains(arena, slope, variable) {
                return None;
            }
            match name.as_str() {
                "exp" => {
                    let value = call(arena, "exp", &[argument]);
                    Some(divide(arena, context, value, slope))
                }
                "sin" => {
                    let value = call(arena, "cos", &[argument]);
                    let negative = negation(arena, context, value);
                    Some(divide(arena, context, negative, slope))
                }
                "cos" => {
                    let value = call(arena, "sin", &[argument]);
                    Some(divide(arena, context, value, slope))
                }
                "tan" => {
                    let cosine = call(arena, "cos", &[argument]);
                    let magnitude = call(arena, "abs", &[cosine]);
                    let logarithm = call(arena, "ln", &[magnitude]);
                    let negative = negation(arena, context, logarithm);
                    Some(divide(arena, context, negative, slope))
                }
                "sinh" => {
                    let value = call(arena, "cosh", &[argument]);
                    Some(divide(arena, context, value, slope))
                }
                "cosh" => {
                    let value = call(arena, "sinh", &[argument]);
                    Some(divide(arena, context, value, slope))
                }
                "tanh" => {
                    let value = call(arena, "cosh", &[argument]);
                    let logarithm = call(arena, "ln", &[value]);
                    Some(divide(arena, context, logarithm, slope))
                }
                "ln" => {
                    let logarithm = call(arena, "ln", &[argument]);
                    let product_value = product(arena, context, &[argument, logarithm]);
                    let negative_argument = negation(arena, context, argument);
                    let numerator = sum(arena, context, &[product_value, negative_argument]);
                    Some(divide(arena, context, numerator, slope))
                }
                "asin" | "acos" | "atan" | "asinh" | "acosh" | "atanh" => {
                    integrate_inverse_function(arena, &name, argument, slope, context)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn integrate_inverse_function(
    arena: &mut Arena,
    name: &str,
    argument: ExprId,
    slope: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let value = call(arena, name, &[argument]);
    let product_value = product(arena, context, &[argument, value]);
    let two = int_expression(arena, 2);
    let one = int_expression(arena, 1);
    let half = rat_expression(arena, &Rat::from_i64(1, 2).ok()?);
    let square = power(arena, context, argument, two);
    let numerator = match name {
        "asin" | "acos" => {
            let negative_square = negation(arena, context, square);
            let inner = sum(arena, context, &[one, negative_square]);
            let root = power(arena, context, inner, half);
            if name == "asin" {
                sum(arena, context, &[product_value, root])
            } else {
                let negative_root = negation(arena, context, root);
                sum(arena, context, &[product_value, negative_root])
            }
        }
        "atan" => {
            let inner = sum(arena, context, &[one, square]);
            let logarithm = call(arena, "ln", &[inner]);
            let correction = product(arena, context, &[half, logarithm]);
            let negative_correction = negation(arena, context, correction);
            sum(arena, context, &[product_value, negative_correction])
        }
        "asinh" => {
            let inner = sum(arena, context, &[one, square]);
            let root = power(arena, context, inner, half);
            let negative_root = negation(arena, context, root);
            sum(arena, context, &[product_value, negative_root])
        }
        "acosh" => {
            let negative_one = negation(arena, context, one);
            let inner = sum(arena, context, &[square, negative_one]);
            let root = power(arena, context, inner, half);
            let negative_root = negation(arena, context, root);
            sum(arena, context, &[product_value, negative_root])
        }
        "atanh" => {
            let negative_square = negation(arena, context, square);
            let inner = sum(arena, context, &[one, negative_square]);
            let logarithm = call(arena, "ln", &[inner]);
            let correction = product(arena, context, &[half, logarithm]);
            sum(arena, context, &[product_value, correction])
        }
        _ => return None,
    };
    Some(divide(arena, context, numerator, slope))
}

fn is_power_of_variable(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    let Node::Call { head, args } = arena.node(expression) else {
        return false;
    };
    args.len() == 2
        && function_name(arena, *head) == Some("power")
        && args[0] == variable
        && as_rat(arena, args[1])
            .is_some_and(|value| value == Rat::from_i64(2, 1).unwrap_or_else(|_| Rat::zero()))
}

fn is_one_plus_square(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    let Node::Call { head, args } = arena.node(expression) else {
        return false;
    };
    if args.len() != 2 || function_name(arena, *head) != Some("plus") {
        return false;
    }
    let one = Rat::from_i64(1, 1).unwrap_or_else(|_| Rat::zero());
    let has_one = args
        .iter()
        .any(|arg| as_rat(arena, *arg).is_some_and(|value| value == one));
    let has_square = args
        .iter()
        .any(|arg| is_power_of_variable(arena, *arg, variable));
    has_one && has_square
}

fn is_negative_square(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    match arena.node(expression) {
        Node::Call { head, args }
            if args.len() == 1 && function_name(arena, *head) == Some("negate") =>
        {
            is_power_of_variable(arena, args[0], variable)
        }
        Node::Call { head, args }
            if args.len() == 2 && function_name(arena, *head) == Some("times") =>
        {
            let one = Rat::from_i64(-1, 1).unwrap_or_else(|_| Rat::zero());
            args.iter()
                .any(|arg| as_rat(arena, *arg).is_some_and(|value| value == one))
                && args
                    .iter()
                    .any(|arg| is_power_of_variable(arena, *arg, variable))
        }
        _ => false,
    }
}

fn is_one_minus_square(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    let Node::Call { head, args } = arena.node(expression) else {
        return false;
    };
    if args.len() != 2 || function_name(arena, *head) != Some("plus") {
        return false;
    }
    let one = Rat::from_i64(1, 1).unwrap_or_else(|_| Rat::zero());
    let has_one = args
        .iter()
        .any(|arg| as_rat(arena, *arg).is_some_and(|value| value == one));
    let has_negative_square = args
        .iter()
        .any(|arg| is_negative_square(arena, *arg, variable));
    has_one && has_negative_square
}

fn sqrt_argument(arena: &Arena, expression: ExprId) -> Option<ExprId> {
    let Node::Call { head, args } = arena.node(expression) else {
        return None;
    };
    if args.len() == 1 && function_name(arena, *head) == Some("sqrt") {
        Some(args[0])
    } else {
        None
    }
}

fn is_polynomial_in(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> bool {
    if !contains(arena, expression, variable) {
        return true;
    }
    match arena.node(expression) {
        Node::Symbol(_) => expression == variable,
        Node::Int(_) | Node::Rat(_) | Node::Real(_) => true,
        Node::Call { head, args } => {
            if *head == context.plus || *head == context.times || *head == context.negate {
                return args
                    .iter()
                    .all(|argument| is_polynomial_in(arena, *argument, variable, context));
            }
            if *head == context.power
                && args.len() == 2
                && let Some(exponent) = as_rat(arena, args[1])
                && exponent.den().is_one()
                && exponent.sign() >= 0
            {
                return is_polynomial_in(arena, args[0], variable, context);
            }
            false
        }
    }
}

fn elementary_argument(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let Node::Call { head, args } = arena.node(expression) else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    let name = function_name(arena, *head)?;
    if !matches!(name, "exp" | "sin" | "cos" | "sinh" | "cosh") {
        return None;
    }
    let argument = args[0];
    let slope = differentiate(arena, argument, variable, context)?;
    if contains(arena, slope, variable) {
        None
    } else {
        Some(expression)
    }
}

fn is_logarithm(arena: &mut Arena, expression: ExprId) -> bool {
    if let Node::Call { head, args } = arena.node(expression) {
        return args.len() == 1 && function_name(arena, *head) == Some("ln");
    }
    false
}

fn integrate_by_parts(
    arena: &mut Arena,
    left: ExprId,
    right: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<Option<ExprId>> {
    let left_poly = is_polynomial_in(arena, left, variable, context);
    let right_poly = is_polynomial_in(arena, right, variable, context);
    let left_elementary = elementary_argument(arena, left, variable, context);
    let right_elementary = elementary_argument(arena, right, variable, context);
    let (u, dv) = if left_poly && right_elementary.is_some() {
        (left, right)
    } else if right_poly && left_elementary.is_some() {
        (right, left)
    } else if is_logarithm(arena, left) && right_poly {
        (left, right)
    } else if is_logarithm(arena, right) && left_poly {
        (right, left)
    } else {
        return Some(None);
    };
    let v = integrate_raw(arena, dv, variable, context)?;
    let du = differentiate(arena, u, variable, context)?;
    let product_value = product(arena, context, &[u, v]);
    if !contains(arena, du, variable) && as_rat(arena, du).is_some_and(|value| value.is_zero()) {
        return Some(Some(product_value));
    }
    let remainder = product(arena, context, &[v, du]);
    let algebra_context = crate::algebra::Context {
        plus: context.plus,
        times: context.times,
        power: context.power,
        negate: context.negate,
        list: context.list,
        equal: context.equal,
    };
    let remainder = crate::algebra::cancel(arena, remainder, &algebra_context).unwrap_or(remainder);
    let remainder = integrate_raw(arena, remainder, variable, context)?;
    let negative = negation(arena, context, remainder);
    Some(Some(sum(arena, context, &[product_value, negative])))
}

fn integrate_inverse_polynomial(
    arena: &mut Arena,
    base: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<Option<ExprId>> {
    let algebra_context = crate::algebra::Context {
        plus: context.plus,
        times: context.times,
        power: context.power,
        negate: context.negate,
        list: context.list,
        equal: context.equal,
    };
    let Some((roots, leading)) =
        crate::algebra::univariate_rational_roots(arena, base, variable, &algebra_context)
    else {
        return Some(None);
    };
    if roots.len() < 2 {
        return Some(None);
    }
    let mut terms = Vec::with_capacity(roots.len());
    for (index, root) in roots.iter().enumerate() {
        let mut product_value = Rat::one();
        for (other_index, other) in roots.iter().enumerate() {
            if index == other_index {
                continue;
            }
            product_value = product_value.mul(&root.sub(other));
        }
        let denominator = leading.mul(&product_value);
        let coefficient = Rat::one().div(&denominator).ok()?;
        let root_expression = rat_expression(arena, root);
        let negative_root = negation(arena, context, root_expression);
        let linear = sum(arena, context, &[variable, negative_root]);
        let magnitude = call(arena, "abs", &[linear]);
        let logarithm = call(arena, "ln", &[magnitude]);
        let coefficient = rat_expression(arena, &coefficient);
        terms.push(product(arena, context, &[coefficient, logarithm]));
    }
    Some(Some(sum(arena, context, &terms)))
}

fn integrate_power(
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
    if contains(arena, exponent, variable) {
        return None;
    }
    let exponent_value = as_rat(arena, exponent)?;
    let minus_one = Rat::from_i64(-1, 1).ok()?;
    if exponent_value == minus_one {
        if let Some(integral) = integrate_inverse_polynomial(arena, base, variable, context)? {
            return Some(integral);
        }
        if is_one_plus_square(arena, base, variable) {
            return Some(call(arena, "atan", &[variable]));
        }
        if let Some(inner) = sqrt_argument(arena, base) {
            if is_one_minus_square(arena, inner, variable) {
                return Some(call(arena, "asin", &[variable]));
            }
            let half = Rat::from_i64(-1, 2).ok()?;
            let half_expression = rat_expression(arena, &half);
            return integrate_power(arena, &[inner, half_expression], variable, context);
        }
    }
    let slope = differentiate(arena, base, variable, context)?;
    if contains(arena, slope, variable) {
        return None;
    }
    let one = Rat::one();
    let raised = exponent_value.add(&one);
    if raised.is_zero() {
        let magnitude = call(arena, "abs", &[base]);
        let logarithm = call(arena, "ln", &[magnitude]);
        return Some(divide(arena, context, logarithm, slope));
    }
    let raised_expression = rat_expression(arena, &raised);
    let raised_power = power(arena, context, base, raised_expression);
    let denominator = product(arena, context, &[raised_expression, slope]);
    Some(divide(arena, context, raised_power, denominator))
}
