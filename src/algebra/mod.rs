//! Exact polynomial algebra.
//!
//! Polynomials are sparse maps from canonical monomials to rational
//! coefficients. Monomials are sorted vectors of symbol and exponent pairs, so
//! equality and ordering need no expression hashing.
//!
//! Factoring covers numeric content, common monomial content, univariate
//! rational roots, and a difference of squares pattern. Cancellation uses a
//! univariate polynomial gcd when both sides depend on one variable and falls
//! back to monomial content otherwise.

mod hensel;
mod multivariate;

use std::collections::{BTreeMap, BTreeSet};

use crate::expr::{Arena, ExprId, Node, SymbolId};
use crate::num::{Int, Rat};

/// Expression heads owned by the evaluator and reused by the algebra layer.
#[derive(Copy, Clone)]
pub(crate) struct Context {
    pub(crate) plus: ExprId,
    pub(crate) times: ExprId,
    pub(crate) power: ExprId,
    pub(crate) negate: ExprId,
    pub(crate) list: ExprId,
    pub(crate) equal: ExprId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Monomial {
    powers: Vec<(SymbolId, u32)>,
}

impl Monomial {
    fn one() -> Self {
        Monomial { powers: Vec::new() }
    }

    fn variable(symbol: SymbolId) -> Self {
        Monomial {
            powers: vec![(symbol, 1)],
        }
    }

    fn is_one(&self) -> bool {
        self.powers.is_empty()
    }

    fn multiply(&self, other: &Monomial) -> Monomial {
        let mut powers = Vec::with_capacity(self.powers.len() + other.powers.len());
        let mut left = 0;
        let mut right = 0;
        while left < self.powers.len() || right < other.powers.len() {
            if right == other.powers.len()
                || (left < self.powers.len() && self.powers[left].0 < other.powers[right].0)
            {
                powers.push(self.powers[left]);
                left += 1;
            } else if left == self.powers.len() || other.powers[right].0 < self.powers[left].0 {
                powers.push(other.powers[right]);
                right += 1;
            } else {
                powers.push((
                    self.powers[left].0,
                    self.powers[left].1 + other.powers[right].1,
                ));
                left += 1;
                right += 1;
            }
        }
        Monomial { powers }
    }

    fn divide(&self, divisor: &Monomial) -> Option<Monomial> {
        let mut powers = Vec::with_capacity(self.powers.len());
        let mut index = 0;
        for (symbol, exponent) in &self.powers {
            let mut remaining = *exponent;
            if index < divisor.powers.len() && divisor.powers[index].0 < *symbol {
                return None;
            }
            if index < divisor.powers.len() && divisor.powers[index].0 == *symbol {
                if divisor.powers[index].1 > remaining {
                    return None;
                }
                remaining -= divisor.powers[index].1;
                index += 1;
            }
            if remaining != 0 {
                powers.push((*symbol, remaining));
            }
        }
        if index != divisor.powers.len() {
            return None;
        }
        Some(Monomial { powers })
    }

    fn pow_u32(&self, exponent: u32) -> Monomial {
        if exponent == 0 {
            return Monomial::one();
        }
        Monomial {
            powers: self
                .powers
                .iter()
                .map(|(symbol, power)| (*symbol, power.saturating_mul(exponent)))
                .collect(),
        }
    }

    fn degree_in(&self, symbol: SymbolId) -> u32 {
        self.powers
            .iter()
            .find(|(candidate, _)| *candidate == symbol)
            .map(|(_, exponent)| *exponent)
            .unwrap_or(0)
    }

