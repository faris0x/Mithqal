use mithqal::expr::Arena;
use mithqal::num::Int;
use mithqal::syntax::{parse_source, parse_statement};
use mithqal::{ExprView, Session};

fn integer_value(session: &mut Session, source: &str) -> i64 {
    let value = session.eval_statement(source).unwrap();
    match session.view(value) {
        ExprView::Integer(value) => value.to_i64().unwrap(),
        _ => panic!("expected an integer for {source}"),
    }
}

#[test]
fn arithmetic_execution_corpus() {
    let mut session = Session::new();
    for left in -30i64..=30 {
        for right in -10i64..=10 {
            let sum = integer_value(&mut session, &format!("{left}+{right}"));
            assert_eq!(sum, left + right, "{left}+{right}");
            let difference = integer_value(&mut session, &format!("{left}-{right}"));
            assert_eq!(difference, left - right, "{left}-{right}");
            let product = integer_value(&mut session, &format!("{left}*{right}"));
            assert_eq!(product, left * right, "{left}*{right}");
        }
    }
}

#[test]
fn power_execution_corpus() {
    let mut session = Session::new();
    for base in -8i64..=8 {
        for exponent in 0u32..=8 {
            if base == 0 && exponent == 0 {
                continue;
            }
            let expected = (base as i128).pow(exponent) as i64;
            let actual = integer_value(&mut session, &format!("({base})^{exponent}"));
            assert_eq!(actual, expected, "{base}^{exponent}");
        }
    }
}

#[test]
fn gcd_lcm_execution_corpus() {
    for left in 1i64..=40 {
        for right in 1i64..=40 {
            let a = Int::from_i64(left);
            let b = Int::from_i64(right);
            let gcd = a.gcd(&b).to_i64().unwrap();
            assert_eq!(gcd, gcd_i64(left, right), "{left},{right}");
            let lcm = left / gcd * right;
            assert_eq!(lcm, effective_lcm(left, right), "{left},{right}");
        }
    }
}

fn gcd_i64(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs()
}

fn effective_lcm(left: i64, right: i64) -> i64 {
    left / gcd_i64(left, right) * right
}

#[test]
fn parser_round_trip_corpus() {
    for left in 0i64..=20 {
        for right in 0i64..=20 {
            let source = format!("{left}+{right}*2");
            let mut arena = Arena::new();
            let expression = parse_statement(&mut arena, &source).unwrap();
            let canonical = mithqal::print::canonical(&arena, expression);
            let reparsed = parse_statement(&mut arena, &canonical).unwrap();
            assert_eq!(expression, reparsed, "{source}");
            let infix = mithqal::print::infix(&arena, expression);
            let reparsed = parse_statement(&mut arena, &infix).unwrap();
            assert_eq!(expression, reparsed, "{source}");
        }
    }
}

#[test]
fn source_statement_count_corpus() {
    let mut arena = Arena::new();
    let source: String = (0..50).map(|value| format!("{value}+1\n")).collect();
    let statements = parse_source(&mut arena, &source).unwrap();
    assert_eq!(statements.len(), 50);
}

#[test]
fn numeric_function_corpus() {
    let mut session = Session::new();
    for value in 0i64..=20 {
        let factorial = integer_value(&mut session, &format!("factorial({value})"));
        assert!(factorial > 0, "factorial({value})");
    }
    for n in 0i64..=20 {
        for k in 0i64..=n {
            let value = integer_value(&mut session, &format!("binomial({n},{k})"));
            assert!(value >= 1, "binomial({n},{k})");
        }
    }
}

#[test]
fn floor_ceil_round_corpus() {
    let mut session = Session::new();
    for numerator in -20i64..=20 {
        for denominator in 1i64..=8 {
            let floor = integer_value(&mut session, &format!("floor({numerator}/{denominator})"));
            let ceil = integer_value(&mut session, &format!("ceil({numerator}/{denominator})"));
            let round = integer_value(&mut session, &format!("round({numerator}/{denominator})"));
            assert!(floor <= ceil, "{numerator}/{denominator}");
            assert!(
                round >= floor && round <= ceil + 1,
                "{numerator}/{denominator}"
            );
        }
    }
}

