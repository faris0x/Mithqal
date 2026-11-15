//! Sparse and dense interpolation helpers for the multivariate stages.

use super::*;

pub(super) type SparseRoots = (Vec<(Rat, u32)>, Vec<Rat>);
pub(super) type SparseSample = (usize, Vec<(Rat, u32)>, usize);
pub(super) type SparseTerm = (usize, i128, Vec<(usize, u32)>);
pub(super) fn substitute_multivariate(
    polynomial: &Poly,
    horizontal: SymbolId,
    variables: &[SymbolId],
    values: &[Rat],
) -> (usize, Vec<Rat>) {
    if values.iter().all(|value| value.den().is_one())
        && let Some(result) =
            substitute_multivariate_i128(polynomial, horizontal, variables, values)
    {
        return result;
    }
    let mut maxima = vec![0u32; variables.len()];
    let mut degree = 0usize;
    for monomial in polynomial.terms.keys() {
        for (symbol, exponent) in &monomial.powers {
            if *symbol == horizontal {
                degree = degree.max(*exponent as usize);
            } else if let Some(index) = variables.iter().position(|variable| variable == symbol) {
                maxima[index] = maxima[index].max(*exponent);
            }
        }
    }
    let mut tables: Vec<Vec<Rat>> = Vec::with_capacity(variables.len());
    for (index, value) in values.iter().enumerate().take(variables.len()) {
        let mut powers = Vec::with_capacity(maxima[index] as usize + 1);
        let mut current = Rat::one();
        for _ in 0..=maxima[index] {
            powers.push(current.clone());
            current = current.mul(value);
        }
        tables.push(powers);
    }
    let mut coefficients = vec![Rat::zero(); degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let mut horizontal_power = 0usize;
        let mut factor = Rat::one();
        for (symbol, exponent) in &monomial.powers {
            if *symbol == horizontal {
                horizontal_power = *exponent as usize;
            } else if let Some(index) = variables.iter().position(|variable| variable == symbol) {
                factor = factor.mul(&tables[index][*exponent as usize]);
            }
        }
        coefficients[horizontal_power] =
            coefficients[horizontal_power].add(&coefficient.mul(&factor));
    }
    (degree, trim(coefficients))
}

pub(super) fn substitute_multivariate_i128(
    polynomial: &Poly,
    horizontal: SymbolId,
    variables: &[SymbolId],
    values: &[Rat],
) -> Option<(usize, Vec<Rat>)> {
    let (degree, coefficients) =
        substitute_multivariate_i128_values(polynomial, horizontal, variables, values)?;
    let mut converted = Vec::with_capacity(coefficients.len());
    for value in coefficients {
        converted.push(Rat::from_i64(i64::try_from(value).ok()?, 1).ok()?);
    }
    Some((degree, trim(converted)))
}

pub(super) fn substitute_multivariate_i128_values(
    polynomial: &Poly,
    horizontal: SymbolId,
    variables: &[SymbolId],
    values: &[Rat],
) -> Option<(usize, Vec<i128>)> {
    let mut maxima = vec![0u32; variables.len()];
    let mut degree = 0usize;
    for monomial in polynomial.terms.keys() {
        for (symbol, exponent) in &monomial.powers {
            if *symbol == horizontal {
                degree = degree.max(*exponent as usize);
            } else if let Some(index) = variables.iter().position(|variable| variable == symbol) {
                maxima[index] = maxima[index].max(*exponent);
            }
        }
    }
    let mut tables: Vec<Vec<i128>> = Vec::with_capacity(variables.len());
    for (index, value) in values.iter().enumerate().take(variables.len()) {
        let value = value.num().to_i128()?;
        let mut powers = Vec::with_capacity(maxima[index] as usize + 1);
        let mut current = 1i128;
        for _ in 0..=maxima[index] {
            powers.push(current);
            current = current.checked_mul(value)?;
        }
        tables.push(powers);
    }
    let mut coefficients = vec![0i128; degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        if !coefficient.den().is_one() {
            return None;
        }
        let coefficient = coefficient.num().to_i128()?;
        let mut horizontal_power = 0usize;
        let mut factor = 1i128;
        for (symbol, exponent) in &monomial.powers {
            if *symbol == horizontal {
                horizontal_power = *exponent as usize;
            } else if let Some(index) = variables.iter().position(|variable| variable == symbol) {
                factor = factor.checked_mul(tables[index][*exponent as usize])?;
            }
        }
        coefficients[horizontal_power] =
            coefficients[horizontal_power].checked_add(coefficient.checked_mul(factor)?)?;
    }
    Some((degree, coefficients))
}

