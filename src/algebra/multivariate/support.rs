//! Shared helper functions for the multivariate factorisation stages.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

pub(super) fn has_rational_root_slice(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    degree: usize,
) -> bool {
    for candidate in [0i64, 1, -1, 2, -2, 3, 10] {
        let Ok(value) = Rat::from_i64(candidate, 1) else {
            continue;
        };
        let (sample_degree, coefficients) =
            substitute_vertical(polynomial, horizontal, vertical, &value);
        if sample_degree != degree {
            continue;
        }
        let (roots, remaining) = rational_roots(&coefficients);
        return remaining.len() <= 1 && !roots.is_empty();
    }
    false
}

pub(super) fn split_linear_polynomial(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Poly, Poly)> {
    let mut coefficient = Poly::zero();
    let mut constant = Poly::zero();
    for (monomial, value) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        let target = match horizontal_power {
            1 => &mut coefficient,
            0 => &mut constant,
            _ => return None,
        };
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        let monomial = Monomial { powers };
        match target.terms.get_mut(&monomial) {
            Some(existing) => *existing = existing.add(value),
            None => {
                target.terms.insert(monomial, value.clone());
            }
        }
    }
    Some((coefficient, constant))
}

pub(super) fn divide_linear_by_dense(
    factor: &Poly,
    divisor: &[Rat],
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<Poly> {
    let (coefficient, constant) = split_linear_polynomial(factor, horizontal, vertical)?;
    let coefficient = coefficient.dense_in(vertical)?;
    let constant = constant.dense_in(vertical)?;
    let (coefficient, coefficient_remainder) = dense_div_rem(&coefficient, divisor)?;
    if !coefficient_remainder.is_empty() {
        return None;
    }
    let (constant, constant_remainder) = dense_div_rem(&constant, divisor)?;
    if !constant_remainder.is_empty() {
        return None;
    }
    Some(
        Poly::variable(horizontal)
            .mul(&Poly::from_dense(vertical, &coefficient))
            .add(&Poly::from_dense(vertical, &constant)),
    )
}

pub(super) fn quadratic_coefficients(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Poly, Poly, Poly)> {
    let mut leading = Poly::zero();
    let mut linear = Poly::zero();
    let mut constant = Poly::zero();
    let mut degree = 0u32;
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        degree = degree.max(horizontal_power);
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        let monomial = Monomial { powers };
        let target = match horizontal_power {
            2 => &mut leading,
            1 => &mut linear,
            0 => &mut constant,
            _ => return None,
        };
        match target.terms.get_mut(&monomial) {
            Some(existing) => *existing = existing.add(coefficient),
            None => {
                target.terms.insert(monomial, coefficient.clone());
            }
        }
    }
    if degree != 2 {
        return None;
    }
    Some((leading, linear, constant))
}

pub(super) fn y_degree(polynomial: &Poly, vertical: SymbolId) -> u32 {
    polynomial
        .terms
        .keys()
        .map(|monomial| monomial.degree_in(vertical))
        .max()
        .unwrap_or(0)
}

pub(super) fn dense_scale(coefficients: &[Rat], scale: &Rat) -> Vec<Rat> {
    trim(coefficients.iter().map(|value| value.mul(scale)).collect())
}

pub(super) fn dense_subtract(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let length = left.len().max(right.len());
    let mut difference = Vec::with_capacity(length);
    for index in 0..length {
        difference.push(
            left.get(index)
                .cloned()
                .unwrap_or_else(Rat::zero)
                .sub(&right.get(index).cloned().unwrap_or_else(Rat::zero)),
        );
    }
    trim(difference)
}

pub(super) fn dense_extended_gcd(
    left: &[Rat],
    right: &[Rat],
) -> Option<(Vec<Rat>, Vec<Rat>, Vec<Rat>)> {
    let mut old_remainder = trim(left.to_vec());
    let mut remainder = trim(right.to_vec());
    let mut old_left = vec![Rat::one()];
    let mut left_coefficient = Vec::new();
    let mut old_right = Vec::new();
    let mut right_coefficient = vec![Rat::one()];
    while !remainder.is_empty() {
        let (quotient, next_remainder) = dense_div_rem(&old_remainder, &remainder)?;
        old_remainder = remainder;
        remainder = next_remainder;
        let next_left = dense_subtract(&old_left, &dense_multiply(&quotient, &left_coefficient));
        old_left = left_coefficient;
        left_coefficient = next_left;
        let next_right = dense_subtract(&old_right, &dense_multiply(&quotient, &right_coefficient));
        old_right = right_coefficient;
        right_coefficient = next_right;
    }
    let constant = old_remainder.first()?.clone();
    if constant.is_zero() {
        return None;
    }
    let inverse = Rat::one().div(&constant).ok()?;
    Some((
        old_remainder,
        dense_scale(&old_left, &inverse),
        dense_scale(&old_right, &inverse),
    ))
}

