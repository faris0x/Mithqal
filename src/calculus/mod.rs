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
    if let Node::Call { head, args } = arena.node(value) {
        if *head == context.negate && args.len() == 1 {
            return args[0];
        }
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

fn function_name<'a>(arena: &'a Arena, head: ExprId) -> Option<&'a str> {
    arena
        .symbol_id(head)
        .map(|symbol| arena.symbol_name(symbol))
}

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
                if dependent.len() == 2 {
                    if let Some(integral) =
                        integrate_by_parts(arena, dependent[0], dependent[1], variable, context)?
                    {
                        constants.push(integral);
                        return Some(product(arena, context, &constants));
                    }
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
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn definite_integrate(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    lower: ExprId,
    upper: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let antiderivative = integrate(arena, expression, variable, context)?;
    let upper_value = if let Some(positive) = infinite_bound(arena, upper) {
        limit_at_infinity(arena, antiderivative, variable, positive, context)?
    } else {
        substitute(arena, antiderivative, variable, upper)
    };
    let lower_value = if let Some(positive) = infinite_bound(arena, lower) {
        limit_at_infinity(arena, antiderivative, variable, positive, context)?
    } else {
        substitute(arena, antiderivative, variable, lower)
    };
    let negative_lower = negation(arena, context, lower_value);
    Some(sum(arena, context, &[upper_value, negative_lower]))
}

fn infinite_bound(arena: &Arena, expression: ExprId) -> Option<bool> {
    if let Node::Symbol(symbol) = arena.node(expression) {
        return if arena.symbol_name(*symbol) == "inf" {
            Some(true)
        } else {
            None
        };
    }
    if let Node::Call { head, args } = arena.node(expression) {
        if args.len() == 1
            && arena
                .symbol_id(*head)
                .map(|symbol| arena.symbol_name(symbol))
                == Some("negate")
        {
            if let Node::Symbol(symbol) = arena.node(args[0]) {
                if arena.symbol_name(*symbol) == "inf" {
                    return Some(false);
                }
            }
        }
    }
    None
}

fn limit_at_infinity(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    positive: bool,
    context: &Context,
) -> Option<ExprId> {
    if !contains(arena, expression, variable) {
        return Some(expression);
    }
    match arena.node(expression) {
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            let name = function_name(arena, head);
            if name == Some("negate") && args.len() == 1 {
                let limit = limit_at_infinity(arena, args[0], variable, positive, context)?;
                return Some(negation(arena, context, limit));
            }
            if name == Some("plus") {
                let mut terms = Vec::with_capacity(args.len());
                for arg in args {
                    terms.push(limit_at_infinity(arena, arg, variable, positive, context)?);
                }
                return Some(sum(arena, context, &terms));
            }
            if name == Some("times") {
                let mut constant = Vec::new();
                let mut dependent = Vec::new();
                for arg in args {
                    if contains(arena, arg, variable) {
                        dependent.push(arg);
                    } else {
                        constant.push(arg);
                    }
                }
                if dependent.len() > 1 {
                    let mut decaying = false;
                    for factor in &dependent {
                        if let Node::Call { head, args } = arena.node(*factor) {
                            if args.len() == 1 && function_name(arena, *head) == Some("exp") {
                                let slope = differentiate(arena, args[0], variable, context)?;
                                if !contains(arena, slope, variable) {
                                    if let Some(slope) = as_rat(arena, slope) {
                                        let decays = if positive {
                                            slope.sign() < 0
                                        } else {
                                            slope.sign() > 0
                                        };
                                        if decays {
                                            decaying = true;
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        if !is_polynomial_in(arena, *factor, variable, context) {
                            return None;
                        }
                    }
                    if decaying {
                        return Some(int_expression(arena, 0));
                    }
                    return None;
                }
                let limit = limit_at_infinity(arena, dependent[0], variable, positive, context)?;
                constant.push(limit);
                return Some(product(arena, context, &constant));
            }
            if name == Some("power") && args.len() == 2 {
                if args[0] == variable {
                    let exponent = as_rat(arena, args[1])?;
                    if exponent.sign() < 0 {
                        return Some(int_expression(arena, 0));
                    }
                }
                return None;
            }
            if name == Some("exp") && args.len() == 1 {
                let slope = differentiate(arena, args[0], variable, context)?;
                if contains(arena, slope, variable) {
                    return None;
                }
                let slope = as_rat(arena, slope)?;
                let decays = if positive {
                    slope.sign() < 0
                } else {
                    slope.sign() > 0
                };
                if decays {
                    return Some(int_expression(arena, 0));
                }
                return None;
            }
            if name == Some("atan") && args.len() == 1 {
                if args[0] == variable {
                    let pi = arena.builtin_head("pi");
                    let two = int_expression(arena, 2);
                    let half = divide(arena, context, pi, two);
                    return if positive {
                        Some(half)
                    } else {
                        Some(negation(arena, context, half))
                    };
                }
            }
            None
        }
        _ => None,
    }
}

fn substitute(arena: &mut Arena, expression: ExprId, variable: ExprId, value: ExprId) -> ExprId {
    if expression == variable {
        return value;
    }
    match arena.node(expression) {
        Node::Call { head, args } => {
            let head = *head;
            let args = args.to_vec();
            let head = substitute(arena, head, variable, value);
            let mut replaced = Vec::with_capacity(args.len());
            for arg in args {
                replaced.push(substitute(arena, arg, variable, value));
            }
            arena.intern_call(head, &replaced)
        }
        _ => expression,
    }
}

fn is_power_of_variable(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    if let Node::Call { head, args } = arena.node(expression) {
        if args.len() == 2 && function_name(arena, *head) == Some("power") {
            return args[0] == variable
                && as_rat(arena, args[1]).is_some_and(|value| {
                    value == Rat::from_i64(2, 1).unwrap_or_else(|_| Rat::zero())
                });
        }
    }
    false
}

fn is_one_plus_square(arena: &Arena, expression: ExprId, variable: ExprId) -> bool {
    if let Node::Call { head, args } = arena.node(expression) {
        if args.len() == 2 && function_name(arena, *head) == Some("plus") {
            let one = Rat::from_i64(1, 1).unwrap_or_else(|_| Rat::zero());
            let has_one = args
                .iter()
                .any(|arg| as_rat(arena, *arg).is_some_and(|value| value == one));
            let has_square = args
                .iter()
                .any(|arg| is_power_of_variable(arena, *arg, variable));
            return has_one && has_square;
        }
    }
    false
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
    if let Node::Call { head, args } = arena.node(expression) {
        if args.len() == 2 && function_name(arena, *head) == Some("plus") {
            let one = Rat::from_i64(1, 1).unwrap_or_else(|_| Rat::zero());
            let has_one = args
                .iter()
                .any(|arg| as_rat(arena, *arg).is_some_and(|value| value == one));
            let has_negative_square = args
                .iter()
                .any(|arg| is_negative_square(arena, *arg, variable));
            return has_one && has_negative_square;
        }
    }
    false
}

fn sqrt_argument(arena: &Arena, expression: ExprId) -> Option<ExprId> {
    if let Node::Call { head, args } = arena.node(expression) {
        if args.len() == 1 && function_name(arena, *head) == Some("sqrt") {
            return Some(args[0]);
        }
    }
    None
}

fn is_polynomial_in(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> bool {
    let mut current = expression;
    for _ in 0..16 {
        if !contains(arena, current, variable) {
            return true;
        }
        let Some(next) = differentiate(arena, current, variable, context) else {
            return false;
        };
        if next == current {
            return false;
        }
        current = next;
    }
    false
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
    if !contains(arena, du, variable) {
        if let Some(zero) = as_rat(arena, du) {
            if zero.is_zero() {
                return Some(Some(product_value));
            }
        }
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
