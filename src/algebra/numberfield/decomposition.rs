//! Prime decomposition and ideal factorization.
//!
//! When a generator has index prime to `p`, Dedekind factorization of its
//! minimal polynomial gives the prime ideals above `p`. When every generator
//! has index divisible by `p`, small residue algebras are split by enumerating
//! idempotents of `O / pO`. Larger cases return an explicit unsupported result.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::collections::BTreeSet;
use crate::num::Int;

use super::super::intmatrix::divides;
use super::super::poly_z::{
    ZPoly, factor_mod_p_with_multiplicity, invert_mod, poly_pow_mod, reduce_mod,
};
use super::super::ratmatrix::RatMatrix;
use super::element::FieldElement;
use super::ideal::Ideal;
use super::order::Order;

/// A generator of the maximal order and its minimal polynomial.
#[derive(Clone, Debug)]
pub(crate) struct MonogenicModel {
    pub(crate) generator: FieldElement,
    pub(crate) minimal_polynomial: ZPoly,
    pub(crate) index: Int,
}

/// A prime ideal above a rational prime.
#[derive(Clone, Debug)]
pub(crate) struct PrimeIdeal {
    ideal: Ideal,
    prime: Int,
    residue_degree: u32,
    ramification: u32,
    residue_modulus: Vec<Int>,
    frobenius: Vec<Int>,
}

impl PrimeIdeal {
    /// The underlying ideal lattice.
    pub(crate) fn ideal(&self) -> &Ideal {
        &self.ideal
    }

    /// The rational prime below this ideal.
    pub(crate) fn prime(&self) -> &Int {
        &self.prime
    }

    /// The residue degree.
    pub(crate) fn residue_degree(&self) -> u32 {
        self.residue_degree
    }

    /// The ramification index.
    pub(crate) fn ramification_index(&self) -> u32 {
        self.ramification
    }

    /// The ideal norm, equal to `p^f`.
    pub(crate) fn norm(&self) -> &Int {
        self.ideal.norm()
    }

    /// The monic residue field modulus over `F_p`.
    pub(crate) fn residue_modulus(&self) -> &[Int] {
        &self.residue_modulus
    }

    /// The Frobenius polynomial `x^p` modulo the residue modulus.
    pub(crate) fn frobenius(&self) -> &[Int] {
        &self.frobenius
    }
}

/// Find a single generator of the maximal order.
pub(crate) fn find_monogenic_model(order: &Order) -> Option<MonogenicModel> {
    find_model_with_index(order, |index| index.is_one())
}

/// Find a generator whose index is not divisible by `prime`.
pub(crate) fn find_p_maximal_generator(order: &Order, prime: &Int) -> Option<MonogenicModel> {
    find_model_with_index(order, |index| {
        !crate::algebra::intmatrix::divides(prime, index)
    })
}

fn find_model_with_index<F>(order: &Order, accept: F) -> Option<MonogenicModel>
where
    F: Fn(&Int) -> bool,
{
    for coefficients in candidate_coordinates(order) {
        if let Some(model) = model_from_coordinates(order, &coefficients)
            && accept(&model.index)
        {
            return Some(model);
        }
    }
    None
}

fn candidate_coordinates(order: &Order) -> BTreeSet<Vec<Int>> {
    let size = order.field().degree();
    let mut candidates = BTreeSet::new();
    for index in 0..size {
        let mut basis = vec![Int::zero(); size];
        basis[index] = Int::one();
        candidates.insert(basis.clone());
        for shift in -2i64..=2 {
            let mut shifted = basis.clone();
            shifted[0] = shifted[0].add(&Int::from_i64(shift));
            candidates.insert(shifted);
        }
    }
    if size <= 6 {
        let total = 3usize.pow(size as u32);
        for mut code in 0usize..total {
            let mut coefficients = vec![Int::zero(); size];
            for entry in &mut coefficients {
                let digit = code % 3;
                code /= 3;
                *entry = Int::from_i64(digit as i64 - 1);
            }
            if coefficients.iter().all(Int::is_zero) {
                continue;
            }
            candidates.insert(coefficients);
        }
    } else {
        for left in 0..size {
            for right in left + 1..size {
                for sign in [-1i64, 1] {
                    let mut coefficients = vec![Int::zero(); size];
                    coefficients[left] = Int::one();
                    coefficients[right] = Int::from_i64(sign);
                    candidates.insert(coefficients);
                }
            }
        }
    }
    candidates
}

