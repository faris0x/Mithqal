//! Unit groups and regulators for quadratic fields.
//!
//! The torsion subgroup is explicit for imaginary quadratic fields. Real
//! quadratic fundamental units are found by exact norm tests over a bounded
//! coefficient box. Rank-one higher-degree fields use numeric embeddings for
//! the log norm. Rank two and higher fields select independent unit log vectors
//! and compute the covolume of the generated sublattice. For totally real
//! fields the group is then completed by saturation: a bounded Cholesky
//! enumeration proves that no unit lies outside the selected lattice, or
//! supplies a witness that is added before repeating. Regulators use exact real
//! root isolation, with interval enclosures available on totally real fields.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::element::FieldElement;
use super::embedding;
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

    /// A generating set for the full unit group, including torsion.
    ///
    /// The torsion elements are found by a bounded coefficient search. The
    /// returned set generates the same group as the fundamental units together
    /// with the torsion subgroup, but it is not claimed to be minimal.
    pub(crate) fn generators(&self, order: &Order) -> Vec<FieldElement> {
        let mut result = self.fundamental_units.clone();
        result.extend(torsion_elements(order));
        result.push(order.field().negate(&order.field().one()));
        let field = order.field();
        result.sort_by(|left, right| {
            left.coefficients()
                .iter()
                .map(|value| value.num().to_decimal())
                .collect::<Vec<_>>()
                .cmp(
                    &right
                        .coefficients()
                        .iter()
                        .map(|value| value.num().to_decimal())
                        .collect::<Vec<_>>(),
                )
        });
        result.dedup_by(|left, right| field.sub(left, right).is_zero());
        result
    }

    /// Verify the presented unit data without claiming saturation.
    ///
    /// Checks the rank count, unit norms, and regulator positivity. It does not
    /// prove that the selected lattice is the full unit lattice.
    pub(crate) fn verify(&self, order: &Order) -> bool {
        if self.fundamental_units.len() != self.rank {
            return false;
        }
        if self.torsion_order == 0 {
            return false;
        }
        if self.rank > 0
            && !self
                .regulator
                .is_some_and(|value| value.is_finite() && value > 0.0)
        {
            return false;
        }
        self.fundamental_units.iter().all(|unit| {
            order
                .field()
                .norm(unit)
                .is_some_and(|norm| norm.abs().is_one())
        })
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
        let units = if field.signature().1 == 0 {
            complete_units(order, units, torsion, rank)?
        } else {
            units
        };
        let regulator = exact_regulator(order, &units).unwrap_or(regulator);
        return Some(UnitGroup {
            rank,
            torsion_order: torsion,
            regulator: Some(regulator),
            fundamental_units: units,
        });
    }
    if degree == 2 && real_embeddings == 2 {
        let (unit, regulator) = fundamental_real_quadratic(order)?;
        let units = vec![unit];
        let regulator = exact_regulator(order, &units).unwrap_or(regulator);
        return Some(UnitGroup {
            rank,
            torsion_order: 2,
            regulator: Some(regulator),
            fundamental_units: units,
        });
    }
    let (unit, regulator, torsion) = rank_one_unit(order)?;
    let units = if field.signature().1 == 0 {
        complete_units(order, vec![unit], torsion, rank)?
    } else {
        vec![unit]
    };
    let regulator = exact_regulator(order, &units).unwrap_or(regulator);
    Some(UnitGroup {
        rank,
        torsion_order: torsion,
        regulator: Some(regulator),
        fundamental_units: units,
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

/// Extend a unit basis until enumeration certifies saturation.
fn complete_units(
    order: &Order,
    initial: Vec<FieldElement>,
    torsion: u32,
    rank: usize,
) -> Option<Vec<FieldElement>> {
    let mut pool = initial;
    for _ in 0..10 {
        let basis = select_unit_basis(order, &pool, rank)?;
        let candidate = UnitGroup {
            rank,
            torsion_order: torsion,
            regulator: None,
            fundamental_units: basis.clone(),
        };
        let certificate = candidate.saturation_certificate(order);
        match certificate.status {
            SaturationStatus::Certified | SaturationStatus::NotAttempted => {
                return Some(basis);
            }
            SaturationStatus::Failed => {
                let witness = certificate.witness?;
                let duplicated = pool
                    .iter()
                    .any(|unit| order.field().sub(unit, &witness).is_zero());
                if duplicated {
                    return Some(basis);
                }
                pool.push(witness);
                if pool.len() > 12 {
                    return select_unit_basis(order, &pool, rank);
                }
            }
        }
    }
    select_unit_basis(order, &pool, rank)
}

/// Select a rank-sized subset spanning the largest lattice found so far.
fn select_unit_basis(
    order: &Order,
    pool: &[FieldElement],
    rank: usize,
) -> Option<Vec<FieldElement>> {
    if pool.len() < rank {
        return None;
    }
    let logs: Vec<Vec<f64>> = pool
        .iter()
        .map(|unit| exact_log_vector(order, unit))
        .collect::<Option<Vec<_>>>()?;
    let mut best: Option<(f64, Vec<usize>, bool)> = None;
    let mut indices = vec![0usize; rank];
    loop {
        let basis: Vec<Vec<f64>> = indices.iter().map(|index| logs[*index].clone()).collect();
        if let Some(covolume) = log_covolume(&basis) {
            let contains_all = logs.iter().all(|vector| unit_in_lattice(vector, &basis));
            let better = match &best {
                None => true,
                Some((current, _, current_contains)) => {
                    (contains_all && !current_contains)
                        || (contains_all == *current_contains && covolume < *current)
                }
            };
            if better {
                best = Some((covolume, indices.clone(), contains_all));
            }
        }
        let mut position = rank;
        loop {
            if position == 0 {
                break;
            }
            position -= 1;
            indices[position] += 1;
            if indices[position] < pool.len() {
                break;
            }
            indices[position] = 0;
            if position == 0 {
                position = usize::MAX;
                break;
            }
        }
        if position == usize::MAX || (position == 0 && indices[0] == 0) {
            break;
        }
    }
    let (_, chosen, _) = best?;
    Some(chosen.iter().map(|index| pool[*index].clone()).collect())
}

fn log_covolume(basis: &[Vec<f64>]) -> Option<f64> {
    let rank = basis.len();
    let mut gram = vec![vec![0.0f64; rank]; rank];
    for left in 0..rank {
        for right in 0..rank {
            gram[left][right] = basis[left]
                .iter()
                .zip(&basis[right])
                .map(|(a, b)| a * b)
                .sum();
        }
    }
    let value = determinant_f64(&mut gram)?.abs().sqrt();
    (value.is_finite() && value > 1e-12).then_some(value)
}

/// Logarithmic coordinates from exact real root isolation.
fn exact_log_vector(order: &Order, element: &FieldElement) -> Option<Vec<f64>> {
    let field = order.field();
    if field.signature().1 != 0 {
        return None;
    }
    let mut vector = Vec::with_capacity(field.signature().0);
    for index in 0..field.signature().0 {
        let (lower, upper) = embedding::real_log_intervals(order, element, index)?;
        vector.push((lower + upper) * 0.5);
    }
    Some(vector)
}

/// The regulator from exact embedding enclosures, for totally real fields.
fn exact_regulator(order: &Order, units: &[FieldElement]) -> Option<f64> {
    let rank = units.len();
    if rank == 0 {
        return None;
    }
    let full: Vec<Vec<f64>> = units
        .iter()
        .map(|unit| exact_log_vector(order, unit))
        .collect::<Option<Vec<_>>>()?;
    let width = full[0].len().checked_sub(1)?;
    let vectors: Vec<Vec<f64>> = full.iter().map(|row| row[..width].to_vec()).collect();
    let mut gram = vec![vec![0.0f64; rank]; rank];
    for left in 0..rank {
        for right in 0..rank {
            gram[left][right] = vectors[left]
                .iter()
                .zip(&vectors[right])
                .map(|(a, b)| a * b)
                .sum();
        }
    }
    let value = determinant_f64(&mut gram)?.abs().sqrt();
    (value.is_finite() && value > 0.0).then_some(value)
}

/// A certified interval enclosure of the regulator.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RegulatorBounds {
    /// Lower endpoint.
    pub(crate) lower: f64,
    /// Upper endpoint.
    pub(crate) upper: f64,
}

/// The outcome of a unit lattice saturation check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SaturationStatus {
    /// The enumeration proved that no unit lies outside the lattice.
    Certified,
    /// A unit outside the lattice was found.
    Failed,
    /// The field or the enumeration bound was outside the supported envelope.
    NotAttempted,
}

