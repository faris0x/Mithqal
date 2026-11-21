//! Unit groups and regulators for quadratic fields.
//!
//! The torsion subgroup is explicit for imaginary quadratic fields. Real
//! quadratic fundamental units are found by exact norm tests over a bounded
//! coefficient box. Rank-one higher-degree fields use numeric embeddings for
//! the log norm. Rank two and higher fields select independent unit log vectors
//! and compute the covolume of the generated sublattice. These higher-rank
//! regulators are experimental and not certified saturated.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::element::FieldElement;
use super::field::NumberField;
use super::order::Order;

/// A computed unit group.
#[derive(Clone, Debug)]
pub(crate) struct UnitGroup {
    rank: usize,
    torsion_order: u32,
    regulator: Option<f64>,
    fundamental_units: Vec<FieldElement>,
}

impl UnitGroup {
    /// The free rank `r1 + r2 - 1`.
    pub(crate) fn rank(&self) -> usize {
        self.rank
    }

    /// The order of the torsion subgroup.
    pub(crate) fn torsion_order(&self) -> u32 {
        self.torsion_order
    }

    /// The regulator, or `None` when the rank is zero.
    pub(crate) fn regulator(&self) -> Option<f64> {
        self.regulator
    }

    /// The fundamental units, one per free generator.
    pub(crate) fn fundamental_units(&self) -> &[FieldElement] {
        &self.fundamental_units
    }
}

/// Compute the unit group for a quadratic field.
pub(crate) fn unit_group(order: &Order) -> Option<UnitGroup> {
    let field = order.field();
    let degree = field.degree();
    let (real_embeddings, complex_pairs) = field.signature();
    let rank = real_embeddings + complex_pairs - 1;
    if rank == 0 {
        let torsion = if degree <= 2 {
            torsion_order(field)
        } else {
            general_torsion_order(order)?
        };
        return Some(UnitGroup {
            rank,
            torsion_order: torsion,
            regulator: None,
            fundamental_units: Vec::new(),
        });
    }
    if rank >= 2 {
        let (units, regulator, torsion) = rank_r_unit(order, rank)?;
        return Some(UnitGroup {
            rank,
            torsion_order: torsion,
            regulator: Some(regulator),
            fundamental_units: units,
        });
    }
    if degree == 2 && real_embeddings == 2 {
        let (unit, regulator) = fundamental_real_quadratic(order)?;
        return Some(UnitGroup {
            rank,
            torsion_order: 2,
            regulator: Some(regulator),
            fundamental_units: vec![unit],
        });
    }
    let (unit, regulator, torsion) = rank_one_unit(order)?;
    Some(UnitGroup {
        rank,
        torsion_order: torsion,
        regulator: Some(regulator),
        fundamental_units: vec![unit],
    })
}

fn torsion_order(field: &NumberField) -> u32 {
    if field.degree() == 2 {
        let coefficients = field.defining_polynomial().coeffs.clone();
        if coefficients.len() == 3 {
            let constant = coefficients[0].clone();
            let linear = coefficients[1].clone();
            if linear.is_zero() {
                if constant == Int::one() {
                    return 4;
                }
                if constant == Int::from_i64(3) {
                    return 6;
                }
            }
        }
    }
    2
}

fn fundamental_real_quadratic(order: &Order) -> Option<(FieldElement, f64)> {
    let field = order.field();
    let coefficients = field.defining_polynomial().coeffs.clone();
    if coefficients.len() != 3 {
        return None;
    }
    let linear = coefficients[1].to_i64()? as f64;
    let constant = coefficients[0].to_i64()? as f64;
    let discriminant = linear * linear - 4.0 * constant;
    if discriminant <= 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let embeddings = [(-linear + root) / 2.0, (-linear - root) / 2.0];
    let size = field.degree();
    let mut best: Option<(FieldElement, f64)> = None;
    let mut coordinates = vec![Int::zero(); size];
    for first in -20i64..=20 {
        coordinates[0] = Int::from_i64(first);
        for second in -20i64..=20 {
            if first == 0 && second == 0 {
                continue;
            }
            coordinates[1] = Int::from_i64(second);
            let element = order.element_from_coordinates(&coordinates);
            let norm = field.norm(&element)?.abs();
            if !norm.is_one() {
                continue;
            }
            let mut power = vec![Rat::zero(); size];
            for (index, value) in coordinates.iter().enumerate() {
                let basis = order.basis().column(index);
                for row in 0..size {
                    power[row] = power[row]
                        .add(&crate::algebra::ratmatrix::rat_from_int(value).mul(&basis[row]));
                }
            }
            let mut logarithm = 0.0f64;
            for embedding in embeddings {
                let mut value = 0.0f64;
                for coefficient in power.iter().rev() {
                    value = value * embedding + rat_to_f64(coefficient)?;
                }
                let absolute = value.abs();
                if absolute <= 0.0 {
                    return None;
                }
                logarithm = logarithm.max(absolute.ln().abs());
            }
            if logarithm <= 1e-12 {
                continue;
            }
            let replace = best
                .as_ref()
                .is_none_or(|(_, current)| logarithm < *current);
            if replace {
                best = Some((element, logarithm));
            }
        }
    }
    best
}