fn model_from_coordinates(order: &Order, coefficients: &[Int]) -> Option<MonogenicModel> {
    let field = order.field();
    let size = field.degree();
    let generator = order.element_from_coordinates(coefficients);
    let minimal = field.minimal_polynomial(&generator)?;
    if minimal.len() != size + 1 {
        return None;
    }
    let minimal_polynomial = ZPoly::from_rat_coeffs(&minimal)?;
    if minimal_polynomial.degree() != Some(size) || !minimal_polynomial.is_monic() {
        return None;
    }
    let descending: Vec<crate::num::Rat> = minimal.iter().rev().cloned().collect();
    let value = field.evaluate_descending(&descending, generator.coefficients());
    if !value.iter().all(crate::num::Rat::is_zero) {
        return None;
    }
    let mut powers = Vec::with_capacity(size);
    let mut power = field.one();
    for _ in 0..size {
        powers.push(power.coefficients().to_vec());
        power = field.multiply(&power, &generator);
    }
    let lattice = RatMatrix::from_columns(powers)?;
    let coordinates = order.basis().inverse()?.multiply(&lattice)?;
    if !coordinates
        .to_rows()
        .iter()
        .flatten()
        .all(|value| value.den().is_one())
    {
        return None;
    }
    let integer_coordinates = RatMatrix::from_rows(
        coordinates
            .to_rows()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|value| {
                        crate::num::Rat::from_ints(value.num(), crate::num::Int::one())
                            .unwrap_or_else(|_| crate::num::Rat::zero())
                    })
                    .collect()
            })
            .collect(),
    )?;
    let determinant = integer_coordinates.determinant()?.abs();
    if !determinant.den().is_one() {
        return None;
    }
    let index = determinant.num();
    Some(MonogenicModel {
        generator,
        minimal_polynomial,
        index,
    })
}

/// The prime ideals above a rational prime.
pub(crate) fn decompose_prime(
    order: &Order,
    model: &MonogenicModel,
    prime: &Int,
) -> Option<Vec<PrimeIdeal>> {
    if divides(prime, &model.index) {
        return None;
    }
    let size = order.field().degree();
    let factors = factor_mod_p_with_multiplicity(&model.minimal_polynomial, prime)?;
    let mut result = Vec::with_capacity(factors.len());
    let mut total = 0u32;
    for (factor, multiplicity) in factors {
        let degree = factor.degree()? as u32;
        total += degree * multiplicity;
        let mut value = order.field().zero();
        for coefficient in factor.coeffs.iter().rev() {
            value = order.field().multiply(&value, &model.generator);
            value = order.field().add(
                &value,
                &order.field().element_from_integer(coefficient.clone()),
            );
        }
        let mut generators = Vec::with_capacity(size * 2);
        for index in 0..size {
            let mut column = vec![Int::zero(); size];
            column[index] = prime.clone();
            generators.push(column);
        }
        // The ideal (p, g(beta)) is generated as an O-module, so include
        // g(beta) times every order basis element.
        for index in 0..size {
            let mut basis = vec![Int::zero(); size];
            basis[index] = Int::one();
            let basis_element = order.element_from_coordinates(&basis);
            let product = order.field().multiply(&value, &basis_element);
            let coordinates = order.coordinates_of(&product)?;
            let mut column = Vec::with_capacity(size);
            for entry in coordinates {
                if !entry.den().is_one() {
                    return None;
                }
                column.push(entry.num());
            }
            generators.push(column);
        }
        let ideal = Ideal::from_generators(&generators, size)?;
        if !generators.iter().all(|generator| ideal.contains(generator)) {
            return None;
        }
        let expected = prime.pow_u32(degree);
        if ideal.norm() != &expected {
            return None;
        }
        let frobenius = poly_pow_mod(&[Int::zero(), Int::one()], prime, &factor.coeffs, prime)?;
        let frobenius = if frobenius.is_empty() {
            vec![Int::zero()]
        } else {
            frobenius
        };
        result.push(PrimeIdeal {
            ideal,
            prime: prime.clone(),
            residue_degree: degree,
            ramification: multiplicity,
            residue_modulus: factor.coeffs,
            frobenius,
        });
    }
    if total != size as u32 {
        return None;
    }
    Some(result)
}

