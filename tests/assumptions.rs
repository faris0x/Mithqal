use mithqal::{ErrorCode, Session};

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
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
    assert_eq!(text("sqrt(x^2)"), "abs(x)");
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
    for source in ["assume(x+y,abs(x))", "assume(1,abs(x))"] {
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

#[test]
fn qualified_names_are_distinct_assumption_targets() {
    let mut session = Session::new();
    let mut eval = |source: &str| {
        let value = session.eval_statement(source).unwrap();
        session.format_infix(value).unwrap()
    };
    assert_eq!(eval("assume(foo;x>0,abs(foo;x))"), "foo;x");
    assert_eq!(eval("assume(bar;x<0,abs(bar;x))"), "-bar;x");
    assert_eq!(eval("assume(foo;x>0,abs(bar;x))"), "abs(bar;x)");
}

#[test]
fn relations_and_numeric_bounds_propagate() {
    assert_eq!(text("assume(x>y,assume(y>0,abs(x)))"), "x");
    assert_eq!(text("assume(x>y,assume(y>0,abs(y)))"), "y");
    assert_eq!(text("assume(x>1,abs(x))"), "x");
    assert_eq!(text("assume(x<-1,abs(x))"), "-x");
    assert_eq!(text("assume(x>=y,assume(y>0,abs(x)))"), "x");
}

#[test]
fn disjunction_and_negation_are_supported() {
    assert_eq!(text("assume(or(x>0,x>=0),abs(x))"), "x");
    assert_eq!(text("assume(or(x>0,x<0),abs(x))"), "abs(x)");
    assert_eq!(text("assume(not(x>0),abs(x))"), "-x");
    assert_eq!(text("assume(not(x>=0),abs(x))"), "-x");
    assert_eq!(text("assume(not(or(x>0,x<0)),abs(x))"), "0");
}

#[test]
fn parity_facts_drive_number_theory_queries() {
    assert_eq!(text("assume(even(x),isEven(x))"), "true");
    assert_eq!(text("assume(odd(x),isEven(x))"), "false");
    assert_eq!(text("assume(odd(x),isOdd(x))"), "true");
}

#[test]
fn domain_and_parity_facts_are_recorded() {
    assert_eq!(
        text("assume(and(integer(x),even(x)),assumptions(x))"),
        "[integer(x)&&even(x)]"
    );
    assert_eq!(
        text("assume(element(x,integers),assumptions(x))"),
        "[element(x,integers)]"
    );
}

#[test]
fn equality_propagates_signs() {
    assert_eq!(text("assume(x==y,assume(y>0,abs(x)))"), "x");
    assert_eq!(text("assume(x==y,assume(x<0,abs(y)))"), "-y");
}

#[test]
fn contradictory_assumptions_are_diagnostics() {
    let mut session = Session::new();
    for source in [
        "assume(and(x>0,x<0),abs(x))",
        "assume(and(even(x),odd(x)),abs(x))",
        "assume(and(x>y,y>x),abs(x))",
        "assume(and(x==y,x=/=y),abs(x))",
    ] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalAssumption, "{source}");
    }
}

#[test]
fn assumptions_are_scoped_and_introspectable() {
    let mut session = Session::new();
    let value = session
        .eval_statement("assume(x>0,assumptions(x))")
        .unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "[x>0]");
    let value = session.eval_statement("assumptions(x)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "[]");
}