    fn variables(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.powers.iter().map(|(symbol, _)| *symbol)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Poly {
    terms: BTreeMap<Monomial, Rat>,
}

const MAX_EXPONENT: i64 = 4096;
const MAX_TERMS: usize = 20_000;
const MAX_TERM_PRODUCTS: usize = 2_000_000;
const MAX_RESULTANT_DEGREE: usize = 64;

impl Poly {
    fn zero() -> Self {
        Poly {
            terms: BTreeMap::new(),
        }
    }

    fn constant(value: Rat) -> Self {
        let mut terms = BTreeMap::new();
        if !value.is_zero() {
            terms.insert(Monomial::one(), value);
        }
        Poly { terms }
    }

    fn variable(symbol: SymbolId) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(Monomial::variable(symbol), Rat::one());
        Poly { terms }
    }

    fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    fn is_constant(&self) -> bool {
        self.terms.len() <= 1 && self.terms.keys().all(Monomial::is_one)
    }

    fn constant_value(&self) -> Option<Rat> {
        if self.is_constant() {
            Some(
                self.terms
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_else(Rat::zero),
            )
        } else {
            None
        }
    }

    fn add(&self, other: &Poly) -> Poly {
        let mut out = self.clone();
        out.add_assign(other);
        out
    }

    fn add_assign(&mut self, other: &Poly) {
        for (monomial, coefficient) in &other.terms {
            let entry = self.terms.entry(monomial.clone()).or_insert_with(Rat::zero);
            *entry = entry.add(coefficient);
            if entry.is_zero() {
                self.terms.remove(monomial);
            }
        }
    }

    fn neg(&self) -> Poly {
        Poly {
            terms: self
                .terms
                .iter()
                .map(|(monomial, coefficient)| (monomial.clone(), coefficient.neg()))
                .collect(),
        }
    }

    fn sub(&self, other: &Poly) -> Poly {
        self.add(&other.neg())
    }

    fn mul(&self, other: &Poly) -> Poly {
        let mut out = Poly::zero();
        for (left_monomial, left_coefficient) in &self.terms {
            for (right_monomial, right_coefficient) in &other.terms {
                let monomial = left_monomial.multiply(right_monomial);
                let coefficient = left_coefficient.mul(right_coefficient);
                let entry = out.terms.entry(monomial.clone()).or_insert_with(Rat::zero);
                *entry = entry.add(&coefficient);
                if entry.is_zero() {
                    out.terms.remove(&monomial);
                }
            }
        }
        out
    }

    fn mul_checked(&self, other: &Poly) -> Option<Poly> {
        if self.terms.len().saturating_mul(other.terms.len()) > MAX_TERM_PRODUCTS {
            return None;
        }
        let mut out = Poly::zero();
        for (left_monomial, left_coefficient) in &self.terms {
            for (right_monomial, right_coefficient) in &other.terms {
                let monomial = left_monomial.multiply(right_monomial);
                let coefficient = left_coefficient.mul(right_coefficient);
                let entry = out.terms.entry(monomial.clone()).or_insert_with(Rat::zero);
                *entry = entry.add(&coefficient);
                if entry.is_zero() {
                    out.terms.remove(&monomial);
                }
                if out.terms.len() > MAX_TERMS {
                    return None;
                }
            }
        }
        Some(out)
    }

    fn scale(&self, factor: &Rat) -> Poly {
        if factor.is_zero() || self.is_zero() {
            return Poly::zero();
        }
        Poly {
            terms: self
                .terms
                .iter()
                .map(|(monomial, coefficient)| (monomial.clone(), coefficient.mul(factor)))
                .collect(),
        }
    }

    fn pow_u32(&self, exponent: u32) -> Poly {
        let mut result = Poly::constant(Rat::one());
        let mut base = self.clone();
        let mut power = exponent;
        while power > 0 {
            if power & 1 == 1 {
                result = result.mul(&base);
            }
            power >>= 1;
            if power > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    fn pow_u32_checked(&self, exponent: u32) -> Option<Poly> {
        let mut result = Poly::constant(Rat::one());
        let mut base = self.clone();
        let mut power = exponent;
        while power > 0 {
            if power & 1 == 1 {
                result = result.mul_checked(&base)?;
            }
            power >>= 1;
            if power > 0 {
                base = base.mul_checked(&base)?;
            }
        }
        Some(result)
    }

    fn variables(&self) -> BTreeSet<SymbolId> {
        let mut variables = BTreeSet::new();
        for monomial in self.terms.keys() {
            for variable in monomial.variables() {
                variables.insert(variable);
            }
        }
        variables
    }

    fn leading_term(&self) -> Option<(&Monomial, &Rat)> {
        self.terms.iter().next_back()
    }

    fn content(&self) -> Option<Rat> {
        if self.terms.is_empty() {
            return None;
        }
        let mut numerator = Int::zero();
        let mut denominator = Int::one();
        for coefficient in self.terms.values() {
            numerator = numerator.gcd(&coefficient.num().abs());
            denominator = lcm(&denominator, &coefficient.den());
        }
        Rat::from_ints(numerator, denominator).ok()
    }

    fn monomial_content(&self) -> Option<Monomial> {
        let mut powers: Option<Vec<(SymbolId, u32)>> = None;
        for monomial in self.terms.keys() {
            match &mut powers {
                None => powers = Some(monomial.powers.clone()),
                Some(current) => {
                    let mut next = Vec::with_capacity(current.len());
                    let mut left = 0;
                    let mut right = 0;
                    while left < current.len() && right < monomial.powers.len() {
                        match current[left].0.cmp(&monomial.powers[right].0) {
                            core::cmp::Ordering::Less => left += 1,
                            core::cmp::Ordering::Greater => right += 1,
                            core::cmp::Ordering::Equal => {
                                let exponent = current[left].1.min(monomial.powers[right].1);
                                if exponent != 0 {
                                    next.push((current[left].0, exponent));
                                }
                                left += 1;
                                right += 1;
                            }
                        }
                    }
                    *current = next;
                }
            }
        }
        powers.map(|powers| Monomial { powers })
    }

    fn divide_monomial(&self, divisor: &Monomial) -> Option<Poly> {
        let mut terms = BTreeMap::new();
        for (monomial, coefficient) in &self.terms {
            let reduced = monomial.divide(divisor)?;
            terms.insert(reduced, coefficient.clone());
        }
        Some(Poly { terms })
    }

    fn dense_in(&self, variable: SymbolId) -> Option<Vec<Rat>> {
        let mut degree = 0u32;
        for monomial in self.terms.keys() {
            for symbol in monomial.variables() {
                if symbol != variable {
                    return None;
                }
            }
            degree = degree.max(monomial.degree_in(variable));
        }
        let mut coefficients = vec![Rat::zero(); degree as usize + 1];
        for (monomial, coefficient) in &self.terms {
            let exponent = monomial.degree_in(variable) as usize;
            coefficients[exponent] = coefficient.clone();
        }
        Some(trim(coefficients))
    }

    fn univariate(&self) -> Option<(SymbolId, Vec<Rat>)> {
        let variables = self.variables();
        if variables.len() != 1 {
            return None;
        }
        let variable = *variables.iter().next()?;
        Some((variable, self.dense_in(variable)?))
    }

    fn from_dense(variable: SymbolId, coefficients: &[Rat]) -> Poly {
        let mut poly = Poly::zero();
        for (exponent, coefficient) in coefficients.iter().enumerate() {
            if coefficient.is_zero() {
                continue;
            }
            let monomial = Monomial::variable(variable).pow_u32(exponent as u32);
            poly.terms.insert(monomial, coefficient.clone());
        }
        poly
    }
}

fn poly_from_expr(arena: &Arena, context: &Context, expression: ExprId) -> Option<Poly> {
    match arena.node(expression) {
        Node::Int(value) => Some(Poly::constant(
            Rat::from_ints(value.clone(), Int::one()).ok()?,
        )),
        Node::Rat(value) => Some(Poly::constant(value.clone())),
        Node::Real(_) => None,
        Node::Symbol(symbol) => Some(Poly::variable(*symbol)),
        Node::Call { head, args } => {
            if *head == context.plus {
                let mut sum = Poly::zero();
                for arg in args.iter() {
                    let term = poly_from_expr(arena, context, *arg)?;
                    sum.add_assign(&term);
                }
                Some(sum)
            } else if *head == context.times {
                let mut product = Poly::constant(Rat::one());
                for arg in args.iter() {
                    product = product.mul_checked(&poly_from_expr(arena, context, *arg)?)?;
                }
                Some(product)
            } else if *head == context.negate {
                if args.len() != 1 {
                    return None;
                }
                Some(poly_from_expr(arena, context, args[0])?.neg())
            } else if *head == context.power {
                if args.len() != 2 {
                    return None;
                }
                let exponent = integer_value(arena, args[1])?;
                if exponent < 0 || exponent > MAX_EXPONENT {
                    return None;
                }
                Some(poly_from_expr(arena, context, args[0])?.pow_u32_checked(exponent as u32)?)
            } else {
                None
            }
        }
    }
}

fn integer_value(arena: &Arena, expression: ExprId) -> Option<i64> {
    match arena.node(expression) {
        Node::Int(value) => value.to_i64(),
        Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
        _ => None,
    }
}

fn expression_from_rat(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

fn expression_from_poly(arena: &mut Arena, poly: &Poly, context: &Context) -> ExprId {
    if poly.is_zero() {
        return arena.intern_int(Int::zero());
    }
    let mut terms = Vec::with_capacity(poly.terms.len());
    for (monomial, coefficient) in &poly.terms {
        let coefficient_expression = expression_from_rat(arena, coefficient);
        if monomial.is_one() {
            terms.push(coefficient_expression);
            continue;
        }
        let mut factors = Vec::with_capacity(monomial.powers.len() + 1);
        if !coefficient.is_one() {
            factors.push(coefficient_expression);
        }
        for (symbol, exponent) in &monomial.powers {
            let base = arena.symbol_expr(*symbol);
            if *exponent == 1 {
                factors.push(base);
            } else {
                let exponent = arena.intern_int(Int::from_u64(*exponent as u64));
                factors.push(arena.intern_call(context.power, &[base, exponent]));
            }
        }
        terms.push(product(arena, &factors, context));
    }
    sum(arena, &terms, context)
}

fn product(arena: &mut Arena, factors: &[ExprId], context: &Context) -> ExprId {
    match factors.len() {
        0 => arena.intern_int(Int::one()),
        1 => factors[0],
        _ => arena.intern_call(context.times, factors),
    }
}

fn sum(arena: &mut Arena, terms: &[ExprId], context: &Context) -> ExprId {
    match terms.len() {
        0 => arena.intern_int(Int::zero()),
        1 => terms[0],
        _ => arena.intern_call(context.plus, terms),
    }
}

fn negate(arena: &mut Arena, value: ExprId, context: &Context) -> ExprId {
    arena.intern_call(context.negate, &[value])
}

pub(crate) fn expand(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    let poly = poly_from_expr(arena, context, expression)?;
    if poly.terms.len() > MAX_TERMS {
        return None;
    }
    Some(expression_from_poly(arena, &poly, context))
}

fn trim(mut coefficients: Vec<Rat>) -> Vec<Rat> {
    while coefficients.last().is_some_and(Rat::is_zero) {
        coefficients.pop();
    }
    coefficients
}

fn dense_degree(coefficients: &[Rat]) -> usize {
    coefficients
        .iter()
        .rposition(|coefficient| !coefficient.is_zero())
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn dense_div_rem(dividend: &[Rat], divisor: &[Rat]) -> Option<(Vec<Rat>, Vec<Rat>)> {
    let divisor = trim(divisor.to_vec());
    if divisor.is_empty() {
        return None;
    }
    let mut remainder = trim(dividend.to_vec());
    let divisor_degree = dense_degree(&divisor);
    if remainder.len() < divisor_degree {
        return Some((Vec::new(), remainder));
    }
    let mut quotient = vec![Rat::zero(); remainder.len() - divisor_degree + 1];
    let leading = divisor[divisor_degree - 1].clone();
    while remainder.len() >= divisor_degree {
        let degree = remainder.len() - divisor_degree;
        let coefficient = remainder[remainder.len() - 1].div(&leading).ok()?;
        quotient[degree] = coefficient.clone();
        for index in 0..divisor_degree {
            let position = degree + index;
            let value = remainder[position].sub(&coefficient.mul(&divisor[index]));
            remainder[position] = value;
        }
        remainder = trim(remainder);
    }
    Some((trim(quotient), remainder))
}

fn dense_gcd(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let mut left = trim(left.to_vec());
    let mut right = trim(right.to_vec());
    while !right.is_empty() {
        let remainder = match dense_div_rem(&left, &right) {
            Some((_, remainder)) => remainder,
            None => return vec![Rat::one()],
        };
        left = right;
        right = remainder;
    }
    if left.is_empty() {
        return vec![Rat::one()];
    }
    let leading = left.last().cloned().unwrap_or_else(Rat::one);
    left.iter()
        .map(|coefficient| coefficient.div(&leading).unwrap_or_else(|_| Rat::zero()))
        .collect()
}

fn dense_eval(coefficients: &[Rat], value: &Rat) -> Rat {
    let mut result = Rat::zero();
    for coefficient in coefficients.iter().rev() {
        result = result.mul(value).add(coefficient);
    }
    result
}

fn dense_derivative(coefficients: &[Rat]) -> Vec<Rat> {
    let mut derivative = Vec::with_capacity(coefficients.len().saturating_sub(1));
    for (index, coefficient) in coefficients.iter().enumerate().skip(1) {
        let factor = Rat::from_i64(index as i64, 1).unwrap_or_else(|_| Rat::one());
        derivative.push(coefficient.mul(&factor));
    }
    trim(derivative)
}

fn dense_multiply(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }
    let mut product = vec![Rat::zero(); left.len() + right.len() - 1];
    for (left_index, left_coefficient) in left.iter().enumerate() {
        for (right_index, right_coefficient) in right.iter().enumerate() {
            let index = left_index + right_index;
            product[index] = product[index].add(&left_coefficient.mul(right_coefficient));
        }
    }
    trim(product)
}

fn dense_power(base: &[Rat], exponent: u32) -> Vec<Rat> {
    let mut result = vec![Rat::one()];
    for _ in 0..exponent {
        result = dense_multiply(&result, base);
    }
    result
}

fn dense_nth_root(coefficients: &[Rat], degree: u32) -> Option<Vec<Rat>> {
    if degree < 2 {
        return None;
    }
    let coefficients = trim(coefficients.to_vec());
    if coefficients.is_empty() {
        return None;
    }
    let degree_usize = degree as usize;
    let mut shift = 0usize;
    while coefficients.get(shift).is_some_and(Rat::is_zero) {
        shift += 1;
    }
    if shift >= coefficients.len() || !shift.is_multiple_of(degree_usize) {
        return None;
    }
    let core = &coefficients[shift..];
    let constant = rational_nth_root(core.first()?, degree)?;
    if constant.is_zero() {
        return None;
    }
    let core_degree = core.len().checked_sub(1)?;
    if !core_degree.is_multiple_of(degree_usize) {
        return None;
    }
    let root_degree = core_degree / degree_usize;
    let mut root = vec![Rat::zero(); root_degree + 1];
    root[0] = constant.clone();
    let denominator = Rat::from_i64(degree as i64, 1)
        .ok()?
        .mul(&rat_power(&constant, degree - 1));
    for power in 1..=root_degree {
        let expansion = dense_power(&root, degree);
        let target = core.get(power).cloned().unwrap_or_else(Rat::zero);
        let current = expansion.get(power).cloned().unwrap_or_else(Rat::zero);
        root[power] = target.sub(&current).div(&denominator).ok()?;
    }
    if dense_power(&root, degree) != core {
        return None;
    }
    let mut result = vec![Rat::zero(); shift / degree_usize];
    result.extend(root);
    Some(result)
}

fn rat_power(value: &Rat, exponent: u32) -> Rat {
    let mut result = Rat::one();
    for _ in 0..exponent {
        result = result.mul(value);
    }
    result
}

fn rational_nth_root(value: &Rat, degree: u32) -> Option<Rat> {
    if degree == 0 {
        return None;
    }
    let numerator = integer_nth_root(&value.num(), degree)?;
    let denominator = integer_nth_root(&value.den(), degree)?;
    if denominator.is_zero() {
        return None;
    }
    Rat::from_ints(numerator, denominator).ok()
}

fn integer_nth_root(value: &Int, degree: u32) -> Option<Int> {
    let value = value.to_i64()?;
    if value == 0 {
        return Some(Int::zero());
    }
    if value < 0 && degree.is_multiple_of(2) {
        return None;
    }
    let magnitude = value.unsigned_abs() as i128;
    let mut low = 0i128;
    let mut high = magnitude;
    let mut best = 0i128;
    while low <= high {
        let middle = low + (high - low) / 2;
        let mut power = 1i128;
        let mut overflow = false;
        for _ in 0..degree {
            match power.checked_mul(middle) {
                Some(next) => power = next,
                None => {
                    overflow = true;
                    break;
                }
            }
        }
        if overflow || power > magnitude {
            high = middle - 1;
        } else {
            best = middle;
            low = middle + 1;
        }
    }
    let root = Int::from_i64(i64::try_from(best).ok()?);
    Some(if value < 0 { root.neg() } else { root })
}

fn primitive_normalize(coefficients: &[Rat]) -> Vec<Rat> {
    let Some((_, values)) = integerize(coefficients) else {
        return trim(coefficients.to_vec());
    };
    let mut values: Vec<Rat> = values
        .into_iter()
        .map(|value| Rat::from_ints(value, Int::one()).unwrap_or_else(|_| Rat::zero()))
        .collect();
    if values.last().is_some_and(|value| value.sign() < 0) {
        values = values.iter().map(Rat::neg).collect();
    }
    trim(values)
}

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

fn lcm(left: &Int, right: &Int) -> Int {
    if left.is_zero() || right.is_zero() {
        return Int::zero();
    }
    let gcd = left.gcd(right).abs();
    left.mul(right)
        .divrem(&gcd)
        .map(|(quotient, _)| quotient.abs())
        .unwrap_or_else(|_| Int::one())
}

fn integer_sqrt(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(middle);
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

fn rational_sqrt(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_sqrt(numerator)?;
    let denominator = integer_sqrt(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
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

fn integer_cbrt(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let cube = (middle as i128) * (middle as i128) * (middle as i128);
        if cube == value as i128 {
            return Some(middle);
        }
        if cube < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

fn rational_cbrt(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_cbrt(numerator)?;
    let denominator = integer_cbrt(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
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

fn extract_common_factor(poly: &Poly) -> Option<(Poly, Poly)> {
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

fn factor_by_grouping(poly: &Poly) -> Option<(Poly, Poly)> {
    let size = poly.terms.len();
    if size < 4 || size > 8 || size % 2 != 0 {
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

fn integerize(coefficients: &[Rat]) -> Option<(Rat, Vec<Int>)> {
    let mut denominator = Int::one();
    for coefficient in coefficients {
        denominator = lcm(&denominator, &coefficient.den());
    }
    let mut values = Vec::with_capacity(coefficients.len());
    let mut content = Int::zero();
    for coefficient in coefficients {
        let (quotient, _) = denominator.divrem(&coefficient.den()).ok()?;
        let value = coefficient.num().mul(&quotient);
        content = content.gcd(&value.abs());
        values.push(value);
    }
    if content.is_zero() {
        return None;
    }
    for value in &mut values {
        *value = value.divrem(&content).ok()?.0;
    }
    Some((Rat::from_ints(content, denominator).ok()?, values))
}

fn divisors(value: i64) -> Vec<i64> {
    let magnitude = value.unsigned_abs();
    if magnitude == 0 || magnitude > 1_000_000_000_000 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut divisor = 1i64;
    while (divisor as u128) * (divisor as u128) <= magnitude as u128 {
        if magnitude % divisor as u64 == 0 {
            let other = (magnitude / divisor as u64) as i64;
            out.push(divisor);
            if other != divisor {
                out.push(other);
            }
        }
        divisor += 1;
    }
    out
}

fn rational_roots(coefficients: &[Rat]) -> (Vec<(Rat, u32)>, Vec<Rat>) {
    let Some((_, integer_coefficients)) = integerize(coefficients) else {
        return (Vec::new(), trim(coefficients.to_vec()));
    };
    let mut remaining: Vec<Rat> = integer_coefficients
        .iter()
        .map(|value| Rat::from_ints(value.clone(), Int::one()).unwrap_or_else(|_| Rat::zero()))
        .collect();
    remaining = trim(remaining);
    let mut roots = Vec::new();

    while remaining.len() > 1 && remaining[0].is_zero() {
        roots.push((Rat::zero(), 1));
        let Some((quotient, _)) = dense_div_rem(&remaining, &[Rat::zero(), Rat::one()]) else {
            break;
        };
        remaining = quotient;
    }
    if remaining.len() <= 1 {
        return (roots, remaining);
    }

    let (Some(constant), Some(leading)) = (
        remaining[0].num().to_i64(),
        remaining.last().and_then(|value| value.num().to_i64()),
    ) else {
        return (roots, remaining);
    };
    if leading == 0 {
        return (roots, remaining);
    }

    let mut candidates = BTreeSet::new();
    for numerator in divisors(constant.abs()) {
        for denominator in divisors(leading.abs()) {
            if denominator <= 0 {
                continue;
            }
            let gcd = gcd_i64(numerator, denominator);
            let numerator = numerator / gcd;
            let denominator = denominator / gcd;
            candidates.insert((numerator, denominator));
            candidates.insert((-numerator, denominator));
        }
    }

    for (numerator, denominator) in candidates {
        let candidate = match Rat::from_i64(numerator, denominator) {
            Ok(value) => value,
            Err(_) => continue,
        };
        loop {
            if remaining.len() <= 1 || !dense_eval(&remaining, &candidate).is_zero() {
                break;
            }
            let factor = [
                Rat::from_i64(-numerator, 1).unwrap_or_else(|_| Rat::zero()),
                Rat::from_i64(denominator, 1).unwrap_or_else(|_| Rat::one()),
            ];
            let Some((quotient, remainder)) = dense_div_rem(&remaining, &factor) else {
                break;
            };
            if !remainder.is_empty() {
                break;
            }
            remaining = quotient;
            push_root(&mut roots, candidate.clone());
        }
    }
    (roots, remaining)
}

fn gcd_i64(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs().max(1)
}

fn push_root(roots: &mut Vec<(Rat, u32)>, root: Rat) {
    for (candidate, multiplicity) in roots.iter_mut() {
        if *candidate == root {
            *multiplicity += 1;
            return;
        }
    }
    roots.push((root, 1));
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

#[derive(Clone)]
struct RationalFunction {
    numerator: Poly,
    denominator: Poly,
}

impl RationalFunction {
    fn one() -> Self {
        RationalFunction {
            numerator: Poly::constant(Rat::one()),
            denominator: Poly::constant(Rat::one()),
        }
    }

    fn add(&self, other: &RationalFunction) -> RationalFunction {
        RationalFunction {
            numerator: self
                .numerator
                .mul(&other.denominator)
                .add(&other.numerator.mul(&self.denominator)),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    fn mul(&self, other: &RationalFunction) -> RationalFunction {
        RationalFunction {
            numerator: self.numerator.mul(&other.numerator),
            denominator: self.denominator.mul(&other.denominator),
        }
    }

    fn neg(&self) -> RationalFunction {
        RationalFunction {
            numerator: self.numerator.neg(),
            denominator: self.denominator.clone(),
        }
    }

    fn pow(&self, exponent: i64) -> Option<RationalFunction> {
        let power = exponent.unsigned_abs() as u32;
        let numerator = self.numerator.pow_u32(power);
        let denominator = self.denominator.pow_u32(power);
        if exponent >= 0 {
            Some(RationalFunction {
                numerator,
                denominator,
            })
        } else {
            Some(RationalFunction {
                numerator: denominator,
                denominator: numerator,
            })
        }
    }
}

fn rational_from_expr(
    arena: &Arena,
    context: &Context,
    expression: ExprId,
) -> Option<RationalFunction> {
    match arena.node(expression) {
        Node::Int(_) | Node::Rat(_) | Node::Symbol(_) => Some(RationalFunction {
            numerator: poly_from_expr(arena, context, expression)?,
            denominator: Poly::constant(Rat::one()),
        }),
        Node::Real(_) => None,
        Node::Call { head, args } => {
            if *head == context.plus {
                let mut sum = RationalFunction {
                    numerator: Poly::zero(),
                    denominator: Poly::constant(Rat::one()),
                };
                for arg in args.iter() {
                    sum = sum.add(&rational_from_expr(arena, context, *arg)?);
                }
                Some(sum)
            } else if *head == context.times {
                let mut product = RationalFunction::one();
                for arg in args.iter() {
                    product = product.mul(&rational_from_expr(arena, context, *arg)?);
                }
                Some(product)
            } else if *head == context.negate {
                if args.len() != 1 {
                    return None;
                }
                Some(rational_from_expr(arena, context, args[0])?.neg())
            } else if *head == context.power {
                if args.len() != 2 {
                    return None;
                }
                let exponent = integer_value(arena, args[1])?;
                Some(rational_from_expr(arena, context, args[0])?.pow(exponent)?)
            } else {
                None
            }
        }
    }
}

fn cancel_function(function: &RationalFunction) -> RationalFunction {
    let mut numerator = function.numerator.clone();
    let mut denominator = function.denominator.clone();
    if numerator.is_zero() || denominator.is_zero() {
        return RationalFunction {
            numerator: Poly::zero(),
            denominator: Poly::constant(Rat::one()),
        };
    }

    if let (Some(left), Some(right)) =
        (numerator.monomial_content(), denominator.monomial_content())
    {
        let common = common_monomial(&left, &right);
        if !common.is_one() {
            if let Some(reduced) = numerator.divide_monomial(&common) {
                numerator = reduced;
            }
            if let Some(reduced) = denominator.divide_monomial(&common) {
                denominator = reduced;
            }
        }
    }

    let numerator_variables = numerator.variables();
    let denominator_variables = denominator.variables();
    if numerator_variables.len() == 1 && numerator_variables == denominator_variables {
        let variable = *numerator_variables.iter().next().expect("one variable");
        if let (Some(numerator_dense), Some(denominator_dense)) =
            (numerator.dense_in(variable), denominator.dense_in(variable))
        {
            let gcd = dense_gcd(&numerator_dense, &denominator_dense);
            if gcd.len() > 1 {
                if let Some(value) = divide_by_gcd(&numerator_dense, &gcd, variable) {
                    numerator = value;
                }
                if let Some(value) = divide_by_gcd(&denominator_dense, &gcd, variable) {
                    denominator = value;
                }
            }
        }
    }

    if let Some((_, coefficient)) = denominator.leading_term() {
        let coefficient = coefficient.clone();
        if !coefficient.is_zero()
            && !coefficient.is_one()
            && let Ok(inverse) = Rat::one().div(&coefficient)
        {
            numerator = numerator.scale(&inverse);
            denominator = denominator.scale(&inverse);
        }
    }
    RationalFunction {
        numerator,
        denominator,
    }
}

fn divide_by_gcd(coefficients: &[Rat], gcd: &[Rat], variable: SymbolId) -> Option<Poly> {
    let (quotient, remainder) = dense_div_rem(coefficients, gcd)?;
    if remainder.is_empty() {
        Some(Poly::from_dense(variable, &quotient))
    } else {
        None
    }
}

fn common_monomial(left: &Monomial, right: &Monomial) -> Monomial {
    let mut powers = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.powers.len() && right_index < right.powers.len() {
        match left.powers[left_index].0.cmp(&right.powers[right_index].0) {
            core::cmp::Ordering::Less => left_index += 1,
            core::cmp::Ordering::Greater => right_index += 1,
            core::cmp::Ordering::Equal => {
                let exponent = left.powers[left_index].1.min(right.powers[right_index].1);
                if exponent != 0 {
                    powers.push((left.powers[left_index].0, exponent));
                }
                left_index += 1;
                right_index += 1;
            }
        }
    }
    Monomial { powers }
}

fn rational_expression(
    arena: &mut Arena,
    function: &RationalFunction,
    context: &Context,
) -> ExprId {
    let numerator = expression_from_poly(arena, &function.numerator, context);
    if let Some(value) = function.denominator.constant_value() {
        if value.is_one() {
            return numerator;
        }
        let inverse = Rat::one().div(&value).ok().unwrap_or_else(Rat::one);
        let scaled = function.numerator.scale(&inverse);
        return expression_from_poly(arena, &scaled, context);
    }
    let denominator = expression_from_poly(arena, &function.denominator, context);
    let exponent = arena.intern_int(Int::from_i64(-1));
    let reciprocal = arena.intern_call(context.power, &[denominator, exponent]);
    product(arena, &[numerator, reciprocal], context)
}

pub(crate) fn together(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    let function = rational_from_expr(arena, context, expression)?;
    let function = cancel_function(&function);
    Some(rational_expression(arena, &function, context))
}

pub(crate) fn cancel(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    together(arena, expression, context)
}

fn variable_argument(arena: &Arena, expression: ExprId) -> Option<SymbolId> {
    arena.symbol_id(expression)
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

fn determinant(mut matrix: Vec<Vec<Rat>>) -> Rat {
    let mut result = Rat::one();
    let size = matrix.len();
    for column in 0..size {
        let Some(pivot_row) = (column..size).find(|row| !matrix[*row][column].is_zero()) else {
            return Rat::zero();
        };
        if pivot_row != column {
            matrix.swap(pivot_row, column);
            result = result.neg();
        }
        let pivot = matrix[column][column].clone();
        result = result.mul(&pivot);
        for row in (column + 1)..size {
            if matrix[row][column].is_zero() {
                continue;
            }
            let factor = match matrix[row][column].div(&pivot) {
                Ok(value) => value,
                Err(_) => return Rat::zero(),
            };
            for inner in column..size {
                let value = matrix[row][inner].sub(&factor.mul(&matrix[column][inner]));
                matrix[row][inner] = value;
            }
        }
    }
    result
}

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

pub(crate) fn degree(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let poly = poly_from_expr(arena, context, expression)?;
    if poly.is_zero() {
        return None;
    }
    let coefficients = poly.dense_in(variable)?;
    Some(arena.intern_int(Int::from_i64(coefficients.len() as i64 - 1)))
}

pub(crate) fn coefficient(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    index: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let index = integer_value(arena, index)?;
    if index < 0 {
        return None;
    }
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let value = coefficients
        .get(index as usize)
        .cloned()
        .unwrap_or_else(Rat::zero);
    Some(expression_from_rat(arena, &value))
}

pub(crate) fn polynomial_divide(
    arena: &mut Arena,
    dividend: ExprId,
    divisor: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let dividend = poly_from_expr(arena, context, dividend)?.dense_in(variable)?;
    let divisor = poly_from_expr(arena, context, divisor)?.dense_in(variable)?;
    if divisor.is_empty() {
        return None;
    }
    let (quotient, remainder) = dense_div_rem(&dividend, &divisor)?;
    let quotient = expression_from_poly(arena, &Poly::from_dense(variable, &quotient), context);
    let remainder = expression_from_poly(arena, &Poly::from_dense(variable, &remainder), context);
    let list = arena.builtin_head("list");
    Some(arena.intern_call(list, &[quotient, remainder]))
}

pub(crate) fn discriminant(
    arena: &mut Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<ExprId> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let degree = coefficients.len().saturating_sub(1);
    let value = match degree {
        0 | 1 => Rat::one(),
        2 => coefficients[1].mul(&coefficients[1]).sub(
            &Rat::from_i64(4, 1)
                .ok()?
                .mul(&coefficients[2])
                .mul(&coefficients[0]),
        ),
        3 => {
            let a = &coefficients[3];
            let b = &coefficients[2];
            let c = &coefficients[1];
            let d = &coefficients[0];
            let b2 = b.mul(b);
            let c2 = c.mul(c);
            let a2 = a.mul(a);
            let d2 = d.mul(d);
            let b3 = b2.mul(b);
            let c3 = c2.mul(c);
            let b2c2 = b2.mul(&c2);
            let ac3 = a.mul(&c3);
            let b3d = b3.mul(d);
            let a2d2 = a2.mul(&d2);
            let abcd = a.mul(b).mul(c).mul(d);
            b2c2.sub(&Rat::from_i64(4, 1).ok()?.mul(&ac3))
                .sub(&Rat::from_i64(4, 1).ok()?.mul(&b3d))
                .sub(&Rat::from_i64(27, 1).ok()?.mul(&a2d2))
                .add(&Rat::from_i64(18, 1).ok()?.mul(&abcd))
        }
        _ => return None,
    };
    Some(expression_from_rat(arena, &value))
}

/// The value of a rational function as its variable tends to infinity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RationalLimit {
    Finite(Rat),
    Infinite(i8),
}

/// Evaluate a rational expression at positive or negative infinity.
///
/// Returns `None` when the expression is not rational in the variable or the
/// denominator is identically zero.
pub(crate) fn rational_limit_at_infinity(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
    positive: bool,
) -> Option<RationalLimit> {
    let variable = variable_argument(arena, variable)?;
    let function = cancel_function(&rational_from_expr(arena, context, expression)?);
    if function.denominator.is_zero() {
        return None;
    }
    let numerator = function.numerator.dense_in(variable)?;
    let denominator = function.denominator.dense_in(variable)?;
    if numerator.is_empty() {
        return Some(RationalLimit::Finite(Rat::zero()));
    }
    if denominator.is_empty() {
        return None;
    }
    let numerator_degree = numerator.len() as i64 - 1;
    let denominator_degree = denominator.len() as i64 - 1;
    let numerator_leading = numerator.last()?;
    let denominator_leading = denominator.last()?;
    if numerator_degree < denominator_degree {
        return Some(RationalLimit::Finite(Rat::zero()));
    }
    if numerator_degree == denominator_degree {
        return numerator_leading
            .div(denominator_leading)
            .ok()
            .map(RationalLimit::Finite);
    }
    let ratio = numerator_leading.div(denominator_leading).ok()?;
    let difference = numerator_degree - denominator_degree;
    let sign = if !positive && difference % 2 != 0 {
        -ratio.sign()
    } else {
        ratio.sign()
    };
    Some(RationalLimit::Infinite(sign))
}

pub(crate) fn univariate_rational_root_multiplicities(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<Vec<(Rat, u32)>> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    Some(rational_roots(&coefficients).0)
}

pub(crate) fn univariate_rational_roots(
    arena: &Arena,
    expression: ExprId,
    variable: ExprId,
    context: &Context,
) -> Option<(Vec<Rat>, Rat)> {
    let variable = variable_argument(arena, variable)?;
    let coefficients = poly_from_expr(arena, context, expression)?.dense_in(variable)?;
    let leading = coefficients.last()?.clone();
    let (roots, remaining) = rational_roots(&coefficients);
    if remaining.len() > 1 {
        return None;
    }
    if roots.iter().any(|(_, multiplicity)| *multiplicity != 1) {
        return None;
    }
    Some((roots.into_iter().map(|(root, _)| root).collect(), leading))
}
