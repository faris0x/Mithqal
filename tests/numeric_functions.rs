use mithqal::ExprId;
use mithqal::{ErrorCode, ExprView, Int, Rat, Real, Session};

fn evaluate(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value).unwrap()
}

fn integer(source: &str) -> i64 {
    let (session, value) = evaluate(source);
    match session.view(value).unwrap() {
        ExprView::Integer(value) => value.to_i64().unwrap(),
        _ => panic!("expected an integer for {source}"),
    }
}

#[test]
fn gcd_of_integers() {
    assert_eq!(integer("gcd(12,18)"), 6);
    assert_eq!(integer("gcd(18,12)"), 6);
    assert_eq!(integer("gcd(0,5)"), 5);
    assert_eq!(integer("gcd(5,0)"), 5);
    assert_eq!(integer("gcd(0,0)"), 0);
    assert_eq!(integer("gcd(-12,18)"), 6);
    assert_eq!(integer("gcd(12,-18)"), 6);
    assert_eq!(integer("gcd(-12,-18)"), 6);
    assert_eq!(integer("gcd(12,18,24)"), 6);
    assert_eq!(integer("gcd(17,19)"), 1);
    assert_eq!(integer("gcd(100,75)"), 25);
}

#[test]
fn lcm_of_integers() {
    assert_eq!(integer("lcm(12,18)"), 36);
    assert_eq!(integer("lcm(18,12)"), 36);
    assert_eq!(integer("lcm(0,5)"), 0);
    assert_eq!(integer("lcm(5,0)"), 0);
    assert_eq!(integer("lcm(0,0)"), 0);
    assert_eq!(integer("lcm(-12,18)"), 36);
    assert_eq!(integer("lcm(12,-18)"), 36);
    assert_eq!(integer("lcm(12,18,24)"), 72);
    assert_eq!(integer("lcm(17,19)"), 323);
    assert_eq!(integer("lcm(100,75)"), 300);
}

#[test]
fn factorials() {
    let expected = [
        1,
        1,
        2,
        6,
        24,
        120,
        720,
        5040,
        40320,
        362880,
        3628800,
        39916800,
        479001600,
        6227020800,
        87178291200,
        1307674368000,
    ];
    for (value, expected) in expected.iter().enumerate() {
        assert_eq!(integer(&format!("factorial({value})")), *expected);
    }
}

#[test]
fn binomial_coefficients() {
    let table = [
        (0, 0, 1),
        (1, 0, 1),
        (1, 1, 1),
        (2, 0, 1),
        (2, 1, 2),
        (2, 2, 1),
        (3, 0, 1),
        (3, 1, 3),
        (3, 2, 3),
        (3, 3, 1),
        (4, 2, 6),
        (5, 2, 10),
        (6, 3, 20),
        (7, 3, 35),
        (8, 4, 70),
        (10, 5, 252),
        (12, 6, 924),
        (20, 10, 184756),
    ];
    for (n, k, expected) in table {
        assert_eq!(integer(&format!("binomial({n},{k})")), expected, "{n},{k}");
    }
    assert_eq!(integer("binomial(3,4)"), 0);
}

#[test]
fn exact_floor_ceil_round() {
    let table = [
        ("floor(7/2)", 3),
        ("ceil(7/2)", 4),
        ("round(7/2)", 4),
        ("floor(-7/2)", -4),
        ("ceil(-7/2)", -3),
        ("round(-7/2)", -4),
        ("floor(5/2)", 2),
        ("ceil(5/2)", 3),
        ("round(5/2)", 3),
        ("floor(-5/2)", -3),
        ("ceil(-5/2)", -2),
        ("round(-5/2)", -3),
        ("floor(8/3)", 2),
        ("ceil(8/3)", 3),
        ("round(8/3)", 3),
        ("floor(-8/3)", -3),
        ("ceil(-8/3)", -2),
        ("round(-8/3)", -3),
    ];
    for (source, expected) in table {
        assert_eq!(integer(source), expected, "{source}");
    }
}

#[test]
fn integer_inputs_are_unchanged_by_rounding() {
    assert_eq!(text("floor(7)"), "7");
    assert_eq!(text("ceil(7)"), "7");
    assert_eq!(text("round(7)"), "7");
    assert_eq!(text("floor(-7)"), "-7");
    assert_eq!(text("ceil(-7)"), "-7");
    assert_eq!(text("round(-7)"), "-7");
}

#[test]
fn approximate_rounding() {
    let sources = [
        "approx(floor(pi),30)",
        "approx(ceil(pi),30)",
        "approx(round(pi),30)",
        "approx(floor(-pi),30)",
        "approx(ceil(-pi),30)",
        "approx(round(-pi),30)",
    ];
    let expected = [3.0, 4.0, 3.0, -4.0, -3.0, -3.0];
    for (source, expected) in sources.into_iter().zip(expected) {
        let (session, value) = evaluate(source);
        match session.view(value).unwrap() {
            ExprView::Real(value) => assert_eq!(value.to_f64(), expected, "{source}"),
            _ => panic!("expected a real for {source}"),
        }
    }
}

#[test]
fn real_rounding_methods() {
    let value = Real::pi(30).unwrap();
    assert_eq!(value.floor().unwrap().to_f64(), 3.0);
    assert_eq!(value.ceil().unwrap().to_f64(), 4.0);
    assert_eq!(value.round().unwrap().to_f64(), 3.0);
    let negative = value.neg().unwrap();
    assert_eq!(negative.floor().unwrap().to_f64(), -4.0);
    assert_eq!(negative.ceil().unwrap().to_f64(), -3.0);
    assert_eq!(negative.round().unwrap().to_f64(), -3.0);
}

#[test]
fn rational_rounding_methods() {
    let value = Rat::from_i64(7, 2).unwrap();
    assert_eq!(value.floor().to_i64().unwrap(), 3);
    assert_eq!(value.ceil().to_i64().unwrap(), 4);
    assert_eq!(value.round().to_i64().unwrap(), 4);
    let negative = Rat::from_i64(-7, 2).unwrap();
    assert_eq!(negative.floor().to_i64().unwrap(), -4);
    assert_eq!(negative.ceil().to_i64().unwrap(), -3);
    assert_eq!(negative.round().to_i64().unwrap(), -4);
    let exact = Rat::from_i64(8, 2).unwrap();
    assert_eq!(exact.floor().to_i64().unwrap(), 4);
    assert_eq!(exact.ceil().to_i64().unwrap(), 4);
    assert_eq!(exact.round().to_i64().unwrap(), 4);
}

#[test]
fn numeric_function_arity_errors() {
    for source in [
        "gcd(1)",
        "lcm(1)",
        "factorial()",
        "factorial(1,2)",
        "binomial(1)",
        "binomial(1,2,3)",
        "floor()",
        "floor(1,2)",
        "ceil()",
        "ceil(1,2)",
        "round()",
        "round(1,2)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn numeric_symbols_are_protected() {
    for name in [
        "gcd",
        "lcm",
        "factorial",
        "binomial",
        "floor",
        "ceil",
        "round",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(&format!("{name}=1")).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{name}");
    }
}

#[test]
fn integer_helpers_hold_large_values() {
    let a = Int::from_i64(123456789);
    let b = Int::from_i64(987654321);
    assert_eq!(a.gcd(&b).to_decimal(), "9");
}
