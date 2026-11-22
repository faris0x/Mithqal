//! Exact number field elements, norms, traces, and minimal polynomials.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::super::dense::{dense_div_rem, dense_multiply, lcm, trim};
use super::super::intmatrix::div_exact;
use super::super::poly_z::ZPoly;
use super::super::ratmatrix::{RatMatrix, rat_from_int};
use super::field::NumberField;

/// An element of a number field in the power basis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FieldElement {
    pub(super) coefficients: Vec<Rat>,
}

impl FieldElement {
    /// The power basis coefficients.
    pub(crate) fn coefficients(&self) -> &[Rat] {
        &self.coefficients
    }

    /// True when the element is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.coefficients.iter().all(Rat::is_zero)
    }

    /// True when the element is one.
    pub(crate) fn is_one(&self) -> bool {
        self.coefficients.first().is_some_and(Rat::is_one)
            && self.coefficients.iter().skip(1).all(Rat::is_zero)
    }
}

impl NumberField {
    /// Build an element and reduce it modulo the defining polynomial.
    pub(crate) fn element(&self, coefficients: Vec<Rat>) -> FieldElement {
        FieldElement {
            coefficients: self.reduce(coefficients),
        }
    }

    /// The zero element.
    pub(crate) fn zero(&self) -> FieldElement {
        self.element(Vec::new())
    }

    /// The one element.
    pub(crate) fn one(&self) -> FieldElement {
        self.element(vec![Rat::one()])
    }

    /// Build an element from a rational number.
    pub(crate) fn element_from_rational(&self, value: Rat) -> FieldElement {
        self.element(vec![value])
    }

    /// Build an element from an integer.
    pub(crate) fn element_from_integer(&self, value: Int) -> FieldElement {
        self.element_from_rational(rat_from_int(&value))
    }

    /// The sum of two elements.
    pub(crate) fn add(&self, left: &FieldElement, right: &FieldElement) -> FieldElement {
        let coefficients = left
            .coefficients
            .iter()
            .zip(&right.coefficients)
            .map(|(a, b)| a.add(b))
            .collect();
        self.element(coefficients)
    }

    /// The difference of two elements.
    pub(crate) fn sub(&self, left: &FieldElement, right: &FieldElement) -> FieldElement {
        let coefficients = left
            .coefficients
            .iter()
            .zip(&right.coefficients)
            .map(|(a, b)| a.sub(b))
            .collect();
        self.element(coefficients)
    }

    /// The negation of an element.
    pub(crate) fn negate(&self, element: &FieldElement) -> FieldElement {
        self.element(element.coefficients.iter().map(Rat::neg).collect())
    }

    /// The product of two elements.
    pub(crate) fn multiply(&self, left: &FieldElement, right: &FieldElement) -> FieldElement {
        FieldElement {
            coefficients: self.multiply_coefficients(&left.coefficients, &right.coefficients),
        }
    }

    /// The inverse, or `None` for zero.
    pub(crate) fn inverse(&self, element: &FieldElement) -> Option<FieldElement> {
        let coefficients = trim(element.coefficients.clone());
        if coefficients.is_empty() {
            return None;
        }
        let defining = self.defining.to_rat_coeffs();
        let (gcd, mutual, _) = poly_extended_gcd(&coefficients, &defining)?;
        if gcd.len() != 1 || gcd[0].is_zero() {
            return None;
        }
        let reciprocal = Rat::one().div(&gcd[0]).ok()?;
        Some(FieldElement {
            coefficients: self.reduce(poly_scale(&mutual, &reciprocal)),
        })
    }

    /// The quotient, or `None` when the divisor is zero.
    pub(crate) fn divide(
        &self,
        numerator: &FieldElement,
        denominator: &FieldElement,
    ) -> Option<FieldElement> {
        let inverse = self.inverse(denominator)?;
        Some(self.multiply(numerator, &inverse))
    }

