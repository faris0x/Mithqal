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

fn assert_factor_matches(source: &str) {
    let mut session = Session::new();
    let factored = session
        .eval_statement(&format!("factor({source})"))
        .unwrap();
    let expanded = session
        .eval_statement(&format!("expand({})", session.format_infix(factored)))
        .unwrap();
    let target = session
        .eval_statement(&format!("expand({source})"))
        .unwrap();
    assert_eq!(
        expanded,
        target,
        "factor mismatch for {source}: {}",
        session.format_infix(factored)
    );
}

#[test]
fn expand_products_and_powers() {
    assert_eq!(text("expand((x+1)^2)"), "1+x^2+2*x");
    assert_eq!(text("expand((x+y)^3)"), "x^3+y^3+3*x*y^2+3*y*x^2");
    assert_eq!(text("expand((x+y+z)^2)"), "x^2+y^2+z^2+2*x*y+2*x*z+2*y*z");
    assert_eq!(text("expand((x+1)(x-1))"), "-1+x^2");
    assert_eq!(text("expand(2*(x+1))"), "2+2*x");
    assert_eq!(text("expand(0*(x+1))"), "0");
    assert_eq!(text("expand(x^2)"), "x^2");
}

#[test]
fn expand_keeps_non_polynomial_inputs_unevaluated() {
    assert!(text("expand(sin(x+1))").contains("expand"));
    assert!(text("expand(exp(x))").contains("expand"));
}

#[test]
fn factor_univariate_rational_roots() {
    assert_factor_matches("x^2-1");
    assert_factor_matches("x^4-1");
    assert_factor_matches("6*x^2+5*x+1");
    assert_factor_matches("2*x^2+4*x");
    assert_factor_matches("x^3-3*x^2+3*x-1");
    assert_factor_matches("x^4-5*x^2+4");
    assert_factor_matches("2*x+1");
    assert_factor_matches("1/2*x+1/2");
    assert_factor_matches("x^2/4-1");
}

#[test]
fn factor_repeated_roots_and_content() {
    assert_eq!(text("factor((x-1)^3)"), "(-1+x)^3");
    assert_eq!(text("factor(4*x^2+4*x+1)"), "(1+2*x)^2");
    assert_eq!(text("factor(2*x^2+4*x)"), "2*x*(2+x)");
    assert_eq!(text("factor(-x^2-x)"), "-(x*(1+x))");
    assert_eq!(text("factor(x^2+1)"), "1+x^2");
}

#[test]
fn factor_multivariate_patterns() {
    assert_eq!(text("factor(x^2-y^2)"), "(x+y)*(x-y)");
    assert_eq!(text("factor(x^2+2*x*y+y^2)"), "(x+y)^2");
    assert_eq!(text("factor(x^2-2*x*y+y^2)"), "(x-y)^2");
}

#[test]
fn polynomial_gcd_is_monic() {
    assert_eq!(text("polynomialGcd(x^2-1,x^2-1,x)"), "-1+x^2");
    assert_eq!(text("polynomialGcd(x^2-1,x^2+2*x+1,x)"), "1+x");
    assert_eq!(text("polynomialGcd(2*x^2-2,4*x+4,x)"), "1+x");
    assert!(text("polynomialGcd(x*y,x,x)").contains("polynomialGcd"));
}

#[test]
fn resultant_values() {
    assert_eq!(text("resultant(x^2-1,x-2,x)"), "3");
    assert_eq!(text("resultant(x^2-1,x^2-4,x)"), "9");
    assert_eq!(text("resultant(x^2-3,x-2,x)"), "1");
    assert_eq!(text("resultant(x-2,x^2-1,x)"), "3");
    assert_eq!(text("resultant(2,x-1,x)"), "2");
    assert_eq!(text("resultant(2,3,x)"), "1");
}

#[test]
fn together_combines_fractions() {
    assert_eq!(text("together(1/x+1/y)"), "(x+y)*(x*y)^(-1)");
    assert_eq!(text("together(1/(x+1)+1/(x-1))"), "2*x*(-1+x^2)^(-1)");
    assert_eq!(text("together(x/x)"), "1");
}

#[test]
fn cancel_removes_common_factors() {
    assert_eq!(text("cancel((x^2-1)/(x+1))"), "-1+x");
    assert_eq!(text("cancel((x^2-1)/(x^2-2*x+1))"), "(1+x)*(-1+x)^(-1)");
    assert_eq!(text("cancel((x*y+x)/(x))"), "1+y");
}

