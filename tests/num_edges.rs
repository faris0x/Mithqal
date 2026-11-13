use core::cmp::Ordering;

use cas::num::{Int, NumError, Rat};

#[test]
fn integer_zero_and_one() {
    assert!(Int::zero().is_zero());
    assert!(!Int::zero().is_one());
    assert!(Int::one().is_one());
    assert_eq!(Int::one().to_decimal(), "1");
}

#[test]
fn integer_invalid_decimal_literals() {
    for text in ["", "abc", "1/2", " 5", "5x", "--1"] {
        assert!(matches!(Int::from_decimal(text), Err(NumError::Parse)));
    }
    assert_eq!(Int::from_decimal("-0").unwrap().to_decimal(), "0");
}

#[test]
fn integer_signed_boundaries() {
    let minimum = Int::from_i64(i64::MIN);
    let maximum = Int::from_i64(i64::MAX);
    assert_eq!(minimum.to_i64(), Some(i64::MIN));
    assert_eq!(maximum.to_i64(), Some(i64::MAX));
    assert_eq!(Int::from_u64(u64::MAX).to_i64(), None);
    assert_eq!(Int::from_i64(2).pow_u32(63).to_i64(), None);
}

#[test]
fn integer_negation_and_absolute_value() {
    let value = Int::from_i64(-7);
    assert_eq!(value.neg().to_decimal(), "7");
    assert_eq!(value.abs().to_decimal(), "7");
    assert_eq!(Int::zero().abs().to_decimal(), "0");
    assert!(Int::zero().neg().is_zero());
}

#[test]
fn integer_power_boundaries() {
    let value = Int::from_i64(5);
    assert_eq!(value.pow_u32(0).to_decimal(), "1");
    assert_eq!(value.pow_u32(1).to_decimal(), "5");
    assert_eq!(value.pow_u32(2).to_decimal(), "25");
    assert_eq!(Int::zero().pow_u32(3).to_decimal(), "0");
}

#[test]
fn integer_truncated_division_signs() {
    let cases = [
        (7, 3, 2, 1),
        (-7, 3, -2, -1),
        (7, -3, -2, 1),
        (-7, -3, 2, -1),
    ];
    for (numerator, denominator, quotient, remainder) in cases {
        let (q, r) = Int::from_i64(numerator)
            .divrem(&Int::from_i64(denominator))
            .unwrap();
        assert_eq!(q.to_decimal(), quotient.to_string());
        assert_eq!(r.to_decimal(), remainder.to_string());
    }
}

#[test]
fn integer_gcd_with_zero() {
    assert_eq!(Int::from_i64(5).gcd(&Int::zero()).to_decimal(), "5");
    assert_eq!(Int::zero().gcd(&Int::from_i64(5)).to_decimal(), "5");
    assert_eq!(Int::zero().gcd(&Int::zero()).to_decimal(), "0");
    assert_eq!(Int::from_i64(-12).gcd(&Int::from_i64(18)).to_decimal(), "6");
}

#[test]
fn integer_comparison_and_clone() {
    let a = Int::from_i64(-3);
    let b = Int::from_i64(-2);
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&a), Ordering::Greater);
    let clone = a.clone();
    assert_eq!(clone, a);
    assert_eq!(clone.to_decimal(), "-3");
}

#[test]
fn integer_decimal_round_trip_large() {
    let value = Int::from_i64(10).pow_u32(200).add(&Int::from_i64(7));
    let text = value.to_decimal();
    let parsed = Int::from_decimal(&text).unwrap();
    assert_eq!(parsed, value);
    assert!(text.ends_with('7'));
}

#[test]
fn rational_invalid_forms() {
    assert!(matches!(Rat::from_i64(1, 0), Err(NumError::DivisionByZero)));
    assert!(matches!(Rat::from_decimal(""), Err(NumError::Parse)));
    assert!(matches!(Rat::from_decimal("x/2"), Err(NumError::Parse)));
}

#[test]
fn rational_zero_has_unit_denominator() {
    let zero = Rat::from_i64(0, -5).unwrap();
    assert!(zero.is_zero());
    assert_eq!(zero.to_decimal(), "0");
    assert_eq!(zero.den().to_decimal(), "1");
}

#[test]
fn rational_extreme_numerator() {
    let value = Rat::from_i64(i64::MIN, -1).unwrap();
    assert_eq!(value.to_decimal(), "9223372036854775808");
    assert_eq!(value.num().to_decimal(), "9223372036854775808");
    assert_eq!(value.den().to_decimal(), "1");
}

#[test]
fn rational_decimal_parsing() {
    assert_eq!(Rat::from_decimal("2/4").unwrap().to_decimal(), "1/2");
    assert_eq!(Rat::from_decimal("-1/2").unwrap().to_decimal(), "-1/2");
    assert_eq!(Rat::from_decimal("5").unwrap().to_decimal(), "5");
}

#[test]
fn rational_accessors_and_signs() {
    let value = Rat::from_i64(-6, 9).unwrap();
    assert_eq!(value.to_decimal(), "-2/3");
    assert_eq!(value.num().to_decimal(), "-2");
    assert_eq!(value.den().to_decimal(), "3");
    assert_eq!(value.sign(), -1);
    assert_eq!(value.abs().to_decimal(), "2/3");
    assert_eq!(value.neg().to_decimal(), "2/3");
}

#[test]
fn rational_one_and_zero_checks() {
    assert!(Rat::one().is_one());
    assert!(!Rat::one().is_zero());
    assert!(Rat::zero().is_zero());
    assert!(!Rat::zero().is_one());
    assert!(Rat::from_i64(-1, -1).unwrap().is_one());
}

#[test]
fn rational_comparison() {
    let half = Rat::from_i64(1, 2).unwrap();
    let third = Rat::from_i64(1, 3).unwrap();
    assert_eq!(third.cmp(&half), Ordering::Less);
    assert_eq!(half.cmp(&third), Ordering::Greater);
    assert_eq!(half.cmp(&Rat::from_i64(2, 4).unwrap()), Ordering::Equal);
}

#[test]
fn rational_add_sub_mul_div_large() {
    let left = Rat::from_i64(1, 3).unwrap();
    let right = Rat::from_i64(5, 7).unwrap();
    assert_eq!(left.add(&right).to_decimal(), "22/21");
    assert_eq!(left.sub(&right).to_decimal(), "-8/21");
    assert_eq!(left.mul(&right).to_decimal(), "5/21");
    assert_eq!(left.div(&right).unwrap().to_decimal(), "7/15");
}

#[test]
fn rational_clone_and_equality() {
    let value = Rat::from_i64(22, 7).unwrap();
    let clone = value.clone();
    assert_eq!(value, clone);
    assert_eq!(clone.to_decimal(), "22/7");
}

#[test]
fn rational_from_integers_zero_denominator() {
    assert!(matches!(
        Rat::from_ints(Int::one(), Int::zero()),
        Err(NumError::DivisionByZero)
    ));
}

#[test]
fn rational_from_decimal_round_trip() {
    for text in ["0", "1/2", "-2/3", "123456789/987654321"] {
        let value = Rat::from_decimal(text).unwrap();
        let reparsed = Rat::from_decimal(&value.to_decimal()).unwrap();
        assert_eq!(value, reparsed);
    }
}
