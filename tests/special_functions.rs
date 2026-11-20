use mithqal::Session;

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn gamma_exact_integers() {
    assert_eq!(text("gamma(1)"), "1");
    assert_eq!(text("gamma(2)"), "1");
    assert_eq!(text("gamma(4)"), "6");
    assert_eq!(text("gamma(5)"), "24");
    assert_eq!(text("gamma(10)"), "362880");
}

#[test]
fn gamma_exact_half_integers() {
    assert_eq!(text("gamma(1/2)"), "sqrt(pi)");
    assert_eq!(text("gamma(3/2)"), "(1/2)*sqrt(pi)");
    assert_eq!(text("gamma(5/2)"), "(3/4)*sqrt(pi)");
    assert_eq!(text("gamma(-1/2)"), "-(2*sqrt(pi))");
    assert_eq!(text("gamma(-3/2)"), "(4/3)*sqrt(pi)");
    assert_eq!(text("gamma(-5/2)"), "-((8/15)*sqrt(pi))");
}

#[test]
fn gamma_poles_are_undefined() {
    assert_eq!(text("gamma(0)"), "undefined");
    assert_eq!(text("gamma(-1)"), "undefined");
    assert_eq!(text("gamma(-8)"), "undefined");
    assert_eq!(text("lgamma(0)"), "undefined");
    assert_eq!(text("lgamma(-1)"), "undefined");
}

#[test]
fn gamma_symbolic_arguments_remain_symbolic() {
    assert_eq!(text("gamma(x)"), "gamma(x)");
    assert_eq!(text("gamma(x^2)"), "gamma(x^2)");
    assert_eq!(text("lgamma(x)"), "lgamma(x)");
    assert_eq!(text("lgamma(x^2)"), "lgamma(x^2)");
}

#[test]
fn lgamma_exact_values() {
    assert_eq!(text("lgamma(2)"), "0");
    assert_eq!(text("lgamma(5)"), "ln(24)");
    assert_eq!(text("lgamma(2+3)"), "ln(24)");
    assert_eq!(text("lgamma(1/2)"), "ln(sqrt(pi))");
    assert_eq!(text("lgamma(-1/2)"), "ln(2*sqrt(pi))");
}

#[test]
fn gamma_approximation_still_works() {
    let mut session = Session::new();
    let value = session.eval_statement("approx(gamma(5),30)").unwrap();
    let text = session.format_infix(value).unwrap();
    assert!(text.starts_with("24."), "{text}");
    let value = session.eval_statement("approx(gamma(1/2),30)").unwrap();
    let text = session.format_infix(value).unwrap();
    assert!(text.starts_with("1.772"), "{text}");
}

#[test]
fn natural_log_of_e_is_exact() {
    assert_eq!(text("ln(e)"), "1");
}

#[test]
fn gamma_negative_arguments_between_minus_one_and_zero() {
    let value = text("gamma(-1/3)");
    assert!(value.starts_with("-4.062353"), "{value}");
    let value = text("gamma(-0.2)");
    assert!(value.starts_with("-5.821148"), "{value}");
    let value = text("lgamma(-1/3)");
    assert!(value.starts_with("1.401"), "{value}");
}

#[test]
fn gamma_numeric_arguments_are_defined() {
    let value = text("gamma(1/3)");
    assert!(value.starts_with("2.678938"), "{value}");
    let value = text("gamma(2+3*i)");
    assert!(value.starts_with("complex("), "{value}");
    let value = text("lgamma(2+3*i)");
    assert!(value.starts_with("complex("), "{value}");
    let value = text("lgamma(1/3)");
    assert!(value.starts_with("0.985420"), "{value}");
}
