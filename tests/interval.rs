use mithqal::{ExprView, Session};

fn evaluate(source: &str) -> (Session, u32) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value)
}

fn interval_bounds(session: &Session, id: u32) -> (f64, f64) {
    let (head, args) = match session.view(id) {
        ExprView::Call { head, args } => (head, args),
        _ => panic!("expected a call"),
    };
    match session.view(head) {
        ExprView::Symbol(name) => assert_eq!(name, "interval"),
        _ => panic!("expected an interval head"),
    }
    assert_eq!(args.len(), 2);
    let low = match session.view(args[0]) {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        _ => panic!("expected a numeric endpoint"),
    };
    let high = match session.view(args[1]) {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        _ => panic!("expected a numeric endpoint"),
    };
    (low, high)
}

fn check(source: &str, low: f64, high: f64) {
    let (session, value) = evaluate(source);
    let (actual_low, actual_high) = interval_bounds(&session, value);
    assert!(
        (actual_low - low).abs() < 1e-9,
        "{source}: low {actual_low}, expected {low}"
    );
    assert!(
        (actual_high - high).abs() < 1e-9,
        "{source}: high {actual_high}, expected {high}"
    );
}

#[test]
fn interval_addition() {
    check("interval(1,2)+3", 4.0, 5.0);
    check("3+interval(1,2)", 4.0, 5.0);
    check("interval(1,2)+interval(3,4)", 4.0, 6.0);
}

#[test]
fn interval_multiplication() {
    check("interval(1,2)*2", 2.0, 4.0);
    check("2*interval(-1,2)", -2.0, 4.0);
    check("interval(1,2)*interval(3,4)", 3.0, 8.0);
    check("interval(-2,1)*interval(-3,4)", -8.0, 6.0);
}

#[test]
fn interval_negation() {
    check("-interval(1,2)", -2.0, -1.0);
    check("-interval(-3,-1)", 1.0, 3.0);
}

#[test]
fn interval_powers() {
    check("interval(1,2)^2", 1.0, 4.0);
    check("interval(-2,3)^2", 0.0, 9.0);
    check("interval(-2,3)^3", -8.0, 27.0);
    check("interval(1,2)^-1", 0.5, 1.0);
}

#[test]
fn interval_spanning_zero_has_no_reciprocal() {
    let (session, value) = evaluate("interval(-1,1)^-1");
    match session.view(value) {
        ExprView::Call { head, .. } => match session.view(head) {
            ExprView::Symbol(name) => assert_eq!(name, "power"),
            _ => panic!("expected a power call"),
        },
        _ => panic!("expected a call"),
    }
}

#[test]
fn invalid_intervals_fail() {
    let (_session, value) = evaluate("interval(2,1)");
    assert_eq!(value, _session.last_result().unwrap());
    assert_eq!(_session.format_infix(value), "failed");
}

#[test]
fn symbolic_intervals_stay_symbolic() {
    let (session, value) = evaluate("interval(a,b)+1");
    let text = session.format_infix(value);
    assert!(text.contains("interval(a,b)"), "{text}");
}

#[test]
fn interval_subtraction() {
    check("interval(1,2)-3", -2.0, -1.0);
    check("interval(1,2)-interval(3,4)", -3.0, -1.0);
    check("interval(-2,-1)-interval(1,2)", -4.0, -2.0);
}

#[test]
fn interval_division_by_scalar() {
    check("interval(2,4)/2", 1.0, 2.0);
    check("interval(1,2)*(1/2)", 0.5, 1.0);
    check("2/interval(1,2)", 1.0, 2.0);
}

#[test]
fn interval_multiple_terms() {
    check("interval(1,2)+interval(3,4)+interval(5,6)", 9.0, 12.0);
    check("interval(1,2)*interval(3,4)*interval(5,6)", 15.0, 48.0);
    check("interval(1,2)+3+4", 8.0, 9.0);
    check("interval(1,2)*2*3", 6.0, 12.0);
}

#[test]
fn interval_zero_widths() {
    check("interval(0,0)", 0.0, 0.0);
    check("interval(3,3)", 3.0, 3.0);
    check("interval(1,2)*0", 0.0, 0.0);
    check("interval(-5,7)*0", 0.0, 0.0);
}

#[test]
fn interval_powers_extended() {
    check("interval(1,2)^0", 1.0, 1.0);
    check("interval(1,2)^1", 1.0, 2.0);
    check("interval(1,2)^4", 1.0, 16.0);
    check("interval(-2,-1)^2", 1.0, 4.0);
    check("interval(-2,-1)^3", -8.0, -1.0);
    check("interval(-2,-1)^4", 1.0, 16.0);
    check("interval(0,2)^2", 0.0, 4.0);
    check("interval(-1,0)^2", 0.0, 1.0);
    check("interval(1,2)^-2", 0.25, 1.0);
    check("interval(2,4)^-1", 0.25, 0.5);
}

