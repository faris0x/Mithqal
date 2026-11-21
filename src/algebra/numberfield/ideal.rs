//! Integral ideals as lattices, the codifferent, and the different.
//!
//! Integral ideals use a column basis in maximal order coordinates, with norms
//! as lattice indices. Fractional ideals add a denominator. The layer covers
//! multiplication, inversion, division, powers, factorization valuation, the
//! codifferent, and the different. Class groups and units remain for the next
//! pass.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::super::dense::lcm;
use super::super::intmatrix::{IntMatrix, div_exact};
use super::super::lattice::{
    column_hermite_normal_form, generator_lattice_basis, smith_normal_form_with_column_transform,
};
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
        let form = generator_lattice_basis(&matrix)?;
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
        self.contains_all(core::slice::from_ref(&coordinates.to_vec()))
    }

    /// True when every listed coordinate vector lies in the ideal.
    pub(crate) fn contains_all(&self, coordinates: &[Vec<Int>]) -> bool {
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
        for coordinates in coordinates {
            let column =
                match RatMatrix::from_columns(vec![coordinates.iter().map(rat_from_int).collect()])
                {
                    Some(column) => column,
                    None => return false,
                };
            let Some(solution) = inverse.multiply(&column) else {
                return false;
            };
            if !solution
                .to_rows()
                .iter()
                .flatten()
                .all(|value| value.den().is_one())
            {
                return false;
            }
        }
        true
    }
}

impl Ideal {
    /// The unit ideal.
    pub(crate) fn identity(order: &Order) -> Option<Self> {
        let size = order.field().degree();
        let mut basis = Vec::with_capacity(size);
        for index in 0..size {
            let mut column = vec![Int::zero(); size];
            column[index] = Int::one();
            basis.push(column);
        }
        Ideal::from_basis(basis)
    }

    /// The product of two integral ideals.
    pub(crate) fn multiply(&self, order: &Order, other: &Ideal) -> Option<Self> {
        let size = order.field().degree();
        let mut generators = Vec::with_capacity(size * size);
        for left in &self.basis {
            for right in &other.basis {
                generators.push(order.multiply_coordinates(left, right));
            }
        }
        let product = Ideal::from_generators(&generators, size)?;
        if product.norm() != &self.norm.mul(&other.norm) {
            return None;
        }
        Some(product)
    }
    /// A nonnegative power.
    pub(crate) fn power(&self, order: &Order, exponent: u32) -> Option<Self> {
        let mut result = Ideal::identity(order)?;
        let mut factor = self.clone();
        let mut exponent = exponent;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result.multiply(order, &factor)?;
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = factor.multiply(order, &factor)?;
            }
        }
        Some(result)
    }

    /// True when every element of this ideal lies in `other`.
    pub(crate) fn is_subset_of(&self, other: &Ideal) -> bool {
        self.basis.iter().all(|column| other.contains(column))
    }

    /// True when the two ideals have the same lattice.
    pub(crate) fn equals(&self, other: &Ideal) -> bool {
        let Ok(left) = IntMatrix::from_columns(self.basis.clone()).ok_or(()) else {
            return false;
        };
        let Ok(right) = IntMatrix::from_columns(other.basis.clone()).ok_or(()) else {
            return false;
        };
        column_hermite_normal_form(&left) == column_hermite_normal_form(&right)
    }
}

/// A fractional ideal `(1/d) I`.
#[derive(Clone, Debug)]
pub(crate) struct FractionalIdeal {
    basis: Vec<Vec<Int>>,
    denominator: Int,
    norm: Rat,
}

impl FractionalIdeal {
    /// Build from a canonical numerator lattice and positive denominator.
    pub(crate) fn from_numerator(basis: Vec<Vec<Int>>, denominator: Int) -> Option<Self> {
        let size = basis.len();
        if size == 0 || basis.iter().any(|column| column.len() != size) {
            return None;
        }
        if denominator.sign() <= 0 {
            return None;
        }
        let matrix = IntMatrix::from_columns(basis)?;
        let form = column_hermite_normal_form(&matrix);
        if form.column_count() != size {
            return None;
        }
        let mut basis: Vec<Vec<Int>> = (0..size).map(|column| form.column(column)).collect();
        let determinant = form.determinant()?.abs();
        if determinant.is_zero() {
            return None;
        }
        let mut content = Int::zero();
        for column in &basis {
            for value in column {
                content = content.gcd(&value.abs());
            }
        }
        let common = content.gcd(&denominator);
        let divisor = if common.is_zero() { Int::one() } else { common };
        let denominator = div_exact(&denominator, &divisor);
        if !divisor.is_one() {
            for column in &mut basis {
                for value in column {
                    *value = div_exact(value, &divisor);
                }
            }
        }
        let determinant = IntMatrix::from_columns(basis.clone())?.determinant()?.abs();
        let norm = Rat::from_ints(determinant, denominator.pow_u32(size as u32)).ok()?;
        Some(FractionalIdeal {
            basis,
            denominator,
            norm,
        })
    }

