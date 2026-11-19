//! Shared evaluator helpers for signs, exact values, and numeric folding.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

pub(super) fn require_arity(
    args: &[ExprId],
    arity: usize,
    message: &str,
) -> Result<(), Diagnostic> {
    if args.len() == arity {
        Ok(())
    } else {
        Err(Diagnostic::eval(ErrorCode::EvalArity, message))
    }
}

pub(super) fn sign_of(value: i8) -> Sign {
    match value.cmp(&0) {
        core::cmp::Ordering::Less => Sign::Negative,
        core::cmp::Ordering::Equal => Sign::Zero,
        core::cmp::Ordering::Greater => Sign::Positive,
    }
}

pub(super) fn flip_sign(sign: Sign) -> Sign {
    match sign {
        Sign::Negative => Sign::Positive,
        Sign::Zero => Sign::Zero,
        Sign::Positive => Sign::Negative,
        Sign::NonNegative => Sign::NonPositive,
        Sign::NonPositive => Sign::NonNegative,
    }
}

pub(super) fn multiply_signs(left: Sign, right: Sign) -> Option<Sign> {
    let left = match left {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
        _ => return None,
    };
    let right = match right {
        Sign::Negative => -1,
        Sign::Zero => 0,
        Sign::Positive => 1,
        _ => return None,
    };
    Some(sign_of((left * right) as i8))
}

pub(super) fn interval_contains_phase(low: f64, high: f64, phase: f64, period: f64) -> bool {
    let first = ((low - phase) / period).ceil();
    let last = ((high - phase) / period).floor();
    first <= last
}

pub(super) fn sign_product(left: Sign, right: Sign) -> Option<Sign> {
    multiply_signs(left, right)
}

pub(super) fn is_zero_value(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.is_zero(),
        Node::Rat(value) => value.is_zero(),
        Node::Real(value) => value.sign() == 0,
        _ => false,
    }
}

pub(super) fn is_one_value(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.to_i64() == Some(1),
        Node::Rat(value) => value.num().to_i64() == Some(1) && value.den().is_one(),
        Node::Real(value) => value.to_f64() == 1.0,
        _ => false,
    }
}

pub(super) fn nonnegative_integer(arena: &Arena, expression: ExprId) -> Option<u32> {
    let value = match arena.node(expression) {
        Node::Int(value) => value.to_i64()?,
        Node::Rat(value) if value.den().is_one() => value.num().to_i64()?,
        _ => return None,
    };
    if value < 0 || value > u32::MAX as i64 {
        return None;
    }
    Some(value as u32)
}

