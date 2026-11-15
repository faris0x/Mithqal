use mithqal::{ErrorCode, ExprView, Session};

fn evaluate(source: &str) -> (Session, u32) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value)
}

#[test]
fn differentiate_polynomials() {
    assert_eq!(text("differentiate(x^2,x)"), "2*x");
    assert_eq!(text("differentiate(x^3+2*x,x)"), "2+3*x^2");
    assert_eq!(text("differentiate(x^2*y,x)"), "2*x*y");
    assert_eq!(text("differentiate(x^2+3*x+5,x)"), "3+2*x");
    assert_eq!(text("differentiate(5,x)"), "0");
    assert_eq!(text("differentiate(y,x)"), "0");
    assert_eq!(text("differentiate(sin(2),x)"), "0");
    assert_eq!(text("differentiate(2^3,x)"), "0");
    assert_eq!(text("differentiate(f(y),x)"), "0");
}

#[test]
fn differentiate_elementary_functions() {
    assert_eq!(text("differentiate(sin(x),x)"), "cos(x)");
    assert_eq!(text("differentiate(cos(x^2),x)"), "-(2*x*sin(x^2))");
    assert_eq!(text("differentiate(exp(2*x),x)"), "2*exp(2*x)");
    assert_eq!(text("differentiate(ln(x),x)"), "x^(-1)");
    assert_eq!(text("differentiate(tan(x),x)"), "cos(x)^(-2)");
    assert_eq!(text("differentiate(sqrt(x),x)"), "(2*sqrt(x))^(-1)");
    assert_eq!(text("differentiate(x^x,x)"), "(1+ln(x))*x^x");
}

#[test]
fn differentiate_inverse_functions() {
    assert_eq!(text("differentiate(asin(x),x)"), "(1-x^2)^(-1/2)");
    assert_eq!(text("differentiate(acos(x),x)"), "-(1-x^2)^(-1/2)");
    assert_eq!(text("differentiate(atan(x),x)"), "(1+x^2)^(-1)");
    assert_eq!(text("differentiate(asinh(x),x)"), "(1+x^2)^(-1/2)");
    assert_eq!(text("differentiate(atanh(x),x)"), "-(-1+x^2)^(-1)");
}

#[test]
fn differentiate_products_and_quotients() {
    assert_eq!(text("differentiate(sin(x)*cos(x),x)"), "-sin(x)^2+cos(x)^2");
    assert_eq!(text("differentiate(x/(x+1),x)"), "(1+x^2+2*x)^(-1)");
}

#[test]
fn integrate_polynomials() {
    assert_eq!(text("integrate(x^2,x)"), "(1/3)*x^3");
    assert_eq!(text("integrate(2*x+1,x)"), "x+x^2");
    assert_eq!(text("integrate(x^3-2*x+1,x)"), "x-x^2+(1/4)*x^4");
    assert_eq!(text("integrate(x*(x+1),x)"), "(1/3)*x^3+(1/2)*x^2");
}

#[test]
fn integrate_elementary_functions() {
    assert_eq!(text("integrate(sin(x),x)"), "-cos(x)");
    assert_eq!(text("integrate(cos(2*x),x)"), "(1/2)*sin(2*x)");
    assert_eq!(text("integrate(exp(3*x),x)"), "(1/3)*exp(3*x)");
    assert_eq!(text("integrate(ln(x),x)"), "-x+x*ln(x)");
    assert_eq!(text("integrate(x^(-1/2),x)"), "2*x^(1/2)");
    assert_eq!(text("integrate(x^-2,x)"), "-x^(-1)");
    assert_eq!(text("integrate(1/x,x)"), "ln(abs(x))");
}

#[test]
fn integrate_linear_substitutions() {
    assert_eq!(text("integrate(1/(2*x+1),x)"), "(1/2)*ln(abs(1+2*x))");
    assert_eq!(text("integrate(sin(2*x+1),x)"), "-((1/2)*cos(1+2*x))");
    assert_eq!(text("integrate(1/(x+1),x)"), "ln(abs(1+x))");
    assert_eq!(text("integrate(2*sin(3*x),x)"), "-((2/3)*cos(3*x))");
}