#[derive(Clone, Copy)]
struct Complex {
    real: f64,
    imaginary: f64,
}

impl Complex {
    fn new(real: f64, imaginary: f64) -> Self {
        Complex { real, imaginary }
    }

    fn add(self, other: Self) -> Self {
        Complex::new(self.real + other.real, self.imaginary + other.imaginary)
    }

    fn sub(self, other: Self) -> Self {
        Complex::new(self.real - other.real, self.imaginary - other.imaginary)
    }

    fn mul(self, other: Self) -> Self {
        Complex::new(
            self.real * other.real - self.imaginary * other.imaginary,
            self.real * other.imaginary + self.imaginary * other.real,
        )
    }

    fn div(self, other: Self) -> Option<Self> {
        let denominator = other.real * other.real + other.imaginary * other.imaginary;
        if denominator == 0.0 {
            return None;
        }
        Some(Complex::new(
            (self.real * other.real + self.imaginary * other.imaginary) / denominator,
            (self.imaginary * other.real - self.real * other.imaginary) / denominator,
        ))
    }

    fn abs(self) -> f64 {
        (self.real * self.real + self.imaginary * self.imaginary).sqrt()
    }
}

fn numeric_roots(field: &NumberField) -> Option<Vec<Complex>> {
    let degree = field.degree();
    let coefficients = field.defining_polynomial().to_rat_coeffs();
    let mut descending = Vec::with_capacity(coefficients.len());
    for value in coefficients.iter().rev() {
        descending.push(rat_to_f64(value)?);
    }
    let mut roots = Vec::with_capacity(degree);
    let seed = Complex::new(0.4, 0.9);
    let mut current = Complex::new(1.0, 0.0);
    for _ in 0..degree {
        roots.push(current);
        current = current.mul(seed);
    }
    for _ in 0..500 {
        let mut maximum = 0.0f64;
        for index in 0..degree {
            let numerator = evaluate_complex(&descending, roots[index]);
            let mut denominator = Complex::new(1.0, 0.0);
            for other in 0..degree {
                if other != index {
                    denominator = denominator.mul(roots[index].sub(roots[other]));
                }
            }
            let delta = numerator.div(denominator)?;
            roots[index] = roots[index].sub(delta);
            maximum = maximum.max(delta.abs());
        }
        if maximum < 1e-13 {
            break;
        }
    }
    let mut residual = 0.0f64;
    for root in &roots {
        let value = evaluate_complex(&descending, *root).abs();
        let scale = (1.0 + root.abs()).powi(degree as i32);
        residual = residual.max(value / scale);
    }
    if residual > 1e-6 {
        return None;
    }
    Some(roots)
}

fn evaluate_complex(coefficients: &[f64], value: Complex) -> Complex {
    let mut result = Complex::new(0.0, 0.0);
    for coefficient in coefficients {
        result = result.mul(value).add(Complex::new(*coefficient, 0.0));
    }
    result
}

fn log_norm(element: &FieldElement, roots: &[Complex]) -> Option<f64> {
    let mut maximum = 0.0f64;
    for root in roots {
        let value = evaluate_complex(
            &element
                .coefficients()
                .iter()
                .map(rat_to_f64)
                .collect::<Option<Vec<_>>>()?,
            *root,
        );
        let absolute = value.abs();
        if absolute <= 0.0 {
            return None;
        }
        maximum = maximum.max(absolute.ln().abs());
    }
    Some(maximum)
}

