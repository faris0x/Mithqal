//! Number field construction and the power basis reduction map.

#[allow(unused_imports)]
use crate::prelude::*;

use crate::num::{Int, Rat};

use super::super::dense::{dense_div_rem, dense_multiply, trim};
use super::super::poly_z::ZPoly;
use super::sturm;

/// Why a defining polynomial was rejected.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum FieldError {
    /// The polynomial is zero.
    Zero,
    /// The polynomial has degree zero.
    Constant,
    /// The polynomial is not monic.
    NotMonic,
    /// The polynomial factors over the rationals.
    Reducible,
    /// Irreducibility could not be decided.
    Undecided,
    /// The power basis discriminant could not be computed.
    Discriminant,
    /// The real root count was inconsistent.
    Signature,
}

/// A number field `Q(alpha)` defined by a monic irreducible integer polynomial.
#[derive(Clone, Debug)]
pub(crate) struct NumberField {
    pub(super) defining: ZPoly,
    pub(super) degree: usize,
    pub(super) signature: (usize, usize),
    pub(super) power_discriminant: Int,
}

impl NumberField {
    /// Build a number field from a defining polynomial.
    pub(crate) fn new(defining: ZPoly) -> Result<Self, FieldError> {
        if defining.is_zero() {
            return Err(FieldError::Zero);
        }
        let Some(degree) = defining.degree() else {
            return Err(FieldError::Zero);
        };
        if degree == 0 {
            return Err(FieldError::Constant);
        }
        if !defining.is_monic() {
            return Err(FieldError::NotMonic);
        }
        match defining.irreducibility() {
            Some(true) => {}
            Some(false) => return Err(FieldError::Reducible),
            None => return Err(FieldError::Undecided),
        }
        let derivative = defining.derivative();
        let resultant = defining
            .resultant(&derivative)
            .ok_or(FieldError::Discriminant)?;
        let sign_parity = degree * (degree - 1) / 2;
        let power_discriminant = if sign_parity.is_multiple_of(2) {
            resultant
        } else {
            resultant.neg()
        };
        let real_roots = sturm::real_root_count(&defining).ok_or(FieldError::Signature)?;
        if real_roots > degree || !(degree - real_roots).is_multiple_of(2) {
            return Err(FieldError::Signature);
        }
        let signature = (real_roots, (degree - real_roots) / 2);
        Ok(NumberField {
            defining,
            degree,
            signature,
            power_discriminant,
        })
    }

    /// The defining polynomial.
    pub(crate) fn defining_polynomial(&self) -> &ZPoly {
        &self.defining
    }

    /// The field degree.
    pub(crate) fn degree(&self) -> usize {
        self.degree
    }

    /// The signature `(r1, r2)`.
    pub(crate) fn signature(&self) -> (usize, usize) {
        self.signature
    }

    /// The discriminant of the power basis.
    pub(crate) fn power_discriminant(&self) -> &Int {
        &self.power_discriminant
    }

    /// Reduce a polynomial in `alpha` to a coefficient vector of length degree.
    pub(crate) fn reduce(&self, coefficients: Vec<Rat>) -> Vec<Rat> {
        let mut coefficients = trim(coefficients);
        if coefficients.len() >= self.degree {
            let divisor = self.defining.to_rat_coeffs();
            if let Some((_, remainder)) = dense_div_rem(&coefficients, &divisor) {
                coefficients = trim(remainder);
            }
        }
        coefficients.resize(self.degree, Rat::zero());
        coefficients
    }

    /// Multiply two power basis coefficient vectors modulo the defining
    /// polynomial.
    pub(crate) fn multiply_coefficients(&self, left: &[Rat], right: &[Rat]) -> Vec<Rat> {
        self.reduce(dense_multiply(left, right))
    }

