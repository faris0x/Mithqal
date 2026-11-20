use mithqal::{ExprId, ExprView, Session};

const NUMERIC: &str = include_str!("fixtures/differential/numeric.tsv");

const DIGITS: u32 = 50;

#[derive(Debug)]
struct Record {
    line: usize,
    name: &'static str,
    kind: &'static str,
    argument: &'static str,
    real: &'static str,
    imaginary: &'static str,
}

fn records() -> Vec<Record> {
    NUMERIC
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .map(|(index, line)| {
            let fields: Vec<&'static str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                5,
                "numeric.tsv line {}: expected 5 fields, found {fields:?}",
                index + 1
            );
            Record {
                line: index + 1,
                name: fields[0],
                kind: fields[1],
                argument: fields[2],
                real: fields[3],
                imaginary: fields[4],
            }
        })
        .collect()
}

fn check_kind(record: &Record) {
    match record.kind {
        "real" => assert!(
            !record.argument.starts_with("complex("),
            "numeric.tsv line {}: real fixture has a complex argument",
            record.line
        ),
        "complex" => assert!(
            record.argument.starts_with("complex("),
            "numeric.tsv line {}: complex fixture has a real argument",
            record.line
        ),
        kind => panic!(
            "numeric.tsv line {}: unknown fixture kind {kind}",
            record.line
        ),
    }
}

fn number(session: &Session, id: ExprId) -> f64 {
    match session.view(id).unwrap() {
        ExprView::Integer(value) => value
            .to_i64()
            .unwrap_or_else(|| value.to_string().parse().expect("integer out of f64 range"))
            as f64,
        ExprView::Rational(value) => {
            value.num().to_string().parse::<f64>().unwrap()
                / value.den().to_string().parse::<f64>().unwrap()
        }
        ExprView::Real(value) => value.to_f64(),
        other => panic!("expected a real number, found {other:?}"),
    }
}

fn parts(session: &Session, id: ExprId) -> (f64, f64) {
    match session.view(id).unwrap() {
        ExprView::Call { head, args } => match session.view(head).unwrap() {
            ExprView::Symbol("complex") if args.len() == 2 => {
                (number(session, args[0]), number(session, args[1]))
            }
            other => panic!("expected a complex call, found head {other:?}"),
        },
        _ => (number(session, id), 0.0),
    }
}

fn expected(text: &str) -> f64 {
    text.parse()
        .unwrap_or_else(|error| panic!("expected value {text} is not a number: {error}"))
}

fn assert_close(actual: f64, expected: f64, label: &str, record: &Record) {
    if actual == expected {
        return;
    }
    let tolerance = 5e-12 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "numeric.tsv line {}: {label} of {}({}) was {actual:e}, expected {expected:e}",
        record.line,
        record.name,
        record.argument
    );
}

#[test]
fn differential_mpmath_numeric_fixtures() {
    for record in records() {
        check_kind(&record);
        let mut session = Session::new();
        let source = format!("approx({}({}),{DIGITS})", record.name, record.argument);
        let value = session
            .eval_statement(&source)
            .unwrap_or_else(|error| panic!("{source} failed: {error:?}"));
        let (real, imaginary) = parts(&session, value);
        assert_close(real, expected(record.real), "real part", &record);
        assert_close(
            imaginary,
            expected(record.imaginary),
            "imaginary part",
            &record,
        );
    }
}

#[test]
fn differential_mpmath_numeric_accuracy_is_reported() {
    for record in records() {
        check_kind(&record);
        let mut session = Session::new();
        let source = format!("approx({}({}),{DIGITS})", record.name, record.argument);
        let value = session.eval_statement(&source).unwrap();
        let ExprView::Real(real) = session.view(value).unwrap() else {
            continue;
        };
        let digits = real.precision_digits();
        assert!(
            digits >= 40.0 || digits.is_infinite(),
            "numeric.tsv line {}: {}({}) reported only {digits} digits of precision",
            record.line,
            record.name,
            record.argument
        );
    }
}
