use mithqal::{ExprId, Session};

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format(value).unwrap()
}

fn eval_value(session: &mut Session, source: &str) -> ExprId {
    session.eval_statement(source).unwrap()
}

#[test]
fn begin_and_end_change_the_current_context() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "$Context"), "\"Global\"");
    assert_eq!(eval(&mut session, "begin(foo)"), "\"foo\"");
    assert_eq!(eval(&mut session, "$Context"), "\"foo\"");
    assert_eq!(eval(&mut session, "begin(bar)"), "\"bar\"");
    assert_eq!(eval(&mut session, "$Context"), "\"bar\"");
    assert_eq!(eval(&mut session, "end()"), "\"bar\"");
    assert_eq!(eval(&mut session, "$Context"), "\"foo\"");
    assert_eq!(eval(&mut session, "end()"), "\"foo\"");
    assert_eq!(eval(&mut session, "$Context"), "\"Global\"");
    assert!(session.eval_statement("end()").is_err());
}

#[test]
fn short_names_are_created_in_the_current_context() {
    let mut session = Session::new();
    eval(&mut session, "begin(foo)");
    assert_eq!(eval(&mut session, "phase1LocalName=5"), "5");
    assert_eq!(eval(&mut session, "phase1LocalName"), "5");
    assert_eq!(eval(&mut session, "foo;phase1LocalName"), "5");
    assert_eq!(eval(&mut session, "context(phase1LocalName)"), "\"foo\"");
    assert_eq!(eval(&mut session, "end()"), "\"foo\"");
    assert_eq!(eval(&mut session, "foo;phase1LocalName"), "5");
}

#[test]
fn qualified_names_bypass_the_context_path() {
    let mut session = Session::new();
    eval(&mut session, "foo;x=5");
    eval(&mut session, "bar;x=7");
    assert_eq!(eval(&mut session, "foo;x"), "5");
    assert_eq!(eval(&mut session, "bar;x"), "7");
    assert_eq!(eval(&mut session, "context(foo;x)"), "\"foo\"");
    assert_eq!(eval(&mut session, "context(bar;x)"), "\"bar\"");
}

#[test]
fn canonical_names_keep_their_context() {
    let mut session = Session::new();
    let symbol = session.parse_statement("foo;x").unwrap();
    assert_eq!(session.format(symbol).unwrap(), "foo;x");
    eval(&mut session, "begin(foo)");
    assert_eq!(session.format_infix(symbol).unwrap(), "x");
}

#[test]
fn nested_qualified_names_use_the_leading_segments_as_context() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "a;b;c=1"), "1");
    assert_eq!(eval(&mut session, "a;b;c"), "1");
    assert_eq!(eval(&mut session, "context(a;b;c)"), "\"a;b\"");
    assert_eq!(eval(&mut session, "$Context"), "\"Global\"");
}

#[test]
fn context_introspection_lists_known_contexts() {
    let mut session = Session::new();
    eval(&mut session, "begin(foo)");
    eval(&mut session, "end()");
    eval(&mut session, "begin(bar)");
    eval(&mut session, "end()");
    assert_eq!(eval(&mut session, "contains(contexts(),\"bar\")"), "true");
    assert_eq!(eval(&mut session, "contains(contexts(),\"foo\")"), "true");
    assert_eq!(eval(&mut session, "$ContextPath"), "list(\"Global\")");
}

#[test]
fn context_changes_invalidate_parsed_and_evaluated_results() {
    let mut session = Session::new();
    let first = eval_value(&mut session, "$Context");
    assert_eq!(session.format(first).unwrap(), "\"Global\"");
    eval(&mut session, "begin(foo)");
    let second = eval_value(&mut session, "$Context");
    assert_eq!(session.format(second).unwrap(), "\"foo\"");
    assert_ne!(first, second);
}

#[test]
fn context_names_are_validated() {
    let mut session = Session::new();
    assert!(session.eval_statement("begin(1)").is_err());
    assert!(session.eval_statement("begin(a;;b)").is_err());
    assert!(session.eval_statement("begin(a;)").is_err());
}
