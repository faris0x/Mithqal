//! Dense rational matrices and scalar operations.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Matrix {
    pub(super) rows: usize,
    pub(super) columns: usize,
    pub(super) entries: Vec<Rat>,
}

impl Matrix {
    pub(super) fn new(rows: usize, columns: usize) -> Self {
        Matrix {
            rows,
            columns,
            entries: vec![Rat::zero(); rows * columns],
        }
    }

    pub(super) fn index(&self, row: usize, column: usize) -> usize {
        row * self.columns + column
    }

    pub(super) fn get(&self, row: usize, column: usize) -> Rat {
        self.entries[self.index(row, column)].clone()
    }

    pub(super) fn set(&mut self, row: usize, column: usize, value: Rat) {
        let index = self.index(row, column);
        self.entries[index] = value;
    }

    pub(super) fn swap_rows(&mut self, left: usize, right: usize) {
        if left == right {
            return;
        }
        for column in 0..self.columns {
            let left_index = self.index(left, column);
            let right_index = self.index(right, column);
            self.entries.swap(left_index, right_index);
        }
    }

    pub(super) fn scale_row(&mut self, row: usize, factor: &Rat) {
        for column in 0..self.columns {
            let value = self.get(row, column).mul(factor);
            self.set(row, column, value);
        }
    }

    pub(super) fn subtract_row(&mut self, target: usize, source: usize, factor: &Rat) {
        for column in 0..self.columns {
            let value = self
                .get(target, column)
                .sub(&factor.mul(&self.get(source, column)));
            self.set(target, column, value);
        }
    }

    pub(super) fn identity(size: usize) -> Self {
        let mut matrix = Matrix::new(size, size);
        for index in 0..size {
            matrix.set(index, index, Rat::one());
        }
        matrix
    }

    pub(super) fn transpose(&self) -> Matrix {
        let mut out = Matrix::new(self.columns, self.rows);
        for row in 0..self.rows {
            for column in 0..self.columns {
                out.set(column, row, self.get(row, column));
            }
        }
        out
    }

    pub(super) fn multiply(&self, other: &Matrix) -> Option<Matrix> {
        if self.columns != other.rows {
            return None;
        }
        let mut out = Matrix::new(self.rows, other.columns);
        for row in 0..self.rows {
            for column in 0..other.columns {
                let mut value = Rat::zero();
                for inner in 0..self.columns {
                    value = value.add(&self.get(row, inner).mul(&other.get(inner, column)));
                }
                out.set(row, column, value);
            }
        }
        Some(out)
    }

    pub(super) fn determinant(&self) -> Option<Rat> {
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
            for row in (column + 1)..size {
                if matrix.get(row, column).is_zero() {
                    continue;
                }
                let factor = matrix.get(row, column).div(&pivot_value).ok()?;
                matrix.subtract_row(row, column, &factor);
            }
        }
        Some(determinant)
    }

    pub(super) fn rref(&self) -> (Matrix, Vec<usize>) {
        let mut matrix = self.clone();
        let mut pivots = Vec::new();
        let mut pivot_row = 0usize;
        for column in 0..matrix.columns {
            if pivot_row >= matrix.rows {
                break;
            }
            let Some(pivot) =
                (pivot_row..matrix.rows).find(|row| !matrix.get(*row, column).is_zero())
            else {
                continue;
            };
            matrix.swap_rows(pivot, pivot_row);
            let value = matrix.get(pivot_row, column);
            let inverse = Rat::one().div(&value).ok();
            let Some(inverse) = inverse else {
                continue;
            };
            matrix.scale_row(pivot_row, &inverse);
            for row in 0..matrix.rows {
                if row == pivot_row {
                    continue;
                }
                let factor = matrix.get(row, column);
                if !factor.is_zero() {
                    matrix.subtract_row(row, pivot_row, &factor);
                }
            }
            pivots.push(column);
            pivot_row += 1;
        }
        (matrix, pivots)
    }

    pub(super) fn rank(&self) -> usize {
        self.rref().1.len()
    }

    pub(super) fn inverse(&self) -> Option<Matrix> {
        if self.rows != self.columns {
            return None;
        }
        let size = self.rows;
        let mut augmented = Matrix::new(size, size * 2);
        for row in 0..size {
            for column in 0..size {
                augmented.set(row, column, self.get(row, column));
            }
            augmented.set(row, size + row, Rat::one());
        }
        let (reduced, pivots) = augmented.rref();
        if pivots.len() < size
            || pivots
                .iter()
                .take(size)
                .copied()
                .enumerate()
                .any(|(index, column)| index != column)
        {
            return None;
        }
        let mut inverse = Matrix::new(size, size);
        for row in 0..size {
            for column in 0..size {
                inverse.set(row, column, reduced.get(row, size + column));
            }
        }
        Some(inverse)
    }

    pub(super) fn trace(&self) -> Option<Rat> {
        if self.rows != self.columns {
            return None;
        }
        let mut value = Rat::zero();
        for index in 0..self.rows {
            value = value.add(&self.get(index, index));
        }
        Some(value)
    }

    pub(super) fn solve(&self, right: &[Rat]) -> Option<Vec<Rat>> {
        if right.len() != self.rows {
            return None;
        }
        let mut augmented = Matrix::new(self.rows, self.columns + 1);
        for (row, value) in right.iter().enumerate().take(self.rows) {
            for column in 0..self.columns {
                augmented.set(row, column, self.get(row, column));
            }
            augmented.set(row, self.columns, value.clone());
        }
        let (reduced, pivots) = augmented.rref();
        for row in 0..reduced.rows {
            let all_zero = (0..self.columns).all(|column| reduced.get(row, column).is_zero());
            if all_zero && !reduced.get(row, self.columns).is_zero() {
                return None;
            }
        }
        if pivots.len() != self.columns {
            return None;
        }
        let mut solution = vec![Rat::zero(); self.columns];
        for (row, column) in pivots.iter().take(self.columns).enumerate() {
            solution[*column] = reduced.get(row, self.columns);
        }
        Some(solution)
    }
}