/// Prime decomposition without requiring a global monogenic generator.
///
/// Tries a generator whose index is prime to `p`, which is enough for
/// Dedekind factorization. When every generator has index divisible by `p`,
/// the finite `F_p` algebra fallback is used for small residue algebras.
pub(crate) fn decompose_prime_general(order: &Order, prime: &Int) -> Option<Vec<PrimeIdeal>> {
    let total = prime.to_i64()?.checked_pow(order.field().degree() as u32)?;
    if total > 0 && total <= 10000 {
        return brute_force_prime_ideals(order, prime);
    }
    if let Some(model) = find_p_maximal_generator(order, prime)
        && let Some(result) = decompose_prime(order, &model, prime)
    {
        return Some(result);
    }
    brute_force_prime_ideals(order, prime)
}

fn brute_force_prime_ideals(order: &Order, prime: &Int) -> Option<Vec<PrimeIdeal>> {
    let size = order.field().degree();
    let total = prime.to_i64()?.checked_pow(size as u32)?;
    if total <= 0 || total > 10000 {
        return None;
    }
    let total = total as usize;
    let one = order.coordinates_of(&order.field().one())?;
    let mut one_vector = Vec::with_capacity(size);
    for value in one {
        if !value.den().is_one() {
            return None;
        }
        one_vector.push(reduce_mod(&value.num(), prime));
    }
    let zero = vec![Int::zero(); size];
    let mut idempotents = Vec::new();
    for code in 0..total {
        let vector = decode_vector(code, prime, size)?;
        if vector == zero {
            continue;
        }
        if multiply_vectors(order, &vector, &vector, prime) == vector {
            idempotents.push(vector);
        }
    }
    let mut primitive = Vec::new();
    for candidate in &idempotents {
        if candidate == &zero {
            continue;
        }
        let mut is_primitive = true;
        for other in &idempotents {
            if other == &zero || other == candidate {
                continue;
            }
            if multiply_vectors(order, candidate, other, prime) == *other {
                is_primitive = false;
                break;
            }
        }
        if is_primitive {
            primitive.push(candidate.clone());
        }
    }
    if primitive.is_empty() {
        primitive.push(one_vector.clone());
    }
    let mut sum = zero.clone();
    for idempotent in &primitive {
        sum = add_vectors(&sum, idempotent, prime);
    }
    if sum != one_vector {
        return None;
    }
    for left in 0..primitive.len() {
        for right in left + 1..primitive.len() {
            if multiply_vectors(order, &primitive[left], &primitive[right], prime) != zero {
                return None;
            }
        }
    }
    let mut units = Vec::with_capacity(size);
    for index in 0..size {
        let mut unit = vec![Int::zero(); size];
        unit[index] = Int::one();
        units.push(unit);
    }
    let mut p_generators = Vec::with_capacity(size);
    for index in 0..size {
        let mut column = vec![Int::zero(); size];
        column[index] = prime.clone();
        p_generators.push(column);
    }
    let p_ideal = Ideal::from_generators(&p_generators, size)?;
    let mut result = Vec::with_capacity(primitive.len());
    for idempotent in primitive {
        let columns = units
            .iter()
            .map(|unit| multiply_vectors(order, &idempotent, unit, prime))
            .collect::<Vec<_>>();
        let degree = rank_mod_prime(&columns, prime)? as u32;
        if degree == 0 {
            return None;
        }
        let complement = sub_vectors(&one_vector, &idempotent, prime);
        let mut generators = p_generators.clone();
        for unit in &units {
            generators.push(multiply_vectors(order, &complement, unit, prime));
        }
        let ideal = Ideal::from_generators(&generators, size)?;
        if ideal.norm() != &prime.pow_u32(degree) {
            return None;
        }
        let modulus = find_residue_modulus(order, &idempotent, degree, prime, total)?;
        let frobenius = poly_pow_mod(&[Int::zero(), Int::one()], prime, &modulus, prime)?;
        let frobenius = if frobenius.is_empty() {
            vec![Int::zero()]
        } else {
            frobenius
        };
        let ramification = prime_valuation(order, &p_ideal, &ideal)?;
        if ramification == 0 {
            return None;
        }
        result.push(PrimeIdeal {
            ideal,
            prime: prime.clone(),
            residue_degree: degree,
            ramification,
            residue_modulus: modulus,
            frobenius,
        });
    }
    let total_degree: u32 = result
        .iter()
        .map(|ideal| ideal.residue_degree * ideal.ramification)
        .sum();
    if total_degree != size as u32 {
        return None;
    }
    Some(result)
}

