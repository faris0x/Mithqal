//! Integral ideals as lattices, the codifferent, and the different.
//!
//! The first layer represents an integral ideal by a column basis in the
//! maximal order coordinates. Norms are lattice indices. Ideal multiplication
//! and class groups remain for the next pass.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::super::dense::lcm;
use super::super::intmatrix::{IntMatrix, div_exact};
use super::super::lattice::{column_hermite_normal_form, smith_normal_form_with_column_transform};
use super::super::ratmatrix::{RatMatrix, rat_from_int};
use super::element::FieldElement;
use super::order::Order;

/// An integral ideal with a column basis in maximal order coordinates.
#[derive(Clone, Debug)]
pub(crate) struct Ideal {
    basis: Vec<Vec<Int>>,
    norm: Int,
}

impl Ideal {
    /// Build from a column basis in order coordinates.
    pub(crate) fn from_basis(basis: Vec<Vec<Int>>) -> Option<Self> {
        let size = basis.len();
        if size == 0 || basis.iter().any(|column| column.len() != size) {
            return None;
        }
        let matrix = IntMatrix::from_columns(basis.clone())?;
        let determinant = matrix.determinant()?.abs();
        if determinant.is_zero() {
            return None;
        }
        Some(Ideal {
            basis,
            norm: determinant,
        })
    }

    /// Build from generators in order coordinates by taking a lattice basis.
    pub(crate) fn from_generators(generators: &[Vec<Int>], size: usize) -> Option<Self> {
        let matrix = IntMatrix::from_columns(generators.to_vec())?;
        if matrix.row_count() != size {
            return None;
        }
        let form = column_hermite_normal_form(&matrix);
        if form.column_count() != size {
            return None;
        }
        Ideal::from_basis((0..size).map(|column| form.column(column)).collect())
    }

    /// Build from a rational power basis.
    pub(crate) fn from_power_basis(order: &Order, basis: &RatMatrix) -> Option<Self> {
        let coordinates = order.basis().inverse()?.multiply(basis)?;
        let mut columns = Vec::with_capacity(coordinates.column_count());
        for column in 0..coordinates.column_count() {
            let mut values = Vec::with_capacity(coordinates.row_count());
            for row in 0..coordinates.row_count() {
                let value = coordinates.get(row, column);
                if !value.den().is_one() {
                    return None;
                }
                values.push(value.num());
            }
            columns.push(values);
        }
        Ideal::from_basis(columns)
    }

    /// The column basis in order coordinates.
    pub(crate) fn basis(&self) -> &[Vec<Int>] {
        &self.basis
    }

    /// The ideal norm.
    pub(crate) fn norm(&self) -> &Int {
        &self.norm
    }

    /// True when the coordinates lie in the ideal.
    pub(crate) fn contains(&self, coordinates: &[Int]) -> bool {
        let matrix = match IntMatrix::from_columns(self.basis.clone()) {
            Some(matrix) => matrix,
            None => return false,
        };
        let rational = match RatMatrix::from_rows(
            matrix
                .to_rows()
                .iter()
                .map(|row| row.iter().map(rat_from_int).collect())
                .collect(),
        ) {
            Some(value) => value,
            None => return false,
        };
        let Some(inverse) = rational.inverse() else {
            return false;
        };
        let column =
            match RatMatrix::from_columns(vec![coordinates.iter().map(rat_from_int).collect()]) {
                Some(column) => column,
                None => return false,
            };
        let Some(solution) = inverse.multiply(&column) else {
            return false;
        };
        solution
            .to_rows()
            .iter()
            .flatten()
            .all(|value| value.den().is_one())
    }
}

/// The codifferent basis in the power basis.
pub(crate) fn codifferent(order: &Order) -> Option<RatMatrix> {
    let inverse = order.trace_matrix().inverse()?;
    order.basis().multiply(&inverse)
}

/// The different as an integral ideal.
pub(crate) fn different(order: &Order) -> Option<Ideal> {
    let codifferent = codifferent(order)?;
    let basis = inverse_ideal_lattice(order, &codifferent)?;
    let ideal = Ideal::from_power_basis(order, &basis)?;
    if ideal.norm() != &order.discriminant().abs() {
        return None;
    }
    Some(ideal)
}

/// The inverse fractional lattice `{ x : x I subset O }`.
pub(crate) fn inverse_ideal_lattice(order: &Order, basis: &RatMatrix) -> Option<RatMatrix> {
    let order_inverse = order.basis().inverse()?;
    let size = order.field().degree();
    let mut rows = Vec::with_capacity(size * size);
    for column in 0..basis.column_count() {
        let element = order.field().element(basis.column(column));
        let action = order.field().multiplication_matrix(&element);
        let condition = order_inverse.multiply(&action)?;
        rows.extend(condition.to_rows());
    }
    let conditions = RatMatrix::from_rows(rows)?;
    let result = preimage_lattice(&conditions)?;
    if result.column_count() != size || result.row_count() != size {
        return None;
    }
    Some(result)
}