fn rank_one_unit(order: &Order) -> Option<(FieldElement, f64, u32)> {
    let field = order.field();
    let size = field.degree();
    let roots = numeric_roots(field)?;
    let radius = if size <= 4 { 8i64 } else { 5 };
    let total = (2 * radius + 1).checked_pow(size as u32)?;
    if total > 3_000_000 {
        return None;
    }
    let mut best: Option<(FieldElement, f64)> = None;
    let mut torsion = 2u32;
    let mut coordinates = vec![Int::zero(); size];
    for code in 0..total {
        let mut code = code;
        let mut nonzero = false;
        for entry in &mut coordinates {
            let digit = code % (2 * radius + 1);
            code /= 2 * radius + 1;
            let value = digit - radius;
            if value != 0 {
                nonzero = true;
            }
            *entry = Int::from_i64(value);
        }
        if !nonzero {
            continue;
        }
        let element = order.element_from_coordinates(&coordinates);
        let norm = field.norm(&element)?.abs();
        if !norm.is_one() {
            continue;
        }
        let logarithm = log_norm(&element, &roots)?;
        if logarithm < 1e-8 {
            for exponent in 1..=(6 * size as u32 + 6) {
                if field.power(&element, exponent).is_one() {
                    torsion = torsion.max(exponent);
                    break;
                }
            }
            continue;
        }
        let replace = best
            .as_ref()
            .is_none_or(|(_, current)| logarithm < *current);
        if replace {
            best = Some((element, logarithm));
        }
    }
    let (unit, regulator) = best?;
    Some((unit, regulator, torsion))
}

fn log_vector(element: &FieldElement, roots: &[Complex]) -> Option<Vec<f64>> {
    let coefficients = element
        .coefficients()
        .iter()
        .map(rat_to_f64)
        .collect::<Option<Vec<_>>>()?;
    let mut vector = Vec::with_capacity(roots.len());
    for root in roots {
        let value = evaluate_complex(&coefficients, *root);
        let absolute = value.abs();
        if absolute <= 0.0 {
            return None;
        }
        vector.push(absolute.ln());
    }
    Some(vector)
}

fn rank_r_unit(order: &Order, rank: usize) -> Option<(Vec<FieldElement>, f64, u32)> {
    let field = order.field();
    let size = field.degree();
    let roots = numeric_roots(field)?;
    let radius = if size <= 4 { 5i64 } else { 3 };
    let total = (2 * radius + 1).checked_pow(size as u32)?;
    if total > 3_000_000 {
        return None;
    }
    let mut torsion = 2u32;
    let mut units: Vec<(Vec<f64>, FieldElement)> = Vec::new();
    let mut coordinates = vec![Int::zero(); size];
    for code in 0..total {
        let mut code = code;
        let mut nonzero = false;
        for entry in &mut coordinates {
            let digit = code % (2 * radius + 1);
            code /= 2 * radius + 1;
            let value = digit - radius;
            if value != 0 {
                nonzero = true;
            }
            *entry = Int::from_i64(value);
        }
        if !nonzero {
            continue;
        }
        let element = order.element_from_coordinates(&coordinates);
        if !field.norm(&element)?.abs().is_one() {
            continue;
        }
        let vector = log_vector(&element, &roots)?;
        if vector.iter().all(|value| value.abs() < 1e-8) {
            for exponent in 1..=(6 * size as u32 + 6) {
                if field.power(&element, exponent).is_one() {
                    torsion = torsion.max(exponent);
                    break;
                }
            }
            continue;
        }
        units.push((vector, element));
    }
    let mut selected: Vec<Vec<f64>> = Vec::new();
    let mut fundamental = Vec::new();
    for (vector, element) in units {
        let mut residual = vector.clone();
        for basis in &selected {
            let mut dot = 0.0;
            let mut norm = 0.0;
            for (a, b) in residual.iter().zip(basis) {
                dot += a * b;
                norm += b * b;
            }
            if norm <= 1e-18 {
                continue;
            }
            let factor = dot / norm;
            for (value, base) in residual.iter_mut().zip(basis) {
                *value -= factor * base;
            }
        }
        let residual_norm = residual.iter().map(|value| value * value).sum::<f64>();
        if residual_norm > 1e-10 {
            selected.push(vector);
            fundamental.push(element);
            if selected.len() == rank {
                break;
            }
        }
    }
    if selected.len() != rank {
        return None;
    }
    let mut gram = vec![vec![0.0f64; rank]; rank];
    for i in 0..rank {
        for j in 0..rank {
            gram[i][j] = selected[i]
                .iter()
                .zip(&selected[j])
                .map(|(a, b)| a * b)
                .sum();
        }
    }
    let regulator = determinant_f64(&mut gram)?.abs().sqrt();
    if regulator <= 0.0 {
        return None;
    }
    Some((fundamental, regulator, torsion))
}