#[test]
fn interval_with_fractional_endpoints() {
    check("interval(1/3,2/3)+1", 4.0 / 3.0, 5.0 / 3.0);
    check("interval(1/2,1/2)*2", 1.0, 1.0);
}

#[test]
fn interval_with_real_endpoints() {
    check(
        "interval(approx(pi,30),approx(pi,30)+1)",
        3.141592653589793,
        4.141592653589793,
    );
}

#[test]
fn approximate_point_intervals_widen_for_error() {
    let (session, value) = evaluate("interval(approx(1/3,30),approx(1/3,30))+0");
    let args = match session.view(value) {
        ExprView::Call { args, .. } => args,
        _ => panic!("expected an interval"),
    };
    let low = match session.view(args[0]) {
        ExprView::Real(value) => value,
        _ => panic!("expected a real endpoint"),
    };
    let high = match session.view(args[1]) {
        ExprView::Real(value) => value,
        _ => panic!("expected a real endpoint"),
    };
    assert!(high.cmp(low) == core::cmp::Ordering::Greater);
}

#[test]
fn interval_bounds_are_outward() {
    let (session, value) = evaluate("interval(1/3,2/3)+interval(1/7,2/7)");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 10.0 / 21.0 + 1e-12, "low {low}");
    assert!(high >= 20.0 / 21.0 - 1e-12, "high {high}");

    let (session, value) = evaluate("interval(1/3,2/3)*interval(1/7,2/7)");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1.0 / 21.0 + 1e-12, "low {low}");
    assert!(high >= 4.0 / 21.0 - 1e-12, "high {high}");

    let (session, value) = evaluate("interval(1/3,2/3)^2");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1.0 / 9.0 + 1e-12, "low {low}");
    assert!(high >= 4.0 / 9.0 - 1e-12, "high {high}");
}

#[test]
fn interval_arity_errors() {
    let mut session = Session::new();
    for source in ["interval()", "interval(1)", "interval(1,2,3)"] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, mithqal::ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn interval_compiled_path() {
    let mut session = Session::new();
    let value = session.eval_compiled_statement("interval(1,2)+3").unwrap();
    let (low, high) = interval_bounds(&session, value);
    assert!((low - 4.0).abs() < 1e-9);
    assert!((high - 5.0).abs() < 1e-9);
}

#[test]
fn intervals_survive_collection() {
    let mut session = Session::new();
    let value = session.eval_statement("interval(1,2)+3").unwrap();
    session.collect_garbage(&[value]);
    let (low, high) = interval_bounds(&session, value);
    assert!((low - 4.0).abs() < 1e-9);
    assert!((high - 5.0).abs() < 1e-9);
}

#[test]
fn interval_elementary_functions() {
    let (session, value) = evaluate("sqrt(interval(1,4))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1.0 + 1e-12 && high >= 2.0 - 1e-12);
    let (session, value) = evaluate("exp(interval(0,1))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1.0 + 1e-12 && high >= std::f64::consts::E - 1e-12);
    let (session, value) = evaluate("ln(interval(1,2))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1e-12 && high >= 2f64.ln() - 1e-12);
    let (session, value) = evaluate("abs(interval(-2,3))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 0.0 && high >= 3.0 - 1e-12);
    let (session, value) = evaluate("sin(interval(0,1))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1e-12 && high >= 1f64.sin() - 1e-12 && high < 0.9);
    let (session, value) = evaluate("cos(interval(0,1))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1f64.cos() + 1e-12 && low > 0.5 && high >= 1.0 - 1e-12);
    assert!(text("ln(interval(-1,2))").contains("ln"));
    assert!(text("sqrt(interval(-4,-1))").contains("sqrt"));
}

#[test]
fn interval_trigonometric_bounds() {
    let (session, value) = evaluate("sin(interval(1,2))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 1f64.sin() + 1e-12 && high >= 1.0 - 1e-12);
    let (session, value) = evaluate("cos(interval(1,2))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= 2f64.cos() + 1e-12 && high >= 1f64.cos() - 1e-12);
    let (session, value) = evaluate("sin(interval(-2,2))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= -1.0 + 1e-12 && high >= 1.0 - 1e-12);
    let (session, value) = evaluate("cos(interval(3,4))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= -1.0 + 1e-12 && high >= 4f64.cos() - 1e-12);
    let (session, value) = evaluate("sin(interval(-1,1))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= -1f64.sin() + 1e-12 && high >= 1f64.sin() - 1e-12);
    let (session, value) = evaluate("sin(interval(0,7))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= -1.0 + 1e-12 && high >= 1.0 - 1e-12);
    let (session, value) = evaluate("cos(interval(0,7))");
    let (low, high) = interval_bounds(&session, value);
    assert!(low <= -1.0 + 1e-12 && high >= 1.0 - 1e-12);
}
