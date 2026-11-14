use mithqal::{ExprView, Session};

fn value_text(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value)
}

#[test]
fn factor_expand_round_trips() {
    let polynomials = [
        "x^2-1",
        "x^3-x",
        "x^3+y^3",
        "x^3-y^3",
        "6*x^2+5*x+1",
        "4*x^2+4*x+1",
        "x^4-1",
        "x^4-5*x^2+4",
        "x^2+2*x*y+y^2",
        "x^2-y^2",
        "2*x^2+4*x",
        "27*x^3-64*y^3",
    ];
    for polynomial in polynomials {
        let mut session = Session::new();
        let factored = session
            .eval_statement(&format!("factor({polynomial})"))
            .unwrap();
        let expanded = session
            .eval_statement(&format!("expand({})", session.format_infix(factored)))
            .unwrap();
        let target = session
            .eval_statement(&format!("expand({polynomial})"))
            .unwrap();
        assert_eq!(expanded, target, "{polynomial}");
    }
}

#[test]
fn integral_derivative_round_trips() {
    for polynomial in ["x", "x^2", "x^3+2*x+1", "4*x^3-3*x^2+2*x-1"] {
        let mut session = Session::new();
        let integral = session
            .eval_statement(&format!("integrate({polynomial},x)"))
            .unwrap();
        let derivative = session
            .eval_statement(&format!(
                "expand(differentiate({},x))",
                session.format_infix(integral)
            ))
            .unwrap();
        let target = session
            .eval_statement(&format!("expand({polynomial})"))
            .unwrap();
        assert_eq!(derivative, target, "{polynomial}");
    }
}

#[test]
fn determinant_is_multiplicative_for_small_integers() {
    let matrices = [
        (1, 2, 3, 4, 2, 1, 1, 3),
        (2, 0, 0, 3, 1, 1, 1, 1),
        (1, 0, 0, 1, 5, 6, 7, 8),
        (0, 1, 1, 0, 1, 2, 3, 4),
        (-1, 2, 3, -4, 2, -1, 1, 3),
    ];
    for (a, b, c, d, e, f, g, h) in matrices {
        let mut session = Session::new();
        let product = value_text(
            &mut session,
            &format!("det(matmul([[{a},{b}],[{c},{d}]],[[{e},{f}],[{g},{h}]]))"),
        );
        let left = value_text(&mut session, &format!("det([[{a},{b}],[{c},{d}]])"));
        let right = value_text(&mut session, &format!("det([[{e},{f}],[{g},{h}]])"));
        let expected = value_text(&mut session, &format!("{left}*{right}"));
        assert_eq!(product, expected, "{a},{b},{c},{d},{e},{f},{g},{h}");
    }
}

#[test]
fn inverse_times_matrix_is_identity() {
    for (a, b, c, d) in [
        (1, 2, 3, 4),
        (2, 0, 0, 4),
        (1, 1, 1, 2),
        (3, 1, 2, 1),
        (-1, 2, 3, -4),
    ] {
        let mut session = Session::new();
        let value = value_text(
            &mut session,
            &format!("matmul([[{a},{b}],[{c},{d}]],inverse([[{a},{b}],[{c},{d}]]))"),
        );
        assert_eq!(value, "[[1,0],[0,1]]", "{a},{b},{c},{d}");
    }
}

#[test]
fn nullspace_annihilates_its_matrix() {
    let cases = [(1i64, 2, 2, 4), (2, 4, 3, 6), (1, 1, 1, 1), (0, 1, 0, 2)];
    for (a, b, c, d) in cases {
        let mut session = Session::new();
        let value = session
            .eval_statement(&format!("nullspace([[{a},{b}],[{c},{d}]])"))
            .unwrap();
        let vectors = match session.view(value) {
            ExprView::Call { args, .. } => args.to_vec(),
            _ => panic!("expected a list"),
        };
        for vector in vectors {
            let (x, y) = match session.view(vector) {
                ExprView::Call { args, .. } => {
                    (session.format_infix(args[0]), session.format_infix(args[1]))
                }
                _ => panic!("expected a vector"),
            };
            let first = value_text(&mut session, &format!("{a}*({x})+{b}*({y})"));
            let second = value_text(&mut session, &format!("{c}*({x})+{d}*({y})"));
            assert_eq!(first, "0", "{a},{b},{c},{d}");
            assert_eq!(second, "0", "{a},{b},{c},{d}");
        }
    }
}

#[test]
fn constant_lists_have_zero_variance() {
    for value in -5i64..=5 {
        let mut session = Session::new();
        let source = format!("variance([{value},{value},{value},{value}])");
        assert_eq!(value_text(&mut session, &source), "0", "{value}");
        let source = format!("sampleVariance([{value},{value},{value},{value}])");
        assert_eq!(value_text(&mut session, &source), "0", "{value}");
    }
}

#[test]
fn mean_of_consecutive_integers() {
    for size in 1i64..=30 {
        let values = (1..=size)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let mut session = Session::new();
        let mean = value_text(&mut session, &format!("mean([{values}])"));
        let expected = if size % 2 == 0 {
            format!("{}/2", size + 1)
        } else {
            ((size + 1) / 2).to_string()
        };
        assert_eq!(mean, expected, "{size}");
    }
}

#[test]
fn divisor_count_matches_manual_count() {
    for value in 1i64..=120 {
        let mut count = 0;
        for divisor in 1..=value {
            if value % divisor == 0 {
                count += 1;
            }
        }
        let mut session = Session::new();
        let result = session
            .eval_statement(&format!("divisorCount({value})"))
            .unwrap();
        let ExprView::Integer(result) = session.view(result) else {
            panic!("expected an integer");
        };
        assert_eq!(result.to_i64().unwrap(), count, "{value}");
    }
}

#[test]
fn totient_below_prime_count_identity() {
    let mut sum = 0i64;
    let mut session = Session::new();
    for value in 1i64..=100 {
        let result = session
            .eval_statement(&format!("totient({value})"))
            .unwrap();
        let ExprView::Integer(result) = session.view(result) else {
            panic!("expected an integer");
        };
        sum += result.to_i64().unwrap();
    }
    assert!(sum > 0);
    assert_eq!(sum, 3044);
}
