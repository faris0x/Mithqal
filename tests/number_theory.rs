use mithqal::ExprId;
use mithqal::{ErrorCode, Session};

fn evaluate(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value).unwrap()
}

fn integer(source: &str) -> i64 {
    let (session, value) = evaluate(source);
    match session.view(value).unwrap() {
        mithqal::ExprView::Integer(value) => value.to_i64().unwrap(),
        _ => panic!("expected an integer for {source}"),
    }
}

#[test]
fn primality() {
    let primes = [
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97,
    ];
    for value in 0..=100 {
        let expected = primes.contains(&value);
        let actual = text(&format!("isPrime({value})"));
        assert_eq!(actual, if expected { "true" } else { "false" }, "{value}");
    }
    assert_eq!(text("isPrime(-7)"), "false");
    assert_eq!(text("isPrime(1000000007)"), "true");
}

#[test]
fn next_primes() {
    assert_eq!(integer("nextPrime(-10)"), 2);
    assert_eq!(integer("nextPrime(0)"), 2);
    assert_eq!(integer("nextPrime(1)"), 2);
    assert_eq!(integer("nextPrime(2)"), 3);
    assert_eq!(integer("nextPrime(10)"), 11);
    assert_eq!(integer("nextPrime(96)"), 97);
    assert_eq!(integer("nextPrime(100)"), 101);
}

#[test]
fn integer_factorization() {
    assert_eq!(text("factorInteger(1)"), "[]");
    assert_eq!(text("factorInteger(2)"), "[[2,1]]");
    assert_eq!(text("factorInteger(12)"), "[[2,2],[3,1]]");
    assert_eq!(text("factorInteger(360)"), "[[2,3],[3,2],[5,1]]");
    assert_eq!(text("factorInteger(-12)"), "[[-1,1],[2,2],[3,1]]");
    assert_eq!(text("factorInteger(97)"), "[[97,1]]");
    assert_eq!(
        text("factorInteger(1000000007*1000000009)"),
        "[[1000000007,1],[1000000009,1]]"
    );
    assert!(text("factorInteger(0)").contains("factorInteger"));
}

#[test]
fn divisors_and_totients() {
    assert_eq!(text("divisors(1)"), "[1]");
    assert_eq!(text("divisors(12)"), "[1,2,3,4,6,12]");
    assert_eq!(text("divisors(36)"), "[1,2,3,4,6,9,12,18,36]");
    assert_eq!(text("divisors(97)"), "[1,97]");
    assert_eq!(text("totient(1)"), "1");
    assert_eq!(text("totient(9)"), "6");
    assert_eq!(text("totient(36)"), "12");
    assert_eq!(text("totient(97)"), "96");
}

#[test]
fn divisor_sums() {
    assert_eq!(text("sigma(1)"), "1");
    assert_eq!(text("sigma(6)"), "12");
    assert_eq!(text("sigma(12)"), "28");
    assert_eq!(text("sigma(36)"), "91");
    assert_eq!(text("sigma(97)"), "98");
}

#[test]
fn parity_and_divisibility() {
    assert_eq!(text("isEven(0)"), "true");
    assert_eq!(text("isEven(2)"), "true");
    assert_eq!(text("isEven(-2)"), "true");
    assert_eq!(text("isEven(3)"), "false");
    assert_eq!(text("isOdd(1)"), "true");
    assert_eq!(text("isOdd(-3)"), "true");
    assert_eq!(text("isOdd(4)"), "false");
    assert_eq!(text("divides(3,12)"), "true");
    assert_eq!(text("divides(5,12)"), "false");
    assert_eq!(text("divides(-3,12)"), "true");
    assert_eq!(text("divides(1,0)"), "true");
}

#[test]
fn modular_arithmetic() {
    assert_eq!(text("mod(7,3)"), "1");
    assert_eq!(text("mod(-7,3)"), "2");
    assert_eq!(text("mod(7,-3)"), "1");
    assert_eq!(text("mod(-7,-3)"), "2");
    assert_eq!(text("mod(0,5)"), "0");
    assert_eq!(text("powerMod(2,10,1000)"), "24");
    assert_eq!(text("powerMod(3,0,7)"), "1");
    assert_eq!(text("powerMod(3,4,5)"), "1");
    assert_eq!(text("powerMod(2,1000000000,1000000007)"), "140625001");
    assert!(text("powerMod(2,-1,5)").contains("powerMod"));
    assert!(text("mod(1,0)").contains("mod"));
}

