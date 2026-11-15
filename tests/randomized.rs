use mithqal::{ExprView, Session};

fn cases() -> u64 {
    std::env::var("MITHQAL_PROPERTY_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(32)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(6_364_136_223_846_793_005) ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }
}

fn polynomial(rng: &mut Rng, variables: &[&str], maximum_degree: i64) -> String {
    let terms = rng.range(1, 6) as usize;
    let mut parts = Vec::with_capacity(terms);
    for _ in 0..terms {
        let coefficient = rng.range(-5, 5);
        if coefficient == 0 {
            continue;
        }
        let variable = variables[(rng.next() as usize) % variables.len()];
        let degree = rng.range(0, maximum_degree);
        let term = match degree {
            0 => coefficient.to_string(),
            1 => format!("{coefficient}*{variable}"),
            _ => format!("{coefficient}*{variable}^{degree}"),
        };
        parts.push(term);
    }
    if parts.is_empty() {
        "1".to_string()
    } else {
        parts.join("+")
    }
}

fn format(session: &Session, value: mithqal::ExprId) -> String {
    session.format_infix(value).unwrap()
}

fn integer(session: &mut Session, source: &str) -> i64 {
    let value = session.eval_statement(source).unwrap();
    match session.view(value).unwrap() {
        ExprView::Integer(value) => value.to_i64().unwrap(),
        _ => panic!("expected an integer from {source}"),
    }
}

#[test]
fn random_polynomial_factorisation_preserves_the_polynomial() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed);
        let univariate = rng.next().is_multiple_of(2);
        let source = if univariate {
            polynomial(&mut rng, &["x"], 5)
        } else {
            polynomial(&mut rng, &["x", "y"], 3)
        };
        let mut session = Session::new();
        let factored = session
            .eval_statement(&format!("factor({source})"))
            .unwrap();
        let expanded = session
            .eval_statement(&format!("expand({})", format(&session, factored)))
            .unwrap();
        let target = session
            .eval_statement(&format!("expand({source})"))
            .unwrap();
        assert_eq!(expanded, target, "seed {seed}, source {source}");
    }
}

#[test]
fn random_polynomial_integrals_differentiate_back() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0xa5a5_5a5a);
        let source = polynomial(&mut rng, &["x"], 5);
        let mut session = Session::new();
        let integral = session
            .eval_statement(&format!("integrate({source},x)"))
            .unwrap();
        let derivative = session
            .eval_statement(&format!(
                "expand(differentiate({},x))",
                format(&session, integral)
            ))
            .unwrap();
        let target = session
            .eval_statement(&format!("expand({source})"))
            .unwrap();
        assert_eq!(derivative, target, "seed {seed}, source {source}");
    }
}

#[test]
fn random_two_by_two_inverses_and_determinants_hold() {
    let mut checked = 0;
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x1234_5678);
        let (a, b, c, d) = (
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
        );
        if a * d - b * c == 0 {
            continue;
        }
        let mut session = Session::new();
        let inverse = session
            .eval_statement(&format!(
                "matmul([[{a},{b}],[{c},{d}]],inverse([[{a},{b}],[{c},{d}]]))"
            ))
            .unwrap();
        assert_eq!(
            format(&session, inverse),
            "[[1,0],[0,1]]",
            "seed {seed}, matrix [[{a},{b}],[{c},{d}]]"
        );
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn random_integer_factorisations_reconstruct_the_value() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x00dd_ba11);
        let value = rng.range(2, 400);
        let mut session = Session::new();
        let factors = session
            .eval_statement(&format!("factorInteger({value})"))
            .unwrap();
        let mut product = 1i64;
        let entries = match session.view(factors).unwrap() {
            ExprView::Call { args, .. } => args.to_vec(),
            _ => panic!("expected a list for {value}"),
        };
        for entry in entries {
            let pair = match session.view(entry).unwrap() {
                ExprView::Call { args, .. } => args.to_vec(),
                _ => panic!("expected a factor pair for {value}"),
            };
            let base = match session.view(pair[0]).unwrap() {
                ExprView::Integer(base) => base.to_i64().unwrap(),
                _ => panic!("expected an integer base"),
            };
            let exponent = match session.view(pair[1]).unwrap() {
                ExprView::Integer(exponent) => exponent.to_i64().unwrap(),
                _ => panic!("expected an integer exponent"),
            };
            product *= base.pow(exponent as u32);
        }
        assert_eq!(product, value, "seed {seed}, value {value}");
    }
}

#[test]
fn random_divisor_counts_match_trial_division() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0xfeed_face);
        let value = rng.range(1, 300);
        let mut expected = 0;
        for divisor in 1..=value {
            if value % divisor == 0 {
                expected += 1;
            }
        }
        let mut session = Session::new();
        assert_eq!(
            integer(&mut session, &format!("divisorCount({value})")),
            expected,
            "seed {seed}, value {value}"
        );
    }
}

#[test]
fn random_assumption_signs_simplify_absolute_values() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x00c0_ffee);
        let left = rng.next().is_multiple_of(2);
        let right = rng.next().is_multiple_of(2);
        let left_relation = if left { "x>0" } else { "x<0" };
        let right_relation = if right { "y>0" } else { "y<0" };
        let expected = if left == right { "x*y" } else { "-(x*y)" };
        let mut session = Session::new();
        let value = session
            .eval_statement(&format!(
                "assume(and({left_relation},{right_relation}),abs(x*y))"
            ))
            .unwrap();
        assert_eq!(format(&session, value), expected, "seed {seed}");
    }
}

#[test]
fn random_expression_printing_round_trips() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x7777_1111);
        let source = polynomial(&mut rng, &["x", "y", "z"], 3);
        let mut session = Session::new();
        let value = session.eval_statement(&source).unwrap();
        let canonical = session.format(value).unwrap();
        let reparsed = session.parse_statement(&canonical).unwrap();
        assert_eq!(reparsed, value, "seed {seed}, source {source}");
    }
}

#[test]
fn random_two_by_two_determinants_are_multiplicative() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x2468_ace0);
        let (a, b, c, d) = (
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
        );
        let (e, f, g, h) = (
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
            rng.range(-4, 4),
        );
        let expected = (a * d - b * c) * (e * h - f * g);
        let mut session = Session::new();
        let actual = integer(
            &mut session,
            &format!("det(matmul([[{a},{b}],[{c},{d}]],[[{e},{f}],[{g},{h}]]))"),
        );
        assert_eq!(actual, expected, "seed {seed}");
    }
}