pub(super) struct SparseEvaluator {
    degree: usize,
    terms: Vec<SparseTerm>,
}

impl SparseEvaluator {
    pub(super) fn new(
        polynomial: &Poly,
        horizontal: SymbolId,
        variables: &[SymbolId],
    ) -> Option<Self> {
        let mut degree = 0usize;
        let mut terms = Vec::with_capacity(polynomial.terms.len());
        for (monomial, coefficient) in &polynomial.terms {
            if !coefficient.den().is_one() {
                return None;
            }
            let coefficient = coefficient.num().to_i128()?;
            let mut horizontal_power = 0usize;
            let mut powers = Vec::new();
            for (symbol, exponent) in &monomial.powers {
                if *symbol == horizontal {
                    horizontal_power = *exponent as usize;
                } else {
                    let index = variables.iter().position(|variable| variable == symbol)?;
                    powers.push((index, *exponent));
                }
            }
            degree = degree.max(horizontal_power);
            terms.push((horizontal_power, coefficient, powers));
        }
        Some(SparseEvaluator { degree, terms })
    }

    pub(super) fn sample_roots(&self, point: &[Rat]) -> Option<SparseSample> {
        let mut coordinates = Vec::with_capacity(point.len());
        for coordinate in point {
            if !coordinate.den().is_one() {
                return None;
            }
            coordinates.push(coordinate.num().to_i128()?);
        }
        let mut coefficients = vec![0i128; self.degree + 1];
        for (horizontal_power, coefficient, powers) in &self.terms {
            let mut factor = 1i128;
            for (index, exponent) in powers {
                let mut value = 1i128;
                for _ in 0..*exponent {
                    value = value.checked_mul(coordinates[*index])?;
                }
                factor = factor.checked_mul(value)?;
            }
            coefficients[*horizontal_power] =
                coefficients[*horizontal_power].checked_add(coefficient.checked_mul(factor)?)?;
        }
        if let Some((roots, remaining)) = rational_roots_i128(&coefficients) {
            return Some((self.degree, roots, remaining.len()));
        }
        let mut converted = Vec::with_capacity(coefficients.len());
        for value in coefficients {
            converted.push(Rat::from_i64(i64::try_from(value).ok()?, 1).ok()?);
        }
        let (roots, remaining) = rational_roots(&converted);
        Some((self.degree, roots, remaining.len()))
    }
}

pub(super) fn sample_rational_roots(
    polynomial: &Poly,
    horizontal: SymbolId,
    variables: &[SymbolId],
    values: &[Rat],
) -> Option<SparseSample> {
    if let Some((degree, coefficients)) =
        substitute_multivariate_i128_values(polynomial, horizontal, variables, values)
        && let Some((roots, remaining)) = rational_roots_i128(&coefficients)
    {
        return Some((degree, roots, remaining.len()));
    }
    let (degree, coefficients) = substitute_multivariate(polynomial, horizontal, variables, values);
    let (roots, remaining) = rational_roots(&coefficients);
    Some((degree, roots, remaining.len()))
}