#[test]
fn number_theory_leaves_symbolic_inputs_unevaluated() {
    for source in [
        "isPrime(x)",
        "nextPrime(x)",
        "factorInteger(x)",
        "divisors(x)",
        "totient(x)",
        "sigma(x)",
        "isEven(x)",
        "isOdd(x)",
        "divides(x,2)",
        "mod(x,3)",
        "powerMod(2,x,5)",
    ] {
        assert!(
            text(source).contains(source.split('(').next().unwrap()),
            "{source}"
        );
    }
}

#[test]
fn number_theory_arity_errors() {
    for source in [
        "isPrime()",
        "isPrime(1,2)",
        "nextPrime()",
        "factorInteger()",
        "divisors()",
        "totient()",
        "sigma()",
        "isEven()",
        "isOdd()",
        "divides(1)",
        "divides(1,2,3)",
        "mod(1)",
        "powerMod(1,2)",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn number_theory_symbols_are_protected() {
    for name in [
        "isPrime",
        "nextPrime",
        "factorInteger",
        "divisors",
        "totient",
        "sigma",
        "isEven",
        "isOdd",
        "divides",
        "mod",
        "powerMod",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(&format!("{name}=1")).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{name}");
    }
}

#[test]
fn number_theory_from_the_language() {
    let source = r#"
mod numbers
let n=36
prt isPrime(n+1)
prt divisors(n)
prt totient(n)
if isEven(n) then
  prt mod(n,5)
end
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    assert_eq!(
        session.output(),
        &["true", "[1,2,3,4,6,9,12,18,36]", "12", "1"]
    );
}

#[test]
fn previous_primes() {
    assert_eq!(integer("previousPrime(3)"), 2);
    assert_eq!(integer("previousPrime(10)"), 7);
    assert_eq!(integer("previousPrime(100)"), 97);
    assert!(text("previousPrime(2)").contains("previousPrime"));
}

#[test]
fn divisor_counts() {
    assert_eq!(text("divisorCount(1)"), "1");
    assert_eq!(text("divisorCount(12)"), "6");
    assert_eq!(text("divisorCount(36)"), "9");
    assert_eq!(text("divisorCount(97)"), "2");
    assert_eq!(text("divisorCount(360)"), "24");
}

#[test]
fn moebius_and_radicals() {
    assert_eq!(text("moebius(1)"), "1");
    assert_eq!(text("moebius(2)"), "-1");
    assert_eq!(text("moebius(6)"), "1");
    assert_eq!(text("moebius(12)"), "0");
    assert_eq!(text("moebius(30)"), "-1");
    assert_eq!(text("radical(1)"), "1");
    assert_eq!(text("radical(12)"), "6");
    assert_eq!(text("radical(360)"), "30");
    assert_eq!(text("isSquareFree(30)"), "true");
    assert_eq!(text("isSquareFree(12)"), "false");
}

#[test]
fn prime_counts() {
    assert_eq!(text("primeCount(0)"), "0");
    assert_eq!(text("primeCount(1)"), "0");
    assert_eq!(text("primeCount(2)"), "1");
    assert_eq!(text("primeCount(10)"), "4");
    assert_eq!(text("primeCount(100)"), "25");
    assert_eq!(text("primeCount(1000)"), "168");
}

#[test]
fn fibonacci_numbers() {
    assert_eq!(text("fibonacci(0)"), "0");
    assert_eq!(text("fibonacci(1)"), "1");
    assert_eq!(text("fibonacci(2)"), "1");
    assert_eq!(text("fibonacci(10)"), "55");
    assert_eq!(text("fibonacci(20)"), "6765");
    assert_eq!(text("fibonacci(30)"), "832040");
}

#[test]
fn perfect_squares() {
    assert_eq!(text("isPerfectSquare(0)"), "true");
    assert_eq!(text("isPerfectSquare(1)"), "true");
    assert_eq!(text("isPerfectSquare(144)"), "true");
    assert_eq!(text("isPerfectSquare(145)"), "false");
    assert_eq!(text("isPerfectSquare(-4)"), "false");
}

#[test]
fn modular_inverses_and_jacobi_symbols() {
    assert_eq!(text("modInverse(3,11)"), "4");
    assert_eq!(text("modInverse(2,5)"), "3");
    assert_eq!(text("modInverse(10,17)"), "12");
    assert!(text("modInverse(2,4)").contains("modInverse"));
    assert_eq!(text("jacobi(1,3)"), "1");
    assert_eq!(text("jacobi(2,15)"), "1");
    assert_eq!(text("jacobi(3,15)"), "0");
    assert!(text("jacobi(2,4)").contains("jacobi"));
}