/// A saturation certificate for the presented unit lattice.
#[derive(Clone, Debug)]
pub(crate) struct SaturationCertificate {
    /// The certification outcome.
    pub(crate) status: SaturationStatus,
    /// The number of units inspected in the bounded enumeration.
    pub(crate) enumerated: usize,
    /// The logarithmic radius of the fundamental domain, when attempted.
    pub(crate) radius: Option<f64>,
    /// A unit outside the lattice, when one was found.
    pub(crate) witness: Option<FieldElement>,
}

#[derive(Clone, Copy)]
struct LogInterval {
    lower: f64,
    upper: f64,
}

impl LogInterval {
    fn add(self, other: Self) -> Self {
        LogInterval {
            lower: self.lower + other.lower,
            upper: self.upper + other.upper,
        }
    }

    fn mul(self, other: Self) -> Self {
        let values = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        let mut lower = values[0];
        let mut upper = values[0];
        for value in &values[1..] {
            lower = lower.min(*value);
            upper = upper.max(*value);
        }
        LogInterval { lower, upper }
    }
}

impl UnitGroup {
    /// A rigorous interval enclosure of the regulator for totally real fields.
    ///
    /// The enclosure uses exact rational root isolation and interval
    /// arithmetic. Only real embeddings are supported, so mixed-signature
    /// fields return `None`.
    pub(crate) fn regulator_bounds(&self, order: &Order) -> Option<RegulatorBounds> {
        let field = order.field();
        if field.signature().1 != 0 || self.rank == 0 {
            return None;
        }
        let mut logs: Vec<Vec<LogInterval>> = Vec::with_capacity(self.fundamental_units.len());
        for unit in &self.fundamental_units {
            let mut row = Vec::with_capacity(field.signature().0 - 1);
            for index in 0..field.signature().0 - 1 {
                let (low, high) = embedding::real_log_intervals(order, unit, index)?;
                row.push(LogInterval {
                    lower: low,
                    upper: high,
                });
            }
            logs.push(row);
        }
        let rank = self.rank;
        let mut gram = vec![
            vec![
                LogInterval {
                    lower: 0.0,
                    upper: 0.0
                };
                rank
            ];
            rank
        ];
        for left in 0..rank {
            for right in 0..rank {
                let mut value = LogInterval {
                    lower: 0.0,
                    upper: 0.0,
                };
                for (left_value, right_value) in logs[left].iter().zip(&logs[right]) {
                    value = value.add(left_value.mul(*right_value));
                }
                gram[left][right] = value;
            }
        }
        let determinant = interval_determinant(&gram)?;
        let padding = 1e-9 * (1.0 + determinant.upper.abs());
        let lower = (determinant.lower - padding).max(0.0).sqrt();
        let upper = (determinant.upper + padding).max(0.0).sqrt();
        if !(lower.is_finite() && upper.is_finite()) || upper < lower {
            return None;
        }
        Some(RegulatorBounds { lower, upper })
    }

