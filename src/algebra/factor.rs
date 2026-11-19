//! Polynomial factorisation.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

fn merge_dense_factor(factors: &mut Vec<(Vec<Rat>, u32)>, polynomial: Vec<Rat>, multiplicity: u32) {
    if polynomial.is_empty() {
        return;
    }
    for (existing, current) in factors.iter_mut() {
        if *existing == polynomial {
            *current += multiplicity;
            return;
        }
    }
    factors.push((polynomial, multiplicity));
}

fn factor_square_free_by_roots(coefficients: Vec<Rat>) -> Vec<(Vec<Rat>, u32)> {
    let (roots, remaining) = rational_roots(&coefficients);
    let mut factors = Vec::new();
    for (root, multiplicity) in roots {
        let linear = vec![
            Rat::from_ints(root.num().neg(), Int::one()).unwrap_or_else(|_| Rat::zero()),
            Rat::from_ints(root.den(), Int::one()).unwrap_or_else(|_| Rat::one()),
        ];
        merge_dense_factor(&mut factors, primitive_normalize(&linear), multiplicity);
    }
    if remaining.len() > 1 {
        merge_dense_factor(&mut factors, primitive_normalize(&remaining), 1);
    }
    factors
}

fn factor_dense_square_free(coefficients: &[Rat]) -> Option<Vec<(Vec<Rat>, u32)>> {
    let coefficients = trim(coefficients.to_vec());
    if coefficients.len() <= 1 {
        return Some(Vec::new());
    }
    if coefficients.len() == 2 {
        return Some(vec![(primitive_normalize(&coefficients), 1)]);
    }
    let derivative = dense_derivative(&coefficients);
    if derivative.is_empty() {
        return Some(vec![(primitive_normalize(&coefficients), 1)]);
    }
    let gcd = dense_gcd(&coefficients, &derivative);
    if gcd.len() <= 1 {
        return Some(factor_square_free_by_roots(coefficients));
    }
    let (quotient, remainder) = dense_div_rem(&coefficients, &gcd)?;
    if !remainder.is_empty() {
        return None;
    }
    let mut factors = factor_square_free_by_roots(quotient);
    let inner = factor_dense_square_free(&gcd)?;
    for (polynomial, multiplicity) in inner {
        merge_dense_factor(&mut factors, polynomial, multiplicity);
    }
    Some(factors)
}

fn reconstruct_scalar(coefficients: &[Rat], factors: &[(Vec<Rat>, u32)]) -> Option<Rat> {
    let mut product = vec![Rat::one()];
    for (polynomial, multiplicity) in factors {
        let power = dense_power(polynomial, *multiplicity);
        product = dense_multiply(&product, &power);
    }
    let (quotient, remainder) = dense_div_rem(coefficients, &product)?;
    if !remainder.is_empty() || quotient.len() > 1 {
        return None;
    }
    Some(quotient.first().cloned().unwrap_or_else(Rat::one))
}

fn square_root_term(poly: &Poly) -> Option<Poly> {
    if poly.terms.len() != 1 {
        return None;
    }
    let (monomial, coefficient) = poly.terms.iter().next()?;
    if coefficient.sign() < 0 {
        return None;
    }
    let coefficient = rational_sqrt(coefficient)?;
    let mut powers = Vec::with_capacity(monomial.powers.len());
    for (symbol, exponent) in &monomial.powers {
        if exponent % 2 != 0 {
            return None;
        }
        powers.push((*symbol, exponent / 2));
    }
    let mut terms = BTreeMap::new();
    terms.insert(Monomial { powers }, coefficient);
    Some(Poly { terms })
}

fn factor_difference_of_squares(poly: &Poly) -> Option<(Poly, Poly)> {
    if poly.terms.len() != 2 {
        return None;
    }
    let mut positive = Poly::zero();
    let mut negative = Poly::zero();
    for (monomial, coefficient) in &poly.terms {
        let term = Poly {
            terms: BTreeMap::from([(monomial.clone(), coefficient.clone())]),
        };
        if coefficient.sign() > 0 {
            positive = positive.add(&term);
        } else {
            negative = negative.add(&term.neg());
        }
    }
    if positive.is_zero() || negative.is_zero() {
        return None;
    }
    let left = square_root_term(&positive)?;
    let right = square_root_term(&negative)?;
    Some((left.sub(&right), left.add(&right)))
}

fn cube_root_term(poly: &Poly) -> Option<Poly> {
    if poly.terms.len() != 1 {
        return None;
    }
    let (monomial, coefficient) = poly.terms.iter().next()?;
    if coefficient.sign() < 0 {
        return None;
    }
    let coefficient = rational_cbrt(coefficient)?;
    let mut powers = Vec::with_capacity(monomial.powers.len());
    for (symbol, exponent) in &monomial.powers {
        if exponent % 3 != 0 {
            return None;
        }
        powers.push((*symbol, exponent / 3));
    }
    let mut terms = BTreeMap::new();
    terms.insert(Monomial { powers }, coefficient);
    Some(Poly { terms })
}

