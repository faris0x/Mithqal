use mithqal::{ExprView, Session};

fn evaluate(source: &str) -> (Session, u32) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn complex_parts(session: &Session, id: u32) -> Option<(f64, f64)> {
    let (head, args) = match session.view(id) {
        ExprView::Call { head, args } => (head, args),
        _ => return None,
    };
    match session.view(head) {
        ExprView::Symbol(name) if name == "complex" => {}
        _ => return None,
    }
    let real = match session.view(args[0]) {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64()? as f64,
        ExprView::Rational(value) => value.num().to_i64()? as f64 / value.den().to_i64()? as f64,
        _ => return None,
    };
    let imaginary = match session.view(args[1]) {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64()? as f64,
        ExprView::Rational(value) => value.num().to_i64()? as f64 / value.den().to_i64()? as f64,
        _ => return None,
    };
    Some((real, imaginary))
}

fn check(source: &str, real: f64, imaginary: f64) {
    let (session, value) = evaluate(source);
    if let Some((actual_real, actual_imaginary)) = complex_parts(&session, value) {
        assert!(
            (actual_real - real).abs() < 1e-9,
            "{source}: real {actual_real}"
        );
        assert!(
            (actual_imaginary - imaginary).abs() < 1e-9,
            "{source}: imaginary {actual_imaginary}"
        );
    } else {
        assert!(
            imaginary == 0.0,
            "{source}: expected a complex value with imaginary part {imaginary}"
        );
        let actual_real = real_part(&session, value);
        assert!(
            (actual_real - real).abs() < 1e-9,
            "{source}: real {actual_real}"
        );
    }
}

fn real_part(session: &Session, id: u32) -> f64 {
    match session.view(id) {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        _ => panic!("expected a real value"),
    }
}

#[test]
fn approximate_imaginary_unit() {
    check("approx(i,30)", 0.0, 1.0);
    check("approx(-i,30)", 0.0, -1.0);
    check("approx(2*i,30)", 0.0, 2.0);
    check("approx(-2*i,30)", 0.0, -2.0);
}

#[test]
fn approximate_imaginary_powers() {
    for (source, real, imaginary) in [
        ("approx(i^0,30)", 1.0, 0.0),
        ("approx(i^1,30)", 0.0, 1.0),
        ("approx(i^2,30)", -1.0, 0.0),
        ("approx(i^3,30)", 0.0, -1.0),
        ("approx(i^4,30)", 1.0, 0.0),
        ("approx(i^5,30)", 0.0, 1.0),
        ("approx(i^6,30)", -1.0, 0.0),
        ("approx(i^7,30)", 0.0, -1.0),
        ("approx(i^8,30)", 1.0, 0.0),
    ] {
        if imaginary == 0.0 {
            let (session, value) = evaluate(source);
            assert!((real_part(&session, value) - real).abs() < 1e-9, "{source}");
        } else {
            check(source, real, imaginary);
        }
    }
}

#[test]
fn approximate_complex_literals() {
    check("approx(1+2*i,30)", 1.0, 2.0);
    check("approx(1-2*i,30)", 1.0, -2.0);
    check("approx(-1+2*i,30)", -1.0, 2.0);
    check("approx(1.5+2.5*i,30)", 1.5, 2.5);
    check("approx(0.25-0.75*i,30)", 0.25, -0.75);
}

#[test]
fn approximate_complex_subtraction() {
    check("approx((1+i)-(2-i),30)", -1.0, 2.0);
    check("approx((5+7*i)-(2+3*i),30)", 3.0, 4.0);
}

#[test]
fn approximate_complex_powers() {
    check("approx((1+i)^0,30)", 1.0, 0.0);
    check("approx((1+i)^1,30)", 1.0, 1.0);
    check("approx((1+i)^2,30)", 0.0, 2.0);
    check("approx((1+i)^3,30)", -2.0, 2.0);
    check("approx((1+i)^4,30)", -4.0, 0.0);
    check("approx((1+i)^-1,30)", 0.5, -0.5);
    check("approx((1+i)^-2,30)", 0.0, -0.5);
}

