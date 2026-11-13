use cas::num::{Int, NumError, Rat};

#[test]
fn integer_decimal_round_trip() {
    for text in [
        "0",
        "1",
        "-1",
        "123456789012345678901234567890",
        "-999999999999999999999999999999999",
    ] {
        let value = Int::from_decimal(text).unwrap();
        assert_eq!(value.to_decimal(), text);
    }
}

#[test]
fn integer_arithmetic() {
    let a = Int::from_i64(123456789);
    let b = Int::from_i64(987654321);
    assert_eq!(a.add(&b).to_decimal(), "1111111110");
    assert_eq!(a.sub(&b).to_decimal(), "-864197532");
    assert_eq!(
        Int::from_i64(1_000_000_007)
            .mul(&Int::from_i64(1_000_000_009))
            .to_decimal(),
        "1000000016000000063"
    );
}

#[test]
fn integer_division_large_divisor() {
    let two = Int::from_i64(2);
    let numerator = two.pow_u32(200).add(&Int::from_i64(12345));
    let divisor = two.pow_u32(130).add(&Int::from_i64(7));
    let (quotient, remainder) = numerator.divrem(&divisor).unwrap();
    let rebuilt = quotient.mul(&divisor).add(&remainder);
    assert_eq!(rebuilt, numerator);
    assert!(remainder.abs().cmp(&divisor) == core::cmp::Ordering::Less);
    let (q2, r2) = numerator.divrem(&divisor).unwrap();
    assert_eq!(quotient, q2);
    assert_eq!(remainder, r2);
}

#[test]
fn integer_gcd() {
    let a = Int::from_i64(84);
    let b = Int::from_i64(30);
    assert_eq!(a.gcd(&b).to_decimal(), "6");
}

#[test]
fn integer_division_by_zero() {
    let a = Int::from_i64(5);
    let zero = Int::zero();
    assert!(matches!(a.divrem(&zero), Err(NumError::DivisionByZero)));
}

#[test]
fn rational_canonical_form() {
    assert_eq!(Rat::from_i64(2, 4).unwrap().to_decimal(), "1/2");
    assert_eq!(Rat::from_i64(6, -9).unwrap().to_decimal(), "-2/3");
    assert_eq!(Rat::from_i64(-6, -9).unwrap().to_decimal(), "2/3");
    assert_eq!(Rat::from_i64(0, 5).unwrap().to_decimal(), "0");
}

#[test]
fn rational_arithmetic() {
    let two_thirds = Rat::from_i64(2, 3).unwrap();
    let one_sixth = Rat::from_i64(1, 6).unwrap();
    assert_eq!(two_thirds.add(&one_sixth).to_decimal(), "5/6");
    assert_eq!(two_thirds.sub(&one_sixth).to_decimal(), "1/2");
    assert_eq!(two_thirds.mul(&one_sixth).to_decimal(), "1/9");
    assert_eq!(two_thirds.div(&one_sixth).unwrap().to_decimal(), "4");
}

#[test]
fn rational_from_integers() {
    let num = Int::from_i64(-10);
    let den = Int::from_i64(4);
    assert_eq!(Rat::from_ints(num, den).unwrap().to_decimal(), "-5/2");
}

#[test]
fn rational_division_by_zero() {
    let value = Rat::from_i64(1, 2).unwrap();
    let zero = Rat::zero();
    assert!(matches!(value.div(&zero), Err(NumError::DivisionByZero)));
    assert!(matches!(Rat::from_i64(1, 0), Err(NumError::DivisionByZero)));
}