fn factor_sum_or_difference_of_cubes(poly: &Poly) -> Option<(Poly, Poly)> {
    if poly.terms.len() != 2 {
        return None;
    }
    let mut positive = Poly::zero();
    let mut negative = Poly::zero();
    for (monomial, coefficient) in &poly.terms {
        let term = Poly {
            terms: BTreeMap::from([(monomial.clone(), coefficient.clone())]),
        };
        if coefficient.sign() > 0 {
            positive = positive.add(&term);
        } else {
            negative = negative.add(&term.neg());
        }
    }
    if positive.is_zero() {
        return None;
    }
    let left = cube_root_term(&positive)?;
    if negative.is_zero() {
        return None;
    }
    let right = cube_root_term(&negative)?;
    let left_squared = left.mul(&left);
    let right_squared = right.mul(&right);
    let cross = left.mul(&right);
    let first = left.sub(&right);
    let second = left_squared.add(&cross).add(&right_squared);
    Some((first, second))
}

fn factor_sum_of_cubes(poly: &Poly) -> Option<(Poly, Poly)> {
    if poly.terms.len() != 2 {
        return None;
    }
    let mut terms = poly.terms.iter();
    let (left_monomial, left_coefficient) = terms.next()?;
    let (right_monomial, right_coefficient) = terms.next()?;
    if left_coefficient.sign() <= 0 || right_coefficient.sign() <= 0 {
        return None;
    }
    let left = cube_root_term(&Poly {
        terms: BTreeMap::from([(left_monomial.clone(), left_coefficient.clone())]),
    })?;
    let right = cube_root_term(&Poly {
        terms: BTreeMap::from([(right_monomial.clone(), right_coefficient.clone())]),
    })?;
    let left_squared = left.mul(&left);
    let right_squared = right.mul(&right);
    let cross = left.mul(&right);
    let first = left.add(&right);
    let second = left_squared.sub(&cross).add(&right_squared);
    Some((first, second))
}

fn term_poly(monomial: &Monomial, coefficient: &Rat) -> Poly {
    Poly {
        terms: BTreeMap::from([(monomial.clone(), coefficient.clone())]),
    }
}

pub(crate) fn extract_common_factor(poly: &Poly) -> Option<(Poly, Poly)> {
    let content = poly.content()?;
    let scaled = poly.scale(&Rat::one().div(&content).ok()?);
    let monomial = scaled.monomial_content()?;
    let residual = scaled.divide_monomial(&monomial)?;
    let factor = if monomial.is_one() {
        Poly::constant(content)
    } else {
        Poly {
            terms: BTreeMap::from([(monomial, content)]),
        }
    };
    Some((factor, residual))
}

pub(crate) fn factor_by_grouping(poly: &Poly) -> Option<(Poly, Poly)> {
    let size = poly.terms.len();
    if !(4..=8).contains(&size) || !size.is_multiple_of(2) {
        return None;
    }
    let terms: Vec<(&Monomial, &Rat)> = poly.terms.iter().collect();
    let half = size / 2;
    let limit = 1usize << size;
    for mask in 0..limit {
        if mask & 1 == 0 || mask.count_ones() as usize != half {
            continue;
        }
        let mut left = Poly::zero();
        let mut right = Poly::zero();
        for (index, (monomial, coefficient)) in terms.iter().enumerate() {
            let term = term_poly(monomial, coefficient);
            if mask & (1 << index) != 0 {
                left = left.add(&term);
            } else {
                right = right.add(&term);
            }
        }
        let (Some((left_factor, left_residual)), Some((right_factor, right_residual))) =
            (extract_common_factor(&left), extract_common_factor(&right))
        else {
            continue;
        };
        if left_residual == right_residual && !left_residual.is_constant() {
            return Some((left_factor.add(&right_factor), left_residual));
        }
    }
    None
}