#[test]
fn calculus_leaves_unsupported_forms_unevaluated() {
    assert!(text("differentiate(f(x),x)").contains("differentiate"));
    assert!(text("integrate(f(x),x)").contains("integrate"));
    assert!(text("integrate(sin(x^2),x)").contains("integrate"));
}

#[test]
fn fundamental_theorem_round_trip() {
    let mut session = Session::new();
    let integral = session.eval_statement("integrate(x^3-2*x+1,x)").unwrap();
    let derivative = session
        .eval_statement(&format!(
            "expand(differentiate({},x))",
            session.format_infix(integral)
        ))
        .unwrap();
    let target = session.eval_statement("expand(x^3-2*x+1)").unwrap();
    assert_eq!(derivative, target);
}

#[test]
fn calculus_arity_errors() {
    for source in [
        "differentiate(x)",
        "differentiate(x,x,x)",
        "integrate(x)",
        "integrate(x,x,x)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn calculus_symbols_are_protected() {
    let mut session = Session::new();
    for name in ["differentiate", "integrate"] {
        let source = format!("{name}=1");
        let error = session.eval_statement(&source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "source {source}");
    }
}

#[test]
fn calculus_results_survive_collection() {
    let mut session = Session::new();
    let value = session.eval_statement("differentiate(x^3,x)").unwrap();
    session.collect_garbage(&[value]);
    match session.view(value) {
        ExprView::Call { .. } => {}
        _ => panic!("expected a derivative call"),
    }
    assert_eq!(session.format_infix(value), "3*x^2");
}

#[test]
fn integrate_inverse_function_patterns() {
    assert_eq!(text("integrate(1/(1+x^2),x)"), "atan(x)");
    assert_eq!(text("integrate(1/sqrt(1-x^2),x)"), "asin(x)");
    assert_eq!(text("integrate(1/sqrt(x),x)"), "2*x^(1/2)");
}

#[test]
fn definite_integrals() {
    assert_eq!(text("integrate(x^2,x,0,3)"), "9");
    assert_eq!(text("integrate(2*x+1,x,1,2)"), "4");
    assert_eq!(text("integrate(x^3-2*x+1,x,0,1)"), "1/4");
    assert_eq!(text("integrate(x^2,x,1,0)"), "-1/3");
    assert_eq!(text("integrate(1/x,x,1,2)"), "ln(2)");
}

#[test]
fn definite_integrals_accept_symbolic_bounds() {
    assert_eq!(text("integrate(x^2,x,a,b)"), "-(1/3)*a^3+(1/3)*b^3");
    assert_eq!(text("integrate(sin(x),x,0,1)"), "1-cos(1)");
}

#[test]
fn definite_integrals_leave_unsupported_forms_unevaluated() {
    assert!(text("integrate(f(x),x,0,1)").contains("integrate"));
}

#[test]
fn integration_by_parts() {
    assert_eq!(text("integrate(x*exp(x),x)"), "-exp(x)+x*exp(x)");
    assert_eq!(text("integrate(x*sin(x),x)"), "-x*cos(x)+sin(x)");
    assert_eq!(text("integrate(x*ln(x),x)"), "-(1/4)*x^2+(1/2)*ln(x)*x^2");
    assert_eq!(
        text("integrate(x^2*exp(x),x)"),
        "-2*(-exp(x)+x*exp(x))+exp(x)*x^2"
    );
}

#[test]
fn partial_fraction_integrals() {
    assert_eq!(
        text("integrate(1/(x^2-1),x)"),
        "-(1/2)*ln(abs(1+x))+(1/2)*ln(abs(-1+x))"
    );
    assert_eq!(
        text("integrate(1/((x-1)*(x-2)),x)"),
        "ln(abs(-2+x))-ln(abs(-1+x))"
    );
    assert_eq!(
        text("integrate(1/(2*x^2-2),x)"),
        "-(1/4)*ln(abs(1+x))+(1/4)*ln(abs(-1+x))"
    );
}

#[test]
fn improper_integrals() {
    assert_eq!(text("integrate(exp(-x),x,0,inf)"), "1");
    assert_eq!(text("integrate(1/x^2,x,1,inf)"), "1");
    assert_eq!(text("integrate(1/(1+x^2),x,0,inf)"), "(1/2)*pi");
    assert_eq!(text("integrate(x*exp(-x),x,0,inf)"), "1");
    assert!(text("integrate(1/x,x,1,inf)").contains("integrate"));
    assert!(text("integrate(exp(x),x,0,inf)").contains("integrate"));
}

#[test]
fn improper_integrals_with_finite_singularities() {
    assert_eq!(text("integrate(ln(x),x,0,1)"), "-1");
    assert_eq!(text("integrate(1/sqrt(x),x,0,1)"), "2");
    assert_eq!(text("integrate(1/sqrt(1-x),x,0,1)"), "2");
    assert_eq!(text("integrate(x^(-2/3),x,0,1)"), "3");
    assert_eq!(text("integrate(1/sqrt(1-x^2),x,0,1)"), "(1/2)*pi");
    assert_eq!(text("integrate(1/(x^2+1),x,0,1)"), "(1/4)*pi");
}

#[test]
fn divergent_integrals_with_finite_singularities_stay_unevaluated() {
    for source in [
        "integrate(1/x,x,0,1)",
        "integrate(1/x^2,x,0,1)",
        "integrate(1/(x-1),x,0,1)",
        "integrate(1/(x-1),x,0,2)",
        "integrate(1/x,x,-1,1)",
        "integrate(1/x^2,x,-1,1)",
    ] {
        let result = text(source);
        assert!(result.contains("integrate"), "{source} => {result}");
    }
}

#[test]
fn transcendental_interior_singularities_stay_unevaluated() {
    for source in [
        "integrate(tan(x),x,0,2)",
        "integrate(tan(x),x,0,3)",
        "integrate(ln(cos(x)),x,0,2)",
    ] {
        let result = text(source);
        assert!(result.contains("integrate"), "{source} => {result}");
    }
    assert_eq!(text("integrate(tan(x),x,0,1)"), "-ln(abs(cos(1)))");
}

#[test]
fn integrate_inverse_functions() {
    assert_eq!(text("integrate(asin(x),x)"), "(1-x^2)^(1/2)+x*asin(x)");
    assert_eq!(text("integrate(acos(x),x)"), "-(1-x^2)^(1/2)+x*acos(x)");
    assert_eq!(text("integrate(atan(x),x)"), "-(1/2)*ln(1+x^2)+x*atan(x)");
    assert_eq!(text("integrate(asinh(x),x)"), "-(1+x^2)^(1/2)+x*asinh(x)");
    assert_eq!(text("integrate(acosh(x),x)"), "-(-1+x^2)^(1/2)+x*acosh(x)");
    assert_eq!(text("integrate(atanh(x),x)"), "(1/2)*ln(1-x^2)+x*atanh(x)");
}

#[test]
fn inverse_function_integrals_differentiate_back() {
    for source in [
        "asin(x)", "acos(x)", "atan(x)", "asinh(x)", "acosh(x)", "atanh(x)",
    ] {
        let mut session = Session::new();
        let integral = session
            .eval_statement(&format!("integrate({source},x)"))
            .unwrap();
        let derivative = session
            .eval_statement(&format!(
                "differentiate({},x)",
                session.format_infix(integral)
            ))
            .unwrap();
        let target = session.eval_statement(source).unwrap();
        assert_eq!(
            session.format_infix(derivative),
            session.format_infix(target),
            "round trip failed for {source}"
        );
    }
}