#[test]
fn approximate_complex_reciprocals() {
    check("approx(1/i,30)", 0.0, -1.0);
    check("approx(1/(1+i),30)", 0.5, -0.5);
    check("approx(1/(2+3*i),30)", 2.0 / 13.0, -3.0 / 13.0);
}

#[test]
fn approximate_complex_constructor_and_calls() {
    check("approx(complex(1,2),30)", 1.0, 2.0);
    check("approx(complex(1,2)+complex(3,4),30)", 4.0, 6.0);
    check("approx(complex(1,2)*complex(3,4),30)", -5.0, 10.0);
    check("approx(complex(1,2)^2,30)", -3.0, 4.0);
    check(
        "approx(complex(1,2)/complex(3,4),30)",
        11.0 / 25.0,
        2.0 / 25.0,
    );
}

#[test]
fn approximate_complex_error_and_symbolic_paths() {
    let (session, value) = evaluate("approx(1/0+2*i,30)");
    assert_eq!(session.format_infix(value), "undefined");
    let (session, value) = evaluate("approx(x+i,30)");
    assert!(
        session.format_infix(value).contains("approx"),
        "{}",
        session.format_infix(value)
    );
    let (session, value) = evaluate("approx(sin(i),30)");
    assert!(
        session.format_infix(value).contains("approx"),
        "{}",
        session.format_infix(value)
    );
}

#[test]
fn approximate_complex_digit_counts() {
    check("approx(1+i,5)", 1.0, 1.0);
    check("approx(1+i,50)", 1.0, 1.0);
    check("approx((1/3)+(2/3)*i,30)", 1.0 / 3.0, 2.0 / 3.0);
}

#[test]
fn approximate_complex_compiled_path() {
    let mut session = Session::new();
    let value = session.eval_compiled_statement("approx(1+2*i,30)").unwrap();
    let (real, imaginary) = complex_parts(&session, value).unwrap();
    assert!((real - 1.0).abs() < 1e-9);
    assert!((imaginary - 2.0).abs() < 1e-9);
}

#[test]
fn approximate_complex_addition() {
    check("approx(1+i,30)", 1.0, 1.0);
    let (session, value) = evaluate("approx((1+i)+(2-i),30)");
    match session.view(value) {
        ExprView::Real(value) => assert!((value.to_f64() - 3.0).abs() < 1e-9),
        _ => panic!("expected a real"),
    }
}

#[test]
fn approximate_complex_multiplication() {
    check("approx((2+3*i)*(4-i),30)", 11.0, 10.0);
    check("approx((1+i)^2,30)", 0.0, 2.0);
    let (session, value) = evaluate("approx((1+i)*(1-i),30)");
    match session.view(value) {
        ExprView::Real(value) => assert!((value.to_f64() - 2.0).abs() < 1e-9),
        _ => panic!("expected a real"),
    }
}

#[test]
fn approximate_complex_division() {
    check("approx((1+i)/(1-i),30)", 0.0, 1.0);
    check("approx(1/i,30)", 0.0, -1.0);
}

#[test]
fn approximate_complex_absolute_value() {
    let (session, value) = evaluate("approx(abs(3+4*i),30)");
    match session.view(value) {
        ExprView::Real(value) => assert!((value.to_f64() - 5.0).abs() < 1e-9),
        _ => panic!("expected a real"),
    }
}

#[test]
fn complex_constructor_normalizes_zero_imaginary() {
    let (session, value) = evaluate("complex(1,0)");
    assert_eq!(session.format_infix(value), "1");
    let (session, value) = evaluate("complex(1,2)");
    match session.view(value) {
        ExprView::Call { head, args } => {
            match session.view(head) {
                ExprView::Symbol(name) => assert_eq!(name, "complex"),
                _ => panic!("expected a complex head"),
            }
            assert_eq!(args.len(), 2);
        }
        _ => panic!("expected a call"),
    }
}

#[test]
fn approximate_complex_values_survive_collection() {
    let mut session = Session::new();
    let value = session.eval_statement("approx(1+2*i,30)").unwrap();
    session.collect_garbage(&[value]);
    let (real, imaginary) = complex_parts(&session, value).unwrap();
    assert!((real - 1.0).abs() < 1e-9);
    assert!((imaginary - 2.0).abs() < 1e-9);
}
