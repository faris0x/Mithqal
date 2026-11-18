use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

const CORRELATED: &str = "distribution(setOf([[[0,0],1/2],[[1,1],1/2]]))";
const PRODUCT: &str = "productDistribution(uniform([0,1]),uniform([0,1]))";

#[test]
fn log2_is_exact_for_powers_of_two() {
    assert_eq!(show("log2(1/4)"), "-2");
    assert_eq!(show("log2(1/2)"), "-1");
    assert_eq!(show("log2(1)"), "0");
    assert_eq!(show("log2(2)"), "1");
    assert_eq!(show("log2(8)"), "3");
}

#[test]
fn dyadic_entropy_is_exact() {
    assert_eq!(show("entropy(uniform([1,2]))"), "1");
    assert_eq!(show("entropy(uniform([1,2,3,4]))"), "2");
    assert_eq!(show(&format!("entropy({CORRELATED})")), "1");
}

#[test]
fn non_dyadic_entropy_is_symbolic_and_approximable() {
    assert_eq!(show("log2(1/3)"), "ln(1/3)/ln(2)");
    let approximation = show("approx(entropy(uniform([1,2,3])),6)");
    assert!(
        approximation.starts_with("1.5849"),
        "approximation: {approximation}"
    );
}

#[test]
fn marginals_and_mutual_information() {
    assert_eq!(
        show(&format!("marginalLeft({PRODUCT})")),
        "[[0,1/2],[1,1/2]]"
    );
    assert_eq!(show(&format!("mutualInformation({PRODUCT})")), "0");
    assert_eq!(
        show(&format!(
            "conditionalEntropy({PRODUCT},fn(outcome)=>nth(outcome,0))"
        )),
        "1"
    );
    assert_eq!(show(&format!("mutualInformation({CORRELATED})")), "1");
    assert_eq!(
        show(&format!(
            "conditionalEntropy({CORRELATED},fn(outcome)=>nth(outcome,0))"
        )),
        "0"
    );
}

#[test]
fn divergences_are_exact_when_the_ratios_are_dyadic() {
    assert_eq!(show("klDivergence(uniform([1,2]),uniform([1,2,3,4]))"), "1");
    assert_eq!(show("crossEntropy(uniform([1,2]),uniform([1,2,3,4]))"), "2");
    assert_eq!(show("klDivergence(uniform([1,2]),uniform([1,2]))"), "0");
    assert_eq!(
        show("klDivergence(uniform([1,2]),distribution(setOf([[1,1]])))"),
        "undefined"
    );
}