pub(super) fn binomial(n: u32, k: u32) -> Rat {
    let mut result = 1u128;
    for index in 1..=k {
        result = result * (n - k + index) as u128 / index as u128;
    }
    Rat::from_i64(result as i64, 1).unwrap_or_else(|_| Rat::one())
}

pub(super) fn taylor_slices(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
    maximum: u32,
) -> Vec<Vec<Rat>> {
    let mut slices = vec![Vec::new(); maximum as usize + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        for power in 0..=vertical_power {
            let factor =
                binomial(vertical_power, power).mul(&rat_power(value, vertical_power - power));
            let contribution = coefficient.mul(&factor);
            let slice = &mut slices[power as usize];
            if slice.len() <= horizontal_power {
                slice.resize(horizontal_power + 1, Rat::zero());
            }
            slice[horizontal_power] = slice[horizontal_power].add(&contribution);
        }
    }
    for slice in &mut slices {
        *slice = trim(core::mem::take(slice));
    }
    slices
}

pub(super) fn polynomial_from_t_slices(
    slices: &[Vec<Rat>],
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
) -> Poly {
    let t = Poly::variable(vertical).sub(&Poly::constant(value.clone()));
    let mut result = Poly::zero();
    let mut power = Poly::constant(Rat::one());
    for slice in slices {
        let coefficient = Poly::from_dense(horizontal, slice);
        result = result.add(&coefficient.mul(&power));
        power = power.mul(&t);
    }
    result
}

pub(super) fn lift_bivariate_pair(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
    left: &[Rat],
    right: &[Rat],
    maximum: u32,
) -> Option<(Poly, Poly)> {
    let (gcd, _, bezout_right) = dense_extended_gcd(left, right)?;
    if gcd.len() != 1 {
        return None;
    }
    let slices = taylor_slices(polynomial, horizontal, vertical, value, maximum);
    let mut left_slices = vec![trim(left.to_vec())];
    let mut right_slices = vec![trim(right.to_vec())];
    for power in 1..=maximum as usize {
        let mut residual = slices.get(power).cloned().unwrap_or_default();
        for index in 1..power {
            residual = dense_subtract(
                &residual,
                &dense_multiply(&left_slices[index], &right_slices[power - index]),
            );
        }
        let projected = dense_multiply(&bezout_right, &residual);
        let (_, correction_left) = dense_div_rem(&projected, left)?;
        let correction_left = trim(correction_left);
        let numerator = dense_subtract(&residual, &dense_multiply(&correction_left, right));
        let (correction_right, remainder) = dense_div_rem(&numerator, left)?;
        if !remainder.is_empty() {
            return None;
        }
        left_slices.push(correction_left);
        right_slices.push(correction_right);
    }
    Some((
        polynomial_from_t_slices(&left_slices, horizontal, vertical, value),
        polynomial_from_t_slices(&right_slices, horizontal, vertical, value),
    ))
}

pub(super) fn coefficient_polynomials(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<Vec<Poly>> {
    let degree = x_degree(polynomial, horizontal);
    let mut coefficients = vec![Poly::zero(); degree + 1];
    for (monomial, value) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        coefficients[horizontal_power]
            .terms
            .insert(Monomial { powers }, value.clone());
    }
    Some(coefficients)
}

pub(super) fn evaluate_at_horizontal(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Poly,
) -> Poly {
    let Some(coefficients) = coefficient_polynomials(polynomial, horizontal, vertical) else {
        return Poly::zero();
    };
    let mut result = Poly::zero();
    for coefficient in coefficients.iter().rev() {
        result = result.mul(value).add(coefficient);
    }
    result
}

