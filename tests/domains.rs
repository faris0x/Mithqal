use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn combinatorial_counts_are_exact() {
    assert_eq!(show("permutationCount(5,2)"), "20");
    assert_eq!(show("multinomial([2,1])"), "3");
    assert_eq!(show("multinomial([1,1,1])"), "6");
    assert_eq!(show("catalan(5)"), "42");
    assert_eq!(show("derangementCount(4)"), "9");
    assert_eq!(show("stirlingSecond(4,2)"), "7");
    assert_eq!(show("stirlingSecond(0,0)"), "1");
    assert_eq!(show("stirlingSecond(3,5)"), "0");
}

#[test]
fn finite_enumerations() {
    assert_eq!(
        show("permutationsOf([1,2,3])"),
        "[[1,2,3],[2,1,3],[2,3,1],[1,3,2],[3,1,2],[3,2,1]]"
    );
    assert_eq!(show("length(permutationsOf([1,2,3]))"), "6");
    assert_eq!(show("combinationsOf([1,2,3],2)"), "[[1,2],[1,3],[2,3]]");
    assert_eq!(show("combinationsOf([1,2,3],4)"), "[]");
}

#[test]
fn finite_distributions_are_exact() {
    assert_eq!(
        show("uniform([1,2,3,4])"),
        "[[1,1/4],[2,1/4],[3,1/4],[4,1/4]]"
    );
    assert_eq!(show("isDistribution(uniform([1,2]))"), "true");
    assert_eq!(show("isDistribution(setOf([[1,1/2]]))"), "false");
    assert_eq!(show("isDistribution(setOf([[1,1/2],[1,1/2]]))"), "false");
    assert_eq!(show("support(uniform([1,2]))"), "[1,2]");
}

#[test]
fn probability_expectation_and_variance() {
    assert_eq!(show("probabilityOf(uniform([1,2,3,4]),[1,2])"), "1/2");
    assert_eq!(show("expectation(uniform([1,2,3,4]),fn(x)=>x)"), "5/2");
    assert_eq!(show("varianceOf(uniform([1,2,3,4]),fn(x)=>x)"), "5/4");
    assert_eq!(show("probabilityOf(uniform([1,2,3,4]),[5])"), "0");
}

#[test]
fn conditioning_renormalizes() {
    assert_eq!(
        show("conditional(uniform([1,2,3,4]),[1,2])"),
        "[[1,1/2],[2,1/2]]"
    );
    assert_eq!(show("conditional(uniform([1,2]),[3])"), "undefined");
    assert_eq!(
        show("probabilityOf(conditional(uniform([1,2,3,4]),[1,2]),[1])"),
        "1/2"
    );
}

#[test]
fn product_distributions_are_independent() {
    assert_eq!(
        show("productDistribution(uniform([1,2]),uniform([3,4]))"),
        "[[[1,3],1/4],[[1,4],1/4],[[2,3],1/4],[[2,4],1/4]]"
    );
    assert_eq!(
        show(
            "expectation(productDistribution(uniform([1,2]),uniform([3,4])),fn(pair)=>nth(pair,0)*nth(pair,1))"
        ),
        "21/4"
    );
}
