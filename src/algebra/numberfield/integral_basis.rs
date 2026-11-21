//! Computation of the maximal order by Round 2.
//!
//! The current discriminant factorization uses the machine integer factorizer,
//! so the power discriminant must fit in `i64`. The SNF, multiplier ring, and
//! verification costs grow quickly with degree, so the first envelope is small
//! degree fields.

#[allow(unused_imports)]
use crate::prelude::*;

use core::cmp::Ordering;

use crate::num::Int;
use crate::numbertheory::factor_u64;

use super::field::NumberField;
use super::order::Order;

/// The maximal order of a number field.
///
/// Starts with the power basis, then applies the p-radical Round 2 step at
/// every prime whose square divides the order discriminant.
pub(crate) fn maximal_order(field: &NumberField) -> Option<Order> {
    let mut order = Order::power_basis(field)?;
    let primes = discriminant_primes(&field.power_discriminant().abs())?;
    for (prime, exponent) in primes {
        if exponent < 2 {
            continue;
        }
        let prime = Int::from_i64(prime as i64);
        let mut steps = 0usize;
        loop {
            let next = order.p_maximal_overorder(&prime)?;
            if next.index().cmp(order.index()) == Ordering::Equal {
                break;
            }
            order = next;
            steps += 1;
            if steps > 64 {
                return None;
            }
        }
    }
    if !order.associativity_holds() || !order.basis_is_integral() {
        return None;
    }
    Some(order)
}

/// True when the power basis is already the maximal order.
pub(crate) fn power_basis_is_maximal(field: &NumberField) -> Option<bool> {
    let order = Order::power_basis(field)?;
    let primes = discriminant_primes(&field.power_discriminant().abs())?;
    for (prime, exponent) in primes {
        if exponent < 2 {
            continue;
        }
        let prime = Int::from_i64(prime as i64);
        let next = order.p_maximal_overorder(&prime)?;
        if next.index().cmp(order.index()) != Ordering::Equal {
            return Some(false);
        }
    }
    Some(true)
}

/// Check that an order is maximal at every prime whose square divides its
/// discriminant.
pub(crate) fn verify_maximal_order(order: &Order) -> bool {
    if !order.associativity_holds() || !order.basis_is_integral() {
        return false;
    }
    let Some(primes) = discriminant_primes(&order.discriminant().abs()) else {
        return false;
    };
    for (prime, exponent) in primes {
        if exponent < 2 {
            continue;
        }
        let prime = Int::from_i64(prime as i64);
        let Some(next) = order.p_maximal_overorder(&prime) else {
            return false;
        };
        if next.index().cmp(order.index()) != Ordering::Equal {
            return false;
        }
    }
    true
}

fn discriminant_primes(discriminant: &Int) -> Option<Vec<(u64, u32)>> {
    if discriminant.is_zero() {
        return None;
    }
    let magnitude = discriminant.to_i64()?;
    if magnitude < 0 {
        return None;
    }
    Some(factor_u64(magnitude as u64))
}

#[cfg(test)]
mod tests {
    use super::super::order::Order;
    use super::*;
    use crate::algebra::ratmatrix::RatMatrix;
    use crate::num::Rat;