pub(super) fn divide_by_linear_root(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    root: &Poly,
) -> Option<Poly> {
    let coefficients = coefficient_polynomials(polynomial, horizontal, vertical)?;
    let degree = coefficients.len().checked_sub(1)?;
    let mut quotient = vec![Poly::zero(); degree];
    if degree == 0 {
        return None;
    }
    quotient[degree - 1] = coefficients[degree].clone();
    for power in (1..degree).rev() {
        let next = coefficients[power].add(&root.mul(&quotient[power]));
        quotient[power - 1] = next;
    }
    let remainder = coefficients[0].add(&root.mul(&quotient[0]));
    if remainder != Poly::zero() {
        return None;
    }
    let mut result = Poly::zero();
    for (power, coefficient) in quotient.iter().enumerate() {
        for (monomial, value) in &coefficient.terms {
            let mut powers = Vec::new();
            if power > 0 {
                powers.push((horizontal, power as u32));
            }
            for (symbol, exponent) in &monomial.powers {
                powers.push((*symbol, *exponent));
            }
            powers.sort_by_key(|(symbol, _)| *symbol);
            result.terms.insert(Monomial { powers }, value.clone());
        }
    }
    Some(result)
}

pub(super) fn ordered_variables(arena: &Arena, polynomial: &Poly) -> Vec<SymbolId> {
    let mut variables: Vec<SymbolId> = polynomial.variables().into_iter().collect();
    variables.sort_by(|left, right| {
        arena
            .symbol_name(*left)
            .cmp(arena.symbol_name(*right))
            .then(left.cmp(right))
    });
    variables
}

pub(super) fn bivariate_variables(
    arena: &Arena,
    polynomial: &Poly,
) -> Option<(SymbolId, SymbolId)> {
    let variables = ordered_variables(arena, polynomial);
    if variables.len() != 2 {
        return None;
    }
    Some((variables[0], variables[1]))
}

pub(super) fn x_degree(polynomial: &Poly, horizontal: SymbolId) -> usize {
    polynomial
        .terms
        .keys()
        .map(|monomial| monomial.degree_in(horizontal) as usize)
        .max()
        .unwrap_or(0)
}

pub(super) fn poly_denominator_lcm(polynomial: &Poly) -> Int {
    let mut denominator = Int::one();
    for coefficient in polynomial.terms.values() {
        denominator = lcm(&denominator, &coefficient.den());
    }
    denominator
}

pub(super) fn constant_leading_coefficient(polynomial: &Poly, horizontal: SymbolId) -> Option<Rat> {
    let degree = x_degree(polynomial, horizontal) as u32;
    let mut leading = None;
    for (monomial, coefficient) in &polynomial.terms {
        if monomial.degree_in(horizontal) != degree {
            continue;
        }
        if monomial
            .powers
            .iter()
            .any(|(symbol, exponent)| *symbol != horizontal && *exponent != 0)
        {
            return None;
        }
        if leading.is_some() {
            return None;
        }
        leading = Some(coefficient.clone());
    }
    let leading = leading?;
    if leading.is_zero() {
        None
    } else {
        Some(leading)
    }
}

pub(super) fn is_monic_in(polynomial: &Poly, horizontal: SymbolId) -> bool {
    let degree = x_degree(polynomial, horizontal) as u32;
    let mut found = false;
    for (monomial, coefficient) in &polynomial.terms {
        if monomial.degree_in(horizontal) != degree {
            continue;
        }
        if monomial
            .powers
            .iter()
            .any(|(symbol, exponent)| *symbol != horizontal && *exponent != 0)
            || !coefficient.is_one()
            || found
        {
            return false;
        }
        found = true;
    }
    found
}

pub(super) fn substitute_vertical(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
) -> (usize, Vec<Rat>) {
    let mut degree = 0usize;
    let mut vertical_degree = 0u32;
    for monomial in polynomial.terms.keys() {
        for (symbol, exponent) in &monomial.powers {
            if *symbol == horizontal {
                degree = degree.max(*exponent as usize);
            } else if *symbol == vertical {
                vertical_degree = vertical_degree.max(*exponent);
            }
        }
    }
    let mut powers = Vec::with_capacity(vertical_degree as usize + 1);
    let mut current = Rat::one();
    for _ in 0..=vertical_degree {
        powers.push(current.clone());
        current = current.mul(value);
    }
    let mut coefficients = vec![Rat::zero(); degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        coefficients[horizontal_power] =
            coefficients[horizontal_power].add(&coefficient.mul(&powers[vertical_power as usize]));
    }
    (degree, trim(coefficients))
}

pub(super) fn rat_power(value: &Rat, exponent: u32) -> Rat {
    let mut result = Rat::one();
    for _ in 0..exponent {
        result = result.mul(value);
    }
    result
}