    /// A nonnegative power.
    pub(crate) fn power(&self, element: &FieldElement, exponent: u32) -> FieldElement {
        let mut result = self.one();
        let mut factor = element.clone();
        let mut exponent = exponent;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = self.multiply(&result, &factor);
            }
            exponent >>= 1;
            if exponent > 0 {
                factor = self.multiply(&factor, &factor);
            }
        }
        result
    }

    /// The multiplication matrix in the power basis.
    ///
    /// Column `j` is the coefficient vector of `alpha^j * element`.
    pub(crate) fn multiplication_matrix(&self, element: &FieldElement) -> RatMatrix {
        let degree = self.degree;
        let defining = self.defining.to_rat_coeffs();
        let mut columns = Vec::with_capacity(degree);
        let mut current = element.coefficients.clone();
        for _ in 0..degree {
            columns.push(current.clone());
            let top = current[degree - 1].clone();
            let mut next = vec![Rat::zero(); degree];
            for (target, source) in next.iter_mut().skip(1).zip(&current) {
                *target = source.clone();
            }
            for index in 0..degree {
                next[index] = next[index].sub(&top.mul(&defining[index]));
            }
            current = next;
        }
        RatMatrix::from_columns(columns).unwrap_or_else(|| RatMatrix::new(degree, degree))
    }

    /// The norm.
    pub(crate) fn norm(&self, element: &FieldElement) -> Option<Rat> {
        let value = self.multiplication_matrix(element).determinant();
        #[cfg(debug_assertions)]
        if let Some(expected) = self.norm_via_resultant(element) {
            debug_assert_eq!(value, Some(expected), "norm disagreement");
        }
        value
    }

    /// The norm computed independently through a resultant.
    ///
    /// For monic `f`, `Norm(a) = Res(f, a)`. Clearing denominators scales the
    /// resultant by the denominator raised to the field degree.
    pub(crate) fn norm_via_resultant(&self, element: &FieldElement) -> Option<Rat> {
        let mut denominator = Int::one();
        for value in &element.coefficients {
            denominator = lcm(&denominator, &value.den());
        }
        let mut coefficients = Vec::with_capacity(element.coefficients.len());
        for value in &element.coefficients {
            let scale = div_exact(&denominator, &value.den());
            coefficients.push(value.num().mul(&scale));
        }
        let polynomial = ZPoly::new(coefficients);
        let resultant = self.defining.resultant(&polynomial)?;
        let scale = denominator.pow_u32(self.degree as u32);
        Rat::from_ints(resultant, scale).ok()
    }

    /// The trace.
    pub(crate) fn trace(&self, element: &FieldElement) -> Option<Rat> {
        self.multiplication_matrix(element).trace()
    }

    /// The characteristic polynomial in descending coefficient order.
    pub(crate) fn characteristic_polynomial(&self, element: &FieldElement) -> Option<Vec<Rat>> {
        self.multiplication_matrix(element)
            .characteristic_polynomial()
    }

    /// The minimal polynomial in ascending coefficient order.
    ///
    /// The leading coefficient is one.
    pub(crate) fn minimal_polynomial(&self, element: &FieldElement) -> Option<Vec<Rat>> {
        let degree = self.degree;
        let mut powers = vec![{
            let mut vector = vec![Rat::zero(); degree];
            vector[0] = Rat::one();
            vector
        }];
        let mut current = powers[0].clone();
        for _ in 0..degree {
            current = self.multiply_coefficients(&current, &element.coefficients);
            powers.push(current.clone());
            let matrix = RatMatrix::from_columns(powers.clone())?;
            for vector in matrix.nullspace() {
                let Some(last) = vector.last() else {
                    continue;
                };
                if last.is_zero() {
                    continue;
                }
                let reciprocal = Rat::one().div(last).ok()?;
                return Some(vector.iter().map(|value| value.mul(&reciprocal)).collect());
            }
        }
        None
    }

    /// True when the element is an algebraic integer.
    pub(crate) fn is_integral(&self, element: &FieldElement) -> bool {
        let Some(polynomial) = self.minimal_polynomial(element) else {
            return false;
        };
        polynomial.last().is_some_and(Rat::is_one)
            && polynomial.iter().all(|value| value.den().is_one())
    }
}

fn poly_add(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        let a = left.get(index).cloned().unwrap_or_else(Rat::zero);
        let b = right.get(index).cloned().unwrap_or_else(Rat::zero);
        result.push(a.add(&b));
    }
    trim(result)
}

