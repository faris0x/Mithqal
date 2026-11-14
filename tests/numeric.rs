use cas::{Config, ErrorCode, ExprView, NumError, Rat, Real, Session};

fn approx(source: &str) -> (Session, String) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    let text = session.format_infix(value);
    (session, text)
}

#[test]
fn approximate_rationals() {
    let (_session, text) = approx("approx(1/3,30)");
    assert!(
        text.starts_with("0.33333333333333333333333333333"),
        "{text}"
    );
}

#[test]
fn approximate_constants() {
    let (_session, text) = approx("approx(pi,30)");
    assert!(
        text.starts_with("3.14159265358979323846264338327"),
        "{text}"
    );
    let (_session, text) = approx("approx(e,30)");
    assert!(
        text.starts_with("2.71828182845904523536028747135"),
        "{text}"
    );
}

#[test]
fn approximate_expression_trees() {
    let (_session, text) = approx("approx(1/3+1/6,30)");
    assert!(text.starts_with("0.5"), "{text}");
    let (_session, text) = approx("approx(sqrt(2),30)");
    assert!(
        text.starts_with("1.41421356237309504880168872420"),
        "{text}"
    );
    let (_session, text) = approx("approx(sin(pi/6),30)");
    assert!(text.starts_with("0.5"), "{text}");
    let (_session, text) = approx("approx(2^100,30)");
    assert!(
        text.starts_with("1267650600228229401496703205376"),
        "{text}"
    );
}

#[test]
fn approximate_error_values() {
    let (_session, text) = approx("approx(1/0,30)");
    assert_eq!(text, "undefined");
}

#[test]
fn approximate_symbolic_values_stay_unevaluated() {
    let (_session, text) = approx("approx(x,30)");
    assert_eq!(text, "approx(x,30)");
}

#[test]
fn approximate_digit_count_is_checked() {
    let mut session = Session::new();
    for source in ["approx(1/3,0)", "approx(1/3,-1)", "approx(1/3,10001)"] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalDefinition, "source {source}");
    }
}

#[test]
fn real_views_expose_precision() {
    let mut session = Session::new();
    let value = session.eval_statement("approx(pi,30)").unwrap();
    match session.view(value) {
        ExprView::Real(real) => {
            assert!(real.precision_digits() > 25.0);
            assert!(real.accuracy_digits() > 25.0);
        }
        _ => panic!("expected a real"),
    }
}

#[test]
fn real_arithmetic_propagates_error() {
    let third = Real::from_rat(&Rat::from_i64(1, 3).unwrap(), 30).unwrap();
    let sum = third.add(&third).unwrap();
    assert!(sum.precision_digits() < third.precision_digits());
    let product = third.mul(&third).unwrap();
    assert!(product.precision_digits() <= third.precision_digits() + 1.0);
}

#[test]
fn real_domain_errors() {
    let negative = Real::from_rat(&Rat::from_i64(-1, 1).unwrap(), 30).unwrap();
    assert!(matches!(negative.sqrt(), Err(NumError::Domain)));
    assert!(matches!(negative.ln(), Err(NumError::Domain)));
    let zero = Real::from_rat(&Rat::zero(), 30).unwrap();
    assert!(matches!(third().div(&zero), Err(NumError::DivisionByZero)));
}

#[test]
fn real_decimal_output() {
    let zero = Real::from_rat(&Rat::zero(), 30).unwrap();
    assert_eq!(zero.to_decimal(), "0.0");
    let half = Real::from_rat(&Rat::from_i64(1, 2).unwrap(), 30).unwrap();
    assert!(half.to_decimal().starts_with("0.5"));
    let negative = Real::from_rat(&Rat::from_i64(-3, 2).unwrap(), 30).unwrap();
    assert!(negative.to_decimal().starts_with("-1.5"));
}

#[test]
fn approximate_values_survive_collection() {
    let mut session = Session::new();
    let value = session.eval_statement("approx(pi,30)").unwrap();
    session.collect_garbage(&[value]);
    assert!(session.format_infix(value).starts_with("3.14159"));
}

#[test]
fn approximate_budget_errors_are_reported() {
    let config = Config {
        max_steps: 1,
        max_depth: 8,
        ..Config::default()
    };
    let mut session = Session::with_config(config);
    let error = session.eval_statement("approx(pi,30)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}

fn third() -> Real {
    Real::from_rat(&Rat::from_i64(1, 3).unwrap(), 30).unwrap()
}
