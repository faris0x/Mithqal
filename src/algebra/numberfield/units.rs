//! Unit groups and regulators for quadratic fields.
//!
//! The torsion subgroup is explicit for imaginary quadratic fields. Real
//! quadratic fundamental units are found by exact norm tests over a bounded
//! coefficient box, and the regulator uses the two real embeddings. Higher
//! degree unit groups need the general log lattice and are not implemented yet.

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
    if degree > 2 {
        return None;
    }
    let (real_embeddings, complex_pairs) = field.signature();
    let rank = real_embeddings + complex_pairs - 1;
    if rank == 0 {
        return Some(UnitGroup {
            rank,
            torsion_order: torsion_order(field),
            regulator: None,
            fundamental_units: Vec::new(),
        });
    }
    if degree != 2 || real_embeddings != 2 {
        return None;
    }
    let (unit, regulator) = fundamental_real_quadratic(order)?;
    Some(UnitGroup {
        rank,
        torsion_order: 2,
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
    fn rational_field_has_only_sign_units() {
        let field = field(&[0, 1]);
        let order = maximal_order(&field).unwrap();
        let units = unit_group(&order).unwrap();
        assert_eq!(units.rank(), 0);
        assert_eq!(units.torsion_order(), 2);
    }
}
