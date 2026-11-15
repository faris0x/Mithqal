use mithqal::{ExprId, ExprView, Session};

const ALGEBRA: &str = include_str!("fixtures/differential/algebra.tsv");
const CALCULUS: &str = include_str!("fixtures/differential/calculus.tsv");
const LINALG: &str = include_str!("fixtures/differential/linalg.tsv");
const PROBES: &str = include_str!("fixtures/differential/probes.tsv");
const FAILURES: &str = include_str!("fixtures/differential/failures.tsv");
const NUMBER_THEORY: &str = include_str!("fixtures/differential/numbertheory.tsv");
const STATISTICS: &str = include_str!("fixtures/differential/statistics.tsv");

fn records(source: &'static str) -> Vec<Vec<&'static str>> {
    source
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('\t').collect())
        .collect()
}

fn evaluate(session: &mut Session, source: &str) -> ExprId {
    session.eval_statement(source).unwrap()
}

fn text(session: &Session, value: ExprId) -> String {
    session.format_infix(value).unwrap()
}

fn factor_units(session: &Session, value: ExprId) -> usize {
    match session.view(value).unwrap() {
        ExprView::Integer(_) | ExprView::Rational(_) | ExprView::Real(_) => 0,
        ExprView::Symbol(_) => 1,
        ExprView::Call { head, args } => {
            let name = match session.view(head).unwrap() {
                ExprView::Symbol(name) => name,
                _ => return 1,
            };
            match name {
                "times" => args
                    .iter()
                    .map(|argument| factor_units(session, *argument))
                    .sum(),
                "negate" if args.len() == 1 => factor_units(session, args[0]),
                "power" if args.len() == 2 => {
                    let exponent = match session.view(args[1]).unwrap() {
                        ExprView::Integer(value) => value.to_i64().unwrap_or(1).max(1) as usize,
                        _ => 1,
                    };
                    factor_units(session, args[0]) * exponent
                }
                _ => 1,
            }
        }
        _ => 1,
    }
}

fn assert_equivalent(session: &mut Session, left: &str, right: &str, context: &str) {
    let source = format!("cancel(together(({left})-({right})))");
    let value = evaluate(session, &source);
    assert_eq!(text(session, value), "0", "{context}: {source}");
}

fn assert_equivalent_values(session: &mut Session, left: ExprId, right: ExprId, context: &str) {
    let left = text(session, left);
    let right = text(session, right);
    assert_equivalent(session, &left, &right, context);
}

#[test]
fn differential_algebra_fixtures() {
    for record in records(ALGEBRA) {
        let mut session = Session::new();
        match record[0] {
            "factor" => {
                let source = record[1];
                let expected = record[2];
                let factored = evaluate(&mut session, &format!("factor({source})"));
                let expected_value = evaluate(&mut session, expected);
                let units = factor_units(&session, factored);
                let expected_units = factor_units(&session, expected_value);
                assert_eq!(
                    units,
                    expected_units,
                    "{source}: factored {}, units {units}, expected {expected_units}",
                    text(&session, factored)
                );
                let factored_text = text(&session, factored);
                let expanded = evaluate(&mut session, &format!("expand({factored_text})"));
                let target = evaluate(&mut session, &format!("expand({source})"));
                assert_eq!(
                    expanded,
                    target,
                    "{source}: factored {}",
                    text(&session, factored)
                );
            }
            "expand" => {
                let source = record[1];
                let expected = record[2];
                let actual = evaluate(&mut session, &format!("expand({source})"));
                let expected_value = evaluate(&mut session, expected);
                assert_equivalent_values(&mut session, actual, expected_value, source);
            }
            kind => panic!("unknown algebra fixture kind {kind}"),
        }
    }
}

#[test]
fn differential_calculus_fixtures() {
    for record in records(CALCULUS) {
        let mut session = Session::new();
        match record[0] {
            "diff" => {
                let variable = record[1];
                let source = record[2];
                let expected = record[3];
                let actual = evaluate(&mut session, &format!("differentiate({source},{variable})"));
                let expected_value = evaluate(&mut session, expected);
                assert_equivalent_values(&mut session, actual, expected_value, source);
            }
            "integrate" => {
                let variable = record[1];
                let source = record[2];
                let expected = record[3];
                let actual = evaluate(&mut session, &format!("integrate({source},{variable})"));
                let actual_text = text(&session, actual);
                assert!(
                    !actual_text.starts_with("integrate("),
                    "{source}: integral stayed unevaluated"
                );
                let derivative = evaluate(
                    &mut session,
                    &format!("differentiate({actual_text},{variable})"),
                );
                let derivative_text = text(&session, derivative);
                assert_equivalent(&mut session, &derivative_text, source, source);
                let expected_value = evaluate(&mut session, expected);
                let expected_text = text(&session, expected_value);
                let difference = evaluate(
                    &mut session,
                    &format!("differentiate(({actual_text})-({expected_text}),{variable})"),
                );
                let difference_text = text(&session, difference);
                assert_equivalent(&mut session, &difference_text, "0", source);
            }
            "limit" => {
                let variable = record[1];
                let point = record[2];
                let source = record[3];
                let expected = record[4];
                let actual = evaluate(&mut session, &format!("limit({source},{variable},{point})"));
                let expected_value = evaluate(&mut session, expected);
                assert_equivalent_values(&mut session, actual, expected_value, source);
            }
            kind => panic!("unknown calculus fixture kind {kind}"),
        }
    }
}