    fn poly(coefficients: &[i64]) -> crate::algebra::poly_z::ZPoly {
        crate::algebra::poly_z::ZPoly::new(
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

    #[test]
    fn monogenic_fields_keep_the_power_basis() {
        for (coefficients, discriminant, degree) in [
            (&[-2i64, 0, 1][..], 8i64, 2usize),
            (&[-2, 0, 0, 1][..], -108, 3),
            (&[1, 1, 1, 1, 1][..], 125, 4),
            (&[1, 0, 0, 0, 1][..], 256, 4),
        ] {
            let field = field(coefficients);
            assert_eq!(power_basis_is_maximal(&field), Some(true));
            let order = maximal_order(&field).unwrap();
            assert_eq!(order.index(), &Int::one());
            assert_eq!(order.discriminant(), &Int::from_i64(discriminant));
            assert!(order.associativity_holds());
            assert!(order.basis_is_integral());
            assert_eq!(order.field().degree(), degree);
        }
    }

    #[test]
    fn sqrt_five_needs_round_two() {
        let field = field(&[-5, 0, 1]);
        assert_eq!(power_basis_is_maximal(&field), Some(false));
        let order = maximal_order(&field).unwrap();
        assert_eq!(order.index(), &Int::from_i64(2));
        assert_eq!(order.discriminant(), &Int::from_i64(5));
        assert!(order.associativity_holds());
        assert!(order.basis_is_integral());
        assert!(verify_maximal_order(&order));

        let golden = field.element(vec![rational(1, 2), rational(1, 2)]);
        let coordinates = order
            .basis()
            .inverse()
            .unwrap()
            .multiply(&RatMatrix::from_columns(vec![golden.coefficients().to_vec()]).unwrap())
            .unwrap();
        assert!(
            coordinates
                .to_rows()
                .iter()
                .flatten()
                .all(|value| value.den().is_one())
        );
    }

    #[test]
    fn index_two_cubic_needs_round_two() {
        let field = field(&[-8, -2, -1, 1]);
        assert_eq!(power_basis_is_maximal(&field), Some(false));
        let order = maximal_order(&field).unwrap();
        assert_eq!(order.index(), &Int::from_i64(2));
        assert_eq!(order.discriminant(), &Int::from_i64(-503));
        assert!(verify_maximal_order(&order));
    }

    #[test]
    fn half_integer_quadratic_fields_need_round_two() {
        for (coefficients, discriminant) in [
            (&[3i64, 0, 1][..], -3i64),
            (&[7, 0, 1][..], -7),
            (&[-13, 0, 1][..], 13),
        ] {
            let field = field(coefficients);
            assert_eq!(power_basis_is_maximal(&field), Some(false));
            let order = maximal_order(&field).unwrap();
            assert_eq!(order.index(), &Int::from_i64(2));
            assert_eq!(order.discriminant(), &Int::from_i64(discriminant));
            assert!(order.associativity_holds());
            assert!(order.basis_is_integral());
            assert!(verify_maximal_order(&order));
        }
    }

    #[test]
    fn prime_three_round_two() {
        let field = field(&[-9, -9, -4, 1]);
        assert_eq!(power_basis_is_maximal(&field), Some(false));
        let order = maximal_order(&field).unwrap();
        assert_eq!(order.index(), &Int::from_i64(3));
        assert_eq!(order.discriminant(), &Int::from_i64(-679));
        assert!(verify_maximal_order(&order));
    }

    #[test]
    fn repeated_round_two_steps() {
        for (coefficients, index, discriminant) in [
            (&[-6i64, -7, -4, 1][..], 4i64, -211i64),
            (&[-5, -6, -4, 1][..], 5, -107),
            (&[-8, -8, -4, 1][..], 8, -83),
        ] {
            let field = field(coefficients);
            let order = maximal_order(&field).unwrap();
            assert_eq!(order.index(), &Int::from_i64(index));
            assert_eq!(order.discriminant(), &Int::from_i64(discriminant));
            assert!(verify_maximal_order(&order));
            assert!(order.basis_is_integral());
        }
    }

    #[test]
    fn imaginary_quadratic_field_is_monogenic() {
        let field = field(&[5, 0, 1]);
        assert_eq!(power_basis_is_maximal(&field), Some(true));
        let order = maximal_order(&field).unwrap();
        assert_eq!(order.index(), &Int::one());
        assert_eq!(order.discriminant(), &Int::from_i64(-20));
    }

    #[test]
    fn p_maximal_step_contains_the_old_order_and_stabilizes() {
        let field = field(&[-5, 0, 1]);
        let power = Order::power_basis(&field).unwrap();
        let prime = Int::from_i64(2);
        let enlarged = power.p_maximal_overorder(&prime).unwrap();
        assert_eq!(enlarged.index(), &Int::from_i64(2));
        assert_eq!(enlarged.contains(&power), Some(true));
        let stable = enlarged.p_maximal_overorder(&prime).unwrap();
        assert_eq!(stable.index(), enlarged.index());
        assert!(
            enlarged
                .p_maximal_overorder(&Int::from_i64(5))
                .unwrap()
                .index()
                == enlarged.index()
        );
    }

    #[test]
    fn power_basis_multiplication_table_matches_field_arithmetic() {
        let field = field(&[-2, 0, 1]);
        let order = Order::power_basis(&field).unwrap();
        assert_eq!(order.index(), &Int::one());
        assert_eq!(order.discriminant(), &Int::from_i64(8));
        assert!(order.associativity_holds());
        assert!(order.basis_is_integral());
        // omega_1 is 1 and omega_2 is alpha, so alpha * alpha = 2.
        assert_eq!(order.multiplication_table()[1][1][0], Int::from_i64(2));
        assert_eq!(order.multiplication_table()[1][1][1], Int::zero());
        assert_eq!(order.multiplication_table()[0][1][1], Int::one());
    }

    #[test]
    fn small_cubic_orders_satisfy_the_discriminant_identity() {
        let mut checked = 0usize;
        for constant in -3i64..=3 {
            for linear in -4i64..=4 {
                for quadratic in -3i64..=3 {
                    let polynomial = poly(&[constant, linear, quadratic, 1]);
                    let Some(true) = polynomial.irreducibility() else {
                        continue;
                    };
                    let field = NumberField::new(polynomial).unwrap();
                    let order = maximal_order(&field).unwrap();
                    assert!(order.associativity_holds());
                    assert!(order.basis_is_integral());
                    let expected = order.discriminant().mul(&order.index().mul(order.index()));
                    assert_eq!(&expected, field.power_discriminant());
                    if checked.is_multiple_of(8) {
                        assert!(verify_maximal_order(&order));
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked >= 100);
    }

    #[test]
    fn maximal_order_rejects_an_invalid_prime() {
        let field = field(&[-2, 0, 1]);
        let order = Order::power_basis(&field).unwrap();
        assert!(order.p_maximal_overorder(&Int::zero()).is_none());
        assert!(order.p_maximal_overorder(&Int::from_i64(-3)).is_none());
    }
}