#[allow(clippy::needless_range_loop)]
fn determinant_f64(matrix: &mut [Vec<f64>]) -> Option<f64> {
    let size = matrix.len();
    let mut determinant = 1.0f64;
    for column in 0..size {
        let pivot = (column..size).find(|row| matrix[*row][column].abs() > 1e-14)?;
        if pivot != column {
            matrix.swap(pivot, column);
            determinant = -determinant;
        }
        let value = matrix[column][column];
        determinant *= value;
        for row in column + 1..size {
            let factor = matrix[row][column] / value;
            for entry in column..size {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
        }
    }
    Some(determinant)
}

fn general_torsion_order(order: &Order) -> Option<u32> {
    let field = order.field();
    let size = field.degree();
    let radius = 3i64;
    let total = (2 * radius + 1).checked_pow(size as u32)?;
    let mut torsion = 1u32;
    let mut coordinates = vec![Int::zero(); size];
    for code in 0..total {
        let mut code = code;
        for entry in &mut coordinates {
            let digit = code % (2 * radius + 1);
            code /= 2 * radius + 1;
            *entry = Int::from_i64(digit - radius);
        }
        let element = order.element_from_coordinates(&coordinates);
        if !field.norm(&element)?.abs().is_one() {
            continue;
        }
        for exponent in 1..=(6 * size as u32 + 6) {
            if field.power(&element, exponent).is_one() {
                torsion = torsion.max(exponent);
                break;
            }
        }
    }
    Some(torsion)
}

fn rat_to_f64(value: &Rat) -> Option<f64> {
    let numerator = value.num().to_i64()? as f64;
    let denominator = value.den().to_i64()? as f64;
    Some(numerator / denominator)
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn imaginary_quadratic_torsion_orders() {
        for (coefficients, torsion) in [
            (&[1i64, 0, 1][..], 4u32),
            (&[3, 0, 1][..], 6),
            (&[5, 0, 1][..], 2),
            (&[23, 0, 1][..], 2),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let units = unit_group(&order).unwrap();
            assert_eq!(units.rank(), 0);
            assert_eq!(units.torsion_order(), torsion);
            assert!(units.regulator().is_none());
            assert!(units.fundamental_units().is_empty());
        }
    }

    #[test]
    fn real_quadratic_regulators() {
        for (coefficients, expected) in [
            (&[-2i64, 0, 1][..], 0.8813735870),
            (&[-5, 0, 1][..], 0.4812118251),
            (&[-13, 0, 1][..], 1.1947632173),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let units = unit_group(&order).unwrap();
            assert_eq!(units.rank(), 1);
            assert_eq!(units.torsion_order(), 2);
            let regulator = units.regulator().unwrap();
            assert!((regulator - expected).abs() < 1e-6, "{regulator}");
            assert_eq!(units.fundamental_units().len(), 1);
            let unit = &units.fundamental_units()[0];
            assert!(field.norm(unit).unwrap().abs().is_one());
        }
    }

    #[test]
    fn rank_one_higher_degree_units() {
        for (coefficients, torsion) in [
            (&[1i64, 1, 1, 1, 1][..], 10u32),
            (&[1, 0, 0, 0, 1][..], 8),
            (&[-2, 0, 0, 1][..], 2),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let units = unit_group(&order).unwrap();
            assert_eq!(units.rank(), 1);
            assert_eq!(units.torsion_order(), torsion);
            let regulator = units.regulator().unwrap();
            assert!(regulator > 0.0);
            assert_eq!(units.fundamental_units().len(), 1);
            assert!(
                field
                    .norm(&units.fundamental_units()[0])
                    .unwrap()
                    .abs()
                    .is_one()
            );
        }
    }

    #[test]
    fn real_cubic_rank_two_units() {
        let field = field(&[-1, -3, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        assert_eq!(units.rank(), 2);
        assert_eq!(units.torsion_order(), 2);
        assert!(units.regulator().unwrap() > 0.0);
        assert_eq!(units.fundamental_units().len(), 2);
        for unit in units.fundamental_units() {
            assert!(field.norm(unit).unwrap().abs().is_one());
        }
    }

    #[test]
    fn real_quartic_rank_three_units() {
        let field = field(&[1, 0, -10, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        assert_eq!(units.rank(), 3);
        assert_eq!(units.torsion_order(), 2);
        assert!(units.regulator().unwrap() > 0.0);
        assert_eq!(units.fundamental_units().len(), 3);
        for unit in units.fundamental_units() {
            assert!(field.norm(unit).unwrap().abs().is_one());
        }
    }

    #[test]
    fn rational_field_has_only_sign_units() {
        let field = field(&[0, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        assert_eq!(units.rank(), 0);
        assert_eq!(units.torsion_order(), 2);
    }
}