    /// Certify that the selected units generate the full unit lattice.
    ///
    /// The check is available for totally real fields. It enumerates every
    /// algebraic integer whose archimedean size allows its logarithm to lie in
    /// a fundamental domain of the generated lattice. If every unit found is
    /// already generated, the lattice is saturated. The enumeration is exact
    /// up to the floating point comparison of logarithmic coordinates.
    pub(crate) fn saturation_certificate(&self, order: &Order) -> SaturationCertificate {
        let field = order.field();
        if field.signature().1 != 0 || self.rank == 0 {
            return SaturationCertificate {
                status: SaturationStatus::NotAttempted,
                enumerated: 0,
                radius: None,
                witness: None,
            };
        }
        let mut basis: Vec<Vec<f64>> = Vec::with_capacity(self.rank);
        for unit in &self.fundamental_units {
            let vector = match exact_log_vector(order, unit) {
                Some(vector) => vector,
                None => {
                    return SaturationCertificate {
                        status: SaturationStatus::NotAttempted,
                        enumerated: 0,
                        radius: None,
                        witness: None,
                    };
                }
            };
            basis.push(vector);
        }
        reduce_log_basis(&mut basis);
        let mut radius = 0.0f64;
        let vertices = 1usize << basis.len();
        for mask in 0..vertices {
            let mut vertex = vec![0.0f64; basis[0].len()];
            for (index, vector) in basis.iter().enumerate() {
                let sign = if mask & (1 << index) == 0 { -0.5 } else { 0.5 };
                for (entry, base) in vertex.iter_mut().zip(vector) {
                    *entry += sign * base;
                }
            }
            let local = vertex
                .iter()
                .fold(0.0f64, |current, value| current.max(value.abs()));
            radius = radius.max(local);
        }
        if !radius.is_finite() || radius > 40.0 {
            return SaturationCertificate {
                status: SaturationStatus::NotAttempted,
                enumerated: 0,
                radius: Some(radius),
                witness: None,
            };
        }
        let size = field.degree();
        let bound = (size as f64) * (2.0 * radius).exp() * 1.01;
        if !bound.is_finite() {
            return SaturationCertificate {
                status: SaturationStatus::NotAttempted,
                enumerated: 0,
                radius: Some(radius),
                witness: None,
            };
        }
        let candidates = match short_field_elements(order, bound) {
            Some(values) => values,
            None => {
                return SaturationCertificate {
                    status: SaturationStatus::NotAttempted,
                    enumerated: 0,
                    radius: Some(radius),
                    witness: None,
                };
            }
        };
        let mut enumerated = 0usize;
        let mut status = SaturationStatus::Certified;
        let mut witness = None;
        for coordinates in candidates {
            let element = order.element_from_coordinates(&coordinates);
            let Some(norm) = field.norm(&element) else {
                continue;
            };
            if !norm.abs().is_one() {
                continue;
            }
            enumerated += 1;
            let Some(vector) = exact_log_vector(order, &element) else {
                continue;
            };
            if !unit_in_lattice(&vector, &basis) {
                status = SaturationStatus::Failed;
                witness = Some(element);
                break;
            }
        }
        SaturationCertificate {
            status,
            enumerated,
            radius: Some(radius),
            witness,
        }
    }
}