fn find_residue_modulus(
    order: &Order,
    idempotent: &[Int],
    degree: u32,
    prime: &Int,
    total: usize,
) -> Option<Vec<Int>> {
    let size = order.field().degree();
    for code in 0..total {
        let candidate = decode_vector(code, prime, size)?;
        let projected = multiply_vectors(order, &candidate, idempotent, prime);
        if projected.iter().all(Int::is_zero) {
            continue;
        }
        let polynomial = minimal_polynomial_mod_prime(order, &projected, idempotent, prime)?;
        if polynomial.len() - 1 == degree as usize {
            return Some(polynomial);
        }
    }
    None
}

fn minimal_polynomial_mod_prime(
    order: &Order,
    element: &[Int],
    identity: &[Int],
    prime: &Int,
) -> Option<Vec<Int>> {
    let size = order.field().degree();
    let mut powers = vec![identity.to_vec()];
    let mut current = identity.to_vec();
    for _ in 0..size {
        current = multiply_vectors(order, &current, element, prime);
        powers.push(current.clone());
        let nullspace = nullspace_mod_prime(&powers, prime)?;
        for vector in nullspace {
            let Some(last) = vector.last() else {
                continue;
            };
            if last.is_zero() {
                continue;
            }
            let inverse = invert_mod(last, prime)?;
            return Some(
                vector
                    .iter()
                    .map(|value| reduce_mod(&value.mul(&inverse), prime))
                    .collect(),
            );
        }
    }
    None
}

fn prime_valuation(order: &Order, ideal: &Ideal, prime_ideal: &Ideal) -> Option<u32> {
    let mut power = Ideal::identity(order)?;
    let mut exponent = 0u32;
    loop {
        let next = power.multiply(order, prime_ideal)?;
        if ideal.is_subset_of(&next) {
            power = next;
            exponent += 1;
            if exponent > 1024 {
                return None;
            }
        } else {
            break;
        }
    }
    Some(exponent)
}

fn decode_vector(code: usize, prime: &Int, size: usize) -> Option<Vec<Int>> {
    let radix = prime.to_i64()? as usize;
    let mut code = code;
    let mut vector = Vec::with_capacity(size);
    for _ in 0..size {
        vector.push(Int::from_i64((code % radix) as i64));
        code /= radix;
    }
    Some(vector)
}

fn multiply_vectors(order: &Order, left: &[Int], right: &[Int], prime: &Int) -> Vec<Int> {
    order
        .multiply_coordinates(left, right)
        .iter()
        .map(|value| reduce_mod(value, prime))
        .collect()
}

fn add_vectors(left: &[Int], right: &[Int], prime: &Int) -> Vec<Int> {
    left.iter()
        .zip(right)
        .map(|(a, b)| reduce_mod(&a.add(b), prime))
        .collect()
}

fn sub_vectors(left: &[Int], right: &[Int], prime: &Int) -> Vec<Int> {
    left.iter()
        .zip(right)
        .map(|(a, b)| reduce_mod(&a.sub(b), prime))
        .collect()
}

fn nullspace_mod_prime(columns: &[Vec<Int>], prime: &Int) -> Option<Vec<Vec<Int>>> {
    let rows = columns.first().map_or(0, Vec::len);
    let column_count = columns.len();
    if rows == 0 || column_count == 0 {
        return None;
    }
    let mut matrix = vec![vec![Int::zero(); column_count]; rows];
    for column in 0..column_count {
        for row in 0..rows {
            matrix[row][column] = reduce_mod(&columns[column][row], prime);
        }
    }
    let (reduced, pivots) = rref_mod(matrix, prime)?;
    let mut free = Vec::new();
    for column in 0..column_count {
        if !pivots.contains(&column) {
            free.push(column);
        }
    }
    let mut basis = Vec::with_capacity(free.len());
    for free_column in free {
        let mut vector = vec![Int::zero(); column_count];
        vector[free_column] = Int::one();
        for (row, pivot_column) in pivots.iter().enumerate() {
            vector[*pivot_column] = reduce_mod(&reduced[row][free_column].neg(), prime);
        }
        basis.push(vector);
    }
    Some(basis)
}

fn rank_mod_prime(columns: &[Vec<Int>], prime: &Int) -> Option<usize> {
    let rows = columns.first().map_or(0, Vec::len);
    let column_count = columns.len();
    if rows == 0 || column_count == 0 {
        return Some(0);
    }
    let mut matrix = vec![vec![Int::zero(); column_count]; rows];
    for column in 0..column_count {
        for row in 0..rows {
            matrix[row][column] = reduce_mod(&columns[column][row], prime);
        }
    }
    Some(rref_mod(matrix, prime)?.1.len())
}