#[test]
fn language_execution_corpus() {
    for value in 0i64..=20 {
        let source = format!("mod m\nlet x={value}\nprt x+1\nend\n");
        let mut session = Session::new();
        session.eval_source(&source).unwrap();
        assert_eq!(session.output(), &[(value + 1).to_string()]);
    }
}

#[test]
fn explicit_numeric_assertions() {
    assert_eq!(Int::from_i64(2).gcd(&Int::from_i64(4)).to_decimal(), "2");
    assert_eq!(Int::from_i64(4).gcd(&Int::from_i64(6)).to_decimal(), "2");
    assert_eq!(Int::from_i64(6).gcd(&Int::from_i64(9)).to_decimal(), "3");
    assert_eq!(Int::from_i64(9).gcd(&Int::from_i64(12)).to_decimal(), "3");
    assert_eq!(Int::from_i64(12).gcd(&Int::from_i64(18)).to_decimal(), "6");
    assert_eq!(Int::from_i64(18).gcd(&Int::from_i64(24)).to_decimal(), "6");
    assert_eq!(Int::from_i64(24).gcd(&Int::from_i64(30)).to_decimal(), "6");
    assert_eq!(Int::from_i64(30).gcd(&Int::from_i64(36)).to_decimal(), "6");
    assert_eq!(Int::from_i64(36).gcd(&Int::from_i64(42)).to_decimal(), "6");
    assert_eq!(Int::from_i64(42).gcd(&Int::from_i64(48)).to_decimal(), "6");
    assert_eq!(Int::from_i64(7).pow_u32(2).to_decimal(), "49");
    assert_eq!(Int::from_i64(7).pow_u32(3).to_decimal(), "343");
    assert_eq!(Int::from_i64(7).pow_u32(4).to_decimal(), "2401");
    assert_eq!(Int::from_i64(8).pow_u32(2).to_decimal(), "64");
    assert_eq!(Int::from_i64(8).pow_u32(3).to_decimal(), "512");
    assert_eq!(Int::from_i64(8).pow_u32(4).to_decimal(), "4096");
    assert_eq!(Int::from_i64(9).pow_u32(2).to_decimal(), "81");
    assert_eq!(Int::from_i64(9).pow_u32(3).to_decimal(), "729");
    assert_eq!(Int::from_i64(9).pow_u32(4).to_decimal(), "6561");
    assert_eq!(Int::from_i64(10).pow_u32(2).to_decimal(), "100");
    assert_eq!(Int::from_i64(10).pow_u32(3).to_decimal(), "1000");
    assert_eq!(Int::from_i64(10).pow_u32(4).to_decimal(), "10000");
    assert_eq!(Int::from_i64(11).pow_u32(2).to_decimal(), "121");
    assert_eq!(Int::from_i64(11).pow_u32(3).to_decimal(), "1331");
    assert_eq!(Int::from_i64(11).pow_u32(4).to_decimal(), "14641");
    assert_eq!(Int::from_i64(12).pow_u32(2).to_decimal(), "144");
    assert_eq!(Int::from_i64(12).pow_u32(3).to_decimal(), "1728");
    assert_eq!(Int::from_i64(12).pow_u32(4).to_decimal(), "20736");
    assert_eq!(Int::from_i64(13).pow_u32(2).to_decimal(), "169");
    assert_eq!(Int::from_i64(13).pow_u32(3).to_decimal(), "2197");
    assert_eq!(Int::from_i64(13).pow_u32(4).to_decimal(), "28561");
}

#[test]
fn determinant_execution_corpus() {
    let mut session = Session::new();
    for a in -2i64..=2 {
        for b in -2i64..=2 {
            for c in -2i64..=2 {
                for d in -2i64..=2 {
                    let source = format!("det([[{a},{b}],[{c},{d}]])");
                    let value = integer_value(&mut session, &source);
                    assert_eq!(value, a * d - b * c, "{source}");
                }
            }
        }
    }
}