pub(crate) fn list_arguments(arena: &Arena, expression: ExprId) -> Option<Vec<ExprId>> {
    match arena.node(expression) {
        Node::Call { head, args } => {
            let symbol = arena.symbol_id(*head)?;
            if arena.symbol_name(symbol) == "list" {
                Some(args.to_vec())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn as_rat(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn rational_expression(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

pub(crate) fn matrix_from_expr(arena: &Arena, expression: ExprId) -> Option<Matrix> {
    let rows = list_arguments(arena, expression)?;
    if rows.is_empty() {
        return None;
    }
    let mut entries = Vec::new();
    let mut columns = None;
    for row in rows {
        let values = list_arguments(arena, row)?;
        if values.is_empty() {
            return None;
        }
        if let Some(expected) = columns {
            if values.len() != expected {
                return None;
            }
        } else {
            columns = Some(values.len());
        }
        for value in values {
            entries.push(as_rat(arena, value)?);
        }
    }
    Some(Matrix {
        rows: entries.len() / columns?,
        columns: columns?,
        entries,
    })
}

pub(crate) fn vector_from_expr(arena: &Arena, expression: ExprId) -> Option<Vec<Rat>> {
    let values = list_arguments(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    values
        .into_iter()
        .map(|value| as_rat(arena, value))
        .collect()
}

pub(crate) fn matrix_expression(arena: &mut Arena, matrix: &Matrix) -> ExprId {
    let list = arena.builtin_head("list");
    let mut rows = Vec::with_capacity(matrix.rows);
    for row in 0..matrix.rows {
        let mut values = Vec::with_capacity(matrix.columns);
        for column in 0..matrix.columns {
            values.push(rational_expression(arena, &matrix.get(row, column)));
        }
        rows.push(arena.intern_call(list, &values));
    }
    arena.intern_call(list, &rows)
}

pub(crate) fn vector_expression(arena: &mut Arena, values: &[Rat]) -> ExprId {
    let list = arena.builtin_head("list");
    let entries = values
        .iter()
        .map(|value| rational_expression(arena, value))
        .collect::<Vec<_>>();
    arena.intern_call(list, &entries)
}

pub(crate) fn determinant(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    let value = matrix.determinant()?;
    Some(rational_expression(arena, &value))
}

pub(crate) fn rank(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    Some(arena.intern_int(Int::from_i64(matrix.rank() as i64)))
}

pub(crate) fn transpose(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    Some(matrix_expression(arena, &matrix.transpose()))
}

pub(crate) fn inverse(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    Some(matrix_expression(arena, &matrix.inverse()?))
}

pub(crate) fn rref(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    let (reduced, _) = matrix.rref();
    Some(matrix_expression(arena, &reduced))
}

pub(crate) fn trace(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    let value = matrix.trace()?;
    Some(rational_expression(arena, &value))
}

pub(crate) fn matmul(arena: &mut Arena, left: ExprId, right: ExprId) -> Option<ExprId> {
    let left = matrix_from_expr(arena, left)?;
    let right = matrix_from_expr(arena, right)?;
    let product = left.multiply(&right)?;
    Some(matrix_expression(arena, &product))
}

pub(crate) fn identity(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let size = match arena.node(expression) {
        Node::Int(value) => value.to_i64()?,
        _ => return None,
    };
    if !(0..=256).contains(&size) {
        return None;
    }
    Some(matrix_expression(arena, &Matrix::identity(size as usize)))
}

pub(crate) fn solve_linear(arena: &mut Arena, matrix: ExprId, right: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, matrix)?;
    let right = vector_from_expr(arena, right)?;
    let solution = matrix.solve(&right)?;
    Some(vector_expression(arena, &solution))
}

pub(crate) fn dot(arena: &mut Arena, left: ExprId, right: ExprId) -> Option<ExprId> {
    let left = vector_from_expr(arena, left)?;
    let right = vector_from_expr(arena, right)?;
    if left.len() != right.len() {
        return None;
    }
    let mut value = Rat::zero();
    for (left, right) in left.iter().zip(right.iter()) {
        value = value.add(&left.mul(right));
    }
    Some(rational_expression(arena, &value))
}

pub(crate) fn cross(arena: &mut Arena, left: ExprId, right: ExprId) -> Option<ExprId> {
    let left = vector_from_expr(arena, left)?;
    let right = vector_from_expr(arena, right)?;
    if left.len() != 3 || right.len() != 3 {
        return None;
    }
    let result = [
        left[1].mul(&right[2]).sub(&left[2].mul(&right[1])),
        left[2].mul(&right[0]).sub(&left[0].mul(&right[2])),
        left[0].mul(&right[1]).sub(&left[1].mul(&right[0])),
    ];
    Some(vector_expression(arena, &result))
}
