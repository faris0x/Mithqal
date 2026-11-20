use mithqal::Session;

fn text(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

fn defined(source: &str) -> Session {
    let mut session = Session::with_modules(&[]);
    session.eval_source(source).unwrap();
    session
}

#[test]
fn substitution_does_not_capture_closure_parameters() {
    let mut session = defined("f(k_):=fn(y)=>k+y\n");
    assert_eq!(text(&mut session, "apply(f(y),[2])"), "2+y");
    assert_eq!(text(&mut session, "apply(f(3),[2])"), "5");
}

#[test]
fn nested_closures_avoid_capture_at_each_binder() {
    let mut session = defined("g(k_):=fn(x)=>fn(y)=>k+x+y\n");
    assert_eq!(text(&mut session, "apply(apply(g(x),[1]),[2])"), "3+x");
}

#[test]
fn let_bindings_shadow_rule_variables() {
    let mut session = defined("shadow(k_):=let([k=1],k)\n");
    assert_eq!(text(&mut session, "shadow(5)"), "1");
}

#[test]
fn let_bindings_do_not_capture_earlier_values() {
    let mut session = defined("later(k_):=let([a=10,b=k],a+b)\n");
    assert_eq!(text(&mut session, "later(a)"), "10+a");
}

#[test]
fn substitution_leaves_unrelated_closures_alone() {
    let mut session = defined("h(k_):=fn(z)=>k+z\n");
    assert_eq!(text(&mut session, "apply(h(5),[1])"), "6");
}

#[test]
fn sequence_bindings_still_splice() {
    let mut session = defined("s(x__):=list(x)\n");
    assert_eq!(text(&mut session, "s(1,2,3)"), "[1,2,3]");
    assert_eq!(text(&mut session, "s(1)"), "[1]");
}

#[test]
fn rule_construction_avoids_capture() {
    let mut session = defined("mk(v_):=rule(x_,v)\n");
    let value = session.eval_statement("mk(x)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "x1_->x");

    let mut session = defined("mk(v_):=rule(x_,v)\n");
    let value = session.eval_statement("mk(5)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "x_->5");
}

#[test]
fn rule_construction_respects_pattern_binders() {
    let mut session = defined("shadowRule(v_):=rule(v_,v)\n");
    let value = session.eval_statement("shadowRule(5)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "v_->v");
}

#[test]
fn rule_conditions_are_renamed_with_their_binder() {
    let mut session = defined("guarded(v_):=rule(condition(x_,x>v),v)\n");
    let value = session.eval_statement("guarded(x)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "when(x1_,x1>x)->x");
}
