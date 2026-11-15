use mithqal::{ExprId, ExprView, Session};

fn cases() -> u64 {
    std::env::var("MITHQAL_PROPERTY_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(32)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(6_364_136_223_846_793_005) ^ 0xa076_1d64_78bd_642f)
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

fn evaluate(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn number(session: &Session, id: ExprId) -> f64 {
    match session.view(id).unwrap() {
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        ExprView::Real(value) => value.to_f64(),
        _ => panic!("expected a numeric value"),
    }
}

fn interval_bounds(session: &Session, id: ExprId) -> (f64, f64) {
    let args = match session.view(id).unwrap() {
        ExprView::Call { head, args } => match session.view(head).unwrap() {
            ExprView::Symbol("interval") => args,
            _ => panic!("expected an interval"),
        },
        _ => panic!("expected an interval"),
    };
    (number(session, args[0]), number(session, args[1]))
}

fn numeric_parts(session: &Session, id: ExprId) -> (f64, f64) {
    match session.view(id).unwrap() {
        ExprView::Call { head, args } => match session.view(head).unwrap() {
            ExprView::Symbol("complex") => (number(session, args[0]), number(session, args[1])),
            _ => (number(session, id), 0.0),
        },
        _ => (number(session, id), 0.0),
    }
}

#[test]
fn random_interval_addition_contains_exact_endpoints() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x1111_2222);
        let a = rng.range(-6, 4);
        let b = rng.range(a, 6);
        let c = rng.range(-6, 4);
        let d = rng.range(c, 6);
        let source = format!("interval({a},{b})+interval({c},{d})");
        let (session, value) = evaluate(&source);
        let (low, high) = interval_bounds(&session, value);
        assert!(
            low <= (a + c) as f64 + 1e-9,
            "seed {seed}: {source} low {low}"
        );
        assert!(
            high >= (b + d) as f64 - 1e-9,
            "seed {seed}: {source} high {high}"
        );
        assert!(low <= high, "seed {seed}: {source}");
    }
}

#[test]
fn random_interval_multiplication_contains_exact_products() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x3333_4444);
        let a = rng.range(-6, 4);
        let b = rng.range(a, 6);
        let c = rng.range(-6, 4);
        let d = rng.range(c, 6);
        let products = [a * c, a * d, b * c, b * d];
        let expected_low = *products.iter().min().unwrap();
        let expected_high = *products.iter().max().unwrap();
        let source = format!("interval({a},{b})*interval({c},{d})");
        let (session, value) = evaluate(&source);
        let (low, high) = interval_bounds(&session, value);
        assert!(
            low <= expected_low as f64 + 1e-9,
            "seed {seed}: {source} low {low}, expected {expected_low}"
        );
        assert!(
            high >= expected_high as f64 - 1e-9,
            "seed {seed}: {source} high {high}, expected {expected_high}"
        );
        assert!(low <= high, "seed {seed}: {source}");
    }
}

#[test]
fn random_complex_products_and_quotients_match_exact_formulas() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x5555_6666);
        let a = rng.range(-4, 4);
        let b = rng.range(-4, 4);
        let c = rng.range(-4, 4);
        let d = rng.range(-4, 4);
        let real = (a * c - b * d) as f64;
        let imaginary = (a * d + b * c) as f64;
        let source = format!("approx(({a}+({b})*i)*({c}+({d})*i),40)");
        let (session, value) = evaluate(&source);
        let (actual_real, actual_imaginary) = numeric_parts(&session, value);
        assert!(
            (actual_real - real).abs() < 1e-9,
            "seed {seed}: {source} real {actual_real}, expected {real}"
        );
        assert!(
            (actual_imaginary - imaginary).abs() < 1e-9,
            "seed {seed}: {source} imaginary {actual_imaginary}, expected {imaginary}"
        );

        let denominator = c * c + d * d;
        if denominator == 0 {
            continue;
        }
        let real = ((a * c + b * d) as f64) / denominator as f64;
        let imaginary = ((b * c - a * d) as f64) / denominator as f64;
        let source = format!("approx(({a}+({b})*i)/({c}+({d})*i),40)");
        let (session, value) = evaluate(&source);
        let (actual_real, actual_imaginary) = numeric_parts(&session, value);
        assert!(
            (actual_real - real).abs() < 1e-9,
            "seed {seed}: {source} real {actual_real}, expected {real}"
        );
        assert!(
            (actual_imaginary - imaginary).abs() < 1e-9,
            "seed {seed}: {source} imaginary {actual_imaginary}, expected {imaginary}"
        );
    }
}

#[test]
fn random_significance_reports_precision_and_accuracy() {
    for seed in 0..cases() {
        let mut rng = Rng::new(seed ^ 0x7777_8888);
        let value = rng.range(1, 100) * if rng.next().is_multiple_of(2) { 1 } else { -1 };
        let source = format!("approx(pi*{value},30)");
        let (session, expression) = evaluate(&source);
        let ExprView::Real(real) = session.view(expression).unwrap() else {
            panic!("expected a real value for {source}");
        };
        assert_eq!(real.sign(), value.signum() as i8, "seed {seed}");
        assert!(real.precision_digits() >= 25.0, "seed {seed}");
        assert!(
            real.accuracy_digits().is_finite() && real.accuracy_digits() >= 20.0,
            "seed {seed}: accuracy {}",
            real.accuracy_digits()
        );
    }
}
