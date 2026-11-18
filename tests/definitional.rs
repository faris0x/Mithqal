use mithqal::{ErrorCode, Session};

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn list_helpers() {
    assert_eq!(show("append([1,2],[3])"), "[1,2,3]");
    assert_eq!(show("unique([3,1,3,2])"), "[1,2,3]");
    assert_eq!(show("rangeList(4)"), "[0,1,2,3]");
    assert_eq!(show("filter([1,2,3,4],fn(x)=>isEven(x))"), "[2,4]");
    assert_eq!(show("concat([[1,2],[3]])"), "[1,2,3]");
}

#[test]
fn finite_sets_are_canonical_lists() {
    assert_eq!(show("setOf([3,1,3,2])"), "[1,2,3]");
    assert_eq!(show("emptySet()"), "[]");
    assert_eq!(show("singleton(4)"), "[4]");
    assert_eq!(show("union([1,2],[2,3])"), "[1,2,3]");
    assert_eq!(show("intersection([1,2,3],[2,3,4])"), "[2,3]");
    assert_eq!(show("difference([1,2,3],[2])"), "[1,3]");
    assert_eq!(show("member([1,2],2)"), "true");
    assert_eq!(show("member([1,2],3)"), "false");
    assert_eq!(show("subset([1,2],[1,2,3])"), "true");
    assert_eq!(show("subset([1,4],[1,2,3])"), "false");
    assert_eq!(show("cardinality([1,2,3])"), "3");
    assert_eq!(show("isEmpty([])"), "true");
    assert_eq!(show("powerSet([1,2])"), "[[],[1],[2],[1,2]]");
    assert_eq!(show("cartesian([1,2],[3,4])"), "[[1,3],[1,4],[2,3],[2,4]]");
}

#[test]
fn relations_and_functions() {
    assert_eq!(show("domainOf(relationOf([[1,2],[2,3]]))"), "[1,2]");
    assert_eq!(show("rangeOf(relationOf([[1,2],[2,3]]))"), "[2,3]");
    assert_eq!(show("imageOf(relationOf([[1,2],[2,3]]),[1])"), "[2]");
    assert_eq!(
        show("composeRelations(relationOf([[1,2]]),relationOf([[2,3]]))"),
        "[[1,3]]"
    );
    assert_eq!(show("inverseRelation(relationOf([[1,2]]))"), "[[2,1]]");
    assert_eq!(show("isFunction(relationOf([[1,2],[2,3]]))"), "true");
    assert_eq!(show("isFunction(relationOf([[1,2],[1,3]]))"), "false");
    assert_eq!(show("applyRelation(relationOf([[1,2]]),1)"), "2");
    assert_eq!(show("isReflexive(relationOf([[1,1],[2,2]]),[1,2])"), "true");
    assert_eq!(show("isSymmetric(relationOf([[1,2],[2,1]]))"), "true");
    assert_eq!(
        show("isTransitive(relationOf([[1,2],[2,3],[1,3]]))"),
        "true"
    );
    assert_eq!(show("isTransitive(relationOf([[1,2],[2,3]]))"), "false");
}

#[test]
fn structures_check_laws_over_finite_carriers() {
    assert_eq!(
        show("monoidHolds(monoid([0,1,2],fn(a,b)=>mod(a+b,3),0))"),
        "true"
    );
    assert_eq!(
        show("monoidHolds(monoid([0,1,2],fn(a,b)=>mod(a-b,3),0))"),
        "false"
    );
    assert_eq!(
        show("monoidHolds(monoid([0,1,2],fn(a,b)=>mod(a+b,3),1))"),
        "false"
    );
    assert_eq!(
        show("groupHolds(group([0,1,2],fn(a,b)=>mod(a+b,3),0,fn(a)=>mod(-a,3)))"),
        "true"
    );
    assert_eq!(
        show(
            "ringHolds(ring([0,1,2],fn(a,b)=>mod(a+b,3),fn(a,b)=>mod(a*b,3),0,1,fn(a)=>mod(-a,3)))"
        ),
        "true"
    );
    assert_eq!(
        show(
            "ringHolds(ring([0,1,2],fn(a,b)=>mod(a+b,3),fn(a,b)=>mod(a+b,3),0,1,fn(a)=>mod(-a,3)))"
        ),
        "false"
    );
}

#[test]
fn peano_laws_and_von_neumann_numerals() {
    assert_eq!(show("checkPeano(4)"), "true");
    assert_eq!(show("checkVonNeumann(3)"), "true");
    assert_eq!(show("vonNeumann(0)"), "[]");
    assert_eq!(show("vonNeumann(3)"), "[[],[[]],[[],[[]]]]");
    assert_eq!(show("vonNeumann(-1)"), "undefined");
}

#[test]
fn private_helpers_are_not_callable() {
    assert_eq!(show("dedupe([1,1])"), "dedupe([1,1])");
    assert_eq!(show("triples([1])"), "triples([1])");
}

#[test]
fn module_exports_are_protected() {
    let mut session = Session::new();
    for source in ["append=1", "setOf=1", "monoid=1"] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{source}");
    }
}