#[test]
fn identity_multiplication_corpus() {
    let mut session = Session::new();
    for size in 1i64..=5 {
        let source = format!("matmul(identity({size}),identity({size}))");
        let value = session.eval_statement(&source).unwrap();
        let expected = session
            .eval_statement(&format!("identity({size})"))
            .unwrap();
        assert_eq!(value, expected, "{source}");
    }
}

#[test]
fn trace_execution_corpus() {
    let mut session = Session::new();
    for size in 1i64..=6 {
        let source = format!("trace(identity({size}))");
        let value = integer_value(&mut session, &source);
        assert_eq!(value, size, "{source}");
    }
}

#[test]
fn primality_execution_corpus() {
    let mut session = Session::new();
    for value in 0i64..=200 {
        let expected = is_prime_i64(value);
        let text = session
            .eval_statement(&format!("isPrime({value})"))
            .unwrap();
        let expected_text = if expected { "true" } else { "false" };
        assert_eq!(session.format_infix(text), expected_text, "{value}");
    }
}

fn is_prime_i64(value: i64) -> bool {
    if value < 2 {
        return false;
    }
    let mut divisor = 2i64;
    while divisor * divisor <= value {
        if value % divisor == 0 {
            return false;
        }
        divisor += 1;
    }
    true
}

#[test]
fn divisor_count_execution_corpus() {
    for value in 1i64..=100 {
        let mut count = 0;
        for divisor in 1..=value {
            if value % divisor == 0 {
                count += 1;
            }
        }
        let mut session = Session::new();
        let divisors = session
            .eval_statement(&format!("divisors({value})"))
            .unwrap();
        match session.view(divisors) {
            ExprView::Call { args, .. } => assert_eq!(args.len() as i64, count, "{value}"),
            _ => panic!("expected a list for {value}"),
        }
    }
}

#[test]
fn logic_truth_table_corpus() {
    let mut session = Session::new();
    for left in [false, true] {
        for right in [false, true] {
            let source = format!("xor({left},{right})");
            let value = session.eval_statement(&source).unwrap();
            let expected = if left ^ right { "true" } else { "false" };
            assert_eq!(session.format_infix(value), expected, "{source}");
            let source = format!("implies({left},{right})");
            let value = session.eval_statement(&source).unwrap();
            let expected = if !left || right { "true" } else { "false" };
            assert_eq!(session.format_infix(value), expected, "{source}");
        }
    }
}

#[test]
fn mean_execution_corpus() {
    let mut session = Session::new();
    for size in 1i64..=50 {
        let values = (1..=size)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("mean([{values}])");
        let value = session.eval_statement(&source).unwrap();
        let expected = if size % 2 == 0 {
            format!("{}/2", size + 1)
        } else {
            ((size + 1) / 2).to_string()
        };
        assert_eq!(session.format_infix(value), expected, "{size}");
    }
}

#[test]
fn prime_count_execution_corpus() {
    let mut session = Session::new();
    for value in 0i64..=200 {
        let expected = (0..=value).filter(|number| is_prime_i64(*number)).count();
        let result = session
            .eval_statement(&format!("primeCount({value})"))
            .unwrap();
        match session.view(result) {
            ExprView::Integer(result) => {
                assert_eq!(result.to_i64().unwrap(), expected as i64, "{value}")
            }
            _ => panic!("expected an integer"),
        }
    }
}

#[test]
fn fibonacci_execution_corpus() {
    let mut previous = 0i64;
    let mut current = 1i64;
    for index in 0..=40 {
        if index == 0 {
            assert_eq!(previous, 0);
        } else if index == 1 {
            assert_eq!(current, 1);
        } else {
            let next = previous + current;
            previous = current;
            current = next;
        }
        let mut session = Session::new();
        let result = session
            .eval_statement(&format!("fibonacci({index})"))
            .unwrap();
        match session.view(result) {
            ExprView::Integer(value) => {
                let expected = if index == 0 { 0 } else { current };
                assert_eq!(value.to_i64().unwrap(), expected, "{index}");
            }
            _ => panic!("expected an integer"),
        }
    }
}
