use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn statistics_only_loads_statistics_and_kernel_primitives() {
    let mut session = Session::with_modules(&["statistics"]);
    assert_eq!(eval(&mut session, "mean([1,2,3])"), "2");
    assert_eq!(
        eval(&mut session, "permutationCount(4,2)"),
        "permutationCount(4,2)"
    );
}

#[test]
fn selecting_a_domain_loads_its_import_closure() {
    let mut session = Session::with_modules(&["combinatorics"]);
    assert_eq!(eval(&mut session, "permutationCount(4,2)"), "12");
    assert_eq!(eval(&mut session, "union([1],[2])"), "[1,2]");
    assert_eq!(eval(&mut session, "mean([1,2,3])"), "mean([1,2,3])");
}

#[test]
#[should_panic]
fn unknown_domain_is_rejected() {
    let _ = Session::with_modules(&["definitely_missing_domain"]);
}
