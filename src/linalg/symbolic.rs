//! Symbolic matrix operations.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

#[derive(Clone)]
pub(crate) struct SymbolicMatrix {
    pub(super) rows: usize,
    pub(super) columns: usize,
    pub(super) entries: Vec<ExprId>,
}

impl SymbolicMatrix {
    pub(super) fn new(rows: usize, columns: usize) -> Self {
        SymbolicMatrix {
            rows,
            columns,
            entries: vec![ExprId::default(); rows * columns],
        }
    }

    pub(super) fn get(&self, row: usize, column: usize) -> ExprId {
        self.entries[row * self.columns + column]
    }

    pub(super) fn set(&mut self, row: usize, column: usize, value: ExprId) {
        self.entries[row * self.columns + column] = value;
    }
}

pub(crate) fn symbolic_matrix(arena: &Arena, expression: ExprId) -> Option<SymbolicMatrix> {
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
        entries.extend(values);
    }
    Some(SymbolicMatrix {
        rows: entries.len() / columns?,
        columns: columns?,
        entries,
    })
}

pub(crate) fn symbolic_expression(arena: &mut Arena, matrix: &SymbolicMatrix) -> ExprId {
    let list = arena.builtin_head("list");
    let mut rows = Vec::with_capacity(matrix.rows);
    for row in 0..matrix.rows {
        let mut values = Vec::with_capacity(matrix.columns);
        for column in 0..matrix.columns {
            values.push(matrix.get(row, column));
        }
        rows.push(arena.intern_call(list, &values));
    }
    arena.intern_call(list, &rows)
}

pub(crate) fn expression_zero(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.is_zero(),
        Node::Rat(value) => value.is_zero(),
        _ => false,
    }
}

pub(crate) fn expression_add(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    if let (Some(left_value), Some(right_value)) = (as_rat(arena, left), as_rat(arena, right)) {
        return rational_expression(arena, &left_value.add(&right_value));
    }
    let head = arena.builtin_head("plus");
    arena.intern_call(head, &[left, right])
}

pub(crate) fn expression_negate(arena: &mut Arena, value: ExprId) -> ExprId {
    if let Some(value) = as_rat(arena, value) {
        return rational_expression(arena, &value.neg());
    }
    let head = arena.builtin_head("negate");
    arena.intern_call(head, &[value])
}

pub(crate) fn expression_subtract(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    let negative = expression_negate(arena, right);
    expression_add(arena, left, negative)
}

pub(crate) fn expression_multiply(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    if let (Some(left_value), Some(right_value)) = (as_rat(arena, left), as_rat(arena, right)) {
        return rational_expression(arena, &left_value.mul(&right_value));
    }
    let head = arena.builtin_head("times");
    arena.intern_call(head, &[left, right])
}

pub(crate) fn expression_reciprocal(arena: &mut Arena, value: ExprId) -> ExprId {
    if let Some(value) = as_rat(arena, value)
        && !value.is_zero()
        && let Ok(inverse) = Rat::one().div(&value)
    {
        return rational_expression(arena, &inverse);
    }
    let minus_one = arena.intern_int(Int::from_i64(-1));
    let head = arena.builtin_head("power");
    arena.intern_call(head, &[value, minus_one])
}

pub(crate) fn expression_sum(arena: &mut Arena, values: &[ExprId]) -> ExprId {
    let mut result = arena.intern_int(Int::zero());
    for value in values {
        result = expression_add(arena, result, *value);
    }
    result
}

pub(crate) fn symbolic_determinant(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
    if matrix.rows != matrix.columns {
        return None;
    }
    let size = matrix.rows;
    if size == 0 {
        return Some(arena.intern_int(Int::one()));
    }
    if size > 5 {
        return None;
    }
    if size == 1 {
        return Some(matrix.get(0, 0));
    }
    if size == 2 {
        let left = expression_multiply(arena, matrix.get(0, 0), matrix.get(1, 1));
        let right = expression_multiply(arena, matrix.get(0, 1), matrix.get(1, 0));
        return Some(expression_subtract(arena, left, right));
    }
    let mut terms = Vec::new();
    for column in 0..size {
        let entry = matrix.get(0, column);
        if expression_zero(arena, entry) {
            continue;
        }
        let mut minor = SymbolicMatrix::new(size - 1, size - 1);
        for row in 1..size {
            let mut target_column = 0usize;
            for source_column in 0..size {
                if source_column == column {
                    continue;
                }
                minor.set(row - 1, target_column, matrix.get(row, source_column));
                target_column += 1;
            }
        }
        let determinant = symbolic_determinant(arena, &minor)?;
        let mut term = expression_multiply(arena, entry, determinant);
        if column % 2 == 1 {
            term = expression_negate(arena, term);
        }
        terms.push(term);
    }
    Some(expression_sum(arena, &terms))
}