#[test]
fn solve_exact_equations() {
    assert_eq!(text("solve(x^2-4,x)"), "[-2,2]");
    assert_eq!(text("solve(x^2-x-6,x)"), "[-2,3]");
    assert_eq!(text("solve(x^2-6*x+9,x)"), "[3]");
    assert_eq!(text("solve(x^3-x,x)"), "[-1,0,1]");
    assert_eq!(text("solve(2*x+3==0,x)"), "[-3/2]");
}

#[test]
fn solve_symbolic_and_complex_quadratics() {
    assert_eq!(text("solve(x^2-2,x)"), "[-sqrt(2),sqrt(2)]");
    assert_eq!(text("solve(x^2+1,x)"), "[-i,i]");
    assert_eq!(
        text("solve(x^2+x+1,x)"),
        "[(1/2)*(-1-i*sqrt(3)),(1/2)*(-1+i*sqrt(3))]"
    );
}

#[test]
fn solve_reports_trivial_or_unsupported_cases() {
    assert_eq!(text("solve(1,x)"), "[]");
    assert!(text("solve(0,x)").contains("solve"));
    assert!(text("solve(x^3+2,x)").contains("solve"));
    assert!(text("solve(x*y,x)").contains("solve"));
}

#[test]
fn algebra_arity_errors() {
    for source in [
        "expand()",
        "factor(x,y)",
        "together()",
        "cancel()",
        "polynomialGcd(x,y)",
        "resultant(x,y)",
        "solve(x)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn algebra_symbols_are_protected() {
    let mut session = Session::new();
    for name in [
        "expand",
        "factor",
        "together",
        "cancel",
        "polynomialGcd",
        "resultant",
        "solve",
    ] {
        let source = format!("{name}=1");
        let error = session.eval_statement(&source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "source {source}");
    }
}

#[test]
fn factored_results_survive_collection() {
    let mut session = Session::new();
    let value = session.eval_statement("factor(x^4-1)").unwrap();
    session.collect_garbage(&[value]);
    match session.view(value) {
        ExprView::Call { .. } => {}
        _ => panic!("expected a factored call"),
    }
    let value = session.eval_statement("solve(x^2-4,x)").unwrap();
    session.collect_garbage(&[value]);
    assert_eq!(session.format_infix(value), "[-2,2]");
}

#[test]
fn algebra_size_limits_leave_inputs_unevaluated() {
    assert!(text("expand((x+y+z)^128)").contains("expand"));
    assert!(text("resultant(x^65+x+1,x^2+1,x)").contains("resultant"));
}

#[test]
fn cancel_multivariate_monomial_content() {
    assert_eq!(text("cancel((x^2*y+x)/(x*y))"), "(1+x*y)*y^(-1)");
    assert_eq!(text("together(x/y+z/y)"), "(x+z)*y^(-1)");
}

#[test]
fn factor_large_numeric_content() {
    assert_factor_matches("1024*x^2-1024");
    assert_eq!(text("factor(1024*x^2-1024)"), "1024*(-1+x)*(1+x)");
}

#[test]
fn solve_repeated_root_is_reported_once() {
    assert_eq!(text("solve(x^2+2*x+1,x)"), "[-1]");
}

#[test]
fn solve_accepts_equation_syntax() {
    assert_eq!(text("solve(x==1,x)"), "[1]");
    assert_eq!(text("solve(x^2==4,x)"), "[-2,2]");
    assert_eq!(text("solve(x^2-1==0,x)"), "[-1,1]");
}

#[test]
fn factor_handles_degenerate_constants() {
    assert_eq!(text("factor(0)"), "0");
    assert_eq!(text("factor(1)"), "1");
    assert_eq!(text("factor(5)"), "5");
    assert_eq!(text("expand(-(x+1))"), "-1-x");
    assert_eq!(text("together(x)"), "x");
    assert_eq!(text("cancel(1/x)"), "x^(-1)");
}

#[test]
fn factor_extracts_global_negative_sign() {
    assert_factor_matches("-x-y");
    assert_eq!(text("factor(-2*x-2*y)"), "-(2*(x+y))");
}

#[test]
fn polynomial_degrees_and_coefficients() {
    assert_eq!(text("degree(x^3+2*x+1,x)"), "3");
    assert_eq!(text("degree(7,x)"), "0");
    assert_eq!(text("coefficient(x^3+2*x+1,x,0)"), "1");
    assert_eq!(text("coefficient(x^3+2*x+1,x,1)"), "2");
    assert_eq!(text("coefficient(x^3+2*x+1,x,2)"), "0");
    assert_eq!(text("coefficient(x^3+2*x+1,x,3)"), "1");
    assert_eq!(text("coefficient(2*x^2+3*x+4,x,2)"), "2");
    assert!(text("degree(0,x)").contains("degree"));
}

#[test]
fn polynomial_division() {
    assert_eq!(text("polynomialDivide(x^3-1,x-1,x)"), "[1+x+x^2,0]");
    assert_eq!(text("polynomialDivide(x^3+1,x-1,x)"), "[1+x+x^2,2]");
    assert_eq!(text("polynomialDivide(x^2+3*x+2,x+1,x)"), "[2+x,0]");
    assert_eq!(text("polynomialDivide(x^2,x,x)"), "[x,0]");
    assert!(text("polynomialDivide(x,0,x)").contains("polynomialDivide"));
}

#[test]
fn discriminants() {
    assert_eq!(text("discriminant(x^2-4,x)"), "16");
    assert_eq!(text("discriminant(x^2+1,x)"), "-4");
    assert_eq!(text("discriminant(x^2-2*x+1,x)"), "0");
    assert_eq!(text("discriminant(x^3-3*x+2,x)"), "0");
    assert_eq!(text("discriminant(x^3-x+1,x)"), "-23");
    assert!(text("discriminant(x^4+1,x)").contains("discriminant"));
}

#[test]
fn sum_and_difference_of_cubes() {
    assert_factor_matches("x^3-y^3");
    assert_factor_matches("x^3+y^3");
    assert_factor_matches("8*x^3+27*y^3");
    assert_factor_matches("27*x^3-64*y^3");
    assert_eq!(text("factor(x^3+y^3)"), "(x+y)*(-x*y+x^2+y^2)");
}

#[test]
fn multivariate_grouping_factorization() {
    assert_eq!(text("factor(a*x+a*y+b*x+b*y)"), "(a+b)*(x+y)");
    assert_eq!(text("factor(x*y+x*z+y*w+z*w)"), "(w+x)*(y+z)");
    assert_eq!(text("factor(a*x+a*y+b*x+b*y+c*x+c*y)"), "(x+y)*(a+b+c)");
    assert_factor_matches("a*x+a*y+b*x+b*y");
    assert_factor_matches("x*y+x*z+y*w+z*w");
    assert_factor_matches("a*x+a*y+b*x+b*y+c*x+c*y");
}

#[test]
fn binary_quadratic_factorization() {
    assert_eq!(text("factor(x^2+3*x*y+2*y^2)"), "(x+y)*(x+2*y)");
    assert_eq!(text("factor(2*x^2+3*x*y+y^2)"), "(x+y)*(y+2*x)");
    assert_eq!(text("factor(4*x^2+4*x*y+y^2)"), "(y+2*x)^2");
    assert_eq!(text("factor(6*x^2+5*x*y+y^2)"), "(y+2*x)*(y+3*x)");
    for source in [
        "x^2+3*x*y+2*y^2",
        "2*x^2+3*x*y+y^2",
        "4*x^2+4*x*y+y^2",
        "6*x^2+5*x*y+y^2",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn recursive_bivariate_quadratic_factorization() {
    assert_eq!(text("factor(x^2+2*x*y+y^2+2*x+2*y+1)"), "(1+x+y)^2");
    assert_eq!(text("factor(x^2-y^2+2*x+1)"), "(1+x+y)*(1+x-y)");
    assert_eq!(text("factor(x^2+2*x*y+y^2+2*x+2*y)"), "(x+y)*(2+x+y)");
    assert_eq!(text("factor(x^2+2*x*y+y^2+3*x+3*y+2)"), "(1+x+y)*(2+x+y)");
    for source in [
        "x^2+2*x*y+y^2+2*x+2*y+1",
        "x^2-y^2+2*x+1",
        "x^2+2*x*y+y^2+2*x+2*y",
        "x^2+2*x*y+y^2+3*x+3*y+2",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn homogeneous_bivariate_factorization() {
    assert_eq!(text("factor(x^3+3*x^2*y+3*x*y^2+y^3)"), "(x+y)^3");
    assert_eq!(text("factor(x^3-y^3)"), "(x-y)*(x^2+y^2+x*y)");
    assert_eq!(text("factor(x^4+4*x^3*y+6*x^2*y^2+4*x*y^3+y^4)"), "(x+y)^4");
    assert_eq!(
        text("factor(x^4+3*x^3*y+3*x^2*y^2+3*x*y^3+2*y^4)"),
        "(x+y)*(x+2*y)*(x^2+y^2)"
    );
    for source in [
        "x^3+3*x^2*y+3*x*y^2+y^3",
        "x^3-y^3",
        "x^4+4*x^3*y+6*x^2*y^2+4*x*y^3+y^4",
        "x^4+3*x^3*y+3*x^2*y^2+3*x*y^3+2*y^4",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn square_free_univariate_factorization() {
    assert_eq!(text("factor((x^2+1)^3)"), "(1+x^2)^3");
    assert_eq!(text("factor((x-1)^3*(x^2+1)^2)"), "(-1+x)^3*(1+x^2)^2");
    assert_eq!(text("factor((2*x^2+2*x+1)^2)"), "(1+2*x+2*x^2)^2");
    for source in ["(x^2+1)^3", "(x-1)^3*(x^2+1)^2", "(2*x^2+2*x+1)^2"] {
        assert_factor_matches(source);
    }
}

#[test]
fn evaluation_interpolation_factorization() {
    assert_eq!(text("factor((x+y^2)*(x+y^3))"), "(x+y^2)*(x+y^3)");
    assert_eq!(text("factor((x+y^2+1)*(x+y^3+2))"), "(1+x+y^2)*(2+x+y^3)");
    assert_eq!(
        text("factor((x+y)*(x+y^2)*(x+y^3))"),
        "(x+y)*(x+y^2)*(x+y^3)"
    );
    assert_eq!(text("factor((x+y)^2*(x+y^2))"), "(x+y^2)*(x+y)^2");
    for source in [
        "(x+y^2)*(x+y^3)",
        "(x+y^2+1)*(x+y^3+2)",
        "(x+y)*(x+y^2)*(x+y^3)",
        "(x+y)^2*(x+y^2)",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn hensel_lifting_factorization() {
    assert_eq!(text("factor(x^4+3*x^2+2)"), "(1+x^2)*(2+x^2)");
    assert_eq!(
        text("factor(x^4+3*x^3+5*x^2+4*x+2)"),
        "(1+x+x^2)*(2+x^2+2*x)"
    );
    assert_eq!(text("factor(x^4+5*x^2+6)"), "(2+x^2)*(3+x^2)");
    assert_eq!(text("factor(x^4+4)"), "(2-2*x+x^2)*(2+x^2+2*x)");
    assert_eq!(text("factor(x^4+1)"), "1+x^4");
    for source in [
        "x^4+3*x^2+2",
        "x^4+3*x^3+5*x^2+4*x+2",
        "x^4+5*x^2+6",
        "x^4+4",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn multivariate_square_free_factorization() {
    assert_eq!(text("factor((x^2+x*y+1)^2)"), "(1+x^2+x*y)^2");
    assert_eq!(text("factor((x^2+y^2)^2)"), "(x^2+y^2)^2");
    for source in ["(x^2+x*y+1)^2", "(x^2+y^2)^2"] {
        assert_factor_matches(source);
    }
}

#[test]
fn higher_multivariate_powers() {
    assert_eq!(text("factor((x^2+x*y+1)^3)"), "(1+x^2+x*y)^3");
    assert_eq!(text("factor((x+y^2)^4)"), "(x+y^2)^4");
    assert_eq!(text("factor((x^2+y^2)^3)"), "(x^2+y^2)^3");
    for source in ["(x^2+x*y+1)^3", "(x+y^2)^4", "(x^2+y^2)^3"] {
        assert_factor_matches(source);
    }
}

#[test]
fn non_monic_bivariate_splitting() {
    assert_eq!(text("factor((2*x+y^2)*(x+y^3))"), "(x+y^3)*(y^2+2*x)");
    assert_eq!(
        text("factor((2*x+y^2+1)*(x+y^3+2))"),
        "(1+y^2+2*x)*(2+x+y^3)"
    );
    assert_eq!(
        text("factor((3*x+y^2)*(x+y)*(x+y^3))"),
        "(x+y)*(x+y^3)*(y^2+3*x)"
    );
    for source in [
        "(2*x+y^2)*(x+y^3)",
        "(2*x+y^2+1)*(x+y^3+2)",
        "(3*x+y^2)*(x+y)*(x+y^3)",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn polynomial_leading_bivariate_splitting() {
    assert_eq!(text("factor((y*x+1)*(x+y))"), "(1+x*y)*(x+y)");
    assert_eq!(text("factor((y*x+1)*(x+y+2))"), "(1+x*y)*(2+x+y)");
    for source in ["(y*x+1)*(x+y)", "(y*x+1)*(x+y+2)"] {
        assert_factor_matches(source);
    }
}

#[test]
fn deep_multivariate_powers() {
    assert_eq!(text("factor((x+y^2)^8)"), "(x+y^2)^8");
    assert_eq!(text("factor((x+y^2)^12)"), "(x+y^2)^12");
    for source in ["(x+y^2)^8", "(x+y^2)^12"] {
        assert_factor_matches(source);
    }
}

#[test]
fn general_polynomial_leading_splitting() {
    assert_eq!(
        text("factor(((y^2+y)*x+1)*(x+y+2))"),
        "(1+x*y+x*y^2)*(2+x+y)"
    );
    assert_eq!(text("factor(((y+1)*x+y^2)*(x+y))"), "(x+y)*(x+y^2+x*y)");
    assert_eq!(text("factor(((y^2+1)*x+y)*(x+y+1))"), "(1+x+y)*(x+y+x*y^2)");
    for source in [
        "((y^2+y)*x+1)*(x+y+2)",
        "((y+1)*x+y^2)*(x+y)",
        "((y^2+1)*x+y)*(x+y+1)",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn non_monic_multivariate_powers() {
    assert_eq!(text("factor((2*(x+y^2))^3)"), "8*(x+y^2)^3");
    assert_eq!(text("factor((3*(x^2+x*y+1))^2)"), "9*(1+x^2+x*y)^2");
    for source in ["(2*(x+y^2))^3", "(3*(x^2+x*y+1))^2"] {
        assert_factor_matches(source);
    }
}

#[test]
fn three_variable_interpolation_factorization() {
    assert_eq!(text("factor((x+y+z)*(x+y^2+z^2))"), "(x+y+z)*(x+y^2+z^2)");
    assert_eq!(text("factor((x+y*z)*(x+y^2+z^2))"), "(x+y*z)*(x+y^2+z^2)");
    assert_eq!(
        text("factor((x+y+z)*(x+y^2+z^2)*(x+y^3+z^3))"),
        "(x+y+z)*(x+y^2+z^2)*(x+y^3+z^3)"
    );
    assert_eq!(text("factor((x+y+z)*(x-y+z))"), "(x+y+z)*(x+z-y)");
    assert_eq!(text("factor((x+y+z)^2)"), "(x+y+z)^2");
    for source in [
        "(x+y+z)*(x+y^2+z^2)",
        "(x+y*z)*(x+y^2+z^2)",
        "(x+y+z)*(x+y^2+z^2)*(x+y^3+z^3)",
        "(x+y+z)*(x-y+z)",
        "(x+y+z)^2",
    ] {
        assert_factor_matches(source);
    }
}

#[test]
fn bivariate_coefficient_content_factorization() {
    assert_eq!(text("factor((y^2+1)*(x^2+y*x+1))"), "(1+y^2)*(1+x^2+x*y)");
    assert_eq!(text("factor((y+1)*(x+y)^2)"), "(1+y)*(x+y)^2");
    for source in ["(y^2+1)*(x^2+y*x+1)", "(y+1)*(x+y)^2"] {
        assert_factor_matches(source);
    }
}

#[test]
fn bivariate_specialization_root_lift() {
    assert_eq!(text("factor((x+y^2)*(x^2+y^3+1))"), "(x+y^2)*(1+x^2+y^3)");
    assert_eq!(text("factor((x+y^3)*(x^2+y^2+1))"), "(x+y^3)*(1+x^2+y^2)");
    for source in ["(x+y^2)*(x^2+y^3+1)", "(x+y^3)*(x^2+y^2+1)"] {
        assert_factor_matches(source);
    }
}

#[test]
fn bivariate_hensel_non_linear_factors() {
    assert_eq!(text("factor((x^2+y)*(x^2+y^2))"), "(y+x^2)*(x^2+y^2)");
    assert_eq!(
        text("factor((x^2+y*x+1)*(x^2+y^2+1))"),
        "(1+x^2+y^2)*(1+x^2+x*y)"
    );
    assert_eq!(text("factor((x^2+y)*(x^3+y*x+1))"), "(y+x^2)*(1+x^3+x*y)");
    for source in [
        "(x^2+y)*(x^2+y^2)",
        "(x^2+y*x+1)*(x^2+y^2+1)",
        "(x^2+y)*(x^3+y*x+1)",
    ] {
        assert_factor_matches(source);
    }
}