    /// The unit fractional ideal.
    pub(crate) fn identity(order: &Order) -> Option<Self> {
        FractionalIdeal::from_integral(Ideal::identity(order)?)
    }

    /// Promote an integral ideal.
    pub(crate) fn from_integral(ideal: Ideal) -> Option<Self> {
        FractionalIdeal::from_numerator(ideal.basis, Int::one())
    }

    /// Build from a rational power basis.
    pub(crate) fn from_power_basis(order: &Order, basis: &RatMatrix) -> Option<Self> {
        let coordinates = order.basis().inverse()?.multiply(basis)?;
        let size = coordinates.row_count();
        let mut denominator = Int::one();
        for row in coordinates.to_rows().iter() {
            for value in row {
                denominator = lcm(&denominator, &value.den());
            }
        }
        let mut columns = Vec::with_capacity(coordinates.column_count());
        for column in 0..coordinates.column_count() {
            let mut values = Vec::with_capacity(size);
            for row in 0..size {
                let value = coordinates.get(row, column);
                let scale = div_exact(&denominator, &value.den());
                values.push(value.num().mul(&scale));
            }
            columns.push(values);
        }
        FractionalIdeal::from_numerator(columns, denominator)
    }

    /// The numerator basis in order coordinates.
    pub(crate) fn basis(&self) -> &[Vec<Int>] {
        &self.basis
    }

    /// The denominator.
    pub(crate) fn denominator(&self) -> &Int {
        &self.denominator
    }

    /// The fractional ideal norm.
    pub(crate) fn norm(&self) -> &Rat {
        &self.norm
    }

    /// True when the fractional ideal is integral.
    pub(crate) fn is_integral(&self) -> bool {
        self.basis.iter().flatten().all(|value| {
            value
                .divrem(&self.denominator)
                .map(|(_, remainder)| remainder.is_zero())
                .unwrap_or(false)
        })
    }

    /// Convert to an integral ideal when possible.
    pub(crate) fn to_integral(&self) -> Option<Ideal> {
        if !self.is_integral() {
            return None;
        }
        let mut columns = Vec::with_capacity(self.basis.len());
        for column in &self.basis {
            let mut values = Vec::with_capacity(column.len());
            for value in column {
                values.push(div_exact(value, &self.denominator));
            }
            columns.push(values);
        }
        Ideal::from_basis(columns)
    }

    /// The power basis of the fractional ideal.
    pub(crate) fn power_basis(&self, order: &Order) -> Option<RatMatrix> {
        let size = order.field().degree();
        let numerator = RatMatrix::from_columns(
            self.basis
                .iter()
                .map(|column| column.iter().map(rat_from_int).collect())
                .collect(),
        )?;
        let product = order.basis().multiply(&numerator)?;
        let inverse_denominator = Rat::one().div(&rat_from_int(&self.denominator)).ok()?;
        let mut columns = Vec::with_capacity(size);
        for column in 0..product.column_count() {
            columns.push(
                product
                    .column(column)
                    .iter()
                    .map(|value| value.mul(&inverse_denominator))
                    .collect(),
            );
        }
        RatMatrix::from_columns(columns)
    }

    /// The product of two fractional ideals.
    pub(crate) fn multiply(&self, order: &Order, other: &FractionalIdeal) -> Option<Self> {
        let size = order.field().degree();
        let mut generators = Vec::with_capacity(size * size);
        for left in &self.basis {
            for right in &other.basis {
                generators.push(order.multiply_coordinates(left, right));
            }
        }
        let matrix = IntMatrix::from_columns(generators)?;
        let form = generator_lattice_basis(&matrix)?;
        if form.column_count() != size {
            return None;
        }
        FractionalIdeal::from_numerator(
            (0..size).map(|column| form.column(column)).collect(),
            self.denominator.mul(&other.denominator),
        )
    }