fn factor_perfect_square(poly: &Poly) -> Option<Poly> {
    if poly.terms.len() != 3 {
        return None;
    }
    let terms: Vec<(&Monomial, &Rat)> = poly.terms.iter().collect();
    for left_index in 0..terms.len() {
        for right_index in (left_index + 1)..terms.len() {
            let left = Poly {
                terms: BTreeMap::from([(terms[left_index].0.clone(), terms[left_index].1.clone())]),
            };
            let right = Poly {
                terms: BTreeMap::from([(
                    terms[right_index].0.clone(),
                    terms[right_index].1.clone(),
                )]),
            };
            let (Some(left_root), Some(right_root)) =
                (square_root_term(&left), square_root_term(&right))
            else {
                continue;
            };
            let cross_index =
                (0..terms.len()).find(|index| *index != left_index && *index != right_index)?;
            let cross = Poly {
                terms: BTreeMap::from([(
                    terms[cross_index].0.clone(),
                    terms[cross_index].1.clone(),
                )]),
            };
            let two = Rat::from_i64(2, 1).ok()?;
            let positive = left_root.mul(&right_root).scale(&two);
            if cross == positive {
                return Some(left_root.add(&right_root));
            }
            if cross == positive.neg() {
                return Some(left_root.sub(&right_root));
            }
        }
    }
    None
}

pub(crate) fn factor(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    let poly = poly_from_expr(arena, context, expression)?;
    if poly.is_zero() {
        return Some(arena.intern_int(Int::zero()));
    }
    if poly.terms.len() > MAX_TERMS {
        return None;
    }
    let content = poly.content()?;
    let mut factors = Vec::new();
    if !content.is_one() {
        factors.push(expression_from_rat(arena, &content));
    }
    let primitive = poly.scale(&Rat::one().div(&content).ok()?);

    let mut remaining = primitive;
    if remaining
        .terms
        .values()
        .all(|coefficient| coefficient.sign() < 0)
    {
        factors.push(arena.intern_int(Int::from_i64(-1)));
        remaining = remaining.neg();
    }
    if let Some(monomial) = remaining.monomial_content()
        && !monomial.is_one()
    {
        let mut powers = Vec::with_capacity(monomial.powers.len());
        for (symbol, exponent) in &monomial.powers {
            let base = arena.symbol_expr(*symbol);
            if *exponent == 1 {
                powers.push(base);
            } else {
                let exponent = arena.intern_int(Int::from_u64(*exponent as u64));
                powers.push(arena.intern_call(context.power, &[base, exponent]));
            }
        }
        factors.push(product(arena, &powers, context));
        remaining = remaining.divide_monomial(&monomial)?;
    }

    if !remaining.is_constant() {
        if let Some((variable, coefficients)) = remaining.univariate() {
            factors.extend(factor_univariate(arena, variable, &coefficients, context));
        } else if let Some((left, right)) = factor_difference_of_squares(&remaining) {
            factors.push(expression_from_poly(arena, &left, context));
            factors.push(expression_from_poly(arena, &right, context));
        } else if let Some((left, right)) = factor_sum_or_difference_of_cubes(&remaining) {
            factors.push(expression_from_poly(arena, &left, context));
            factors.push(expression_from_poly(arena, &right, context));
        } else if let Some((left, right)) = factor_sum_of_cubes(&remaining) {
            factors.push(expression_from_poly(arena, &left, context));
            factors.push(expression_from_poly(arena, &right, context));
        } else if let Some(staged) = multivariate::factor_stage(arena, &remaining, context) {
            factors.extend(staged);
        } else if let Some(base) = factor_perfect_square(&remaining) {
            let base = expression_from_poly(arena, &base, context);
            let exponent = arena.intern_int(Int::from_i64(2));
            factors.push(arena.intern_call(context.power, &[base, exponent]));
        } else {
            factors.push(expression_from_poly(arena, &remaining, context));
        }
    } else if factors.is_empty() {
        factors.push(expression_from_poly(arena, &remaining, context));
    }

    Some(product(arena, &factors, context))
}

pub(crate) fn factor_dense_components(coefficients: &[Rat]) -> Option<Vec<(Vec<Rat>, u32)>> {
    let coefficients = trim(coefficients.to_vec());
    if coefficients.len() <= 1 {
        return Some(Vec::new());
    }
    let components = factor_dense_square_free(&coefficients)?;
    let mut factors = Vec::new();
    for (component, multiplicity) in components {
        if component.len() > 4
            && let Some(parts) = hensel::factor_integer_polynomial(&component)
        {
            for part in parts {
                factors.push((part, multiplicity));
            }
            continue;
        }
        factors.push((component, multiplicity));
    }
    Some(factors)
}

fn push_dense_factor(
    arena: &mut Arena,
    variable: SymbolId,
    coefficients: &[Rat],
    multiplicity: u32,
    context: &Context,
    factors: &mut Vec<ExprId>,
) {
    if coefficients.len() <= 1 {
        return;
    }
    let polynomial = Poly::from_dense(variable, coefficients);
    let expression = expression_from_poly(arena, &polynomial, context);
    if multiplicity <= 1 {
        factors.push(expression);
    } else {
        let exponent = arena.intern_int(Int::from_u64(multiplicity as u64));
        factors.push(arena.intern_call(context.power, &[expression, exponent]));
    }
}