pub(crate) fn symbolic_cofactor(
    arena: &mut Arena,
    matrix: &SymbolicMatrix,
    row: usize,
    column: usize,
) -> Option<ExprId> {
    let size = matrix.rows;
    let mut minor = SymbolicMatrix::new(size - 1, size - 1);
    let mut target_row = 0usize;
    for source_row in 0..size {
        if source_row == row {
            continue;
        }
        let mut target_column = 0usize;
        for source_column in 0..size {
            if source_column == column {
                continue;
            }
            minor.set(
                target_row,
                target_column,
                matrix.get(source_row, source_column),
            );
            target_column += 1;
        }
        target_row += 1;
    }
    let determinant = symbolic_determinant(arena, &minor)?;
    if (row + column).is_multiple_of(2) {
        Some(determinant)
    } else {
        Some(expression_negate(arena, determinant))
    }
}

pub(crate) fn symbolic_inverse(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
    if matrix.rows != matrix.columns || matrix.rows > 3 || matrix.rows == 0 {
        return None;
    }
    let determinant = symbolic_determinant(arena, matrix)?;
    if expression_zero(arena, determinant) {
        return None;
    }
    let inverse_determinant = expression_reciprocal(arena, determinant);
    let size = matrix.rows;
    let mut adjugate = SymbolicMatrix::new(size, size);
    for row in 0..size {
        for column in 0..size {
            let cofactor = symbolic_cofactor(arena, matrix, column, row)?;
            adjugate.set(row, column, cofactor);
        }
    }
    for entry in &mut adjugate.entries {
        *entry = expression_multiply(arena, *entry, inverse_determinant);
    }
    Some(symbolic_expression(arena, &adjugate))
}

pub(crate) fn symbolic_transpose(arena: &mut Arena, matrix: &SymbolicMatrix) -> ExprId {
    let mut out = SymbolicMatrix::new(matrix.columns, matrix.rows);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            out.set(column, row, matrix.get(row, column));
        }
    }
    symbolic_expression(arena, &out)
}

pub(crate) fn symbolic_trace(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
    if matrix.rows != matrix.columns {
        return None;
    }
    let mut values = Vec::with_capacity(matrix.rows);
    for index in 0..matrix.rows {
        values.push(matrix.get(index, index));
    }
    Some(expression_sum(arena, &values))
}

pub(crate) fn symbolic_matmul(
    arena: &mut Arena,
    left: &SymbolicMatrix,
    right: &SymbolicMatrix,
) -> Option<ExprId> {
    if left.columns != right.rows {
        return None;
    }
    let mut out = SymbolicMatrix::new(left.rows, right.columns);
    for row in 0..left.rows {
        for column in 0..right.columns {
            let mut terms = Vec::with_capacity(left.columns);
            for inner in 0..left.columns {
                terms.push(expression_multiply(
                    arena,
                    left.get(row, inner),
                    right.get(inner, column),
                ));
            }
            out.set(row, column, expression_sum(arena, &terms));
        }
    }
    Some(symbolic_expression(arena, &out))
}

pub(crate) fn determinant_symbolic(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = symbolic_matrix(arena, expression)?;
    symbolic_determinant(arena, &matrix)
}

pub(crate) fn transpose_symbolic(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = symbolic_matrix(arena, expression)?;
    Some(symbolic_transpose(arena, &matrix))
}

pub(crate) fn trace_symbolic(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = symbolic_matrix(arena, expression)?;
    symbolic_trace(arena, &matrix)
}

pub(crate) fn inverse_symbolic(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    if let Some(matrix) = matrix_from_expr(arena, expression)
        && matrix.determinant()?.is_zero()
    {
        return None;
    }
    let matrix = symbolic_matrix(arena, expression)?;
    symbolic_inverse(arena, &matrix)
}

pub(crate) fn matmul_symbolic(arena: &mut Arena, left: ExprId, right: ExprId) -> Option<ExprId> {
    let left = symbolic_matrix(arena, left)?;
    let right = symbolic_matrix(arena, right)?;
    symbolic_matmul(arena, &left, &right)
}
