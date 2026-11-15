use mithqal::{ErrorCode, Session};

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value)
}

#[test]
fn positive_and_negative_abs() {
    assert_eq!(text("assume(x>0,abs(x))"), "x");
    assert_eq!(text("assume(x<0,abs(x))"), "-x");
    assert_eq!(text("assume(x>=0,abs(x))"), "x");
    assert_eq!(text("assume(x<=0,abs(x))"), "-x");
    assert_eq!(text("assume(x==0,abs(x))"), "0");
}

#[test]
fn positive_and_negative_square_roots() {
    assert_eq!(text("assume(x>0,sqrt(x^2))"), "x");
    assert_eq!(text("assume(x<0,sqrt(x^2))"), "-x");
    assert_eq!(text("assume(x>=0,sqrt(x^2))"), "x");
    assert_eq!(text("assume(x<=0,sqrt(x^2))"), "-x");
    assert_eq!(text("sqrt(x^2)"), "sqrt(x^2)");
}

#[test]
fn nested_assumptions() {
    assert_eq!(text("assume(x>0,assume(y<0,abs(x)+abs(y)))"), "x-y");
    assert_eq!(text("assume(x>0,abs(-x))"), "x");
    assert_eq!(text("assume(x<0,abs(-x))"), "-x");
}

#[test]
fn assumptions_apply_to_products_and_powers() {
    assert_eq!(text("assume(x>0,abs(x^3))"), "x^3");
    assert_eq!(text("assume(x<0,abs(x^2))"), "x^2");
    assert_eq!(text("assume(x>0,assume(y>0,abs(x*y)))"), "x*y");
    assert_eq!(text("assume(x>0,assume(y<0,abs(x*y)))"), "-(x*y)");
}

#[test]
fn assumptions_are_scoped() {
    assert_eq!(text("assume(x>0,abs(x))+abs(x)"), "x+abs(x)");
    assert_eq!(text("assuming(x<0,sqrt(x^2))"), "-x");
}

#[test]
fn malformed_assumptions_stay_unevaluated() {
    for source in [
        "assume(x+y,abs(x))",
        "assume(x>1,abs(x))",
        "assume(1,abs(x))",
    ] {
        let result = text(source);
        assert!(result.contains("assume"), "{source} => {result}");
    }
}

#[test]
fn assume_arity_and_protection() {
    for source in [
        "assume(x>0)",
        "assume(x>0,abs(x),1)",
        "assume(abs(x))",
        "assuming(x>0)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
    let mut session = Session::new();
    let error = session.eval_statement("assume=1").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn conjunctive_assumptions() {
    assert_eq!(text("assume(and(x>0,y>0),abs(x*y))"), "x*y");
    assert_eq!(text("assume(and(x>0,y<0),abs(x*y))"), "-(x*y)");
    assert_eq!(text("assume(x>0&&y>0,abs(x*y))"), "x*y");
    assert_eq!(text("assume(and(x>0,y>0,z>0),abs(x*y*z))"), "x*y*z");
    assert_eq!(text("assuming(and(x<0,y<0),abs(x*y))"), "x*y");
}
