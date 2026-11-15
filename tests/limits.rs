use mithqal::ExprId;
use mithqal::{ErrorCode, Session};

fn evaluate(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value).unwrap()
}

#[test]
fn finite_limits() {
    assert_eq!(text("limit(x^2+1,x,2)"), "5");
    assert_eq!(text("limit((x^2-4)/(x-2),x,2)"), "4");
    assert_eq!(text("limit((x-1)/(x-1),x,1)"), "1");
    assert_eq!(text("limit(pi,x,inf)"), "pi");
    assert_eq!(text("limit(abs(x),x,0)"), "0");
}

#[test]
fn limits_at_infinity() {
    assert_eq!(text("limit(x^2,x,inf)"), "inf");
    assert_eq!(text("limit(-x^2,x,inf)"), "-inf");
    assert_eq!(text("limit(x^3,x,inf)"), "inf");
    assert_eq!(text("limit(x^3,x,-inf)"), "-inf");
    assert_eq!(text("limit(1/x,x,inf)"), "0");
    assert_eq!(text("limit(1/x^2,x,inf)"), "0");
    assert_eq!(text("limit((2*x^2+1)/(x^2+3),x,inf)"), "2");
    assert_eq!(text("limit((x+1)/(x^2+1),x,inf)"), "0");
    assert_eq!(text("limit((x^3+1)/(x+1),x,inf)"), "inf");
    assert_eq!(text("limit((x^2+1)/(3*x^2),x,-inf)"), "1/3");
    assert_eq!(text("limit(x^3/(x+1),x,-inf)"), "inf");
    assert_eq!(text("limit(-x^3/(x+1),x,-inf)"), "-inf");
}

#[test]
fn exponential_and_logarithmic_limits() {
    assert_eq!(text("limit(exp(-x),x,inf)"), "0");
    assert_eq!(text("limit(exp(x),x,inf)"), "inf");
    assert_eq!(text("limit(exp(x),x,-inf)"), "0");
    assert_eq!(text("limit(ln(x),x,inf)"), "inf");
    assert_eq!(text("limit(atan(x),x,inf)"), "(1/2)*pi");
    assert_eq!(text("limit(atan(x),x,-inf)"), "-((1/2)*pi)");
    assert_eq!(text("limit(x*exp(-x),x,inf)"), "0");
    assert_eq!(text("limit(x/exp(x),x,inf)"), "0");
    assert_eq!(text("limit(x^2/exp(x),x,inf)"), "0");
    assert_eq!(text("limit(ln(x)/x,x,inf)"), "0");
    assert_eq!(text("limit(exp(-x)+1,x,inf)"), "1");
    assert_eq!(text("limit(2^x,x,inf)"), "inf");
    assert_eq!(text("limit(2^x,x,-inf)"), "0");
    assert_eq!(text("limit((1/2)^x,x,inf)"), "0");
}

#[test]
fn removable_singularities() {
    assert_eq!(text("limit((x^2-1)/(x-1),x,1)"), "2");
    assert_eq!(text("limit((x^2-4)/(x-2),x,2)"), "4");
    assert_eq!(text("limit((x^2-4)/(x-2),x,2,left)"), "4");
}

#[test]
fn standard_indeterminate_limits() {
    assert_eq!(text("limit(sin(x)/x,x,0)"), "1");
    assert_eq!(text("limit((1-cos(x))/x^2,x,0)"), "1/2");
    assert_eq!(text("limit((exp(x)-1)/x,x,0)"), "1");
    assert_eq!(text("limit(ln(x)/(x-1),x,1)"), "1");
    assert_eq!(text("limit(tan(x)/x,x,0)"), "1");
    assert_eq!(text("limit((x-sin(x))/x^3,x,0)"), "1/6");
    assert_eq!(text("limit((exp(x)-1-x)/x^2,x,0)"), "1/2");
}

