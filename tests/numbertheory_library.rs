use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn classical_number_theory_layer_is_available() {
    let mut session = Session::with_modules(&["numbertheory"]);
    assert_eq!(eval(&mut session, "quotient(17,5)"), "3");
    assert_eq!(eval(&mut session, "remainder(17,5)"), "2");
    assert_eq!(eval(&mut session, "extendedGcd(240,46)"), "[2,-9,47]");
    assert_eq!(eval(&mut session, "crt(2,3,3,5)"), "[8,15]");
    assert_eq!(eval(&mut session, "carmichaelLambda(360)"), "12");
    assert_eq!(eval(&mut session, "divisorSigma(2,12)"), "210");
    assert_eq!(eval(&mut session, "unitaryDivisors(12)"), "[1,3,4,12]");
}

#[test]
fn modular_and_prime_functions_work() {
    let mut session = Session::with_modules(&["numbertheory"]);
    assert_eq!(eval(&mut session, "multiplicativeOrder(2,7)"), "3");
    assert_eq!(eval(&mut session, "primitiveRoot(7)"), "3");
    assert_eq!(eval(&mut session, "discreteLog(3,6,7)"), "3");
    assert_eq!(eval(&mut session, "legendreSymbol(2,7)"), "1");
    assert_eq!(eval(&mut session, "nthPrime(10)"), "29");
    assert_eq!(eval(&mut session, "primeGap(10)"), "1");
    assert_eq!(eval(&mut session, "isTwinPrime(11)"), "true");
    assert_eq!(eval(&mut session, "isPrimePower(27)"), "true");
}

#[test]
fn sequences_and_diophantine_helpers_work() {
    let mut session = Session::with_modules(&["numbertheory"]);
    assert_eq!(eval(&mut session, "lucasNumber(10)"), "123");
    assert_eq!(eval(&mut session, "tribonacci(10)"), "81");
    assert_eq!(eval(&mut session, "bellNumber(5)"), "52");
    assert_eq!(eval(&mut session, "partitionNumber(10)"), "42");
    assert_eq!(eval(&mut session, "harmonicNumber(5)"), "137/60");
    assert_eq!(
        eval(&mut session, "pythagoreanTriples(20)"),
        "[[3,4,5],[5,12,13],[8,15,17]]"
    );
    assert_eq!(eval(&mut session, "sumOfTwoSquares(25)"), "[0,5]");
    assert_eq!(eval(&mut session, "continuedFraction(415,93)"), "[4,2,6,7]");
    assert_eq!(
        eval(&mut session, "convergents([4,2,6,7])"),
        "[4,9/2,58/13,415/93]"
    );
    assert_eq!(eval(&mut session, "gaussianNorm(3,4)"), "25");
    assert_eq!(eval(&mut session, "gaussianIsPrime(3,4)"), "false");
}

#[test]
fn number_theory_is_selectable_without_unrelated_domains() {
    let mut session = Session::with_modules(&["numbertheory"]);
    assert_eq!(eval(&mut session, "carmichaelLambda(360)"), "12");
    assert_eq!(eval(&mut session, "mean([1,2,3])"), "mean([1,2,3])");
}
