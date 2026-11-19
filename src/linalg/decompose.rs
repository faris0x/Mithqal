//! Matrix factorisations and spectral operations.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

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

pub(crate) fn exact_eigenvalues(arena: &mut Arena, matrix: &Matrix) -> Option<ExprId> {
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

pub(crate) fn rational_square_root(value: &Rat) -> Option<Rat> {
    let numerator = value.num().to_i64()?;
    let denominator = value.den().to_i64()?;
    let numerator = integer_sqrt_i64(numerator)?;
    let denominator = integer_sqrt_i64(denominator)?;
    Rat::from_i64(numerator, denominator).ok()
}

pub(crate) fn integer_sqrt_i64(value: i64) -> Option<i64> {
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

pub(crate) fn sqrt_expression_simplified(arena: &mut Arena, expression: ExprId) -> ExprId {
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
