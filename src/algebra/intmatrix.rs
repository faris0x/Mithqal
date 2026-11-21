//! Dense integer matrices for exact lattice arithmetic.
//!
//! The representation is row major. Entries are arbitrary precision integers,
//! so the same code serves the small degree number field stack and larger
//! discriminant experiments without a separate machine word path.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::Int;

/// A dense matrix of arbitrary precision integers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IntMatrix {
    rows: usize,
    columns: usize,
    entries: Vec<Int>,
}

impl IntMatrix {
    /// A matrix filled with zeros.
    pub(crate) fn new(rows: usize, columns: usize) -> Self {
        IntMatrix {
            rows,
            columns,
            entries: vec![Int::zero(); rows * columns],
        }
    }

    /// Build from rows, rejecting ragged input.
    pub(crate) fn from_rows(rows: Vec<Vec<Int>>) -> Option<Self> {
        let row_count = rows.len();
        let column_count = rows.first().map_or(0, Vec::len);
        if rows.iter().any(|row| row.len() != column_count) {
            return None;
        }
        let mut entries = Vec::with_capacity(row_count * column_count);
        for row in rows {
            entries.extend(row);
        }
        Some(IntMatrix {
            rows: row_count,
            columns: column_count,
            entries,
        })
    }

    /// Build from columns.
    pub(crate) fn from_columns(columns: Vec<Vec<Int>>) -> Option<Self> {
        let column_count = columns.len();
        let row_count = columns.first().map_or(0, Vec::len);
        if columns.iter().any(|column| column.len() != row_count) {
            return None;
        }
        let mut matrix = IntMatrix::new(row_count, column_count);
        for (column_index, column) in columns.into_iter().enumerate() {
            for (row_index, value) in column.into_iter().enumerate() {
                matrix.set(row_index, column_index, value);
            }
        }
        Some(matrix)
    }

    /// The identity matrix.
    pub(crate) fn identity(size: usize) -> Self {
        let mut matrix = IntMatrix::new(size, size);
        for index in 0..size {
            matrix.set(index, index, Int::one());
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

    /// True when the matrix has no entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn index(&self, row: usize, column: usize) -> usize {
        row * self.columns + column
    }

    /// Read one entry.
    pub(crate) fn get(&self, row: usize, column: usize) -> Int {
        self.entries[self.index(row, column)].clone()
    }

    /// Write one entry.
    pub(crate) fn set(&mut self, row: usize, column: usize, value: Int) {
        let index = self.index(row, column);
        self.entries[index] = value;
    }

    /// Read one row.
    pub(crate) fn row(&self, row: usize) -> Vec<Int> {
        (0..self.columns)
            .map(|column| self.get(row, column))
            .collect()
    }

    /// Read one column.
    pub(crate) fn column(&self, column: usize) -> Vec<Int> {
        (0..self.rows).map(|row| self.get(row, column)).collect()
    }

    /// Replace one row.
    pub(crate) fn set_row(&mut self, row: usize, values: Vec<Int>) {
        debug_assert_eq!(values.len(), self.columns);
        for (column, value) in values.into_iter().enumerate() {
            self.set(row, column, value);
        }
    }

    /// Replace one column.
    pub(crate) fn set_column(&mut self, column: usize, values: Vec<Int>) {
        debug_assert_eq!(values.len(), self.rows);
        for (row, value) in values.into_iter().enumerate() {
            self.set(row, column, value);
        }
    }

    /// All rows as vectors.
    pub(crate) fn to_rows(&self) -> Vec<Vec<Int>> {
        (0..self.rows).map(|row| self.row(row)).collect()
    }

    /// True when every entry is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.entries.iter().all(Int::is_zero)
    }

    /// Swap two rows.
    pub(crate) fn swap_rows(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        for column in 0..self.columns {
            let left_index = self.index(left, column);
            let right_index = self.index(right, column);
            self.entries.swap(left_index, right_index);
        }
    }

    /// Swap two columns.
    pub(crate) fn swap_columns(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        for row in 0..self.rows {
            let left_index = self.index(row, left);
            let right_index = self.index(row, right);
            self.entries.swap(left_index, right_index);
        }
    }

    /// Multiply one row by a scalar.
    pub(crate) fn scale_row(&mut self, row: usize, factor: &Int) {
        for column in 0..self.columns {
            let value = self.get(row, column).mul(factor);
            self.set(row, column, value);
        }
    }

    /// Multiply one column by a scalar.
    pub(crate) fn scale_column(&mut self, column: usize, factor: &Int) {
        for row in 0..self.rows {
            let value = self.get(row, column).mul(factor);
            self.set(row, column, value);
        }
    }