    /// The inverse fractional ideal.
    pub(crate) fn inverse(&self, order: &Order) -> Option<Self> {
        let basis = self.power_basis(order)?;
        let inverse = inverse_ideal_lattice(order, &basis)?;
        FractionalIdeal::from_power_basis(order, &inverse)
    }

    /// The quotient of two fractional ideals.
    pub(crate) fn divide(&self, order: &Order, other: &FractionalIdeal) -> Option<Self> {
        self.multiply(order, &other.inverse(order)?)
    }

    /// A nonnegative power.
    pub(crate) fn power(&self, order: &Order, exponent: u32) -> Option<Self> {
        let mut result = FractionalIdeal::identity(order)?;
        let mut factor = self.clone();
        let mut exponent = exponent;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = result.multiply(order, &factor)?;
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = factor.multiply(order, &factor)?;
            }
        }
        Some(result)
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
    fn ideal_multiplication_and_inversion() {
        let field = field(&[5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let size = field.degree();
        let mut generators = Vec::new();
        for index in 0..size {
            let mut column = vec![Int::zero(); size];
            column[index] = Int::from_i64(2);
            generators.push(column);
        }
        generators.push(vec![Int::one(), Int::one()]);
        let prime = Ideal::from_generators(&generators, size).unwrap();
        assert_eq!(prime.norm(), &Int::from_i64(2));
        let square = prime.multiply(&order, &prime).unwrap();
        assert_eq!(square.norm(), &Int::from_i64(4));
        let two = Ideal::from_generators(
            &(0..size)
                .map(|index| {
                    let mut column = vec![Int::zero(); size];
                    column[index] = Int::from_i64(2);
                    column
                })
                .collect::<Vec<_>>(),
            size,
        )
        .unwrap();
        assert!(square.equals(&two));
        assert!(square.is_subset_of(&two));
        assert!(two.is_subset_of(&square));
    }

    #[test]
    fn fractional_ideals_have_reciprocal_norms() {
        let field = field(&[5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let half = FractionalIdeal::from_numerator(
            (0..field.degree())
                .map(|index| {
                    let mut column = vec![Int::zero(); field.degree()];
                    column[index] = Int::one();
                    column
                })
                .collect(),
            Int::from_i64(2),
        )
        .unwrap();
        assert_eq!(half.norm(), &rational(1, 4));
        assert!(!half.is_integral());
        let inverse = half.inverse(&order).unwrap();
        assert_eq!(inverse.norm(), &rational(4, 1));
        assert_eq!(inverse.denominator(), &Int::one());
        assert!(inverse.is_integral());
        let product = half.multiply(&order, &inverse).unwrap();
        assert_eq!(product.norm(), &rational(1, 1));
        assert!(product.is_integral());
        assert!(
            product
                .to_integral()
                .unwrap()
                .equals(&Ideal::identity(&order).unwrap())
        );
    }

    #[test]
    fn fractional_power_and_division() {
        let field = field(&[5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let mut generators = Vec::new();
        for index in 0..field.degree() {
            let mut column = vec![Int::zero(); field.degree()];
            column[index] = Int::from_i64(2);
            generators.push(column);
        }
        generators.push(vec![Int::one(), Int::one()]);
        let ideal = FractionalIdeal::from_integral(
            Ideal::from_generators(&generators, field.degree()).unwrap(),
        )
        .unwrap();
        let square = ideal.power(&order, 2).unwrap();
        assert_eq!(square.norm(), &rational(4, 1));
        let quotient = square.divide(&order, &ideal).unwrap();
        assert_eq!(quotient.norm(), ideal.norm());
    }

    #[test]
    fn different_times_codifferent_is_the_unit_ideal() {
        for coefficients in [&[-5i64, 0, 1][..], &[-2, 0, 0, 1][..]] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let codifferent =
                FractionalIdeal::from_power_basis(&order, &codifferent(&order).unwrap()).unwrap();
            let different = FractionalIdeal::from_integral(different(&order).unwrap()).unwrap();
            let product = codifferent.multiply(&order, &different).unwrap();
            assert_eq!(product.norm(), &rational(1, 1));
            assert!(
                product
                    .to_integral()
                    .unwrap()
                    .equals(&Ideal::identity(&order).unwrap())
            );
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