pub(super) fn quadratic_roots_match(
    polynomial: &Poly,
    horizontal: SymbolId,
    roots: &[Poly],
) -> bool {
    let [first, second] = roots else {
        return false;
    };
    let degree = x_degree(polynomial, horizontal);
    if degree != 2 {
        return false;
    }
    let mut coefficients = vec![Poly::zero(); degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let power = monomial.degree_in(horizontal) as usize;
        let mut powers = Vec::new();
        for (symbol, exponent) in &monomial.powers {
            if *symbol != horizontal {
                powers.push((*symbol, *exponent));
            }
        }
        coefficients[power]
            .terms
            .insert(Monomial { powers }, coefficient.clone());
    }
    let negative_sum = first.add(second).neg();
    let product = first.mul(second);
    coefficients[2] == Poly::constant(Rat::one())
        && coefficients[1] == negative_sum
        && coefficients[0] == product
}

pub(super) fn merge_poly_factor(factors: &mut Vec<(Poly, u32)>, polynomial: Poly) {
    for (existing, multiplicity) in factors.iter_mut() {
        if *existing == polynomial {
            *multiplicity += 1;
            return;
        }
    }
    factors.push((polynomial, 1));
}

pub(super) fn homogeneous_bivariate_data(
    polynomial: &Poly,
) -> Option<(SymbolId, SymbolId, u32, Vec<Rat>)> {
    let variables = polynomial.variables();
    if variables.len() != 2 {
        return None;
    }
    let mut symbols = variables.into_iter();
    let horizontal = symbols.next()?;
    let vertical = symbols.next()?;
    let mut degree = None;
    for monomial in polynomial.terms.keys() {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        let total = horizontal_power + vertical_power;
        match degree {
            None => degree = Some(total),
            Some(existing) if existing == total => {}
            Some(_) => return None,
        }
    }
    let degree = degree?;
    if degree == 0 {
        return None;
    }
    let mut coefficients = vec![Rat::zero(); degree as usize + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let power = monomial.degree_in(horizontal) as usize;
        coefficients[power] = coefficient.clone();
    }
    Some((horizontal, vertical, degree, trim(coefficients)))
}

pub(super) fn two_linear_factors(factors: Vec<(Poly, u32)>) -> Option<(Poly, Poly)> {
    match factors.as_slice() {
        [(first, 1), (second, 1)] => Some((first.clone(), second.clone())),
        [(factor, 2)] => Some((factor.clone(), factor.clone())),
        _ => None,
    }
}

pub(super) fn linear_coefficients(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Rat, Rat)> {
    let mut horizontal_coefficient = Rat::zero();
    let mut vertical_coefficient = Rat::zero();
    for (monomial, coefficient) in &polynomial.terms {
        match (monomial.degree_in(horizontal), monomial.degree_in(vertical)) {
            (1, 0) => horizontal_coefficient = coefficient.clone(),
            (0, 1) => vertical_coefficient = coefficient.clone(),
            (0, 0) => {}
            _ => return None,
        }
    }
    Some((horizontal_coefficient, vertical_coefficient))
}

pub(super) fn push_factor(
    arena: &mut Arena,
    context: &Context,
    factors: &mut Vec<ExprId>,
    polynomial: Poly,
    multiplicity: u32,
) {
    let expression = expression_from_poly(arena, &polynomial, context);
    if multiplicity <= 1 {
        factors.push(expression);
        return;
    }
    let exponent = arena.intern_int(Int::from_u64(multiplicity as u64));
    factors.push(arena.intern_call(context.power, &[expression, exponent]));
}

pub(super) fn interpolation_slices_split(
    polynomial: &Poly,
    horizontal: SymbolId,
    others: &[SymbolId],
    degree: usize,
) -> bool {
    for sample in [0i64, 1] {
        let value = match Rat::from_i64(sample, 1) {
            Ok(value) => value,
            Err(_) => return false,
        };
        let point = vec![value; others.len()];
        let Some((sample_degree, roots, remaining)) =
            sample_rational_roots(polynomial, horizontal, others, &point)
        else {
            return false;
        };
        let root_count = roots
            .iter()
            .map(|(_, multiplicity)| *multiplicity as usize)
            .sum::<usize>();
        if sample_degree != degree || remaining > 1 || root_count != degree {
            return false;
        }
    }
    true
}

pub(super) fn total_degree(polynomial: &Poly) -> u32 {
    polynomial
        .terms
        .keys()
        .map(|monomial| {
            monomial
                .powers
                .iter()
                .map(|(_, exponent)| exponent)
                .sum::<u32>()
        })
        .max()
        .unwrap_or(0)
}