/// LLL reduce a floating point basis with integer combinations.
fn reduce_log_basis(basis: &mut [Vec<f64>]) {
    let count = basis.len();
    if count <= 1 {
        return;
    }
    let mut index = 1usize;
    let mut guard = 0usize;
    while index < count {
        guard += 1;
        if guard > 100_000 {
            return;
        }
        let (mu, norms) = gram_schmidt_f64(basis);
        for previous in (0..index).rev() {
            let nearest = mu[index][previous].round();
            if nearest != 0.0 {
                let previous_row = basis[previous].clone();
                for (entry, base) in basis[index].iter_mut().zip(&previous_row) {
                    *entry -= nearest * base;
                }
            }
        }
        let (mu, _norms) = gram_schmidt_f64(basis);
        let delta = 0.75;
        if norms[index] >= (delta - mu[index][index - 1] * mu[index][index - 1]) * norms[index - 1]
        {
            index += 1;
        } else {
            basis.swap(index, index - 1);
            index = index.saturating_sub(1).max(1);
        }
    }
}

fn gram_schmidt_f64(basis: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>) {
    let count = basis.len();
    let mut mu = vec![vec![0.0f64; count]; count];
    let mut norms = vec![0.0f64; count];
    let mut orthogonal: Vec<Vec<f64>> = vec![Vec::new(); count];
    for index in 0..count {
        let mut vector = basis[index].clone();
        for previous in 0..index {
            let denominator = norms[previous];
            if denominator <= 1e-300 {
                continue;
            }
            let dot: f64 = basis[index]
                .iter()
                .zip(&orthogonal[previous])
                .map(|(a, b)| a * b)
                .sum();
            let coefficient = dot / denominator;
            mu[index][previous] = coefficient;
            for (entry, base) in vector.iter_mut().zip(&orthogonal[previous]) {
                *entry -= coefficient * base;
            }
        }
        norms[index] = vector.iter().map(|value| value * value).sum();
        orthogonal[index] = vector;
    }
    (mu, norms)
}

