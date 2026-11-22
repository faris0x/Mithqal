//! Conductors of orders in maximal orders.
//!
//! The conductor is `{ x in O_K : x O_K subset O }`. It is computed as a
//! lattice colon and returned as an integral ideal of the maximal order.

#[allow(unused_imports)]
use crate::prelude::*;

use super::super::ratmatrix::RatMatrix;
use super::ideal::{Ideal, preimage_lattice};
use super::order::Order;

/// The conductor ideal of `order` inside `maximal`.
pub(crate) fn conductor(order: &Order, maximal: &Order) -> Option<Ideal> {
    let size = order.field().degree();
    if maximal.field().degree() != size {
        return None;
    }
    let order_inverse = order.basis().inverse()?;
    let mut rows = Vec::with_capacity(size * size);
    for column in 0..size {
        let element = maximal.field().element(maximal.basis().column(column));
        let action = maximal.field().multiplication_matrix(&element);
        let condition = order_inverse.multiply(&action)?;
        rows.extend(condition.to_rows());
    }
    let conditions = RatMatrix::from_rows(rows)?;
    let basis = preimage_lattice(&conditions)?;
    let ideal = Ideal::from_power_basis(maximal, &basis)?;
    for generator in ideal.basis() {
        let mut power = vec![crate::num::Rat::zero(); size];
        for (index, value) in generator.iter().enumerate() {
            let basis = maximal.basis().column(index);
            for row in 0..size {
                power[row] = power[row]
                    .add(&crate::algebra::ratmatrix::rat_from_int(value).mul(&basis[row]));
            }
        }
        let coordinates = order
            .basis()
            .inverse()?
            .multiply(&RatMatrix::from_columns(vec![power])?)?;
        if !coordinates
            .to_rows()
            .iter()
            .flatten()
            .all(|value| value.den().is_one())
        {
            return None;
        }
    }
    Some(ideal)
}

#[cfg(test)]
mod tests {
    use super::super::field::NumberField;
    use super::super::integral_basis::maximal_order;
    use super::*;
    use crate::algebra::poly_z::ZPoly;
    use crate::num::Int;

    fn poly(coefficients: &[i64]) -> ZPoly {
        ZPoly::new(coefficients.iter().map(|v| Int::from_i64(*v)).collect())
    }

    fn field(coefficients: &[i64]) -> NumberField {
        NumberField::new(poly(coefficients)).unwrap()
    }

    #[test]
    fn power_order_conductors() {
        for (coefficients, norm) in [
            (&[-5i64, 0, 1][..], 4i64),
            (&[-8, -2, -1, 1][..], 4),
            (&[-2, 0, 1][..], 1),
            (&[-2, 0, 0, 1][..], 1),
        ] {
            let field = field(coefficients);
            let maximal = maximal_order(&field).unwrap();
            let power = Order::power_basis(&field).unwrap();
            let conductor = conductor(&power, &maximal).unwrap();
            assert_eq!(conductor.norm(), &Int::from_i64(norm));
        }
    }
}