fn factor_univariate(
    arena: &mut Arena,
    variable: SymbolId,
    coefficients: &[Rat],
    context: &Context,
) -> Vec<ExprId> {
    let mut factors = Vec::new();
    let mut coefficients = trim(coefficients.to_vec());
    if coefficients
        .last()
        .is_some_and(|coefficient| coefficient.sign() < 0)
    {
        factors.push(arena.intern_int(Int::from_i64(-1)));
        coefficients = coefficients.iter().map(Rat::neg).collect();
    }
    let derivative = dense_derivative(&coefficients);
    if derivative.is_empty() || dense_gcd(&coefficients, &derivative).len() <= 1 {
        let (roots, remaining) = rational_roots(&coefficients);
        for (root, multiplicity) in roots {
            let numerator = root.num();
            let denominator = root.den();
            let factor = Poly::from_dense(
                variable,
                &[
                    Rat::from_ints(numerator.neg(), Int::one()).unwrap_or_else(|_| Rat::zero()),
                    Rat::from_ints(denominator, Int::one()).unwrap_or_else(|_| Rat::one()),
                ],
            );
            let factor = expression_from_poly(arena, &factor, context);
            if multiplicity == 1 {
                factors.push(factor);
            } else {
                let exponent = arena.intern_int(Int::from_u64(multiplicity as u64));
                factors.push(arena.intern_call(context.power, &[factor, exponent]));
            }
        }
        if remaining.len() > 1 {
            if remaining.len() > 4
                && let Some(parts) = hensel::factor_integer_polynomial(&remaining)
                && parts.len() > 1
            {
                for part in parts {
                    push_dense_factor(arena, variable, &part, 1, context, &mut factors);
                }
            } else {
                push_dense_factor(arena, variable, &remaining, 1, context, &mut factors);
            }
        }
        if factors.is_empty() {
            factors.push(expression_from_poly(
                arena,
                &Poly::from_dense(variable, &coefficients),
                context,
            ));
        }
        return factors;
    }
    let Some(components) = factor_dense_square_free(&coefficients) else {
        factors.push(expression_from_poly(
            arena,
            &Poly::from_dense(variable, &coefficients),
            context,
        ));
        return factors;
    };
    if components.is_empty() {
        factors.push(expression_from_poly(
            arena,
            &Poly::from_dense(variable, &coefficients),
            context,
        ));
        return factors;
    }
    let Some(scalar) = reconstruct_scalar(&coefficients, &components) else {
        factors.push(expression_from_poly(
            arena,
            &Poly::from_dense(variable, &coefficients),
            context,
        ));
        return factors;
    };
    if !scalar.is_one() {
        factors.push(expression_from_rat(arena, &scalar));
    }
    for (component, multiplicity) in components {
        if component.len() > 4
            && let Some(parts) = hensel::factor_integer_polynomial(&component)
            && parts.len() > 1
        {
            for part in parts {
                push_dense_factor(arena, variable, &part, multiplicity, context, &mut factors);
            }
        } else {
            push_dense_factor(
                arena,
                variable,
                &component,
                multiplicity,
                context,
                &mut factors,
            );
        }
    }
    if factors.is_empty() {
        factors.push(expression_from_poly(
            arena,
            &Poly::from_dense(variable, &coefficients),
            context,
        ));
    }
    factors
}

pub(crate) fn polynomial_gcd(
    arena: &mut Arena,
    left: ExprId,
    right: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let left = poly_from_expr(arena, context, left)?.dense_in(variable)?;
    let right = poly_from_expr(arena, context, right)?.dense_in(variable)?;
    let gcd = dense_gcd(&left, &right);
    let poly = Poly::from_dense(variable, &gcd);
    Some(expression_from_poly(arena, &poly, context))
}

pub(crate) fn resultant(
    arena: &mut Arena,
    left: ExprId,
    right: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let left = poly_from_expr(arena, context, left)?.dense_in(variable)?;
    let right = poly_from_expr(arena, context, right)?.dense_in(variable)?;
    let left_degree = left.len().saturating_sub(1);
    let right_degree = right.len().saturating_sub(1);
    if left_degree + right_degree > MAX_RESULTANT_DEGREE {
        return None;
    }
    let size = left_degree + right_degree;
    if size == 0 {
        return Some(arena.intern_int(Int::one()));
    }
    let mut matrix = vec![vec![Rat::zero(); size]; size];
    for row in 0..right_degree {
        for (index, coefficient) in left.iter().rev().enumerate() {
            matrix[row][row + index] = coefficient.clone();
        }
    }
    for row in 0..left_degree {
        for (index, coefficient) in right.iter().rev().enumerate() {
            matrix[right_degree + row][row + index] = coefficient.clone();
        }
    }
    let determinant = determinant(matrix);
    Some(expression_from_rat(arena, &determinant))
}