fn poly_sub(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);
    for index in 0..length {
        let a = left.get(index).cloned().unwrap_or_else(Rat::zero);
        let b = right.get(index).cloned().unwrap_or_else(Rat::zero);
        result.push(a.sub(&b));
    }
    trim(result)
}

fn poly_scale(coefficients: &[Rat], factor: &Rat) -> Vec<Rat> {
    trim(coefficients.iter().map(|value| value.mul(factor)).collect())
}

fn poly_extended_gcd(left: &[Rat], right: &[Rat]) -> Option<(Vec<Rat>, Vec<Rat>, Vec<Rat>)> {
    let mut old_r = trim(left.to_vec());
    let mut r = trim(right.to_vec());
    let mut old_s = vec![Rat::one()];
    let mut s = Vec::new();
    let mut old_t = Vec::new();
    let mut t = vec![Rat::one()];
    while !r.is_empty() {
        let (quotient, remainder) = dense_div_rem(&old_r, &r)?;
        old_r = r;
        r = remainder;
        let next_s = poly_sub(&old_s, &dense_multiply(&quotient, &s));
        old_s = s;
        s = next_s;
        let next_t = poly_sub(&old_t, &dense_multiply(&quotient, &t));
        old_t = t;
        t = next_t;
    }
    if old_r.is_empty() {
        return None;
    }
    let leading = old_r.last()?.clone();
    let reciprocal = Rat::one().div(&leading).ok()?;
    Some((
        poly_scale(&old_r, &reciprocal),
        poly_scale(&old_s, &reciprocal),
        poly_scale(&old_t, &reciprocal),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::poly_z::ZPoly;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| Int::from_i64(*value))
                .collect(),
        )
    }

    fn rational(numerator: i64, denominator: i64) -> Rat {
        Rat::from_i64(numerator, denominator).unwrap()
    }

    fn field(coefficients: &[i64]) -> NumberField {
        NumberField::new(poly(coefficients)).unwrap()
    }

    #[test]
    fn gaussian_integer_norm_trace_and_minimal_polynomial() {
        let field = field(&[1, 0, 1]);
        let element = field.element(vec![rational(3, 1), rational(4, 1)]);
        assert_eq!(field.norm(&element), Some(rational(25, 1)));
        assert_eq!(field.trace(&element), Some(rational(6, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![rational(25, 1), rational(-6, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&element));
        assert_eq!(
            field.characteristic_polynomial(&element),
            Some(vec![rational(1, 1), rational(-6, 1), rational(25, 1)])
        );
    }

    #[test]
    fn quadratic_unit_has_norm_minus_one() {
        let field = field(&[-2, 0, 1]);
        let element = field.element(vec![rational(1, 1), rational(1, 1)]);
        assert_eq!(field.norm(&element), Some(rational(-1, 1)));
        assert_eq!(field.trace(&element), Some(rational(2, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![rational(-1, 1), rational(-2, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&element));
    }

    #[test]
    fn golden_ratio_is_an_algebraic_integer() {
        let field = field(&[-5, 0, 1]);
        let element = field.element(vec![rational(1, 2), rational(1, 2)]);
        assert_eq!(field.norm(&element), Some(rational(-1, 1)));
        assert_eq!(field.trace(&element), Some(rational(1, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![rational(-1, 1), rational(-1, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&element));

        let half = field.element_from_rational(rational(1, 2));
        assert_eq!(field.norm(&half), Some(rational(1, 4)));
        assert!(!field.is_integral(&half));
    }

    #[test]
    fn inverse_and_division_round_trip() {
        let field = field(&[-2, 0, 1]);
        let element = field.element(vec![rational(3, 1), rational(5, 1)]);
        let inverse = field.inverse(&element).unwrap();
        assert!(field.multiply(&element, &inverse).is_one());
        let quotient = field.divide(&field.one(), &element).unwrap();
        assert!(field.multiply(&element, &quotient).is_one());
        assert!(field.inverse(&field.zero()).is_none());
    }

    #[test]
    fn power_agrees_with_repeated_multiplication() {
        let field = field(&[-2, 0, 1]);
        let element = field.element(vec![rational(1, 1), rational(1, 1)]);
        let mut repeated = field.one();
        for _ in 0..5 {
            repeated = field.multiply(&repeated, &element);
        }
        assert_eq!(field.power(&element, 5), repeated);
    }

    #[test]
    fn norm_is_multiplicative_and_trace_is_additive() {
        let field = field(&[-2, 0, 1]);
        let left = field.element(vec![rational(1, 1), rational(2, 1)]);
        let right = field.element(vec![rational(3, 1), rational(-1, 1)]);
        let product = field.multiply(&left, &right);
        assert_eq!(
            field.norm(&product),
            Some(field.norm(&left).unwrap().mul(&field.norm(&right).unwrap()))
        );
        assert_eq!(
            field.trace(&field.add(&left, &right)),
            Some(
                field
                    .trace(&left)
                    .unwrap()
                    .add(&field.trace(&right).unwrap())
            )
        );
    }

    #[test]
    fn multiplication_matrix_represents_multiplication() {
        let field = field(&[1, 0, 1]);
        let left = field.element(vec![rational(1, 2), rational(-1, 1)]);
        let right = field.element(vec![rational(3, 1), rational(4, 1)]);
        let product = field.multiply(&left, &right);
        let matrix = field.multiplication_matrix(&left);
        let image = matrix
            .multiply(&RatMatrix::from_columns(vec![right.coefficients.clone()]).unwrap())
            .unwrap();
        assert_eq!(image.column(0), product.coefficients);
    }

    #[test]
    fn cayley_hamilton_holds() {
        let field = field(&[-5, 0, 1]);
        let element = field.element(vec![rational(1, 2), rational(3, 1)]);
        let characteristic = field.characteristic_polynomial(&element).unwrap();
        let value = field.evaluate_descending(&characteristic, &element.coefficients);
        assert!(value.iter().all(Rat::is_zero));
    }

    #[test]
    fn rational_field_and_zero_element() {
        let field = field(&[0, 1]);
        let three = field.element_from_integer(Int::from_i64(3));
        assert_eq!(field.norm(&three), Some(rational(3, 1)));
        assert_eq!(field.trace(&three), Some(rational(3, 1)));
        assert_eq!(
            field.minimal_polynomial(&three),
            Some(vec![rational(-3, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&three));
        assert_eq!(
            field.minimal_polynomial(&field.zero()),
            Some(vec![rational(0, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&field.zero()));
    }

    #[test]
    fn cubic_generator_has_the_defining_minimal_polynomial() {
        let field = field(&[-2, 0, 0, 1]);
        let alpha = field.element(vec![rational(0, 1), rational(1, 1), rational(0, 1)]);
        assert_eq!(
            field.minimal_polynomial(&alpha),
            Some(vec![
                rational(-2, 1),
                rational(0, 1),
                rational(0, 1),
                rational(1, 1)
            ])
        );
        assert_eq!(field.norm(&alpha), Some(rational(2, 1)));
        assert_eq!(field.trace(&alpha), Some(rational(0, 1)));
        assert!(field.is_integral(&alpha));
        let inverse = field.inverse(&alpha).unwrap();
        assert!(field.multiply(&alpha, &inverse).is_one());
    }

    #[test]
    fn cubic_norms_agree_with_resultants() {
        let field = field(&[-2, 0, 0, 1]);
        let elements = [
            vec![rational(1, 1), rational(1, 1), rational(1, 1)],
            vec![rational(1, 1), rational(-1, 1), rational(0, 1)],
            vec![rational(3, 1), rational(-2, 1), rational(5, 1)],
        ];
        for coefficients in elements {
            let element = field.element(coefficients);
            assert_eq!(field.norm(&element), field.norm_via_resultant(&element));
        }
    }

    #[test]
    fn characteristic_polynomial_encodes_norm_and_trace() {
        let field = field(&[-2, 0, 0, 1]);
        let element = field.element(vec![rational(1, 1), rational(2, 1), rational(-3, 1)]);
        let characteristic = field.characteristic_polynomial(&element).unwrap();
        assert_eq!(characteristic.len(), 4);
        assert_eq!(field.trace(&element), Some(characteristic[1].neg()));
        let norm = characteristic[3].neg();
        assert_eq!(field.norm(&element), Some(norm));
        let value = field.evaluate_descending(&characteristic, &element.coefficients);
        assert!(value.iter().all(Rat::is_zero));
    }

    #[test]
    fn rational_element_in_a_quadratic_field() {
        let field = field(&[-2, 0, 1]);
        let element = field.element_from_rational(rational(3, 2));
        assert_eq!(field.norm(&element), Some(rational(9, 4)));
        assert_eq!(field.trace(&element), Some(rational(3, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![rational(-3, 2), rational(1, 1)])
        );
        assert_eq!(
            field.characteristic_polynomial(&element),
            Some(vec![rational(1, 1), rational(-3, 1), rational(9, 4)])
        );
        assert!(!field.is_integral(&element));
    }

    #[test]
    fn imaginary_quadratic_element() {
        let field = field(&[5, 0, 1]);
        let element = field.element(vec![rational(2, 1), rational(1, 1)]);
        assert_eq!(field.norm(&element), Some(rational(9, 1)));
        assert_eq!(field.trace(&element), Some(rational(4, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![rational(9, 1), rational(-4, 1), rational(1, 1)])
        );
        assert!(field.is_integral(&element));
        assert_eq!(field.norm_via_resultant(&element), Some(rational(9, 1)));
    }

    #[test]
    fn nonintegral_cubic_element() {
        let field = field(&[-2, 0, 0, 1]);
        let element = field.element(vec![rational(0, 1), rational(1, 2), rational(0, 1)]);
        assert_eq!(field.norm(&element), Some(rational(1, 4)));
        assert_eq!(field.trace(&element), Some(rational(0, 1)));
        assert_eq!(
            field.minimal_polynomial(&element),
            Some(vec![
                rational(-1, 4),
                rational(0, 1),
                rational(0, 1),
                rational(1, 1)
            ])
        );
        assert!(!field.is_integral(&element));
    }

    #[test]
    fn field_axioms_hold_for_quadratic_and_cubic_elements() {
        let quadratic = field(&[-2, 0, 1]);
        let a = quadratic.element(vec![rational(1, 1), rational(2, 1)]);
        let b = quadratic.element(vec![rational(-3, 1), rational(1, 1)]);
        let c = quadratic.element(vec![rational(5, 1), rational(-2, 1)]);
        assert_eq!(
            quadratic.multiply(&a, &quadratic.add(&b, &c)),
            quadratic.add(&quadratic.multiply(&a, &b), &quadratic.multiply(&a, &c))
        );
        assert_eq!(
            quadratic.multiply(&quadratic.multiply(&a, &b), &c),
            quadratic.multiply(&a, &quadratic.multiply(&b, &c))
        );
        let product = quadratic.multiply(&a, &b);
        let inverse = quadratic.inverse(&product).unwrap();
        assert!(quadratic.multiply(&product, &inverse).is_one());

        let cubic = field(&[-2, 0, 0, 1]);
        let alpha = cubic.element(vec![rational(0, 1), rational(1, 1), rational(0, 1)]);
        let beta = cubic.element(vec![rational(1, 1), rational(-1, 1), rational(2, 1)]);
        assert_eq!(cubic.multiply(&alpha, &beta), cubic.multiply(&beta, &alpha));
        assert_eq!(
            cubic.norm(&cubic.multiply(&alpha, &beta)),
            Some(cubic.norm(&alpha).unwrap().mul(&cubic.norm(&beta).unwrap()))
        );
    }

    #[test]
    fn division_by_zero_and_power_edge_cases() {
        let field = field(&[-2, 0, 1]);
        let element = field.element(vec![rational(1, 1), rational(2, 1)]);
        assert!(field.divide(&element, &field.zero()).is_none());
        assert!(field.power(&element, 0).is_one());
        assert_eq!(field.power(&element, 1), element);
        assert_eq!(field.power(&element, 2), field.multiply(&element, &element));
    }
}