/// The lattice of `x` with `conditions * x` integral.
pub(crate) fn preimage_lattice(conditions: &RatMatrix) -> Option<RatMatrix> {
    let rows = conditions.row_count();
    let columns = conditions.column_count();
    if rows == 0 || columns == 0 {
        return None;
    }
    let mut denominator = Int::one();
    for row in conditions.to_rows().iter() {
        for value in row {
            denominator = lcm(&denominator, &value.den());
        }
    }
    let mut integer_rows = Vec::with_capacity(rows);
    for row in conditions.to_rows() {
        let mut coefficients = Vec::with_capacity(columns);
        for value in row {
            let scale = div_exact(&denominator, &value.den());
            coefficients.push(value.num().mul(&scale));
        }
        integer_rows.push(coefficients);
    }
    let matrix = IntMatrix::from_rows(integer_rows)?;
    let (diagonal, transform) = smith_normal_form_with_column_transform(&matrix)?;
    let mut basis = Vec::with_capacity(columns);
    for index in 0..columns {
        let diagonal_value = diagonal.get(index, index);
        let scale = if diagonal_value.is_zero() {
            Rat::one()
        } else {
            Rat::from_ints(denominator.clone(), diagonal_value).ok()?
        };
        let column = transform.column(index);
        let mut rational = Vec::with_capacity(columns);
        for value in column {
            rational.push(rat_from_int(&value).mul(&scale));
        }
        basis.push(rational);
    }
    let result = RatMatrix::from_columns(basis)?;
    for column in 0..result.column_count() {
        let image = conditions.multiply(&RatMatrix::from_columns(vec![result.column(column)])?)?;
        if !image
            .to_rows()
            .iter()
            .flatten()
            .all(|value| value.den().is_one())
        {
            return None;
        }
    }
    Some(result)
}

/// Check the dual relation `Tr(different * codifferent) subset Z`.
pub(crate) fn different_is_dual(order: &Order, different: &Ideal, codifferent: &RatMatrix) -> bool {
    let size = order.field().degree();
    let mut different_element = Vec::with_capacity(size);
    for column in different.basis() {
        let mut power = vec![Rat::zero(); size];
        for (index, value) in column.iter().enumerate() {
            let basis = order.basis().column(index);
            for row in 0..size {
                power[row] = power[row].add(&rat_from_int(value).mul(&basis[row]));
            }
        }
        different_element.push(FieldElement {
            coefficients: power,
        });
    }
    for left in &different_element {
        for column in 0..codifferent.column_count() {
            let right = order.field().element(codifferent.column(column));
            let product = order.field().multiply(left, &right);
            let Some(trace) = order.field().trace(&product) else {
                return false;
            };
            if !trace.den().is_one() {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::field::NumberField;
    use super::super::integral_basis::maximal_order;
    use super::*;
    use crate::algebra::poly_z::ZPoly;
    use crate::num::{Int, Rat};

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

    fn rational(numerator: i64, denominator: i64) -> Rat {
        Rat::from_i64(numerator, denominator).unwrap()
    }

    #[test]
    fn preimage_lattice_computes_fractional_denominators() {
        let half = RatMatrix::from_rows(vec![
            vec![rational(1, 2), rational(0, 1)],
            vec![rational(0, 1), rational(1, 2)],
        ])
        .unwrap();
        let conditions = RatMatrix::from_rows(vec![
            vec![rational(2, 1), rational(0, 1)],
            vec![rational(0, 1), rational(2, 1)],
        ])
        .unwrap();
        let basis = preimage_lattice(&conditions).unwrap();
        assert_eq!(basis, half);
    }

    #[test]
    fn codifferent_trace_pairings_are_integral() {
        for coefficients in [&[-2i64, 0, 1][..], &[-5, 0, 1][..], &[-8, -2, -1, 1][..]] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let codiff = codifferent(&order).unwrap();
            let size = field.degree();
            for column in 0..size {
                let left = field.element(codiff.column(column));
                for basis_column in 0..size {
                    let right = field.element(order.basis().column(basis_column));
                    let trace = field.trace(&field.multiply(&left, &right)).unwrap();
                    assert!(trace.den().is_one());
                }
            }
        }
    }

    #[test]
    fn different_norm_is_the_absolute_discriminant() {
        for (coefficients, discriminant) in [
            (&[-2i64, 0, 1][..], 8i64),
            (&[-5, 0, 1][..], 5),
            (&[5, 0, 1][..], 20),
            (&[-8, -2, -1, 1][..], 503),
            (&[1, 1, 1, 1, 1][..], 125),
            (&[-2, 0, 0, 1][..], 108),
            (&[7, 0, 1][..], 7),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let codiff = codifferent(&order).unwrap();
            let diff = different(&order).unwrap();
            assert_eq!(diff.norm(), &Int::from_i64(discriminant));
            assert!(different_is_dual(&order, &diff, &codiff));
        }
    }

    #[test]
    fn ideal_from_generators_has_the_expected_norm() {
        let field = field(&[-5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let size = field.degree();
        // The ideal (2, 1 + sqrt(5)) has norm 2.
        let one_plus = field.add(
            &field.one(),
            &field.element(vec![rational(0, 1), rational(1, 1)]),
        );
        let mut generators = Vec::new();
        for index in 0..size {
            let mut column = vec![Int::zero(); size];
            column[index] = Int::from_i64(2);
            generators.push(column);
        }
        let coordinates = order
            .basis()
            .inverse()
            .unwrap()
            .multiply(&RatMatrix::from_columns(vec![one_plus.coefficients().to_vec()]).unwrap())
            .unwrap();
        generators.push(
            coordinates
                .to_rows()
                .iter()
                .map(|row| row[0].num())
                .collect(),
        );
        let ideal = Ideal::from_generators(&generators, size).unwrap();
        assert_eq!(ideal.norm(), &Int::from_i64(4));
        assert!(ideal.contains(&{
            let mut column = vec![Int::zero(); size];
            column[0] = Int::from_i64(2);
            column
        }));
    }
}
