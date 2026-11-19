use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn tagged_definitions_install_up_values() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "tagSet(f,g(f(x_)),x+1)"), "1+x");
    assert_eq!(eval(&mut session, "g(f(1))"), "2");
    assert_eq!(eval(&mut session, "upValues(f)"), "[g(f(x_)):>1+x]");
}

#[test]
fn delayed_tagged_definitions_hold_the_body() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "tagSetDelayed(f,h(f(x_)),x*2)"), "null");
    assert_eq!(eval(&mut session, "h(f(3))"), "6");
}

#[test]
fn up_set_uses_the_left_hand_head_as_tag() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "upSet(g(f(x_)),x+10)"), "10+x");
    assert_eq!(eval(&mut session, "g(f(2))"), "12");
}

#[test]
fn up_values_precede_down_values() {
    let mut session = Session::new();
    assert_eq!(
        eval(&mut session, "tagSet(f,g(f(x_)),1)\ng(x_):=2\ng(f(3))"),
        "1"
    );
}

#[test]
fn up_values_are_removed_by_clear_all() {
    let mut session = Session::new();
    assert_eq!(
        eval(
            &mut session,
            "tagSet(f,g(f(x_)),1)\nclearAll(f)\nupValues(f)"
        ),
        "[]"
    );
    assert_eq!(eval(&mut session, "g(f(2))"), "g(f(2))");
}

#[test]
fn protected_tags_reject_up_values() {
    let mut session = Session::new();
    assert!(
        session
            .eval_statement("tagSet(plus,g(plus(x_)),1)")
            .is_err()
    );
    assert!(session.eval_statement("upSet(plus(x_),1)").is_err());
}
