use mithqal::{Config, ErrorCode, ExprView, Session};

#[test]
fn session_evaluates_and_formats() {
    let mut session = Session::new();
    let value = session.eval_statement("2/3+1/6").unwrap();
    assert_eq!(session.format_infix(value), "5/6");
    assert_eq!(session.format(value), "5/6");
    assert_eq!(session.last_result(), Some(value));
}

#[test]
fn source_statements_use_newlines_and_comments() {
    let mut session = Session::new();
    let source = "(* first *)\n1+2\n(* second *)\n3+4\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value), "7");
}

#[test]
fn views_expose_expression_shape() {
    let mut session = Session::new();
    let integer = session.eval_statement("42").unwrap();
    match session.view(integer) {
        ExprView::Integer(value) => assert_eq!(value.to_decimal(), "42"),
        _ => panic!("expected an integer"),
    }
    let rational = session.eval_statement("1/2").unwrap();
    match session.view(rational) {
        ExprView::Rational(value) => assert_eq!(value.to_decimal(), "1/2"),
        _ => panic!("expected a rational"),
    }
    let symbol = session.parse_statement("x").unwrap();
    match session.view(symbol) {
        ExprView::Symbol(name) => assert_eq!(name, "x"),
        _ => panic!("expected a symbol"),
    }
    let call = session.parse_statement("f(x)").unwrap();
    match session.view(call) {
        ExprView::Call { head, args } => {
            assert_eq!(args.len(), 1);
            match session.view(head) {
                ExprView::Symbol(name) => assert_eq!(name, "f"),
                _ => panic!("expected a symbol head"),
            }
        }
        _ => panic!("expected a call"),
    }
}

#[test]
fn empty_source_is_an_error() {
    let mut session = Session::new();
    let error = session.eval_source("\n\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn configured_budget_is_enforced() {
    let config = Config {
        max_steps: 1,
        max_depth: 8,
        ..Config::default()
    };
    let mut session = Session::with_config(config);
    let error = session.eval_statement("1+2").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}

#[test]
fn collect_garbage_keeps_working() {
    let mut session = Session::new();
    let value = session.eval_statement("1+2").unwrap();
    session.collect_garbage(&[value]);
    assert_eq!(session.format_infix(value), "3");
}