pub(super) fn rational_roots_i128(coefficients: &[i128]) -> Option<SparseRoots> {
    if coefficients.len() == 2 && coefficients[1] != 0 {
        let root = rat_from_i128(coefficients[0].checked_neg()?, coefficients[1])?;
        let leading = rat_from_i128(coefficients[1], 1)?;
        return Some((vec![(root, 1)], vec![leading]));
    }
    if coefficients.len() == 3 && coefficients[2] != 0 {
        let constant = coefficients[0];
        let linear = coefficients[1];
        let quadratic = coefficients[2];
        let discriminant = linear
            .checked_mul(linear)?
            .checked_sub(4i128.checked_mul(quadratic)?.checked_mul(constant)?)?;
        let leading = rat_from_i128(quadratic, 1)?;
        if discriminant < 0 {
            return Some((Vec::new(), vec![leading]));
        }
        let Some(root) = integer_sqrt_i128(discriminant) else {
            return Some((Vec::new(), vec![leading]));
        };
        let denominator = quadratic.checked_mul(2)?;
        let negative_linear = linear.checked_neg()?;
        if root == 0 {
            let value = rat_from_i128(negative_linear, denominator)?;
            return Some((vec![(value, 2)], vec![leading]));
        }
        let first = rat_from_i128(negative_linear.checked_add(root)?, denominator)?;
        let second = rat_from_i128(negative_linear.checked_sub(root)?, denominator)?;
        return Some((vec![(first, 1), (second, 1)], vec![leading]));
    }
    None
}

pub(super) fn rat_from_i128(numerator: i128, denominator: i128) -> Option<Rat> {
    if denominator == 0 {
        return None;
    }
    let numerator = i64::try_from(numerator).ok()?;
    let denominator = i64::try_from(denominator).ok()?;
    Rat::from_i64(numerator, denominator).ok()
}

pub(super) fn integer_sqrt_i128(value: i128) -> Option<i128> {
    if value < 0 {
        return None;
    }
    let root = (value as u128).isqrt();
    if root.checked_mul(root) == Some(value as u128) {
        i128::try_from(root).ok()
    } else {
        None
    }
}

pub(super) fn interpolate(points: &[(Rat, Rat)], degree: u32) -> Option<Vec<Rat>> {
    if points.is_empty() {
        return None;
    }
    let mut result = vec![Rat::zero()];
    for (index, (value, image)) in points.iter().enumerate() {
        let mut basis = vec![Rat::one()];
        let mut denominator = Rat::one();
        for (other, (other_value, _)) in points.iter().enumerate() {
            if other == index {
                continue;
            }
            basis = dense_multiply(&basis, &[other_value.neg(), Rat::one()]);
            denominator = denominator.mul(&value.sub(other_value));
        }
        let scale = image.div(&denominator).ok()?;
        if result.len() < basis.len() {
            result.resize(basis.len(), Rat::zero());
        }
        for (power, coefficient) in basis.iter().enumerate() {
            result[power] = result[power].add(&coefficient.mul(&scale));
        }
    }
    let result = trim(result);
    if result.len() as u32 > degree + 1 {
        return None;
    }
    Some(result)
}

pub(super) fn monomial_count(variables: usize, degree: u32) -> usize {
    let mut count = 1u128;
    for index in 0..variables {
        let numerator = degree as u128 + index as u128 + 1;
        count = count
            .checked_mul(numerator)
            .map(|value| value / (index as u128 + 1))
            .unwrap_or(u128::MAX);
    }
    usize::try_from(count).unwrap_or(usize::MAX)
}

pub(super) fn sparse_monomials(variables: usize, degree: u32) -> Vec<Vec<u32>> {
    let mut monomials = Vec::new();
    let mut current = vec![0u32; variables];
    for total in 0..=degree {
        generate_monomials(0, total, &mut current, &mut monomials);
    }
    monomials
}

pub(super) fn generate_monomials(
    index: usize,
    remaining: u32,
    current: &mut [u32],
    monomials: &mut Vec<Vec<u32>>,
) {
    if index + 1 == current.len() {
        current[index] = remaining;
        monomials.push(current.to_vec());
        return;
    }
    for value in 0..=remaining {
        current[index] = value;
        generate_monomials(index + 1, remaining - value, current, monomials);
    }
}

pub(super) fn vandermonde_float(points: &[Vec<Rat>], monomials: &[Vec<u32>]) -> Vec<Vec<f64>> {
    let variables = monomials
        .first()
        .map(|exponents| exponents.len())
        .unwrap_or(0);
    let mut maxima = vec![0u32; variables];
    for exponents in monomials {
        for (index, exponent) in exponents.iter().enumerate() {
            maxima[index] = maxima[index].max(*exponent);
        }
    }
    points
        .iter()
        .map(|point| {
            let tables: Vec<Vec<f64>> = point
                .iter()
                .enumerate()
                .take(variables)
                .map(|(index, coordinate)| {
                    let coordinate = coordinate.to_f64();
                    let mut values = Vec::with_capacity(maxima[index] as usize + 1);
                    let mut current = 1.0;
                    for _ in 0..=maxima[index] {
                        values.push(current);
                        current *= coordinate;
                    }
                    values
                })
                .collect();
            monomials
                .iter()
                .map(|exponents| {
                    let mut value = 1.0;
                    for (index, exponent) in exponents.iter().enumerate() {
                        if *exponent > 0 {
                            value *= tables[index][*exponent as usize];
                        }
                    }
                    value
                })
                .collect()
        })
        .collect()
}