#[test]
fn differential_linear_algebra_fixtures() {
    for record in records(LINALG) {
        let mut session = Session::new();
        match record[0] {
            "det" => {
                let matrix = record[1];
                let expected = record[2];
                let actual = evaluate(&mut session, &format!("det({matrix})"));
                let expected_value = evaluate(&mut session, expected);
                assert_equivalent_values(&mut session, actual, expected_value, matrix);
            }
            "inverse" => {
                let matrix = record[1];
                let expected = record[2];
                let actual = evaluate(&mut session, &format!("inverse({matrix})"));
                assert_eq!(
                    text(&session, actual),
                    expected,
                    "{matrix}: inverse did not match the expected inverse"
                );
            }
            kind => panic!("unknown linear algebra fixture kind {kind}"),
        }
    }
}

fn substitute_symbols(source: &str, assignments: &[(String, String)]) -> String {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let name = &source[start..index];
            if let Some((_, value)) = assignments.iter().find(|(symbol, _)| symbol == name) {
                output.push('(');
                output.push_str(value);
                output.push(')');
            } else {
                output.push_str(name);
            }
        } else {
            output.push(byte as char);
            index += 1;
        }
    }
    output
}

fn probe_value(session: &mut Session, assignments: &str, expression: &str) -> f64 {
    let result = evaluate(session, expression);
    let result_text = text(session, result);
    let assignments = if assignments.is_empty() {
        Vec::new()
    } else {
        assignments
            .split(',')
            .map(|assignment| {
                let (name, value) = assignment
                    .split_once('=')
                    .unwrap_or_else(|| panic!("malformed probe assignment {assignment}"));
                (name.to_string(), value.to_string())
            })
            .collect::<Vec<_>>()
    };
    let substituted = substitute_symbols(&result_text, &assignments);
    let value = evaluate(session, &format!("approx(({substituted}),50)"));
    match session.view(value).unwrap() {
        ExprView::Real(value) => value.to_f64(),
        ExprView::Integer(value) => value.to_i64().unwrap() as f64,
        ExprView::Rational(value) => {
            value.num().to_i64().unwrap() as f64 / value.den().to_i64().unwrap() as f64
        }
        _ => panic!("probe did not produce a numeric value for {expression}"),
    }
}

#[test]
fn differential_numeric_probes() {
    for record in records(PROBES) {
        assert_eq!(record.len(), 4, "malformed probe record: {record:?}");
        let mut session = Session::new();
        let actual = probe_value(&mut session, record[1], record[2]);
        let expected: f64 = record[3].parse().unwrap();
        let tolerance = 1e-12 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= tolerance,
            "{} with {}: actual {actual}, expected {expected}",
            record[2],
            record[1]
        );
    }
}

#[test]
fn differential_failure_fixtures() {
    for record in records(FAILURES) {
        assert_eq!(record.len(), 3, "malformed failure record: {record:?}");
        let mut session = Session::new();
        match record[0] {
            "unevaluated" => {
                let value = evaluate(&mut session, record[1]);
                let ExprView::Call { head, .. } = session.view(value).unwrap() else {
                    panic!("{}: expected an unevaluated call", record[1]);
                };
                let ExprView::Symbol(name) = session.view(head).unwrap() else {
                    panic!("{}: expected a symbol head", record[1]);
                };
                assert_eq!(name, record[2], "{}", record[1]);
            }
            "diagnostic" => {
                let error = session.eval_statement(record[1]).unwrap_err();
                assert_eq!(error.code.as_str(), record[2], "{}", record[1]);
            }
            "unchanged" => {
                let actual = evaluate(&mut session, record[1]);
                let expected = evaluate(&mut session, record[2]);
                assert_eq!(
                    text(&session, actual),
                    text(&session, expected),
                    "{}",
                    record[1]
                );
            }
            kind => panic!("unknown failure fixture kind {kind}"),
        }
    }
}

#[test]
fn differential_exact_value_fixtures() {
    for record in records(NUMBER_THEORY)
        .into_iter()
        .chain(records(STATISTICS))
    {
        assert_eq!(record.len(), 3, "malformed exact record: {record:?}");
        assert_eq!(record[0], "equal", "unexpected exact record kind");
        let mut session = Session::new();
        let actual = evaluate(&mut session, record[1]);
        let expected = evaluate(&mut session, record[2]);
        assert_eq!(
            text(&session, actual),
            text(&session, expected),
            "{}",
            record[1]
        );
    }
}
