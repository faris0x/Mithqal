//! Class groups for small-degree number fields.
//!
//! The computation uses the Minkowski bound, prime ideal enumeration, a short
//! element search inside each generating ideal, relation collection, and Smith
//! normal form. The first envelope is deliberately small: degree at most three
//! when relations are needed, with a bounded Minkowski radius.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::Int;

use super::super::lattice::smith_invariants;
use super::decomposition::{PrimeIdeal, decompose_prime_general, factor_ideal};
use super::ideal::{FractionalIdeal, Ideal};
use super::order::Order;

/// A computed class group.
#[derive(Clone, Debug)]
pub(crate) struct ClassGroup {
    invariants: Vec<Int>,
    class_number: Int,
    minkowski_bound: Int,
}

impl ClassGroup {
    /// The cyclic invariants greater than one.
    pub(crate) fn invariants(&self) -> &[Int] {
        &self.invariants
    }

    /// The class number.
    pub(crate) fn class_number(&self) -> &Int {
        &self.class_number
    }

    /// The Minkowski radius used for the computation.
    pub(crate) fn minkowski_bound(&self) -> &Int {
        &self.minkowski_bound
    }
}

/// Compute the class group of a small-degree maximal order.
pub(crate) fn class_group(order: &Order) -> Option<ClassGroup> {
    let bound = minkowski_bound(order)?;
    let bound_value = bound.to_i64()?;
    if bound_value <= 0 {
        return None;
    }
    let base = prime_ideals_up_to(order, bound_value)?;
    if base.is_empty() {
        return Some(ClassGroup {
            invariants: Vec::new(),
            class_number: Int::one(),
            minkowski_bound: bound,
        });
    }
    let size = order.field().degree();
    if size > 3 {
        return None;
    }
    let ideals = enumerate_ideals(order, &base, bound_value)?;
    let count = base.len();
    let mut relations = Vec::new();
    for ideal in &ideals {
        if ideal.norm().is_one() {
            continue;
        }
        let mut left = vec![Int::zero(); count];
        for (factor, exponent) in factor_ideal(order, ideal)? {
            let position = base
                .iter()
                .position(|candidate| candidate.ideal().equals(factor.ideal()))?;
            left[position] = left[position].add(&Int::from_i64(exponent as i64));
        }
        let target = bound.mul(ideal.norm());
        let alpha = short_element(order, ideal, &target)?;
        let principal = principal_ideal(order, &alpha)?;
        let quotient = FractionalIdeal::from_integral(principal.clone())?
            .divide(order, &FractionalIdeal::from_integral(ideal.clone())?)?
            .to_integral()?;
        let mut row = left;
        for (factor, exponent) in factor_ideal(order, &quotient)? {
            let position = base
                .iter()
                .position(|candidate| candidate.ideal().equals(factor.ideal()))?;
            row[position] = row[position].add(&Int::from_i64(exponent as i64));
        }
        let product = ideal.multiply(order, &quotient)?;
        if !product.equals(&principal) {
            return None;
        }
        relations.push(row);
    }
    let matrix = super::super::intmatrix::IntMatrix::from_rows(relations)?;
    let invariants = smith_invariants(&matrix)?;
    if invariants.len() != count {
        return None;
    }
    let mut class_number = Int::one();
    let mut torsion = Vec::new();
    for value in invariants {
        class_number = class_number.mul(&value);
        if !value.is_one() {
            torsion.push(value);
        }
    }
    Some(ClassGroup {
        invariants: torsion,
        class_number,
        minkowski_bound: bound,
    })
}

fn enumerate_ideals(order: &Order, base: &[PrimeIdeal], bound: i64) -> Option<Vec<Ideal>> {
    let mut ideals: Vec<Ideal> = Vec::new();
    let mut queue = vec![Ideal::identity(order)?];
    while let Some(current) = queue.pop() {
        for prime in base {
            let next = current.multiply(order, prime.ideal())?;
            if next.norm().to_i64()? > bound {
                continue;
            }
            if ideals.iter().any(|existing| existing.equals(&next)) {
                continue;
            }
            if ideals.len() > 512 {
                return None;
            }
            ideals.push(next.clone());
            queue.push(next);
        }
    }
    Some(ideals)
}

fn minkowski_bound(order: &Order) -> Option<Int> {
    let field = order.field();
    let degree = field.degree();
    let (_, complex_pairs) = field.signature();
    let discriminant = order.discriminant().abs().to_i64()? as f64;
    let mut factorial = 1.0f64;
    for value in 1..=degree {
        factorial *= value as f64;
    }
    let degree_float = degree as f64;
    let mut bound = (4.0 / core::f64::consts::PI).powi(complex_pairs as i32) * factorial
        / degree_float.powi(degree as i32)
        * discriminant.sqrt();
    bound *= 1.05;
    if !bound.is_finite() || bound <= 0.0 {
        return None;
    }
    Some(Int::from_i64(bound.ceil() as i64))
}