pub(super) fn vandermonde(points: &[Vec<Rat>], monomials: &[Vec<u32>]) -> Vec<Vec<Rat>> {
    if let Some(matrix) = vandermonde_i128(points, monomials) {
        return matrix;
    }
    let variables = monomials
        .first()
        .map(|exponents| exponents.len())
        .unwrap_or(0);
    let mut maxima = vec![0u32; variables];
    for exponents in monomials {
        for (index, exponent) in exponents.iter().enumerate() {
            maxima[index] = maxima[index].max(*exponent);
        }
    }
    points
        .iter()
        .map(|point| {
            let tables: Vec<Vec<Rat>> = point
                .iter()
                .enumerate()
                .take(variables)
                .map(|(index, coordinate)| {
                    let mut values = Vec::with_capacity(maxima[index] as usize + 1);
                    let mut current = Rat::one();
                    for _ in 0..=maxima[index] {
                        values.push(current.clone());
                        current = current.mul(coordinate);
                    }
                    values
                })
                .collect();
            monomials
                .iter()
                .map(|exponents| {
                    let mut value = Rat::one();
                    for (index, exponent) in exponents.iter().enumerate() {
                        if *exponent > 0 {
                            value = value.mul(&tables[index][*exponent as usize]);
                        }
                    }
                    value
                })
                .collect()
        })
        .collect()
}

pub(super) fn vandermonde_i128(
    points: &[Vec<Rat>],
    monomials: &[Vec<u32>],
) -> Option<Vec<Vec<Rat>>> {
    let variables = monomials
        .first()
        .map(|exponents| exponents.len())
        .unwrap_or(0);
    let mut maxima = vec![0u32; variables];
    for exponents in monomials {
        for (index, exponent) in exponents.iter().enumerate() {
            maxima[index] = maxima[index].max(*exponent);
        }
    }
    let mut matrix = Vec::with_capacity(points.len());
    for point in points {
        let mut tables = Vec::with_capacity(variables);
        for (index, coordinate) in point.iter().enumerate().take(variables) {
            if !coordinate.den().is_one() {
                return None;
            }
            let coordinate = coordinate.num().to_i128()?;
            let mut values = Vec::with_capacity(maxima[index] as usize + 1);
            let mut current = 1i128;
            for _ in 0..=maxima[index] {
                values.push(current);
                current = current.checked_mul(coordinate)?;
            }
            tables.push(values);
        }
        let mut row = Vec::with_capacity(monomials.len());
        for exponents in monomials {
            let mut value = 1i128;
            for (index, exponent) in exponents.iter().enumerate() {
                if *exponent > 0 {
                    value = value.checked_mul(tables[index][*exponent as usize])?;
                }
            }
            row.push(Rat::from_i64(i64::try_from(value).ok()?, 1).ok()?);
        }
        matrix.push(row);
    }
    Some(matrix)
}

