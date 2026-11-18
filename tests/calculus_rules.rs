use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn differentiation_rules_cover_elementary_forms() {
    assert_eq!(show("derive(x^3+2*x,x)"), "2+3*x^2");
    assert_eq!(show("derive(x*y,x)"), "y");
    assert_eq!(show("derive(sin(x),x)"), "cos(x)");
    assert_eq!(
        show("derive(exp(x)*sin(x),x)"),
        "cos(x)*exp(x)+exp(x)*sin(x)"
    );
    assert_eq!(show("derive(ln(x^2),x)"), "2/x");
    assert_eq!(show("derive(sqrt(x),x)"), "1/(2*sqrt(x))");
    assert_eq!(show("derive(atan(x),x)"), "1/(1+x^2)");
}

#[test]
fn differentiation_falls_back_to_the_native_kernel() {
    assert_eq!(show("derive(f(x),x)"), "differentiate(f(x),x)");
    assert_eq!(show("derive(y,x)"), "0");
}

#[test]
fn integration_rules_cover_elementary_forms() {
    assert_eq!(show("antiderivative(x^2,x)"), "(1/3)*x^3");
    assert_eq!(show("antiderivative(x,x)"), "(1/2)*x^2");
    assert_eq!(show("antiderivative(1/x,x)"), "ln(x)");
    assert_eq!(show("antiderivative(sin(x),x)"), "-cos(x)");
    assert_eq!(show("antiderivative(cos(x),x)"), "sin(x)");
    assert_eq!(show("antiderivative(exp(x),x)"), "exp(x)");
}

#[test]
fn integration_rules_handle_factors_and_known_antiderivatives() {
    assert_eq!(show("antiderivative(2*x,x)"), "x^2");
    assert_eq!(show("antiderivative(2*x*y,x)"), "y*x^2");
    assert_eq!(show("antiderivative(ln(x),x)"), "-x+x*ln(x)");
    assert_eq!(
        show("antiderivative(atan(x),x)"),
        "-(1/2)*ln(1+x^2)+x*atan(x)"
    );
    assert_eq!(show("antiderivative(y,x)"), "x*y");
}
