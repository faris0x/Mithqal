use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn attributes_can_be_set_cleared_and_listed() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "setAttributes(f,orderless)"), "null");
    assert_eq!(eval(&mut session, "attributes(f)"), "[orderless]");
    assert_eq!(eval(&mut session, "clearAttributes(f,orderless)"), "null");
    assert_eq!(eval(&mut session, "attributes(f)"), "[]");
    assert_eq!(eval(&mut session, "setAttributes(f,flat,listable)"), "null");
    assert_eq!(eval(&mut session, "attributes(f)"), "[flat,listable]");
}

#[test]
fn protected_symbols_reject_definitions_until_unprotected() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "protect(f)"), "null");
    assert!(session.eval_statement("f(x_):=x").is_err());
    assert_eq!(eval(&mut session, "unprotect(f)"), "null");
    assert_eq!(eval(&mut session, "f(x_):=x\nf(2)"), "2");
    assert!(session.eval_statement("unprotect(plus)").is_err());
}

#[test]
fn definition_introspection_reports_values() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "x=5\nownValues(x)"), "[x:>5]");
    let _ = session.eval_source("f(x_):=x*x");
    assert_eq!(eval(&mut session, "downValues(f)"), "[f(x_):>x*x]");
    assert_eq!(eval(&mut session, "definition(f)"), "[[],[],[f(x_):>x*x]]");
    assert_eq!(eval(&mut session, "subValues(f)"), "[]");
    assert_eq!(eval(&mut session, "upValues(f)"), "[]");
}

#[test]
fn clear_all_removes_definitions_and_attributes() {
    let mut session = Session::new();
    assert_eq!(
        eval(
            &mut session,
            "setAttributes(f,orderless)\nf(x_):=x\nclearAll(f)\nattributes(f)"
        ),
        "[]"
    );
    assert_eq!(eval(&mut session, "f(2)"), "f(2)");
}
