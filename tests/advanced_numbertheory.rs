use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn analytic_number_theory_domain_loads() {
    let mut session = Session::with_modules(&["analyticnumbertheory"]);
    assert_eq!(eval(&mut session, "bernoulliNumber(6)"), "1/42");
    assert_eq!(eval(&mut session, "bernoulliNumber(10)"), "5/66");
    assert_eq!(eval(&mut session, "vonMangoldt(9)"), "ln(3)");
    assert_eq!(eval(&mut session, "zetaInteger(2,3)"), "49/36");
    assert_eq!(eval(&mut session, "eulerNumber(4)"), "5");
}

#[test]
fn elliptic_curve_domain_loads() {
    let mut session = Session::with_modules(&["ellipticcurves"]);
    assert_eq!(eval(&mut session, "ellipticDiscriminant(1,1)"), "-496");
    assert_eq!(eval(&mut session, "ellipticJInvariant(1,1)"), "6912/31");
    assert_eq!(eval(&mut session, "ellipticPointCount(1,1,5)"), "9");
    assert_eq!(
        eval(&mut session, "ellipticAdd(1,[0,1],[0,1])"),
        "[1/4,-9/8]"
    );
}

#[test]
fn algebraic_and_padic_domains_load() {
    let mut session = Session::with_modules(&["algebraicnumbertheory"]);
    assert_eq!(eval(&mut session, "quadraticNorm(1,2,5)"), "-19");
    assert_eq!(
        eval(&mut session, "quadraticMultiply(1,2,3,4,5)"),
        "[43,10]"
    );
    assert_eq!(eval(&mut session, "eisensteinNorm(1,2)"), "3");
    assert_eq!(eval(&mut session, "padicValuation(360,3)"), "2");
    assert_eq!(eval(&mut session, "padicNorm(360,3)"), "1/9");
}

#[test]
fn quadratic_form_domain_loads() {
    let mut session = Session::with_modules(&["quadraticforms"]);
    assert_eq!(eval(&mut session, "binaryFormDiscriminant(1,0,1)"), "-4");
    assert_eq!(eval(&mut session, "reducedForms(-20)"), "[[1,0,5],[2,2,3]]");
    assert_eq!(eval(&mut session, "binaryFormClassNumber(-20)"), "2");
}

#[test]
fn cryptographic_number_theory_domain_loads() {
    let mut session = Session::with_modules(&["cryptonumbertheory"]);
    assert_eq!(eval(&mut session, "isCarmichael(561)"), "true");
    assert_eq!(eval(&mut session, "lucasLehmer(7)"), "true");
    assert_eq!(
        eval(&mut session, "rsaDecrypt(rsaEncrypt(65,17,323),17,323)"),
        "65"
    );
    assert_eq!(eval(&mut session, "diffieHellman(5,6,15,23)"), "2");
}