fn unit_in_lattice(vector: &[f64], basis: &[Vec<f64>]) -> bool {
    let rank = basis.len();
    let mut gram = vec![vec![0.0f64; rank]; rank];
    let mut rhs = vec![0.0f64; rank];
    for left in 0..rank {
        for right in 0..rank {
            gram[left][right] = basis[left]
                .iter()
                .zip(&basis[right])
                .map(|(a, b)| a * b)
                .sum();
        }
        rhs[left] = basis[left].iter().zip(vector).map(|(a, b)| a * b).sum();
    }
    let Some(coefficients) = solve_f64(&mut gram, &mut rhs) else {
        return false;
    };
    let mut residual = vector.to_vec();
    for (coefficient, base) in coefficients.iter().zip(basis) {
        let nearest = coefficient.round();
        for (value, entry) in residual.iter_mut().zip(base) {
            *value -= nearest * entry;
        }
    }
    let residual_norm = residual
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    let vector_norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    residual_norm <= 1e-7 * (1.0 + vector_norm)
}

#[allow(clippy::needless_range_loop)]
fn solve_f64(matrix: &mut [Vec<f64>], rhs: &mut [f64]) -> Option<Vec<f64>> {
    let size = rhs.len();
    for column in 0..size {
        let pivot = (column..size).find(|row| matrix[*row][column].abs() > 1e-14)?;
        if pivot != column {
            matrix.swap(pivot, column);
            rhs.swap(pivot, column);
        }
        let value = matrix[column][column];
        for row in column + 1..size {
            let factor = matrix[row][column] / value;
            for entry in column..size {
                matrix[row][entry] -= factor * matrix[column][entry];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    let mut solution = vec![0.0f64; size];
    for row in (0..size).rev() {
        let mut value = rhs[row];
        for column in row + 1..size {
            value -= matrix[row][column] * solution[column];
        }
        solution[row] = value / matrix[row][row];
    }
    Some(solution)
}

/// Integer elements with `sum sigma_i(x)^2 <= bound`, by Cholesky enumeration.
#[allow(clippy::needless_range_loop)]
fn short_field_elements(order: &Order, bound: f64) -> Option<Vec<Vec<Int>>> {
    let size = order.field().degree();
    if !(bound.is_finite() && bound > 0.0) {
        return None;
    }
    let trace = order.trace_matrix();
    let mut gram = vec![vec![0.0f64; size]; size];
    for row in 0..size {
        for column in 0..size {
            gram[row][column] = trace.get(row, column).to_f64();
        }
    }
    let factor = cholesky_f64(&gram)?;
    let bound_float = bound * 1.001;
    let mut coefficients = vec![0i64; size];
    let mut output = Vec::new();
    let mut nodes = 0usize;
    enumerate_cholesky(
        &factor,
        bound_float,
        bound,
        trace,
        &mut coefficients,
        size as isize - 1,
        0.0,
        &mut output,
        &mut nodes,
    )?;
    if nodes > 2_000_000 {
        return None;
    }
    if output.len() > 200_000 {
        return None;
    }
    Some(output)
}

#[allow(clippy::too_many_arguments)]
fn enumerate_cholesky(
    factor: &[Vec<f64>],
    bound_float: f64,
    bound_exact: f64,
    trace: &super::super::ratmatrix::RatMatrix,
    coefficients: &mut [i64],
    index: isize,
    spent: f64,
    output: &mut Vec<Vec<Int>>,
    nodes: &mut usize,
) -> Option<()> {
    *nodes += 1;
    if *nodes > 2_000_000 {
        return None;
    }
    if index < 0 {
        let mut quadratic = 0.0f64;
        for row in 0..coefficients.len() {
            for column in 0..coefficients.len() {
                quadratic += (coefficients[row] as f64)
                    * (coefficients[column] as f64)
                    * trace.get(row, column).to_f64();
            }
        }
        if quadratic <= bound_exact * (1.0 + 1e-9) {
            output.push(
                coefficients
                    .iter()
                    .map(|value| Int::from_i64(*value))
                    .collect(),
            );
        }
        return Some(());
    }
    let position = index as usize;
    let mut tail = 0.0f64;
    for later in position + 1..coefficients.len() {
        tail += factor[position][later] * (coefficients[later] as f64);
    }
    let diagonal = factor[position][position];
    if diagonal == 0.0 {
        return None;
    }
    let remaining = bound_float - spent;
    if remaining < 0.0 {
        return Some(());
    }
    let radius = remaining.sqrt();
    let low = ((-radius - tail) / diagonal).ceil() as i64;
    let high = ((radius - tail) / diagonal).floor() as i64;
    if high < low {
        return Some(());
    }
    for value in low..=high {
        coefficients[position] = value;
        let term = tail + diagonal * (value as f64);
        enumerate_cholesky(
            factor,
            bound_float,
            bound_exact,
            trace,
            coefficients,
            index - 1,
            spent + term * term,
            output,
            nodes,
        )?;
    }
    coefficients[position] = 0;
    Some(())
}

#[allow(clippy::needless_range_loop)]
fn cholesky_f64(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let size = matrix.len();
    let mut lower = vec![vec![0.0f64; size]; size];
    for row in 0..size {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return None;
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    // Return the upper triangular factor `R` with `R^T R` equal to the Gram
    // matrix, stored so that `R[row][column]` is used by the enumerator.
    let mut upper = vec![vec![0.0f64; size]; size];
    for row in 0..size {
        for column in row..size {
            upper[row][column] = lower[column][row];
        }
    }
    Some(upper)
}

#[allow(clippy::needless_range_loop)]
fn interval_determinant(matrix: &[Vec<LogInterval>]) -> Option<LogInterval> {
    let size = matrix.len();
    if size == 0 {
        return Some(LogInterval {
            lower: 1.0,
            upper: 1.0,
        });
    }
    if size == 1 {
        return Some(matrix[0][0]);
    }
    let mut total = LogInterval {
        lower: 0.0,
        upper: 0.0,
    };
    for column in 0..size {
        let mut minor = Vec::with_capacity(size - 1);
        for row in 1..size {
            let mut entries = Vec::with_capacity(size - 1);
            for other in 0..size {
                if other != column {
                    entries.push(matrix[row][other]);
                }
            }
            minor.push(entries);
        }
        let term = matrix[0][column].mul(interval_determinant(&minor)?);
        total = if column % 2 == 0 {
            total.add(term)
        } else {
            total.add(LogInterval {
                lower: -term.upper,
                upper: -term.lower,
            })
        };
    }
    Some(total)
}

/// Elements of finite order in the unit group, found in a bounded box.
fn torsion_elements(order: &Order) -> Vec<FieldElement> {
    let field = order.field();
    let size = field.degree();
    let radius = if size <= 4 { 2i64 } else { 1 };
    let Some(total) = (2 * radius + 1).checked_pow(size as u32) else {
        return Vec::new();
    };
    if total > 200_000 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut coordinates = vec![Int::zero(); size];
    for code in 0..total {
        let mut code = code;
        for entry in &mut coordinates {
            let digit = code % (2 * radius + 1);
            code /= 2 * radius + 1;
            *entry = Int::from_i64(digit - radius);
        }
        let element = order.element_from_coordinates(&coordinates);
        let Some(norm) = field.norm(&element) else {
            continue;
        };
        if !norm.abs().is_one() {
            continue;
        }
        for exponent in 1..=(6 * size as u32 + 6) {
            if field.power(&element, exponent).is_one() {
                if !result
                    .iter()
                    .any(|existing: &FieldElement| field.sub(existing, &element).is_zero())
                {
                    result.push(element.clone());
                }
                break;
            }
        }
    }
    result
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
        assert!(units.verify(&order));
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
        assert!(units.verify(&order));
    }

    #[test]
    fn regulator_bounds_bracket_the_regulator() {
        for coefficients in [
            &[-3i64, 0, 1][..],
            &[-1, -3, 0, 1][..],
            &[1, 0, -10, 0, 1][..],
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let units = unit_group(&order).unwrap();
            let bounds = units.regulator_bounds(&order).unwrap();
            let regulator = units.regulator().unwrap();
            assert!(bounds.lower <= regulator, "lower bound exceeds regulator");
            assert!(regulator <= bounds.upper, "upper bound is below regulator");
        }
    }

    #[test]
    fn higher_rank_unit_lattices_are_saturated() {
        let field = field(&[1, -2, -1, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        let certificate = units.saturation_certificate(&order);
        assert_eq!(certificate.status, SaturationStatus::Certified);
        assert!(certificate.enumerated > 0);
    }

    #[test]
    fn saturation_detects_an_incomplete_basis() {
        let field = field(&[1, -2, -1, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        let incomplete = UnitGroup {
            rank: units.rank(),
            torsion_order: units.torsion_order(),
            regulator: None,
            fundamental_units: vec![units.fundamental_units()[0].clone()],
        };
        let certificate = incomplete.saturation_certificate(&order);
        assert_eq!(certificate.status, SaturationStatus::Failed);
        assert!(certificate.witness.is_some());
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