pub(super) fn integer_sqrt(value: &Int) -> Option<Int> {
    let value = value.to_i64()?;
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(Int::from_i64(middle));
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

pub(super) fn exact_integer(arena: &Arena, expression: ExprId) -> Option<Int> {
    match arena.node(expression) {
        Node::Int(value) => Some(value.clone()),
        Node::Rat(value) if value.den().is_one() => Some(value.num()),
        _ => None,
    }
}

pub(super) fn exact_rational(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

pub(super) fn integer_lcm(left: &Int, right: &Int) -> Int {
    if left.is_zero() || right.is_zero() {
        return Int::zero();
    }
    let gcd = left.gcd(right);
    left.mul(right)
        .divrem(&gcd)
        .map(|(quotient, _)| quotient.abs())
        .unwrap_or_else(|_| Int::one())
}

pub(super) fn integer_factorial(value: i64) -> Option<Int> {
    if (0..=20).contains(&value) {
        let mut result = 1u64;
        for factor in 2..=value as u64 {
            result *= factor;
        }
        return Some(Int::from_u64(result));
    }
    let mut result = Int::one();
    let mut factor = 2i64;
    while factor <= value {
        result = result.mul(&Int::from_i64(factor));
        factor += 1;
    }
    Some(result)
}

pub(super) fn integer_binomial(n: i64, k: i64) -> Option<Int> {
    if k > n {
        return Some(Int::zero());
    }
    let k = k.min(n - k);
    if k > 100_000 {
        return None;
    }
    let mut result = Int::one();
    let mut index = 1i64;
    while index <= k {
        result = result.mul(&Int::from_i64(n - k + index));
        let (quotient, remainder) = result.divrem(&Int::from_i64(index)).ok()?;
        if !remainder.is_zero() {
            return None;
        }
        result = quotient;
        index += 1;
    }
    Some(result)
}

pub(super) fn numeric_add(left: &Numeric, right: &Numeric) -> Option<Numeric> {
    match (left, right) {
        (Numeric::Error(value), _) | (_, Numeric::Error(value)) => Some(Numeric::Error(*value)),
        (Numeric::Real(a), Numeric::Real(b)) => a.add(b).ok().map(Numeric::Real),
        (Numeric::Real(a), Numeric::Complex(c, d)) => {
            let real = a.add(c).ok()?;
            Some(Numeric::Complex(real, d.clone()))
        }
        (Numeric::Complex(c, d), Numeric::Real(a)) => {
            let real = c.add(a).ok()?;
            Some(Numeric::Complex(real, d.clone()))
        }
        (Numeric::Complex(a, b), Numeric::Complex(c, d)) => {
            let real = a.add(c).ok()?;
            let imaginary = b.add(d).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
    }
}

pub(super) fn numeric_mul(left: &Numeric, right: &Numeric) -> Option<Numeric> {
    match (left, right) {
        (Numeric::Error(value), _) | (_, Numeric::Error(value)) => Some(Numeric::Error(*value)),
        (Numeric::Real(a), Numeric::Real(b)) => a.mul(b).ok().map(Numeric::Real),
        (Numeric::Real(a), Numeric::Complex(c, d)) => {
            let real = a.mul(c).ok()?;
            let imaginary = a.mul(d).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
        (Numeric::Complex(c, d), Numeric::Real(a)) => {
            let real = c.mul(a).ok()?;
            let imaginary = d.mul(a).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
        (Numeric::Complex(a, b), Numeric::Complex(c, d)) => {
            let real = a.mul(c).ok()?.sub(&b.mul(d).ok()?).ok()?;
            let imaginary = a.mul(d).ok()?.add(&b.mul(c).ok()?).ok()?;
            Some(Numeric::Complex(real, imaginary))
        }
    }
}

pub(super) fn numeric_neg(value: &Numeric) -> Option<Numeric> {
    match value {
        Numeric::Real(value) => Some(Numeric::Real(value.neg().ok()?)),
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Complex(real, imaginary) => {
            Some(Numeric::Complex(real.neg().ok()?, imaginary.neg().ok()?))
        }
    }
}

pub(super) fn numeric_abs(value: &Numeric) -> Option<Numeric> {
    match value {
        Numeric::Real(value) => Some(Numeric::Real(value.abs().ok()?)),
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Complex(real, imaginary) => {
            let complex = ComplexMpc::from_parts(real, imaginary)?;
            Some(Numeric::Real(complex.abs()?))
        }
    }
}

pub(super) fn numeric_complex_pow(base: &(Real, Real), exponent: &(Real, Real)) -> Option<Numeric> {
    let base = ComplexMpc::from_parts(&base.0, &base.1)?;
    let exponent = ComplexMpc::from_parts(&exponent.0, &exponent.1)?;
    let (real, imaginary) = base.pow(&exponent)?;
    Some(Numeric::Complex(real, imaginary))
}

pub(super) fn numeric_complex_div(
    numerator: &(Real, Real),
    denominator: &(Real, Real),
    digits: u32,
) -> Option<Numeric> {
    let (a, b) = numerator;
    let (c, d) = denominator;
    let c_abs = c.abs().ok()?;
    let d_abs = d.abs().ok()?;
    if c.is_zero() && d.is_zero() {
        return None;
    }
    let (real, imaginary) = if c_abs.cmp(&d_abs) != Ordering::Less {
        let ratio = d.div(c).ok()?;
        let real_denominator = c.add(&d.mul(&ratio).ok()?).ok()?;
        if real_denominator.is_zero() {
            return None;
        }
        let real_numerator = a.add(&b.mul(&ratio).ok()?).ok()?;
        let imaginary_numerator = b.sub(&a.mul(&ratio).ok()?).ok()?;
        (
            real_numerator.div(&real_denominator).ok()?,
            imaginary_numerator.div(&real_denominator).ok()?,
        )
    } else {
        let ratio = c.div(d).ok()?;
        let real_denominator = c.mul(&ratio).ok()?.add(d).ok()?;
        if real_denominator.is_zero() {
            return None;
        }
        let real_numerator = a.mul(&ratio).ok()?.add(b).ok()?;
        let imaginary_numerator = b.mul(&ratio).ok()?.sub(a).ok()?;
        (
            real_numerator.div(&real_denominator).ok()?,
            imaginary_numerator.div(&real_denominator).ok()?,
        )
    };
    let _ = digits;
    Some(Numeric::Complex(real, imaginary))
}

pub(super) fn numeric_pow(value: &Numeric, exponent: i64, digits: u32) -> Option<Numeric> {
    if exponent == 0 {
        return Some(Numeric::Real(Real::one(digits).ok()?));
    }
    match value {
        Numeric::Error(value) => Some(Numeric::Error(*value)),
        Numeric::Real(value) => {
            if value.is_zero() && exponent < 0 {
                return None;
            }
            Some(Numeric::Real(value.pow_si(exponent).ok()?))
        }
        Numeric::Complex(real, imaginary) => {
            let mut power = exponent.unsigned_abs();
            let inverted = exponent < 0;
            let mut base = Numeric::Complex(real.clone(), imaginary.clone());
            if inverted {
                let one = (Real::one(digits).ok()?, Real::zero(digits).ok()?);
                base = numeric_complex_div(&one, &(real.clone(), imaginary.clone()), digits)?;
            }
            let mut result = Numeric::Complex(Real::one(digits).ok()?, Real::zero(digits).ok()?);
            while power > 0 {
                if power & 1 == 1 {
                    result = numeric_mul(&result, &base)?;
                }
                power >>= 1;
                if power > 0 {
                    base = numeric_mul(&base, &base)?;
                }
            }
            Some(result)
        }
    }
}

pub(super) fn gaussian_pow(real: &Rat, imaginary: &Rat, power: i64) -> Option<(Rat, Rat)> {
    if power == 0 {
        return Some((Rat::one(), Rat::zero()));
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut base = (real.clone(), imaginary.clone());
    let mut result = (Rat::one(), Rat::zero());
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = gaussian_mul(&result, &base);
        }
        exponent >>= 1;
        if exponent > 0 {
            base = gaussian_mul(&base, &base);
        }
    }
    if negative {
        let norm = result.0.mul(&result.0).add(&result.1.mul(&result.1));
        if norm.is_zero() {
            return None;
        }
        let inverse = Rat::one().div(&norm).ok()?;
        Some((result.0.mul(&inverse), result.1.neg().mul(&inverse)))
    } else {
        Some(result)
    }
}

pub(super) fn gaussian_mul(left: &(Rat, Rat), right: &(Rat, Rat)) -> (Rat, Rat) {
    let real = left.0.mul(&right.0).sub(&left.1.mul(&right.1));
    let imaginary = left.0.mul(&right.1).add(&left.1.mul(&right.0));
    (real, imaginary)
}

pub(super) fn rat_pow(base: &Rat, power: i64) -> Result<Rat, ()> {
    if power == 0 {
        return Ok(Rat::one());
    }
    let negative = power < 0;
    let mut exponent = power.unsigned_abs();
    let mut result = Rat::one();
    let mut factor = base.clone();
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result.mul(&factor);
        }
        exponent >>= 1;
        if exponent > 0 {
            factor = factor.mul(&factor);
        }
    }
    if negative {
        if result.is_zero() {
            return Err(());
        }
        Rat::one().div(&result).map_err(|_| ())
    } else {
        Ok(result)
    }
}

impl<'a> Evaluator<'a> {
    pub(super) fn is_inference_variable(&self, expression: ExprId) -> bool {
        let Some(symbol) = self.arena.symbol_id(expression) else {
            return false;
        };
        !self
            .arena
            .symbol_attributes(symbol)
            .has(Attributes::CONSTANT)
    }

    pub(super) fn infer_variable(&self, expression: ExprId) -> Result<ExprId, Diagnostic> {
        fn collect(arena: &Arena, expression: ExprId, variables: &mut Vec<ExprId>) {
            match arena.node(expression) {
                Node::Symbol(symbol) => {
                    if !arena.symbol_attributes(*symbol).has(Attributes::CONSTANT)
                        && !variables.contains(&expression)
                    {
                        variables.push(expression);
                    }
                }
                Node::Call { args, .. } => {
                    for argument in args.iter() {
                        collect(arena, *argument, variables);
                    }
                }
                _ => {}
            }
        }

        let mut variables = Vec::new();
        collect(self.arena, expression, &mut variables);
        match variables.as_slice() {
            [variable] => Ok(*variable),
            [] => Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "cannot infer a variable because the expression has no free symbols",
            )),
            many => {
                let names = many
                    .iter()
                    .filter_map(|variable| {
                        self.arena
                            .symbol_id(*variable)
                            .map(|symbol| self.arena.symbol_name(symbol).to_string())
                    })
                    .collect::<Vec<_>>();
                Err(Diagnostic::eval(
                    ErrorCode::EvalArity,
                    format!(
                        "cannot infer a variable because several candidates exist: {}",
                        names.join(", ")
                    ),
                ))
            }
        }
    }
}
