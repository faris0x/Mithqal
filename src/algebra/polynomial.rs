//! Sparse polynomial representation and conversion.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Monomial {
    pub(super) powers: Vec<(SymbolId, u32)>,
}

impl Monomial {
    pub(super) fn one() -> Self {
        Monomial { powers: Vec::new() }
    }

    pub(super) fn variable(symbol: SymbolId) -> Self {
        Monomial {
            powers: vec![(symbol, 1)],
        }
    }

    pub(super) fn is_one(&self) -> bool {
        self.powers.is_empty()
    }

    pub(super) fn multiply(&self, other: &Monomial) -> Monomial {
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

    pub(super) fn divide(&self, divisor: &Monomial) -> Option<Monomial> {
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

    pub(super) fn pow_u32(&self, exponent: u32) -> Monomial {
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

    pub(super) fn degree_in(&self, symbol: SymbolId) -> u32 {
        self.powers
            .iter()
            .find(|(candidate, _)| *candidate == symbol)
            .map(|(_, exponent)| *exponent)
            .unwrap_or(0)
    }

    pub(super) fn variables(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.powers.iter().map(|(symbol, _)| *symbol)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Poly {
    pub(super) terms: BTreeMap<Monomial, Rat>,
}

pub(crate) const MAX_EXPONENT: i64 = 4096;
pub(crate) const MAX_TERMS: usize = 20_000;
pub(crate) const MAX_TERM_PRODUCTS: usize = 2_000_000;
pub(crate) const MAX_RESULTANT_DEGREE: usize = 64;

impl Poly {
    pub(super) fn zero() -> Self {
        Poly {
            terms: BTreeMap::new(),
        }
    }

    pub(super) fn constant(value: Rat) -> Self {
        let mut terms = BTreeMap::new();
        if !value.is_zero() {
            terms.insert(Monomial::one(), value);
        }
        Poly { terms }
    }

    pub(super) fn variable(symbol: SymbolId) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(Monomial::variable(symbol), Rat::one());
        Poly { terms }
    }

    pub(super) fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    pub(super) fn is_constant(&self) -> bool {
        self.terms.len() <= 1 && self.terms.keys().all(Monomial::is_one)
    }

    pub(super) fn constant_value(&self) -> Option<Rat> {
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

    pub(super) fn add(&self, other: &Poly) -> Poly {
        let mut out = self.clone();
        out.add_assign(other);
        out
    }

    pub(super) fn add_assign(&mut self, other: &Poly) {
        for (monomial, coefficient) in &other.terms {
            let entry = self.terms.entry(monomial.clone()).or_insert_with(Rat::zero);
            *entry = entry.add(coefficient);
            if entry.is_zero() {
                self.terms.remove(monomial);
            }
        }
    }

    pub(super) fn neg(&self) -> Poly {
        Poly {
            terms: self
                .terms
                .iter()
                .map(|(monomial, coefficient)| (monomial.clone(), coefficient.neg()))
                .collect(),
        }
    }

    pub(super) fn sub(&self, other: &Poly) -> Poly {
        self.add(&other.neg())
    }

    pub(super) fn mul(&self, other: &Poly) -> Poly {
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

    pub(super) fn mul_checked(&self, other: &Poly) -> Option<Poly> {
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

    pub(super) fn scale(&self, factor: &Rat) -> Poly {
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

    pub(super) fn pow_u32(&self, exponent: u32) -> Poly {
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

    pub(super) fn pow_u32_checked(&self, exponent: u32) -> Option<Poly> {
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

    pub(super) fn variables(&self) -> BTreeSet<SymbolId> {
        let mut variables = BTreeSet::new();
        for monomial in self.terms.keys() {
            for variable in monomial.variables() {
                variables.insert(variable);
            }
        }
        variables
    }

    pub(super) fn leading_term(&self) -> Option<(&Monomial, &Rat)> {
        self.terms.iter().next_back()
    }

    pub(super) fn content(&self) -> Option<Rat> {
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

    pub(super) fn monomial_content(&self) -> Option<Monomial> {
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

    pub(super) fn divide_monomial(&self, divisor: &Monomial) -> Option<Poly> {
        let mut terms = BTreeMap::new();
        for (monomial, coefficient) in &self.terms {
            let reduced = monomial.divide(divisor)?;
            terms.insert(reduced, coefficient.clone());
        }
        Some(Poly { terms })
    }

    pub(super) fn dense_in(&self, variable: SymbolId) -> Option<Vec<Rat>> {
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

    pub(super) fn univariate(&self) -> Option<(SymbolId, Vec<Rat>)> {
        let variables = self.variables();
        if variables.len() != 1 {
            return None;
        }
        let variable = *variables.iter().next()?;
        Some((variable, self.dense_in(variable)?))
    }

    pub(super) fn from_dense(variable: SymbolId, coefficients: &[Rat]) -> Poly {
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

pub(crate) fn poly_from_expr(arena: &Arena, context: &Context, expression: ExprId) -> Option<Poly> {
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
                if !(0..=MAX_EXPONENT).contains(&exponent) {
                    return None;
                }
                Some(poly_from_expr(arena, context, args[0])?.pow_u32_checked(exponent as u32)?)
            } else {
                None
            }
        }
    }
}

pub(crate) fn integer_value(arena: &Arena, expression: ExprId) -> Option<i64> {
    match arena.node(expression) {
        Node::Int(value) => value.to_i64(),
        Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
        _ => None,
    }
}

pub(crate) fn expression_from_rat(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

pub(crate) fn expression_from_poly(arena: &mut Arena, poly: &Poly, context: &Context) -> ExprId {
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

pub(crate) fn product(arena: &mut Arena, factors: &[ExprId], context: &Context) -> ExprId {
    match factors.len() {
        0 => arena.intern_int(Int::one()),
        1 => factors[0],
        _ => arena.intern_call(context.times, factors),
    }
}

pub(crate) fn sum(arena: &mut Arena, terms: &[ExprId], context: &Context) -> ExprId {
    match terms.len() {
        0 => arena.intern_int(Int::zero()),
        1 => terms[0],
        _ => arena.intern_call(context.plus, terms),
    }
}

pub(crate) fn negate(arena: &mut Arena, value: ExprId, context: &Context) -> ExprId {
    arena.intern_call(context.negate, &[value])
}

pub(crate) fn expand(arena: &mut Arena, expression: ExprId, context: &Context) -> Option<ExprId> {
    let poly = poly_from_expr(arena, context, expression)?;
    if poly.terms.len() > MAX_TERMS {
        return None;
    }
    Some(expression_from_poly(arena, &poly, context))
}