fn prime_ideals_up_to(order: &Order, bound: i64) -> Option<Vec<PrimeIdeal>> {
    if bound > 5000 {
        return None;
    }
    let mut ideals = Vec::new();
    for value in 2..=bound {
        if !is_small_prime(value) {
            continue;
        }
        let prime = Int::from_i64(value);
        for ideal in decompose_prime_general(order, &prime)? {
            if ideal.norm().to_i64()? <= bound {
                ideals.push(ideal);
            }
        }
    }
    Some(ideals)
}

fn is_small_prime(value: i64) -> bool {
    if value < 2 {
        return false;
    }
    let mut divisor = 2i64;
    while divisor * divisor <= value {
        if value % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

fn short_element(order: &Order, ideal: &Ideal, target: &Int) -> Option<Vec<Int>> {
    let size = order.field().degree();
    let basis = ideal.basis();
    let ideal_norm = ideal.norm().clone();
    let mut best: Option<(Int, Vec<Int>)> = None;
    for radius in 1i64..=16 {
        let total = (2 * radius + 1).pow(size as u32);
        if total > 2_000_000 {
            break;
        }
        for code in 0..total {
            let mut code = code;
            let mut coefficients = Vec::with_capacity(size);
            let mut nonzero = false;
            for _ in 0..size {
                let digit = code % (2 * radius + 1);
                code /= 2 * radius + 1;
                let value = digit - radius;
                if value != 0 {
                    nonzero = true;
                }
                coefficients.push(Int::from_i64(value));
            }
            if !nonzero {
                continue;
            }
            let mut coordinates = vec![Int::zero(); size];
            for (column, coefficient) in basis.iter().zip(&coefficients) {
                if coefficient.is_zero() {
                    continue;
                }
                for (index, value) in column.iter().enumerate() {
                    coordinates[index] = coordinates[index].add(&coefficient.mul(value));
                }
            }
            let element = order.element_from_coordinates(&coordinates);
            let norm = order.field().norm(&element)?.abs();
            if norm.is_zero() || !norm.den().is_one() {
                continue;
            }
            let norm = norm.num();
            if norm.cmp(target) == core::cmp::Ordering::Greater {
                continue;
            }
            let replace = match &best {
                None => true,
                Some((current, _)) => norm.cmp(current) == core::cmp::Ordering::Less,
            };
            if replace {
                best = Some((norm, coordinates));
            }
        }
        if let Some((norm, _)) = &best
            && norm <= &ideal_norm
        {
            break;
        }
    }
    best.map(|(_, coordinates)| coordinates)
}

fn principal_ideal(order: &Order, coordinates: &[Int]) -> Option<Ideal> {
    let size = order.field().degree();
    let mut generators = Vec::with_capacity(size);
    for index in 0..size {
        let mut unit = vec![Int::zero(); size];
        unit[index] = Int::one();
        generators.push(order.multiply_coordinates(coordinates, &unit));
    }
    Ideal::from_generators(&generators, size)
}

#[cfg(test)]
mod tests {
    use super::super::field::NumberField;
    use super::super::integral_basis::maximal_order;
    use super::*;
    use crate::algebra::poly_z::ZPoly;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| Int::from_i64(*value))
                .collect(),
        )
    }

    fn field(coefficients: &[i64]) -> NumberField {
        NumberField::new(poly(coefficients)).unwrap()
    }

    fn class_number(coefficients: &[i64]) -> (i64, Vec<i64>) {
        let field = field(coefficients);
        let order = maximal_order(&field).unwrap();
        let group = class_group(&order).unwrap();
        (
            group.class_number().to_i64().unwrap(),
            group
                .invariants()
                .iter()
                .map(|value| value.to_i64().unwrap())
                .collect(),
        )
    }

    #[test]
    fn imaginary_quadratic_class_numbers() {
        assert_eq!(class_number(&[1, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[5, 0, 1]), (2, vec![2]));
        assert_eq!(class_number(&[23, 0, 1]), (3, vec![3]));
    }

    #[test]
    fn more_imaginary_quadratic_class_numbers() {
        assert_eq!(class_number(&[7, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[11, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[19, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[6, 0, 1]), (2, vec![2]));
        assert_eq!(class_number(&[10, 0, 1]), (2, vec![2]));
    }

    #[test]
    fn real_quadratic_class_numbers() {
        assert_eq!(class_number(&[-2, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[-5, 0, 1]), (1, Vec::new()));
        assert_eq!(class_number(&[-13, 0, 1]), (1, Vec::new()));
    }

    #[test]
    fn cyclotomic_fifth_field_has_trivial_class_group() {
        assert_eq!(class_number(&[1, 1, 1, 1, 1]), (1, Vec::new()));
    }

    #[test]
    fn minkowski_bound_is_reported() {
        let field = field(&[5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let group = class_group(&order).unwrap();
        assert!(*group.minkowski_bound() >= Int::from_i64(2));
        assert_eq!(group.class_number(), &Int::from_i64(2));
    }
}
