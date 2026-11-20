use mithqal::ExprId;
use mithqal::{ExprView, Session};

fn evaluate(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn complex_parts(session: &Session, id: ExprId) -> Option<(f64, f64)> {
    let (head, args) = match session.view(id).unwrap() {
        ExprView::Call { head, args } => (head, args),
        _ => return None,
    };
    match session.view(head).unwrap() {
        ExprView::Symbol("complex") => {}
        _ => return None,
    }
    let real = match session.view(args[0]).unwrap() {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64()? as f64,
        ExprView::Rational(value) => value.num().to_i64()? as f64 / value.den().to_i64()? as f64,
        _ => return None,
    };
    let imaginary = match session.view(args[1]).unwrap() {
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

fn real_part(session: &Session, id: ExprId) -> f64 {
    match session.view(id).unwrap() {
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
    assert_eq!(session.format_infix(value).unwrap(), "undefined");
    let (session, value) = evaluate("approx(x+i,30)");
    assert!(
        session.format_infix(value).unwrap().contains("approx"),
        "{}",
        session.format_infix(value).unwrap()
    );
    let (session, value) = evaluate("approx(zeta(i),30)");
    assert!(
        session.format_infix(value).unwrap().contains("approx"),
        "{}",
        session.format_infix(value).unwrap()
    );
}

#[test]
fn approximate_complex_exponential_and_logarithm() {
    check("approx(exp(i*pi),30)", -1.0, 0.0);
    check("approx(ln(i),30)", 0.0, std::f64::consts::FRAC_PI_2);
    check("approx(ln(-1),30)", 0.0, std::f64::consts::PI);
}

#[test]
fn approximate_complex_trigonometric_functions() {
    let sinh_one = 1f64.sinh();
    check("approx(sin(i),30)", 0.0, sinh_one);
    let cosh_one = 1f64.cosh();
    check("approx(cos(i),30)", cosh_one, 0.0);
    check("approx(tan(i),30)", 0.0, 1f64.tanh());
}

#[test]
fn approximate_complex_square_root() {
    check("approx(sqrt(3+4*i),30)", 2.0, 1.0);
    check("approx(sqrt(1+i),30)", 1.09868411346781, 0.455089860562227);
    check("approx(sqrt(-1),30)", 0.0, 1.0);
    check("approx(sqrt(-4),30)", 0.0, 2.0);
}

#[test]
fn approximate_complex_non_integer_powers() {
    let expected = (-std::f64::consts::FRAC_PI_2).exp();
    check("approx(i^i,30)", expected, 0.0);
    let ln2 = 2f64.ln();
    check("approx(2^i,30)", ln2.cos(), ln2.sin());
    let root = std::f64::consts::FRAC_1_SQRT_2;
    check("approx(i^(1/2),30)", root, root);
    check("approx((-1)^i,30)", (-std::f64::consts::PI).exp(), 0.0);
}

#[test]
fn approximate_complex_logarithm_of_negative_reals() {
    check("approx(ln(-1),30)", 0.0, std::f64::consts::PI);
    check("approx(ln(-e),30)", 1.0, std::f64::consts::PI);
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
    match session.view(value).unwrap() {
        ExprView::Real(value) => assert!((value.to_f64() - 3.0).abs() < 1e-9),
        _ => panic!("expected a real"),
    }
}

#[test]
fn approximate_complex_multiplication() {
    check("approx((2+3*i)*(4-i),30)", 11.0, 10.0);
    check("approx((1+i)^2,30)", 0.0, 2.0);
    let (session, value) = evaluate("approx((1+i)*(1-i),30)");
    match session.view(value).unwrap() {
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
    match session.view(value).unwrap() {
        ExprView::Real(value) => assert!((value.to_f64() - 5.0).abs() < 1e-9),
        _ => panic!("expected a real"),
    }
}

#[test]
fn complex_constructor_normalizes_zero_imaginary() {
    let (session, value) = evaluate("complex(1,0)");
    assert_eq!(session.format_infix(value).unwrap(), "1");
    let (session, value) = evaluate("complex(1,2)");
    match session.view(value).unwrap() {
        ExprView::Call { head, args } => {
            match session.view(head).unwrap() {
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

#[test]
fn approximate_complex_inverse_trigonometric_functions() {
    check("approx(asin(i),30)", 0.0, 0.881373587019543);
    check(
        "approx(acos(i),30)",
        std::f64::consts::FRAC_PI_2,
        -0.881373587019543,
    );
    check(
        "approx(atan(2*i),30)",
        std::f64::consts::FRAC_PI_2,
        0.5493061443340548,
    );
}

#[test]
fn approximate_real_inverse_trigonometry_promotes_to_complex() {
    check(
        "approx(asin(2),30)",
        std::f64::consts::FRAC_PI_2,
        -1.3169578969248166,
    );
    check("approx(acos(2),30)", 0.0, 1.3169578969248166);
    check(
        "approx(asin(-2),30)",
        -std::f64::consts::FRAC_PI_2,
        1.3169578969248166,
    );
    check(
        "approx(acos(-2),30)",
        std::f64::consts::PI,
        -1.3169578969248166,
    );
    check("approx(acosh(0),30)", 0.0, std::f64::consts::FRAC_PI_2);
    check(
        "approx(atanh(2),30)",
        0.5493061443340548,
        -std::f64::consts::FRAC_PI_2,
    );
    check(
        "approx(atanh(-2),30)",
        -0.5493061443340548,
        std::f64::consts::FRAC_PI_2,
    );
}

#[test]
fn approximate_complex_branch_conformance() {
    check("approx(ln(-1),30)", 0.0, std::f64::consts::PI);
    check(
        "approx(ln(-1-i),30)",
        0.5 * 2f64.ln(),
        -3.0 * std::f64::consts::FRAC_PI_4,
    );
    check("approx(sqrt(-1),30)", 0.0, 1.0);
    check("approx((-1)^(1/2),30)", 0.0, 1.0);
    check("approx((-1)^(1/3),30)", 0.5, 3f64.sqrt() / 2.0);
    check("approx((-8)^(1/3),30)", 1.0, 3f64.sqrt());
    check(
        "approx(acosh(-2),30)",
        (2.0 + 3f64.sqrt()).ln(),
        std::f64::consts::PI,
    );
    check(
        "approx(acosh(-2-i),30)",
        1.4693517443681852,
        -2.6342363503726486,
    );
    check(
        "approx(acosh(-2+i),30)",
        1.4693517443681852,
        2.6342363503726486,
    );
    check("approx(atanh(2)-((ln(3)-ln(-1))/2),30)", 0.0, 0.0);
    check(
        "approx(asin(1+i)+acos(1+i),30)",
        std::f64::consts::FRAC_PI_2,
        0.0,
    );
    check("approx(ln(exp(i*pi)),30)", 0.0, std::f64::consts::PI);
}

#[test]
fn approximate_complex_hyperbolic_functions() {
    check("approx(sinh(i),30)", 0.0, 1f64.sin());
    check("approx(cosh(i),30)", 1f64.cos(), 0.0);
    check("approx(tanh(i),30)", 0.0, 1f64.tan());
    check("approx(asinh(i),30)", 0.0, std::f64::consts::FRAC_PI_2);
    check("approx(acosh(0),30)", 0.0, std::f64::consts::FRAC_PI_2);
}

#[test]
fn approximate_negative_real_powers_use_principal_branch() {
    check("approx((-1)^(1/2),30)", 0.0, 1.0);
    check("approx((-8)^(1/3),30)", 1.0, 3f64.sqrt());
}

#[test]
fn approximate_complex_singularities_stay_unevaluated() {
    let (session, value) = evaluate("approx(ln(0),30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
    let (session, value) = evaluate("approx(atanh(1),30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
    let (session, value) = evaluate("approx(atan(i),30)");
    assert!(session.format_infix(value).unwrap().contains("approx"));
}

#[test]
fn approximate_complex_absolute_value_is_stable() {
    let (session, value) = evaluate("approx(abs(3*10^200+4*10^200*i),30)");
    match session.view(value).unwrap() {
        ExprView::Real(value) => {
            let expected = 5e200;
            assert!((value.to_f64() / expected - 1.0).abs() < 1e-9);
        }
        _ => panic!("expected a real"),
    }
    let (session, value) = evaluate("approx(abs(3*10^-200+4*10^-200*i),30)");
    match session.view(value).unwrap() {
        ExprView::Real(value) => {
            let expected = 5e-200;
            assert!((value.to_f64() / expected - 1.0).abs() < 1e-9);
        }
        _ => panic!("expected a real"),
    }
}

#[test]
fn complex_gamma_values() {
    check(
        "approx(gamma(1+i),30)",
        0.498015668118356,
        -0.1549498283018107,
    );
    check(
        "approx(gamma(1/2+1/2*i),30)",
        0.8181639995417473,
        -0.7633138287139826,
    );
    check(
        "approx(gamma(-1/2+1/2*i),30)",
        -1.58147782825573,
        -0.0548501708277648,
    );
    check(
        "approx(gamma(2+3*i),30)",
        -0.0823952726656119,
        0.0917742874352593,
    );
}

#[test]
fn complex_error_values() {
    check(
        "approx(erf(1+i),30)",
        1.3161512816979476,
        0.1904534692378347,
    );
    check(
        "approx(erfc(1+i),30)",
        -0.3161512816979476,
        -0.1904534692378347,
    );
    check(
        "approx(erfi(1+i),30)",
        0.1904534692378347,
        1.3161512816979476,
    );
}

#[test]
fn complex_log_gamma_values() {
    check(
        "approx(lgamma(1+i),30)",
        -0.6509231993018563,
        -0.3016403204675332,
    );
}
