//! Dense rational matrices for number field arithmetic.
//!
//! Multiplication matrices, trace pairings, change of basis, and the multiplier
//! ring equations all need exact rational linear algebra. This module keeps that
//! layer separate from the integer lattice and the expression-oriented matrix
//! code.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

/// A dense matrix of exact rational numbers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RatMatrix {
    rows: usize,
    columns: usize,
    entries: Vec<Rat>,
}

impl RatMatrix {
    /// A matrix filled with zeros.
    pub(crate) fn new(rows: usize, columns: usize) -> Self {
        RatMatrix {
            rows,
            columns,
            entries: vec![Rat::zero(); rows * columns],
        }
    }

    /// Build from rows, rejecting ragged input.
    pub(crate) fn from_rows(rows: Vec<Vec<Rat>>) -> Option<Self> {
        let row_count = rows.len();
        let column_count = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|row| row.len() != column_count) {
            return None;
        }
        let mut entries = Vec::with_capacity(row_count * column_count);
        for row in rows {
            entries.extend(row);
        }
        Some(RatMatrix {
            rows: row_count,
            columns: column_count,
            entries,
        })
    }

    /// Build from columns.
    pub(crate) fn from_columns(columns: Vec<Vec<Rat>>) -> Option<Self> {
        let column_count = columns.len();
        let row_count = columns.first().map_or(0, Vec::len);
        if columns.iter().any(|column| column.len() != row_count) {
            return None;
        }
        let mut matrix = RatMatrix::new(row_count, column_count);
        for (column_index, column) in columns.into_iter().enumerate() {
            for (row_index, value) in column.into_iter().enumerate() {
                matrix.set(row_index, column_index, value);
            }
        }
        Some(matrix)
    }

    /// The identity matrix.
    pub(crate) fn identity(size: usize) -> Self {
        let mut matrix = RatMatrix::new(size, size);
        for index in 0..size {
            matrix.set(index, index, Rat::one());
        }
        matrix
    }

    /// The number of rows.
    pub(crate) fn row_count(&self) -> usize {
        self.rows
    }

    /// The number of columns.
    pub(crate) fn column_count(&self) -> usize {
        self.columns
    }

    fn index(&self, row: usize, column: usize) -> usize {
        row * self.columns + column
    }

    /// Read one entry.
    pub(crate) fn get(&self, row: usize, column: usize) -> Rat {
        self.entries[self.index(row, column)].clone()
    }

    /// Write one entry.
    pub(crate) fn set(&mut self, row: usize, column: usize, value: Rat) {
        let index = self.index(row, column);
        self.entries[index] = value;
    }

    /// Read one row.
    pub(crate) fn row(&self, row: usize) -> Vec<Rat> {
        (0..self.columns)
            .map(|column| self.get(row, column))
            .collect()
    }

    /// Read one column.
    pub(crate) fn column(&self, column: usize) -> Vec<Rat> {
        (0..self.rows).map(|row| self.get(row, column)).collect()
    }

    /// Replace one row.
    pub(crate) fn set_row(&mut self, row: usize, values: Vec<Rat>) {
        debug_assert_eq!(values.len(), self.columns);
        for (column, value) in values.into_iter().enumerate() {
            self.set(row, column, value);
        }
    }

    /// True when every entry is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.entries.iter().all(Rat::is_zero)
    }

    fn swap_rows(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        for column in 0..self.columns {
            let left_index = self.index(left, column);
            let right_index = self.index(right, column);
            self.entries.swap(left_index, right_index);
        }
    }

    fn scale_row(&mut self, row: usize, factor: &Rat) {
        for column in 0..self.columns {
            let value = self.get(row, column).mul(factor);
            self.set(row, column, value);
        }
    }

    fn add_row_multiple(&mut self, target: usize, source: usize, factor: &Rat) {
        if target == source || factor.is_zero() {
            return;
        }
        for column in 0..self.columns {
            let value = self
                .get(target, column)
                .add(&factor.mul(&self.get(source, column)));
            self.set(target, column, value);
        }
    }

    /// Matrix product.
    pub(crate) fn multiply(&self, other: &RatMatrix) -> Option<RatMatrix> {
        if self.columns != other.rows {
            return None;
        }
        let mut result = RatMatrix::new(self.rows, other.columns);
        for row in 0..self.rows {
            for column in 0..other.columns {
                let mut total = Rat::zero();
                for index in 0..self.columns {
                    total = total.add(&self.get(row, index).mul(&other.get(index, column)));
                }
                result.set(row, column, total);
            }
        }
        Some(result)
    }

    /// The transpose.
    pub(crate) fn transpose(&self) -> RatMatrix {
        let mut result = RatMatrix::new(self.columns, self.rows);
        for row in 0..self.rows {
            for column in 0..self.columns {
                result.set(column, row, self.get(row, column));
            }
        }
        result
    }

    /// The trace, or `None` when the matrix is not square.
    pub(crate) fn trace(&self) -> Option<Rat> {
        if self.rows != self.columns {
            return None;
        }
        let mut total = Rat::zero();
        for index in 0..self.rows {
            total = total.add(&self.get(index, index));
        }
        Some(total)
    }

    /// The determinant by exact Gaussian elimination.
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn determinant(&self) -> Option<Rat> {
        if self.rows != self.columns {
            return None;
        }
        let size = self.rows;
        let mut matrix = self.clone();
        let mut determinant = Rat::one();
        for column in 0..size {
            let Some(pivot) = (column..size).find(|row| !matrix.get(*row, column).is_zero()) else {
                return Some(Rat::zero());
            };
            if pivot != column {
                matrix.swap_rows(pivot, column);
                determinant = determinant.neg();
            }
            let pivot_value = matrix.get(column, column);
            determinant = determinant.mul(&pivot_value);
            for row in column + 1..size {
                let factor = match matrix.get(row, column).div(&pivot_value) {
                    Ok(value) => value,
                    Err(_) => return None,
                };
                matrix.add_row_multiple(row, column, &factor.neg());
            }
        }
        Some(determinant)
    }

    /// The reduced row echelon form and the pivot columns.
    #[allow(clippy::needless_range_loop)]
    fn reduced_row_echelon(&self) -> (RatMatrix, Vec<usize>) {
        let mut matrix = self.clone();
        let mut pivots = Vec::new();
        let mut pivot_row = 0usize;
        for column in 0..self.columns {
            if pivot_row >= self.rows {
                break;
            }
            let Some(found) =
                (pivot_row..self.rows).find(|row| !matrix.get(*row, column).is_zero())
            else {
                continue;
            };
            if found != pivot_row {
                matrix.swap_rows(found, pivot_row);
            }
            let pivot_value = matrix.get(pivot_row, column);
            let inverse = Rat::one().div(&pivot_value).unwrap_or_else(|_| Rat::zero());
            matrix.scale_row(pivot_row, &inverse);
            for row in 0..self.rows {
                if row == pivot_row {
                    continue;
                }
                let factor = matrix.get(row, column);
                if !factor.is_zero() {
                    matrix.add_row_multiple(row, pivot_row, &factor.neg());
                }
            }
            pivots.push(column);
            pivot_row += 1;
        }
        (matrix, pivots)
    }

    /// The rank over the rationals.
    pub(crate) fn rank(&self) -> usize {
        self.reduced_row_echelon().1.len()
    }

    /// A basis of the nullspace, as vectors of column coordinates.
    pub(crate) fn nullspace(&self) -> Vec<Vec<Rat>> {
        let (reduced, pivots) = self.reduced_row_echelon();
        let mut free = Vec::new();
        for column in 0..self.columns {
            if !pivots.contains(&column) {
                free.push(column);
            }
        }
        let mut basis = Vec::with_capacity(free.len());
        for free_column in free {
            let mut vector = vec![Rat::zero(); self.columns];
            vector[free_column] = Rat::one();
            for (row, pivot_column) in pivots.iter().enumerate() {
                vector[*pivot_column] = reduced.get(row, free_column).neg();
            }
            basis.push(vector);
        }
        basis
    }

    /// The inverse by exact Gauss-Jordan elimination.
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn inverse(&self) -> Option<RatMatrix> {
        if self.rows != self.columns {
            return None;
        }
        let size = self.rows;
        let mut matrix = self.clone();
        let mut inverse = RatMatrix::identity(size);
        for column in 0..size {
            let pivot = (column..size).find(|row| !matrix.get(*row, column).is_zero())?;
            if pivot != column {
                matrix.swap_rows(pivot, column);
                inverse.swap_rows(pivot, column);
            }
            let pivot_value = matrix.get(column, column);
            let reciprocal = Rat::one().div(&pivot_value).ok()?;
            matrix.scale_row(column, &reciprocal);
            inverse.scale_row(column, &reciprocal);
            for row in 0..size {
                if row == column {
                    continue;
                }
                let factor = matrix.get(row, column);
                if !factor.is_zero() {
                    matrix.add_row_multiple(row, column, &factor.neg());
                    inverse.add_row_multiple(row, column, &factor.neg());
                }
            }
        }
        Some(inverse)
    }

    /// The characteristic polynomial in descending coefficient order.
    ///
    /// The first coefficient is one. The polynomial is
    /// `x^n + c_1 x^(n-1) + ... + c_n`.
    pub(crate) fn characteristic_polynomial(&self) -> Option<Vec<Rat>> {
        if self.rows != self.columns {
            return None;
        }
        let size = self.rows;
        let mut coefficients = vec![Rat::one()];
        let mut auxiliary = RatMatrix::identity(size);
        for index in 1..=size {
            let product = self.multiply(&auxiliary)?;
            let trace = product.trace()?;
            let divisor = Rat::from_i64(index as i64, 1).ok()?;
            let coefficient = trace.div(&divisor).ok()?.neg();
            coefficients.push(coefficient.clone());
            auxiliary = product;
            for diagonal in 0..size {
                let value = auxiliary.get(diagonal, diagonal).add(&coefficient);
                auxiliary.set(diagonal, diagonal, value);
            }
        }
        Some(coefficients)
    }

    /// All rows as vectors.
    pub(crate) fn to_rows(&self) -> Vec<Vec<Rat>> {
        (0..self.rows).map(|row| self.row(row)).collect()
    }
}

