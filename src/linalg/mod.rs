//! Exact rational linear algebra.

use crate::expr::{Arena, ExprId, Node};
use crate::num::{Int, Rat};

/// A dense matrix of exact rationals in row major order.
#[derive(Clone, Debug, PartialEq)]
struct Matrix {
    rows: usize,
    columns: usize,
    entries: Vec<Rat>,
}

impl Matrix {
    fn new(rows: usize, columns: usize) -> Self {
        Matrix {
            rows,
            columns,
            entries: vec![Rat::zero(); rows * columns],
        }
    }

    fn index(&self, row: usize, column: usize) -> usize {
        row * self.columns + column
    }

    fn get(&self, row: usize, column: usize) -> Rat {
        self.entries[self.index(row, column)].clone()
    }

    fn set(&mut self, row: usize, column: usize, value: Rat) {
        let index = self.index(row, column);
        self.entries[index] = value;
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

    fn subtract_row(&mut self, target: usize, source: usize, factor: &Rat) {
        for column in 0..self.columns {
            let value = self
                .get(target, column)
                .sub(&factor.mul(&self.get(source, column)));
            self.set(target, column, value);
        }
    }

    fn identity(size: usize) -> Self {
        let mut matrix = Matrix::new(size, size);
        for index in 0..size {
            matrix.set(index, index, Rat::one());
        }
        matrix
    }

    fn transpose(&self) -> Matrix {
        let mut out = Matrix::new(self.columns, self.rows);
        for row in 0..self.rows {
            for column in 0..self.columns {
                out.set(column, row, self.get(row, column));
            }
        }
        out
    }

    fn multiply(&self, other: &Matrix) -> Option<Matrix> {
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

    fn determinant(&self) -> Option<Rat> {
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

    fn rref(&self) -> (Matrix, Vec<usize>) {
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

    fn rank(&self) -> usize {
        self.rref().1.len()
    }

    fn inverse(&self) -> Option<Matrix> {
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

    fn trace(&self) -> Option<Rat> {
        if self.rows != self.columns {
            return None;
        }
        let mut value = Rat::zero();
        for index in 0..self.rows {
            value = value.add(&self.get(index, index));
        }
        Some(value)
    }

    fn solve(&self, right: &[Rat]) -> Option<Vec<Rat>> {
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

fn list_arguments(arena: &Arena, expression: ExprId) -> Option<Vec<ExprId>> {
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

fn as_rat(arena: &Arena, expression: ExprId) -> Option<Rat> {
    match arena.node(expression) {
        Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
        Node::Rat(value) => Some(value.clone()),
        _ => None,
    }
}

fn rational_expression(arena: &mut Arena, value: &Rat) -> ExprId {
    if value.den().is_one() {
        arena.intern_int(value.num())
    } else {
        arena.intern_rat(value.clone())
    }
}

fn matrix_from_expr(arena: &Arena, expression: ExprId) -> Option<Matrix> {
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

fn vector_from_expr(arena: &Arena, expression: ExprId) -> Option<Vec<Rat>> {
    let values = list_arguments(arena, expression)?;
    if values.is_empty() {
        return None;
    }
    values
        .into_iter()
        .map(|value| as_rat(arena, value))
        .collect()
}

fn matrix_expression(arena: &mut Arena, matrix: &Matrix) -> ExprId {
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

fn vector_expression(arena: &mut Arena, values: &[Rat]) -> ExprId {
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

#[derive(Clone)]
struct SymbolicMatrix {
    rows: usize,
    columns: usize,
    entries: Vec<ExprId>,
}

impl SymbolicMatrix {
    fn new(rows: usize, columns: usize) -> Self {
        SymbolicMatrix {
            rows,
            columns,
            entries: vec![ExprId::default(); rows * columns],
        }
    }

    fn get(&self, row: usize, column: usize) -> ExprId {
        self.entries[row * self.columns + column]
    }

    fn set(&mut self, row: usize, column: usize, value: ExprId) {
        self.entries[row * self.columns + column] = value;
    }
}

fn symbolic_matrix(arena: &Arena, expression: ExprId) -> Option<SymbolicMatrix> {
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

fn symbolic_expression(arena: &mut Arena, matrix: &SymbolicMatrix) -> ExprId {
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

fn expression_zero(arena: &Arena, expression: ExprId) -> bool {
    match arena.node(expression) {
        Node::Int(value) => value.is_zero(),
        Node::Rat(value) => value.is_zero(),
        _ => false,
    }
}

fn expression_add(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    if let (Some(left_value), Some(right_value)) = (as_rat(arena, left), as_rat(arena, right)) {
        return rational_expression(arena, &left_value.add(&right_value));
    }
    let head = arena.builtin_head("plus");
    arena.intern_call(head, &[left, right])
}

fn expression_negate(arena: &mut Arena, value: ExprId) -> ExprId {
    if let Some(value) = as_rat(arena, value) {
        return rational_expression(arena, &value.neg());
    }
    let head = arena.builtin_head("negate");
    arena.intern_call(head, &[value])
}

fn expression_subtract(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    let negative = expression_negate(arena, right);
    expression_add(arena, left, negative)
}

fn expression_multiply(arena: &mut Arena, left: ExprId, right: ExprId) -> ExprId {
    if let (Some(left_value), Some(right_value)) = (as_rat(arena, left), as_rat(arena, right)) {
        return rational_expression(arena, &left_value.mul(&right_value));
    }
    let head = arena.builtin_head("times");
    arena.intern_call(head, &[left, right])
}

fn expression_reciprocal(arena: &mut Arena, value: ExprId) -> ExprId {
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

fn expression_sum(arena: &mut Arena, values: &[ExprId]) -> ExprId {
    let mut result = arena.intern_int(Int::zero());
    for value in values {
        result = expression_add(arena, result, *value);
    }
    result
}

fn symbolic_determinant(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
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

fn symbolic_cofactor(
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

fn symbolic_inverse(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
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

fn symbolic_transpose(arena: &mut Arena, matrix: &SymbolicMatrix) -> ExprId {
    let mut out = SymbolicMatrix::new(matrix.columns, matrix.rows);
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            out.set(column, row, matrix.get(row, column));
        }
    }
    symbolic_expression(arena, &out)
}

fn symbolic_trace(arena: &mut Arena, matrix: &SymbolicMatrix) -> Option<ExprId> {
    if matrix.rows != matrix.columns {
        return None;
    }
    let mut values = Vec::with_capacity(matrix.rows);
    for index in 0..matrix.rows {
        values.push(matrix.get(index, index));
    }
    Some(expression_sum(arena, &values))
}

fn symbolic_matmul(
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

pub(crate) fn plu(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    if matrix.rows != matrix.columns || matrix.rows == 0 {
        return None;
    }
    let size = matrix.rows;
    let mut upper = matrix.clone();
    let mut lower = Matrix::identity(size);
    let mut permutation = Matrix::identity(size);
    for column in 0..size {
        let pivot = (column..size).find(|row| !upper.get(*row, column).is_zero())?;
        if pivot != column {
            upper.swap_rows(pivot, column);
            permutation.swap_rows(pivot, column);
            for previous in 0..column {
                let left = lower.get(column, previous);
                let right = lower.get(pivot, previous);
                lower.set(column, previous, right);
                lower.set(pivot, previous, left);
            }
        }
        let pivot_value = upper.get(column, column);
        for row in (column + 1)..size {
            let factor = upper.get(row, column).div(&pivot_value).ok()?;
            lower.set(row, column, factor.clone());
            upper.subtract_row(row, column, &factor);
        }
    }
    let list = arena.builtin_head("list");
    let p = matrix_expression(arena, &permutation);
    let l = matrix_expression(arena, &lower);
    let u = matrix_expression(arena, &upper);
    Some(arena.intern_call(list, &[p, l, u]))
}

pub(crate) fn charpoly(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    if matrix.rows != matrix.columns || matrix.rows == 0 || matrix.rows > 8 {
        return None;
    }
    let size = matrix.rows;
    let mut coefficients = Vec::with_capacity(size + 1);
    let mut b = Matrix::identity(size);
    coefficients.push(Rat::one());
    for k in 1..=size {
        let product = matrix.multiply(&b)?;
        let mut c = Rat::zero();
        for index in 0..size {
            c = c.add(&product.get(index, index));
        }
        c = c.div(&Rat::from_i64(k as i64, 1).ok()?).ok()?.neg();
        coefficients.push(c.clone());
        b = product;
        for index in 0..size {
            b.set(index, index, b.get(index, index).add(&c));
        }
    }
    coefficients.reverse();
    let entries = coefficients
        .iter()
        .map(|value| rational_expression(arena, value))
        .collect::<Vec<_>>();
    let list = arena.builtin_head("list");
    Some(arena.intern_call(list, &entries))
}

fn exact_eigenvalues(arena: &mut Arena, matrix: &Matrix) -> Option<ExprId> {
    let list = arena.builtin_head("list");
    if matrix.rows != matrix.columns || matrix.rows == 0 {
        return None;
    }
    let size = matrix.rows;
    if size == 1 {
        let entry = rational_expression(arena, &matrix.get(0, 0));
        return Some(arena.intern_call(list, &[entry]));
    }
    let triangular = (0..size)
        .all(|row| (0..size).all(|column| row == column || matrix.get(row, column).is_zero()))
        || (0..size)
            .all(|row| (0..size).all(|column| row == column || matrix.get(row, column).is_zero()));
    if size > 2 && triangular {
        let entries = (0..size)
            .map(|index| rational_expression(arena, &matrix.get(index, index)))
            .collect::<Vec<_>>();
        return Some(arena.intern_call(list, &entries));
    }
    if size != 2 {
        return None;
    }
    let trace = matrix.get(0, 0).add(&matrix.get(1, 1));
    let determinant = matrix
        .get(0, 0)
        .mul(&matrix.get(1, 1))
        .sub(&matrix.get(0, 1).mul(&matrix.get(1, 0)));
    let variable_symbol = arena.symbol_or_intern("x");
    let variable = arena.symbol_expr(variable_symbol);
    let two = arena.intern_int(Int::from_i64(2));
    let power = arena.builtin_head("power");
    let squared = arena.intern_call(power, &[variable, two]);
    let negative_trace = rational_expression(arena, &trace.neg());
    let times = arena.builtin_head("times");
    let linear = arena.intern_call(times, &[negative_trace, variable]);
    let constant = rational_expression(arena, &determinant);
    let plus = arena.builtin_head("plus");
    let polynomial = arena.intern_call(plus, &[squared, linear, constant]);
    let context = crate::algebra::Context {
        plus,
        times,
        power,
        negate: arena.builtin_head("negate"),
        list,
        equal: arena.builtin_head("equal"),
    };
    crate::algebra::solve(arena, polynomial, variable, &context)
}

pub(crate) fn eigenvalues(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    if let Some(matrix) = matrix_from_expr(arena, expression) {
        return exact_eigenvalues(arena, &matrix);
    }
    let matrix = symbolic_matrix(arena, expression)?;
    if matrix.rows != matrix.columns || matrix.rows == 0 {
        return None;
    }
    let size = matrix.rows;
    let upper = (0..size).all(|row| {
        (0..size).all(|column| row == column || expression_zero(arena, matrix.get(row, column)))
    });
    let lower = (0..size).all(|row| {
        (0..size).all(|column| row == column || expression_zero(arena, matrix.get(row, column)))
    });
    let list = arena.builtin_head("list");
    if size > 2 && (upper || lower) {
        let entries = (0..size)
            .map(|index| matrix.get(index, index))
            .collect::<Vec<_>>();
        return Some(arena.intern_call(list, &entries));
    }
    if size != 2 {
        return None;
    }
    let trace = expression_add(arena, matrix.get(0, 0), matrix.get(1, 1));
    let left = expression_multiply(arena, matrix.get(0, 0), matrix.get(1, 1));
    let right = expression_multiply(arena, matrix.get(0, 1), matrix.get(1, 0));
    let determinant = expression_subtract(arena, left, right);
    let four = arena.intern_int(Int::from_i64(4));
    let squared_trace = expression_multiply(arena, trace, trace);
    let four_determinant = expression_multiply(arena, four, determinant);
    let discriminant = expression_subtract(arena, squared_trace, four_determinant);
    let sqrt_head = arena.builtin_head("sqrt");
    let root = arena.intern_call(sqrt_head, &[discriminant]);
    let two = arena.intern_int(Int::from_i64(2));
    let half = expression_reciprocal(arena, two);
    let numerator = expression_add(arena, trace, root);
    let first = expression_multiply(arena, numerator, half);
    let negative_root = expression_negate(arena, root);
    let numerator = expression_add(arena, trace, negative_root);
    let second = expression_multiply(arena, numerator, half);
    Some(arena.intern_call(list, &[first, second]))
}

pub(crate) fn nullspace(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    let (reduced, pivots) = matrix.rref();
    let mut free = Vec::new();
    for column in 0..matrix.columns {
        if !pivots.contains(&column) {
            free.push(column);
        }
    }
    let list = arena.builtin_head("list");
    let mut vectors = Vec::with_capacity(free.len());
    for column in free {
        let mut values = vec![Rat::zero(); matrix.columns];
        values[column] = Rat::one();
        for (row, pivot) in pivots.iter().enumerate() {
            values[*pivot] = reduced.get(row, column).neg();
        }
        vectors.push(vector_expression(arena, &values));
    }
    Some(arena.intern_call(list, &vectors))
}

fn rational_square_root(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_sqrt_i64(numerator)?;
    let denominator = integer_sqrt_i64(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
}

fn integer_sqrt_i64(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let mut low = 0i64;
    let mut high = value;
    while low <= high {
        let middle = low + (high - low) / 2;
        let square = (middle as i128) * (middle as i128);
        if square == value as i128 {
            return Some(middle);
        }
        if square < value as i128 {
            low = middle + 1;
        } else {
            high = middle - 1;
        }
    }
    None
}

pub(crate) fn eigenvectors(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    if matrix.rows != matrix.columns || matrix.rows == 0 {
        return None;
    }
    let size = matrix.rows;
    let list = arena.builtin_head("list");
    let basis = |arena: &mut Arena, index: usize| {
        let mut values = vec![Rat::zero(); size];
        values[index] = Rat::one();
        vector_expression(arena, &values)
    };
    if size == 1 {
        let vector = basis(arena, 0);
        return Some(arena.intern_call(list, &[vector]));
    }
    let triangular = (0..size)
        .all(|row| (0..size).all(|column| row == column || matrix.get(row, column).is_zero()))
        || (0..size)
            .all(|row| (0..size).all(|column| row == column || matrix.get(row, column).is_zero()));
    if size > 2 {
        if !triangular {
            return None;
        }
        let mut vectors = Vec::with_capacity(size);
        for index in 0..size {
            vectors.push(basis(arena, index));
        }
        return Some(arena.intern_call(list, &vectors));
    }
    let trace = matrix.get(0, 0).add(&matrix.get(1, 1));
    let determinant = matrix
        .get(0, 0)
        .mul(&matrix.get(1, 1))
        .sub(&matrix.get(0, 1).mul(&matrix.get(1, 0)));
    let four = Rat::from_i64(4, 1).ok()?;
    let discriminant = trace.mul(&trace).sub(&four.mul(&determinant));
    if discriminant.sign() >= 0
        && let Some(root) = rational_square_root(&discriminant)
    {
        let two = Rat::from_i64(2, 1).ok()?;
        let first = trace.sub(&root).div(&two).ok()?;
        let second = trace.add(&root).div(&two).ok()?;
        let mut vectors = Vec::with_capacity(2);
        for eigenvalue in [first, second] {
            let a = matrix.get(0, 0).sub(&eigenvalue);
            let b = matrix.get(0, 1);
            let c = matrix.get(1, 0);
            let d = matrix.get(1, 1).sub(&eigenvalue);
            let values = if !b.is_zero() {
                vec![b, eigenvalue.sub(&matrix.get(0, 0))]
            } else if !c.is_zero() {
                vec![eigenvalue.sub(&matrix.get(1, 1)), c.neg()]
            } else if a.is_zero() {
                vec![Rat::one(), Rat::zero()]
            } else if d.is_zero() {
                vec![Rat::zero(), Rat::one()]
            } else {
                vec![Rat::one(), Rat::zero()]
            };
            vectors.push(vector_expression(arena, &values));
        }
        return Some(arena.intern_call(list, &vectors));
    }

    let trace_expression = rational_expression(arena, &trace);
    let discriminant_expression = rational_expression(arena, &discriminant);
    let root = if discriminant.sign() >= 0 {
        sqrt_expression_simplified(arena, discriminant_expression)
    } else {
        let magnitude = rational_expression(arena, &discriminant.neg());
        let sqrt = sqrt_expression_simplified(arena, magnitude);
        let imaginary_symbol = arena.symbol_or_intern("i");
        let imaginary = arena.symbol_expr(imaginary_symbol);
        expression_multiply(arena, imaginary, sqrt)
    };
    let two = arena.intern_int(Int::from_i64(2));
    let half = expression_reciprocal(arena, two);
    let first_numerator = expression_subtract(arena, trace_expression, root);
    let first = expression_multiply(arena, first_numerator, half);
    let second_numerator = expression_add(arena, trace_expression, root);
    let second = expression_multiply(arena, second_numerator, half);
    let a = rational_expression(arena, &matrix.get(0, 0));
    let b = rational_expression(arena, &matrix.get(0, 1));
    let c = rational_expression(arena, &matrix.get(1, 0));
    let d = rational_expression(arena, &matrix.get(1, 1));
    let mut vectors = Vec::with_capacity(2);
    for eigenvalue in [first, second] {
        let values = if !matrix.get(0, 1).is_zero() {
            let component = expression_subtract(arena, eigenvalue, a);
            vec![b, component]
        } else if !matrix.get(1, 0).is_zero() {
            let component = expression_subtract(arena, eigenvalue, d);
            let negative = expression_negate(arena, c);
            vec![component, negative]
        } else {
            vec![arena.intern_int(Int::one()), arena.intern_int(Int::zero())]
        };
        vectors.push(arena.intern_call(list, &values));
    }
    Some(arena.intern_call(list, &vectors))
}

fn sqrt_expression_simplified(arena: &mut Arena, expression: ExprId) -> ExprId {
    let expression = crate::eval::fold_numeric_call(arena, expression).unwrap_or(expression);
    if let Some(value) = as_rat(arena, expression)
        && let Some(root) = rational_square_root(&value)
    {
        return rational_expression(arena, &root);
    }
    let head = arena.builtin_head("sqrt");
    arena.intern_call(head, &[expression])
}

pub(crate) fn qr(arena: &mut Arena, expression: ExprId) -> Option<ExprId> {
    let matrix = matrix_from_expr(arena, expression)?;
    if matrix.rows < matrix.columns || matrix.columns == 0 || matrix.columns > 6 {
        return None;
    }
    if matrix.rank() < matrix.columns {
        return None;
    }
    let rows = matrix.rows;
    let columns = matrix.columns;
    let mut q_columns: Vec<Vec<ExprId>> = Vec::with_capacity(columns);
    let mut r = SymbolicMatrix::new(columns, columns);
    let zero = arena.intern_int(Int::zero());
    for entry in &mut r.entries {
        *entry = zero;
    }
    for column in 0..columns {
        let mut values = Vec::with_capacity(rows);
        for row in 0..rows {
            values.push(rational_expression(arena, &matrix.get(row, column)));
        }
        for (previous, q_column) in q_columns.iter().enumerate().take(column) {
            let mut products = Vec::with_capacity(rows);
            for (row, value) in values.iter().enumerate().take(rows) {
                products.push(expression_multiply(arena, q_column[row], *value));
            }
            let projection = expression_sum(arena, &products);
            r.set(previous, column, projection);
            for (row, value) in values.iter_mut().enumerate().take(rows) {
                let correction = expression_multiply(arena, projection, q_column[row]);
                *value = expression_subtract(arena, *value, correction);
            }
        }
        let mut squares = Vec::with_capacity(rows);
        for value in &values {
            squares.push(expression_multiply(arena, *value, *value));
        }
        let norm_squared = expression_sum(arena, &squares);
        let norm = sqrt_expression_simplified(arena, norm_squared);
        r.set(column, column, norm);
        let inverse = expression_reciprocal(arena, norm);
        let mut q_column = Vec::with_capacity(rows);
        for value in values {
            q_column.push(expression_multiply(arena, value, inverse));
        }
        q_columns.push(q_column);
    }
    let mut q = SymbolicMatrix::new(rows, columns);
    for (column, q_column) in q_columns.iter().enumerate().take(columns) {
        for (row, value) in q_column.iter().enumerate().take(rows) {
            q.set(row, column, *value);
        }
    }
    let list = arena.builtin_head("list");
    let q = symbolic_expression(arena, &q);
    let r = symbolic_expression(arena, &r);
    Some(arena.intern_call(list, &[q, r]))
}