#[test]
fn one_sided_limits() {
    assert_eq!(text("limit(1/x,x,0,right)"), "inf");
    assert_eq!(text("limit(1/x,x,0,left)"), "-inf");
    assert_eq!(text("limit(1/x^2,x,0)"), "inf");
    assert_eq!(text("limit(1/(x-1),x,1,right)"), "inf");
    assert_eq!(text("limit(1/(x-1),x,1,left)"), "-inf");
    assert_eq!(text("limit(abs(x)/x,x,0,right)"), "1");
    assert_eq!(text("limit(abs(x)/x,x,0,left)"), "-1");
    assert_eq!(text("limit(ln(x),x,0,right)"), "-inf");
    assert_eq!(text("limit(sqrt(x),x,0,right)"), "0");
    assert_eq!(text("limit(exp(1/x),x,0,right)"), "inf");
    assert_eq!(text("limit(exp(1/x),x,0,left)"), "0");
}

#[test]
fn two_sided_mismatch_stays_unevaluated() {
    for source in [
        "limit(1/x,x,0)",
        "limit(1/(x-1),x,1)",
        "limit(abs(x)/x,x,0)",
        "limit(ln(x),x,0)",
        "limit(sin(x),x,inf)",
        "limit(cos(x),x,inf)",
        "limit(tan(x),x,inf)",
        "limit(f(x),x,0)",
    ] {
        let result = text(source);
        assert!(result.contains("limit"), "{source} => {result}");
    }
}

#[test]
fn taylor_series() {
    assert_eq!(text("series(exp(x),x,0,3)"), "1+x+(1/6)*x^3+(1/2)*x^2");
    assert_eq!(text("series(sin(x),x,0,5)"), "x-(1/6)*x^3+(1/120)*x^5");
    assert_eq!(text("series(cos(x),x,0,4)"), "1-(1/2)*x^2+(1/24)*x^4");
    assert_eq!(text("series(1/(1-x),x,0,3)"), "1+x+x^2+x^3");
    assert_eq!(text("series(ln(1+x),x,0,3)"), "x-(1/2)*x^2+(1/3)*x^3");
    assert_eq!(text("series(exp(2*x),x,0,3)"), "1+2*x+2*x^2+(4/3)*x^3");
    assert_eq!(text("series(x^2*exp(x),x,0,3)"), "x^2+x^3");
    assert_eq!(text("series(sin(x),x,0,0)"), "0");
}

#[test]
fn taylor_series_around_nonzero_points() {
    assert_eq!(text("series(exp(x),x,1,1)"), "exp(1)+exp(1)*(-1+x)");
    assert_eq!(text("series(x^2,x,1,2)"), "1+(-1+x)^2+2*(-1+x)");
    assert_eq!(text("series(x^3+2*x+1,x,0,3)"), "1+x^3+2*x");
}

#[test]
fn series_leaves_unsupported_forms_unevaluated() {
    for source in [
        "series(f(x),x,0,3)",
        "series(ln(x),x,0,3)",
        "series(x,x,0,-1)",
        "series(x,x,0,21)",
    ] {
        let result = text(source);
        assert!(result.contains("series"), "{source} => {result}");
    }
}

#[test]
fn convergence_checks() {
    assert_eq!(text("converges(sin(x)/x,x,0)"), "true");
    assert_eq!(text("converges(1/x,x,0)"), "false");
    assert_eq!(text("converges(abs(x)/x,x,0)"), "false");
    assert_eq!(text("converges(exp(-x),x,inf)"), "true");
    assert_eq!(text("converges(exp(x),x,inf)"), "false");
    let result = text("converges(sin(1/x),x,0)");
    assert!(result.contains("converges"), "{result}");
}

#[test]
fn limit_arity_errors() {
    for source in [
        "limit(x,x)",
        "limit(x,x,0,right,1)",
        "series(x,x,0)",
        "series(x,x,0,1,2)",
        "converges(x,x)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn limit_symbols_are_protected() {
    let mut session = Session::new();
    for name in ["limit", "series", "converges"] {
        let source = format!("{name}=1");
        let error = session.eval_statement(&source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "source {source}");
    }
}