#[allow(clippy::needless_range_loop)]
pub(super) fn solve_linear_system_float(
    matrix: &[Vec<f64>],
    right_hand_sides: &[Vec<f64>],
) -> Option<Vec<Vec<f64>>> {
    let size = matrix.len();
    let sides = right_hand_sides.len();
    if size == 0 || right_hand_sides.iter().any(|side| side.len() != size) {
        return None;
    }
    let width = size.checked_add(sides)?;
    let mut augmented = vec![vec![0.0f64; width]; size];
    for row in 0..size {
        for column in 0..size {
            augmented[row][column] = matrix[row][column];
        }
        for (side, values) in right_hand_sides.iter().enumerate() {
            augmented[row][size + side] = values[row];
        }
    }
    for column in 0..size {
        let mut pivot = column;
        let mut magnitude = augmented[column][column].abs();
        for row in (column + 1)..size {
            let candidate = augmented[row][column].abs();
            if candidate > magnitude {
                magnitude = candidate;
                pivot = row;
            }
        }
        if magnitude < 1e-12 {
            return None;
        }
        if pivot != column {
            augmented.swap(column, pivot);
        }
        let inverse = 1.0 / augmented[column][column];
        for value in augmented[column].iter_mut().skip(column) {
            *value *= inverse;
        }
        for row in 0..size {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            if factor == 0.0 {
                continue;
            }
            let pivot_row = augmented[column].clone();
            for (value, pivot_value) in augmented[row].iter_mut().zip(pivot_row.iter()).skip(column)
            {
                *value -= factor * pivot_value;
            }
        }
    }
    let mut solutions = Vec::with_capacity(sides);
    for side in 0..sides {
        let mut values = Vec::with_capacity(size);
        for row in 0..size {
            values.push(augmented[row][size + side]);
        }
        solutions.push(values);
    }
    Some(solutions)
}

pub(super) fn integral_solutions(solutions: &[Vec<f64>]) -> Option<Vec<Vec<Rat>>> {
    let mut converted = Vec::with_capacity(solutions.len());
    for solution in solutions {
        let mut values = Vec::with_capacity(solution.len());
        for value in solution {
            if !value.is_finite() {
                return None;
            }
            let rounded = value.round();
            if (value - rounded).abs() > 1e-7 * value.abs().max(1.0) {
                return None;
            }
            let value = i64::try_from(rounded as i128).ok()?;
            values.push(Rat::from_i64(value, 1).ok()?);
        }
        converted.push(values);
    }
    Some(converted)
}

pub(super) fn solve_linear_system_multi(
    mut matrix: Vec<Vec<Rat>>,
    right_hand_sides: &[Vec<Rat>],
) -> Option<Vec<Vec<Rat>>> {
    let size = matrix.len();
    if size == 0 || right_hand_sides.iter().any(|side| side.len() != size) {
        return None;
    }
    let mut solutions = right_hand_sides.to_vec();
    for column in 0..size {
        let pivot = (column..size).find(|row| !matrix[*row][column].is_zero())?;
        matrix.swap(column, pivot);
        for side in &mut solutions {
            side.swap(column, pivot);
        }
        let inverse = Rat::one().div(&matrix[column][column]).ok()?;
        for value in matrix[column].iter_mut().skip(column) {
            *value = value.mul(&inverse);
        }
        for side in &mut solutions {
            side[column] = side[column].mul(&inverse);
        }
        for row in 0..size {
            if row == column || matrix[row][column].is_zero() {
                continue;
            }
            let factor = matrix[row][column].clone();
            let pivot = matrix[column].clone();
            for (index, value) in matrix[row].iter_mut().enumerate().skip(column) {
                *value = value.sub(&factor.mul(&pivot[index]));
            }
            for side in &mut solutions {
                side[row] = side[row].sub(&factor.mul(&side[column]));
            }
        }
    }
    Some(solutions)
}

pub(super) fn tensor_interpolate(
    axes: &[Vec<Rat>],
    values: &[Rat],
    grid: usize,
    degree: u32,
) -> Option<Vec<Rat>> {
    let mut data = values.to_vec();
    let mut stride = 1usize;
    for axis in axes {
        let block = stride.checked_mul(grid)?;
        let outer = data.len().checked_div(block)?;
        let mut next = vec![Rat::zero(); data.len()];
        for outer_index in 0..outer {
            for inner in 0..stride {
                let points: Vec<(Rat, Rat)> = (0..grid)
                    .map(|index| {
                        (
                            axis[index].clone(),
                            data[outer_index * block + index * stride + inner].clone(),
                        )
                    })
                    .collect();
                let coefficients = interpolate(&points, degree)?;
                for (index, coefficient) in coefficients.iter().enumerate() {
                    if index < grid {
                        next[outer_index * block + index * stride + inner] = coefficient.clone();
                    }
                }
            }
        }
        data = next;
        stride = block;
    }
    Some(data)
}