    /// Evaluate a polynomial with descending coefficients at a coefficient
    /// vector.
    pub(crate) fn evaluate_descending(&self, coefficients: &[Rat], element: &[Rat]) -> Vec<Rat> {
        let mut result = vec![Rat::zero(); self.degree];
        for coefficient in coefficients {
            result = self.multiply_coefficients(&result, element);
            result[0] = result[0].add(coefficient);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(
            coefficients
                .iter()
                .map(|value| Int::from_i64(*value))
                .collect(),
        )
    }

    #[test]
    fn quadratic_fields_have_expected_signatures_and_discriminants() {
        let field = NumberField::new(poly(&[-2, 0, 1])).unwrap();
        assert_eq!(field.degree(), 2);
        assert_eq!(field.signature(), (2, 0));
        assert_eq!(field.power_discriminant(), &Int::from_i64(8));

        let field = NumberField::new(poly(&[1, 0, 1])).unwrap();
        assert_eq!(field.signature(), (0, 1));
        assert_eq!(field.power_discriminant(), &Int::from_i64(-4));

        let field = NumberField::new(poly(&[-5, 0, 1])).unwrap();
        assert_eq!(field.signature(), (2, 0));
        assert_eq!(field.power_discriminant(), &Int::from_i64(20));
    }

    #[test]
    fn higher_degree_signatures_match_sturm_counts() {
        let field = NumberField::new(poly(&[1, 1, 1, 1, 1])).unwrap();
        assert_eq!(field.signature(), (0, 2));
        let field = NumberField::new(poly(&[1, 0, 0, 0, 1])).unwrap();
        assert_eq!(field.signature(), (0, 2));
        let field = NumberField::new(poly(&[1, 0, -10, 0, 1])).unwrap();
        assert_eq!(field.signature(), (4, 0));
    }

    #[test]
    fn invalid_defining_polynomials_are_rejected() {
        assert_eq!(
            NumberField::new(ZPoly::zero()).unwrap_err(),
            FieldError::Zero
        );
        assert_eq!(
            NumberField::new(ZPoly::constant(Int::from_i64(3))).unwrap_err(),
            FieldError::Constant
        );
        assert_eq!(
            NumberField::new(poly(&[1, 0, 2])).unwrap_err(),
            FieldError::NotMonic
        );
        assert_eq!(
            NumberField::new(poly(&[-1, 0, 1])).unwrap_err(),
            FieldError::Reducible
        );
    }

    #[test]
    fn reduction_respects_the_defining_relation() {
        let field = NumberField::new(poly(&[-2, 0, 1])).unwrap();
        let square = field.reduce(vec![Rat::zero(), Rat::zero(), Rat::one()]);
        assert_eq!(square, vec![Rat::from_i64(2, 1).unwrap(), Rat::zero()]);
        let product =
            field.multiply_coefficients(&[Rat::one(), Rat::one()], &[Rat::one(), Rat::one()]);
        assert_eq!(
            product,
            vec![Rat::from_i64(3, 1).unwrap(), Rat::from_i64(2, 1).unwrap()]
        );
    }

    #[test]
    fn cubic_fields_have_expected_power_discriminants() {
        let field = NumberField::new(poly(&[-2, 0, 0, 1])).unwrap();
        assert_eq!(field.degree(), 3);
        assert_eq!(field.signature(), (1, 1));
        assert_eq!(field.power_discriminant(), &Int::from_i64(-108));

        let field = NumberField::new(poly(&[-8, -2, -1, 1])).unwrap();
        assert_eq!(field.signature(), (1, 1));
        assert_eq!(field.power_discriminant(), &Int::from_i64(-2012));
    }

    #[test]
    fn cyclotomic_and_fourth_root_discriminants() {
        let field = NumberField::new(poly(&[1, 1, 1, 1, 1])).unwrap();
        assert_eq!(field.signature(), (0, 2));
        assert_eq!(field.power_discriminant(), &Int::from_i64(125));

        let field = NumberField::new(poly(&[1, 0, 0, 0, 1])).unwrap();
        assert_eq!(field.signature(), (0, 2));
        assert_eq!(field.power_discriminant(), &Int::from_i64(256));
    }

    #[test]
    fn irreducible_quartic_with_no_irreducible_modular_reduction() {
        let field = NumberField::new(poly(&[1, 0, -10, 0, 1])).unwrap();
        assert_eq!(field.degree(), 4);
        assert_eq!(field.signature(), (4, 0));
    }

    #[test]
    fn imaginary_quadratic_discriminants() {
        let field = NumberField::new(poly(&[1, 0, 1])).unwrap();
        assert_eq!(field.power_discriminant(), &Int::from_i64(-4));
        let field = NumberField::new(poly(&[3, 0, 1])).unwrap();
        assert_eq!(field.signature(), (0, 1));
        assert_eq!(field.power_discriminant(), &Int::from_i64(-12));
        let field = NumberField::new(poly(&[-5, 0, 1])).unwrap();
        assert_eq!(field.signature(), (2, 0));
        assert_eq!(field.power_discriminant(), &Int::from_i64(20));
    }

    #[test]
    fn reduction_of_a_higher_degree_polynomial() {
        let field = NumberField::new(poly(&[-2, 0, 1])).unwrap();
        let reduced = field.reduce(vec![
            Rat::one(),
            Rat::zero(),
            Rat::zero(),
            Rat::zero(),
            Rat::one(),
        ]);
        assert_eq!(reduced, vec![Rat::from_i64(5, 1).unwrap(), Rat::zero()]);
    }
}