/// Convert an integer to a rational coefficient.
pub(crate) fn rat_from_int(value: &Int) -> Rat {
    Rat::from_ints(value.clone(), Int::one()).unwrap_or_else(|_| Rat::zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[i64]]) -> RatMatrix {
        RatMatrix::from_rows(
            rows.iter()
                .map(|row| {
                    row.iter()
                        .map(|value| Rat::from_i64(*value, 1).unwrap())
                        .collect()
                })
                .collect(),
        )
        .unwrap()
    }

    fn rational(value: i64) -> Rat {
        Rat::from_i64(value, 1).unwrap()
    }

    #[test]
    fn determinant_and_trace_match_known_values() {
        let source = matrix(&[&[1, 2], &[3, 4]]);
        assert_eq!(source.determinant(), Some(rational(-2)));
        assert_eq!(source.trace(), Some(rational(5)));
        let source = matrix(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 1]]);
        assert_eq!(source.determinant(), Some(rational(0)));
    }

    #[test]
    fn inverse_round_trips() {
        let source = matrix(&[&[2, 1], &[1, 1]]);
        let inverse = source.inverse().unwrap();
        assert_eq!(source.multiply(&inverse).unwrap(), RatMatrix::identity(2));
        assert_eq!(matrix(&[&[1, 2], &[2, 4]]).inverse(), None);
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn rank_and_nullspace_are_consistent() {
        let source = matrix(&[&[1, 2, 3], &[2, 4, 6]]);
        assert_eq!(source.rank(), 1);
        let basis = source.nullspace();
        assert_eq!(basis.len(), 2);
        for vector in &basis {
            let mut result = vec![Rat::zero(); 2];
            for row in 0..2 {
                for column in 0..3 {
                    result[row] = result[row].add(&source.get(row, column).mul(&vector[column]));
                }
            }
            assert!(result.iter().all(Rat::is_zero));
        }
    }

    #[test]
    fn characteristic_polynomial_matches_the_trace_and_determinant() {
        let source = matrix(&[&[1, 2], &[3, 4]]);
        assert_eq!(
            source.characteristic_polynomial(),
            Some(vec![rational(1), rational(-5), rational(-2)])
        );
        let source = matrix(&[&[2, 0, 0], &[0, 3, 0], &[0, 0, 5]]);
        assert_eq!(
            source.characteristic_polynomial(),
            Some(vec![
                rational(1),
                rational(-10),
                rational(31),
                rational(-30)
            ])
        );
    }

    #[test]
    fn rational_construction_from_columns_matches_rows() {
        let columns = vec![
            vec![rational(1), rational(3)],
            vec![rational(2), rational(4)],
        ];
        let source = RatMatrix::from_columns(columns).unwrap();
        assert_eq!(source, matrix(&[&[1, 2], &[3, 4]]));
    }

    #[test]
    fn three_by_three_inverse_and_determinant() {
        let source = matrix(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 1]]);
        assert_eq!(source.determinant(), Some(rational(0)));
        assert_eq!(source.inverse(), None);
        let source = matrix(&[&[1, 2, 3], &[0, 1, 4], &[5, 6, 0]]);
        assert_eq!(source.determinant(), Some(rational(1)));
        let inverse = source.inverse().unwrap();
        assert_eq!(source.multiply(&inverse).unwrap(), RatMatrix::identity(3));
    }

    #[test]
    fn rectangular_operations_return_none() {
        let source = RatMatrix::from_rows(vec![
            vec![rational(1), rational(2), rational(3)],
            vec![rational(4), rational(5), rational(6)],
        ])
        .unwrap();
        assert_eq!(source.determinant(), None);
        assert_eq!(source.trace(), None);
        assert_eq!(source.inverse(), None);
        assert_eq!(source.characteristic_polynomial(), None);
    }

    #[test]
    fn nullspace_of_the_zero_matrix_is_full_dimension() {
        let source = RatMatrix::new(2, 3);
        let basis = source.nullspace();
        assert_eq!(basis.len(), 3);
        for (index, vector) in basis.iter().enumerate() {
            assert_eq!(vector[index], rational(1));
        }
        assert_eq!(source.rank(), 0);
    }

    #[test]
    fn low_degree_characteristic_polynomials() {
        assert_eq!(
            matrix(&[&[7]]).characteristic_polynomial(),
            Some(vec![rational(1), rational(-7)])
        );
        assert_eq!(
            RatMatrix::new(2, 2).characteristic_polynomial(),
            Some(vec![rational(1), rational(0), rational(0)])
        );
    }

    #[test]
    fn transpose_distributes_over_multiplication() {
        let left = matrix(&[&[1, 2, 3], &[4, 5, 6]]);
        let right = matrix(&[&[7, 8], &[9, 10], &[11, 12]]);
        let product = left.multiply(&right).unwrap();
        assert_eq!(
            product.transpose(),
            right.transpose().multiply(&left.transpose()).unwrap()
        );
    }

    #[test]
    fn characteristic_polynomial_satisfies_cayley_hamilton() {
        let mut state = 0x1357_9bdf_2468_ace0u64;
        for _ in 0..32 {
            let mut rows = Vec::new();
            for _ in 0..3 {
                let mut row = Vec::new();
                for _ in 0..3 {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    row.push(rational(((state >> 33) % 9) as i64 - 4));
                }
                rows.push(row);
            }
            let source = RatMatrix::from_rows(rows).unwrap();
            let coefficients = source.characteristic_polynomial().unwrap();
            let mut value = RatMatrix::new(3, 3);
            for coefficient in &coefficients {
                value = value.multiply(&source).unwrap();
                for diagonal in 0..3 {
                    let entry = value.get(diagonal, diagonal).add(coefficient);
                    value.set(diagonal, diagonal, entry);
                }
            }
            assert!(value.is_zero());
        }
    }

    #[test]
    fn ragged_rows_are_rejected() {
        assert!(
            RatMatrix::from_rows(vec![vec![rational(1)], vec![rational(2), rational(3)]]).is_none()
        );
    }
}