#[allow(clippy::needless_range_loop)]
fn rref_mod(mut matrix: Vec<Vec<Int>>, prime: &Int) -> Option<(Vec<Vec<Int>>, Vec<usize>)> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut pivots = Vec::new();
    let mut pivot_row = 0usize;
    for column in 0..columns {
        if pivot_row >= rows {
            break;
        }
        let Some(found) = (pivot_row..rows).find(|row| !matrix[*row][column].is_zero()) else {
            continue;
        };
        matrix.swap(pivot_row, found);
        let inverse = invert_mod(&matrix[pivot_row][column], prime)?;
        for entry in 0..columns {
            matrix[pivot_row][entry] = reduce_mod(&matrix[pivot_row][entry].mul(&inverse), prime);
        }
        for row in 0..rows {
            if row == pivot_row {
                continue;
            }
            let factor = matrix[row][column].clone();
            if factor.is_zero() {
                continue;
            }
            for entry in 0..columns {
                matrix[row][entry] = reduce_mod(
                    &matrix[row][entry].sub(&factor.mul(&matrix[pivot_row][entry])),
                    prime,
                );
            }
        }
        pivots.push(column);
        pivot_row += 1;
    }
    Some((matrix, pivots))
}

/// Factor an integral ideal into prime ideals.
pub(crate) fn factor_ideal(order: &Order, ideal: &Ideal) -> Option<Vec<(PrimeIdeal, u32)>> {
    let norm = ideal.norm().to_i64()?;
    if norm <= 0 {
        return None;
    }
    let primes = crate::numbertheory::factor_u64(norm as u64);
    let mut result = Vec::new();
    let mut product = Ideal::identity(order)?;
    for (prime, _) in primes {
        let prime = Int::from_i64(prime as i64);
        let ideals = decompose_prime_general(order, &prime)?;
        for prime_ideal in ideals {
            let exponent = prime_valuation(order, ideal, prime_ideal.ideal())?;
            if exponent == 0 {
                continue;
            }
            let power = prime_ideal.ideal().power(order, exponent)?;
            product = product.multiply(order, &power)?;
            result.push((prime_ideal, exponent));
        }
    }
    if !product.equals(ideal) {
        return None;
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::super::field::NumberField;
    use super::super::integral_basis::maximal_order;
    use super::*;
    use crate::num::Rat;

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

    fn summarise(ideals: &[PrimeIdeal]) -> Vec<(i64, u32, u32, i64)> {
        ideals
            .iter()
            .map(|ideal| {
                (
                    ideal.prime().to_i64().unwrap(),
                    ideal.ramification_index(),
                    ideal.residue_degree(),
                    ideal.norm().to_i64().unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn sqrt_five_splitting_types() {
        let field = field(&[-5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        assert_eq!(model.minimal_polynomial.degree(), Some(2));

        let inert = decompose_prime(&order, &model, &Int::from_i64(2)).unwrap();
        assert_eq!(summarise(&inert), vec![(2, 1, 2, 4)]);

        let ramified = decompose_prime(&order, &model, &Int::from_i64(5)).unwrap();
        assert_eq!(summarise(&ramified), vec![(5, 2, 1, 5)]);

        let split = decompose_prime(&order, &model, &Int::from_i64(11)).unwrap();
        assert_eq!(split.len(), 2);
        assert_eq!(
            split
                .iter()
                .map(|ideal| ideal.ramification_index())
                .sum::<u32>(),
            2
        );
        assert!(split.iter().all(|ideal| ideal.residue_degree() == 1));
    }

    #[test]
    fn cyclotomic_fifth_field_splitting_types() {
        let field = field(&[1, 1, 1, 1, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        let ramified = decompose_prime(&order, &model, &Int::from_i64(5)).unwrap();
        assert_eq!(summarise(&ramified), vec![(5, 4, 1, 5)]);
        let split = decompose_prime(&order, &model, &Int::from_i64(11)).unwrap();
        assert_eq!(split.len(), 4);
        assert!(split.iter().all(|ideal| ideal.ramification_index() == 1));
        assert!(split.iter().all(|ideal| ideal.residue_degree() == 1));
    }

    #[test]
    fn residue_degree_and_frobenius_are_consistent() {
        let field = field(&[-5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        for prime in [2i64, 5, 11] {
            let prime = Int::from_i64(prime);
            let ideals = decompose_prime(&order, &model, &prime).unwrap();
            for ideal in ideals {
                assert!(!ideal.frobenius().is_empty());
                assert_eq!(ideal.norm(), &prime.pow_u32(ideal.residue_degree()));
                assert_eq!(
                    ideal.residue_modulus().len() - 1,
                    ideal.residue_degree() as usize
                );
            }
        }
    }

    #[test]
    fn cubic_field_decomposition() {
        let field = field(&[-2, 0, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        for prime in [2i64, 3, 5] {
            let prime = Int::from_i64(prime);
            let ideals = decompose_prime(&order, &model, &prime).unwrap();
            let total: u32 = ideals
                .iter()
                .map(|ideal| ideal.ramification_index() * ideal.residue_degree())
                .sum();
            assert_eq!(total, 3);
            assert!(ideals.iter().all(|ideal| ideal.ideal().basis().len() == 3));
        }
    }

    #[test]
    fn cyclotomic_fifth_prime_two_is_inert() {
        let field = field(&[1, 1, 1, 1, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        let ideals = decompose_prime(&order, &model, &Int::from_i64(2)).unwrap();
        assert_eq!(summarise(&ideals), vec![(2, 1, 4, 16)]);
    }

    #[test]
    fn prime_norms_multiply_to_the_field_norm() {
        for (coefficients, primes) in [
            (&[-5i64, 0, 1][..], [2i64, 5, 11].as_slice()),
            (&[5, 0, 1][..], [2i64, 3, 5].as_slice()),
            (&[1, 1, 1, 1, 1][..], [2i64, 5, 11].as_slice()),
            (&[-2, 0, 0, 1][..], [2i64, 3, 5].as_slice()),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let model = find_monogenic_model(&order).unwrap();
            for prime in primes {
                let prime = Int::from_i64(*prime);
                let ideals = decompose_prime(&order, &model, &prime).unwrap();
                let mut product = Int::one();
                for ideal in &ideals {
                    product = product.mul(&ideal.norm().pow_u32(ideal.ramification_index()));
                }
                assert_eq!(product, prime.pow_u32(field.degree() as u32));
            }
        }
    }

    #[test]
    fn dedekind_cubic_non_monogenic_decomposition() {
        let field = field(&[-8, -2, -1, 1]);
        let order = maximal_order(&field).unwrap();
        assert!(find_p_maximal_generator(&order, &Int::from_i64(2)).is_none());
        let ramified = decompose_prime_general(&order, &Int::from_i64(2)).unwrap();
        let mut types = summarise(&ramified);
        types.sort();
        assert_eq!(types, vec![(2, 1, 1, 2), (2, 1, 1, 2), (2, 1, 1, 2)]);

        let large = decompose_prime_general(&order, &Int::from_i64(503)).unwrap();
        let total: u32 = large
            .iter()
            .map(|ideal| ideal.ramification_index() * ideal.residue_degree())
            .sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn ideal_factorization_reconstructs_the_ideal() {
        for (coefficients, prime) in [
            (&[-5i64, 0, 1][..], 2i64),
            (&[-5, 0, 1][..], 5),
            (&[5, 0, 1][..], 2),
            (&[-8, -2, -1, 1][..], 2),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            let size = field.degree();
            let mut p_generators = Vec::new();
            for index in 0..size {
                let mut column = vec![Int::zero(); size];
                column[index] = Int::from_i64(prime);
                p_generators.push(column);
            }
            let ideal = Ideal::from_generators(&p_generators, size).unwrap();
            let factors = factor_ideal(&order, &ideal).unwrap();
            assert!(!factors.is_empty());
            let mut product = Ideal::identity(&order).unwrap();
            for (prime_ideal, exponent) in &factors {
                let power = prime_ideal.ideal().power(&order, *exponent).unwrap();
                product = product.multiply(&order, &power).unwrap();
                assert!(
                    prime_ideal
                        .ideal()
                        .is_subset_of(&Ideal::identity(&order).unwrap())
                );
            }
            assert!(product.equals(&ideal));
        }
    }

    #[test]
    fn dedekind_non_monogenic_order_has_no_generator() {
        let field = field(&[-8, -2, -1, 1]);
        let order = maximal_order(&field).unwrap();
        assert!(find_monogenic_model(&order).is_none());
    }

    #[test]
    fn generator_search_handles_a_half_integer_basis() {
        let field = field(&[-5, 0, 1]);
        let order = maximal_order(&field).unwrap();
        let model = find_monogenic_model(&order).unwrap();
        let coefficients = order.coordinates_of(&model.generator).unwrap();
        assert!(coefficients.iter().all(|value| value.den().is_one()));
        let _ = rational(1, 2);
    }
}