    /// Add a multiple of one row to another.
    pub(crate) fn add_row_multiple(&mut self, target: usize, source: usize, factor: &Int) {
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

    /// Add a multiple of one column to another.
    pub(crate) fn add_column_multiple(&mut self, target: usize, source: usize, factor: &Int) {
        if target == source || factor.is_zero() {
            return;
        }
        for row in 0..self.rows {
            let value = self
                .get(row, target)
                .add(&factor.mul(&self.get(row, source)));
            self.set(row, target, value);
        }
    }

    /// Matrix product.
    pub(crate) fn multiply(&self, other: &IntMatrix) -> Option<IntMatrix> {
        if self.columns != other.rows {
            return None;
        }
        let mut result = IntMatrix::new(self.rows, other.columns);
        for row in 0..self.rows {
            for column in 0..other.columns {
                let mut total = Int::zero();
                for index in 0..self.columns {
                    total = total.add(&self.get(row, index).mul(&other.get(index, column)));
                }
                result.set(row, column, total);
            }
        }
        Some(result)
    }

    /// The transpose.
    pub(crate) fn transpose(&self) -> IntMatrix {
        let mut result = IntMatrix::new(self.columns, self.rows);
        for row in 0..self.rows {
            for column in 0..self.columns {
                result.set(column, row, self.get(row, column));
            }
        }
        result
    }

    /// Drop rows that are entirely zero.
    pub(crate) fn without_zero_rows(&self) -> IntMatrix {
        let rows = self
            .to_rows()
            .into_iter()
            .filter(|row| row.iter().any(|value| !value.is_zero()))
            .collect();
        IntMatrix::from_rows(rows).unwrap_or_else(|| IntMatrix::new(0, self.columns))
    }

    /// The determinant by exact Bareiss elimination, or `None` when not square.
    pub(crate) fn determinant(&self) -> Option<Int> {
        if self.rows != self.columns {
            return None;
        }
        let size = self.rows;
        if size == 0 {
            return Some(Int::one());
        }
        let mut matrix = self.clone();
        let mut sign = 1i8;
        let mut previous = Int::one();
        for column in 0..size.saturating_sub(1) {
            if matrix.get(column, column).is_zero() {
                let Some(pivot) =
                    (column + 1..size).find(|row| !matrix.get(*row, column).is_zero())
                else {
                    return Some(Int::zero());
                };
                matrix.swap_rows(column, pivot);
                sign = -sign;
            }
            let pivot_value = matrix.get(column, column);
            for row in column + 1..size {
                for target in column + 1..size {
                    let value = matrix
                        .get(row, target)
                        .mul(&pivot_value)
                        .sub(&matrix.get(row, column).mul(&matrix.get(column, target)));
                    matrix.set(row, target, div_exact(&value, &previous));
                }
                matrix.set(row, column, Int::zero());
            }
            previous = pivot_value;
        }
        let value = matrix.get(size - 1, size - 1);
        Some(if sign < 0 { value.neg() } else { value })
    }
}

/// Exact division that treats a nonzero remainder as an internal invariant
/// violation.
pub(crate) fn div_exact(value: &Int, divisor: &Int) -> Int {
    match value.divrem(divisor) {
        Ok((quotient, remainder)) => {
            debug_assert!(remainder.is_zero(), "inexact integer division");
            quotient
        }
        Err(_) => Int::zero(),
    }
}

/// Euclidean quotient and remainder with a positive divisor.
pub(crate) fn div_rem_euclid(value: &Int, divisor: &Int) -> (Int, Int) {
    let (mut quotient, mut remainder) = match value.divrem(divisor) {
        Ok(pair) => pair,
        Err(_) => return (Int::zero(), Int::zero()),
    };
    if remainder.sign() < 0 {
        if divisor.sign() > 0 {
            quotient = quotient.sub(&Int::one());
            remainder = remainder.add(divisor);
        } else {
            quotient = quotient.add(&Int::one());
            remainder = remainder.sub(divisor);
        }
    }
    (quotient, remainder)
}

/// Extended gcd: returns `(g, s, t)` with `s a + t b = g` and `g >= 0`.
pub(crate) fn extended_gcd(left: &Int, right: &Int) -> (Int, Int, Int) {
    let (mut old_r, mut r) = (left.clone(), right.clone());
    let (mut old_s, mut s) = (Int::one(), Int::zero());
    let (mut old_t, mut t) = (Int::zero(), Int::one());
    while !r.is_zero() {
        let (quotient, remainder) = match old_r.divrem(&r) {
            Ok(pair) => pair,
            Err(_) => break,
        };
        (old_r, r) = (r, remainder);
        let next_s = old_s.sub(&quotient.mul(&s));
        let next_t = old_t.sub(&quotient.mul(&t));
        (old_s, s) = (s, next_s);
        (old_t, t) = (t, next_t);
    }
    if old_r.sign() < 0 {
        (old_r.neg(), old_s.neg(), old_t.neg())
    } else {
        (old_r, old_s, old_t)
    }
}

/// True when `divisor` divides `value`.
pub(crate) fn divides(divisor: &Int, value: &Int) -> bool {
    if divisor.is_zero() {
        return value.is_zero();
    }
    value
        .divrem(divisor)
        .map(|(_, remainder)| remainder.is_zero())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[i64]]) -> IntMatrix {
        IntMatrix::from_rows(
            rows.iter()
                .map(|row| row.iter().map(|value| Int::from_i64(*value)).collect())
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn determinant_matches_hand_computation() {
        assert_eq!(
            matrix(&[&[1, 2], &[3, 4]]).determinant(),
            Some(Int::from_i64(-2))
        );
        assert_eq!(
            matrix(&[&[2, 0, 1], &[1, 3, 2], &[1, 1, 1]]).determinant(),
            Some(Int::zero())
        );
        assert_eq!(
            matrix(&[&[1, 2, 3], &[0, 1, 4], &[5, 6, 0]]).determinant(),
            Some(Int::one())
        );
    }

    #[test]
    fn extended_gcd_reconstructs_inputs() {
        let (g, s, t) = extended_gcd(&Int::from_i64(240), &Int::from_i64(46));
        assert_eq!(g, Int::from_i64(2));
        assert_eq!(
            s.mul(&Int::from_i64(240)).add(&t.mul(&Int::from_i64(46))),
            g
        );
    }

    #[test]
    fn multiplication_and_transpose_agree() {
        let left = matrix(&[&[1, 2, 3], &[4, 5, 6]]);
        let right = matrix(&[&[7, 8], &[9, 10], &[11, 12]]);
        let product = left.multiply(&right).unwrap();
        assert_eq!(product, matrix(&[&[58, 64], &[139, 154]]));
        assert_eq!(
            product.transpose(),
            right.transpose().multiply(&left.transpose()).unwrap()
        );
    }
}
