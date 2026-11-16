use mithqal::ExprId;
use mithqal::{Config, ErrorCode, ExprView, NumError, Rat, Real, Session};

fn approx(source: &str) -> (Session, String) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    let text = session.format_infix(value).unwrap();
    (session, text)
}

fn evaluated(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
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
    match session.view(value).unwrap() {
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
    assert!(session.format_infix(value).unwrap().starts_with("3.14159"));
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

fn real_value(source: &str) -> f64 {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    match session.view(value).unwrap() {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        _ => panic!("expected a real value for {source}"),
    }
}

#[test]
fn real_constructors_and_predicates() {
    let zero = Real::zero(30).unwrap();
    let one = Real::one(30).unwrap();
    assert!(zero.is_zero());
    assert!(!one.is_zero());
    assert_eq!(zero.sign(), 0);
    assert_eq!(one.sign(), 1);
    assert_eq!(Real::from_i64(-5, 30).unwrap().sign(), -1);
    assert_eq!(Real::from_i64(-5, 30).unwrap().to_f64(), -5.0);
}

#[test]
fn real_add_sub_mul_div() {
    let a = Real::from_i64(5, 30).unwrap();
    let b = Real::from_i64(3, 30).unwrap();
    assert_eq!(a.add(&b).unwrap().to_f64(), 8.0);
    assert_eq!(a.sub(&b).unwrap().to_f64(), 2.0);
    assert_eq!(a.mul(&b).unwrap().to_f64(), 15.0);
    assert_eq!(a.div(&b).unwrap().to_f64(), 5.0 / 3.0);
    assert_eq!(a.neg().unwrap().to_f64(), -5.0);
    assert_eq!(a.neg().unwrap().abs().unwrap().to_f64(), 5.0);
}

#[test]
fn real_powers() {
    let two = Real::from_i64(2, 30).unwrap();
    assert_eq!(two.pow_si(0).unwrap().to_f64(), 1.0);
    assert_eq!(two.pow_si(1).unwrap().to_f64(), 2.0);
    assert_eq!(two.pow_si(10).unwrap().to_f64(), 1024.0);
    assert_eq!(two.pow_si(-1).unwrap().to_f64(), 0.5);
    assert_eq!(two.pow_si(-2).unwrap().to_f64(), 0.25);
}

#[test]
fn real_elementary_functions() {
    let zero = Real::zero(30).unwrap();
    let one = Real::one(30).unwrap();
    let two = Real::from_i64(2, 30).unwrap();
    assert_eq!(zero.sin().unwrap().to_f64(), 0.0);
    assert_eq!(zero.cos().unwrap().to_f64(), 1.0);
    assert_eq!(zero.tan().unwrap().to_f64(), 0.0);
    assert_eq!(zero.exp().unwrap().to_f64(), 1.0);
    assert_eq!(one.ln().unwrap().to_f64(), 0.0);
    assert!((two.sqrt().unwrap().to_f64() - 2f64.sqrt()).abs() < 1e-14);
    assert!((zero.atan().unwrap().to_f64() - 0.0).abs() < 1e-14);
    assert!((one.atan().unwrap().to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14);
}

#[test]
fn real_comparisons() {
    let one = Real::from_i64(1, 30).unwrap();
    let two = Real::from_i64(2, 30).unwrap();
    assert_eq!(one.cmp(&two), core::cmp::Ordering::Less);
    assert_eq!(two.cmp(&one), core::cmp::Ordering::Greater);
    assert_eq!(one.cmp(&one.clone()), core::cmp::Ordering::Equal);
}

#[test]
fn real_precision_responds_to_digits() {
    let low = Real::from_rat(&Rat::from_i64(1, 3).unwrap(), 10).unwrap();
    let high = Real::from_rat(&Rat::from_i64(1, 3).unwrap(), 40).unwrap();
    assert!(high.precision_digits() > low.precision_digits());
    assert!(high.accuracy_digits() > low.accuracy_digits());
    assert!(low.to_decimal().starts_with("0.3333333333"));
}

#[test]
fn approximate_integers_and_negatives() {
    assert_eq!(real_value("approx(0,30)"), 0.0);
    assert_eq!(real_value("approx(42,30)"), 42.0);
    assert_eq!(real_value("approx(-7,30)"), -7.0);
    assert_eq!(real_value("approx(1/2,30)"), 0.5);
    assert_eq!(real_value("approx(-3/4,30)"), -0.75);
}

#[test]
fn approximate_arithmetic_values() {
    assert_eq!(real_value("approx(1+2,30)"), 3.0);
    assert_eq!(real_value("approx(2*3+4,30)"), 10.0);
    assert_eq!(real_value("approx(2^-2,30)"), 0.25);
    assert_eq!(
        real_value("approx(2^100,30)"),
        1267650600228229401496703205376.0
    );
    assert_eq!(real_value("approx(sqrt(2)^2,30)"), 2.0);
    assert_eq!(real_value("approx(exp(ln(2)),30)"), 2.0);
    assert!((real_value("approx(atan(1),30)") - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
}

#[test]
fn approximate_general_powers() {
    assert!((real_value("approx(9^(1/2),30)") - 3.0).abs() < 1e-9);
    assert!((real_value("approx(27^(1/3),30)") - 3.0).abs() < 1e-9);
    assert!((real_value("approx(2^pi,30)") - 2f64.powf(std::f64::consts::PI)).abs() < 1e-9);
    assert!((real_value("approx(2^(1/2),30)") - 2f64.sqrt()).abs() < 1e-14);
}

#[test]
fn real_general_power_method() {
    let two = Real::from_i64(2, 30).unwrap();
    let half = Real::from_rat(&Rat::from_i64(1, 2).unwrap(), 30).unwrap();
    assert!((two.pow(&half).unwrap().to_f64() - 2f64.sqrt()).abs() < 1e-14);
    let pi = Real::pi(30).unwrap();
    assert!((two.pow(&pi).unwrap().to_f64() - 2f64.powf(std::f64::consts::PI)).abs() < 1e-12);
}

#[test]
fn approximate_default_digits() {
    let (session, value) = evaluated("approx(pi)");
    match session.view(value).unwrap() {
        ExprView::Real(value) => assert!(value.to_decimal().starts_with("3.1415")),
        _ => panic!("expected a real"),
    }
}

#[test]
fn approximate_symbolic_sums_stay_unevaluated() {
    let (session, value) = evaluated("approx(1+x,30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
}

#[test]
fn approximate_rational_powers_are_numeric() {
    let (session, value) = evaluated("approx(2^(1/2),30)");
    match session.view(value).unwrap() {
        ExprView::Real(value) => {
            assert!((value.to_f64() - 2f64.sqrt()).abs() < 1e-14);
        }
        _ => panic!("expected a real"),
    }
}

#[test]
fn approximate_digit_count_must_be_an_integer() {
    let mut session = Session::new();
    let error = session.eval_statement("approx(1/3,1/2)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn real_hyperbolic_functions() {
    let one = Real::one(30).unwrap();
    let two = Real::from_i64(2, 30).unwrap();
    let half = Real::from_rat(&Rat::from_i64(1, 2).unwrap(), 30).unwrap();
    assert!((one.sinh().unwrap().to_f64() - 1f64.sinh()).abs() < 1e-14);
    assert!((one.cosh().unwrap().to_f64() - 1f64.cosh()).abs() < 1e-14);
    assert!((one.tanh().unwrap().to_f64() - 1f64.tanh()).abs() < 1e-14);
    assert!((two.asinh().unwrap().to_f64() - 2f64.asinh()).abs() < 1e-14);
    assert!((two.acosh().unwrap().to_f64() - 2f64.acosh()).abs() < 1e-14);
    assert!((half.atanh().unwrap().to_f64() - 0.5f64.atanh()).abs() < 1e-14);
}

#[test]
fn real_inverse_hyperbolic_domains() {
    let zero = Real::zero(30).unwrap();
    let one = Real::one(30).unwrap();
    let two = Real::from_i64(2, 30).unwrap();
    assert!(matches!(zero.acosh(), Err(NumError::Domain)));
    assert!(matches!(one.atanh(), Err(NumError::Domain)));
    assert!(matches!(two.atanh(), Err(NumError::Domain)));
    assert!((one.acosh().unwrap().to_f64()).abs() < 1e-14);
    assert_eq!(zero.sinh().unwrap().to_f64(), 0.0);
    assert_eq!(zero.cosh().unwrap().to_f64(), 1.0);
}

#[test]
fn approximate_hyperbolic_evaluates() {
    assert!((real_value("approx(sinh(1),30)") - 1f64.sinh()).abs() < 1e-12);
    assert!((real_value("approx(cosh(1),30)") - 1f64.cosh()).abs() < 1e-12);
    assert!((real_value("approx(tanh(1),30)") - 1f64.tanh()).abs() < 1e-12);
    assert!((real_value("approx(asinh(2),30)") - 2f64.asinh()).abs() < 1e-12);
    assert!((real_value("approx(acosh(2),30)") - 2f64.acosh()).abs() < 1e-12);
    assert!((real_value("approx(atanh(1/2),30)") - 0.5f64.atanh()).abs() < 1e-12);
    assert!((real_value("approx(sinh(1)^2-cosh(1)^2,30)") + 1.0).abs() < 1e-12);
}

#[test]
fn real_power_of_zero_base_has_finite_error() {
    let zero = Real::zero(30).unwrap();
    let exponent = Real::from_rat(&Rat::from_i64(5, 2).unwrap(), 30).unwrap();
    let value = zero.pow(&exponent).unwrap();
    assert_eq!(value.to_f64(), 0.0);
    assert!(value.precision_digits().is_infinite());
    assert_eq!(real_value("approx(0^pi,30)"), 0.0);
}

#[test]
fn real_hyperbolic_error_shrinks_precision() {
    let one = Real::one(30).unwrap();
    let value = one.sinh().unwrap();
    assert!(value.precision_digits() < one.precision_digits() + 1.0);
    assert!(value.accuracy_digits() < one.accuracy_digits());
}

#[test]
fn special_functions() {
    let value = |source: &str| real_value(source);
    assert!((value("approx(gamma(5),30)") - 24.0).abs() < 1e-12);
    assert!((value("approx(gamma(1/2),30)") - std::f64::consts::PI.sqrt()).abs() < 1e-12);
    assert!((value("approx(lgamma(5),30)") - 24f64.ln()).abs() < 1e-12);
    assert!((value("approx(erf(0),30)")).abs() < 1e-15);
    assert!((value("approx(erf(1),30)") - 0.8427007929497149).abs() < 1e-12);
    assert!((value("approx(erfc(1),30)") - 0.15729920705028513).abs() < 1e-12);
}

#[test]
fn special_function_domains() {
    let (session, value) = evaluated("approx(gamma(x),30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
    let (session, value) = evaluated("approx(lgamma(x),30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
}
