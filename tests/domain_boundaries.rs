use mithqal::Session;

fn eval(modules: &[&str], source: &str) -> String {
    let mut session = Session::with_modules(modules);
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn number_theory_policy_is_library_code() {
    assert_eq!(eval(&["numbertheory"], "totient(12)"), "4");
    assert_eq!(eval(&["numbertheory"], "divisors(12)"), "[1,2,3,4,6,12]");
    assert_eq!(eval(&["numbertheory"], "fibonacci(12)"), "144");
    assert_eq!(eval(&["numbertheory"], "nextPrime(12)"), "13");
}

#[test]
fn algebra_and_calculus_policy_is_library_code() {
    assert_eq!(eval(&["algebra"], "factor(x^2-1)"), "(-1+x)*(1+x)");
    assert_eq!(eval(&["calculus"], "differentiate(x^2,x)"), "2*x");
    assert_eq!(eval(&["calculus"], "integrate(x^2,0,5)"), "125/3");
}

#[test]
fn linear_algebra_policy_is_library_code() {
    assert_eq!(
        eval(&["linearalgebra"], "matmul([[1,2],[3,4]],[[5,6],[7,8]])"),
        "[[19,22],[43,50]]"
    );
    assert_eq!(eval(&["linearalgebra"], "det([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["linearalgebra"], "trace(identity(3))"), "3");
}

#[test]
fn algebraic_and_analytic_number_theory_policy_is_library_code() {
    assert_eq!(
        eval(&["algebraicnumbertheory"], "quadraticClassNumber(-23)"),
        "3"
    );
    assert_eq!(
        eval(&["algebraicnumbertheory"], "quadraticFundamentalUnit(3)"),
        "[2,1]"
    );
    assert_eq!(eval(&["analyticnumbertheory"], "summatoryMobius(10)"), "-1");
}

#[test]
fn kernel_accelerators_remain_available() {
    assert_eq!(
        eval(&["numbertheory"], "factorInteger(12)"),
        "[[2,2],[3,1]]"
    );
    assert_eq!(eval(&["linearalgebra"], "det([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["algebra"], "kernelExpand((x+1)^2)"), "1+x^2+2*x");
}
