use mithqal::{Config, ErrorCode, ErrorKind, ExprView, Session};

#[test]
fn new_session_has_no_result() {
    let session = Session::new();
    assert_eq!(session.last_result(), None);
}

#[test]
fn default_config_values() {
    let config = Config::default();
    assert!(config.max_steps > 0);
    assert!(config.max_depth > 0);
    assert!(config.initial_arena_bytes > 0);
    assert!(config.gc_threshold_bytes > 0);
}

#[test]
fn evaluated_values_become_the_last_result() {
    let mut session = Session::new();
    let value = session.eval_statement("1+2").unwrap();
    assert_eq!(session.last_result(), Some(value));
    assert_eq!(session.format_infix(value), "3");
    let next = session.eval_statement("x+x").unwrap();
    assert_eq!(session.last_result(), Some(next));
    assert_eq!(session.format_infix(next), "2*x");
}

#[test]
fn parsed_handles_can_be_evaluated_later() {
    let mut session = Session::new();
    let parsed = session.parse_statement("f(1+2)").unwrap();
    assert_eq!(session.format(parsed), "f(plus(1,2))");
    let value = session.eval(parsed).unwrap();
    assert_eq!(session.format_infix(value), "f(3)");
}

#[test]
fn eval_source_returns_the_last_statement() {
    let mut session = Session::new();
    let value = session.eval_source("1+2\n3+4\n").unwrap();
    assert_eq!(session.format_infix(value), "7");
}

#[test]
fn eval_source_handles_comments() {
    let mut session = Session::new();
    let value = session
        .eval_source("(* first *)\n1+2\n(* second *)\n3+4\n")
        .unwrap();
    assert_eq!(session.format_infix(value), "7");
}

#[test]
fn eval_source_reports_parse_errors() {
    let mut session = Session::new();
    let error = session.eval_source("1 + 2").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseWhitespace);
}

#[test]
fn eval_source_stops_at_the_first_evaluation_error() {
    let mut session = Session::new();
    let error = session.eval_source("1+2\n[1,2]+[3]\n4+5\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
    let last = session.last_result().unwrap();
    assert_eq!(session.format_infix(last), "3");
}

#[test]
fn empty_source_is_rejected() {
    let mut session = Session::new();
    let error = session.eval_source("\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.kind, ErrorKind::Parse);
}

#[test]
fn parse_source_returns_one_handle_per_statement() {
    let mut session = Session::new();
    let statements = session.parse_source("1\n2+3\n4+5+6\n").unwrap();
    assert_eq!(statements.len(), 3);
    assert_eq!(session.format(statements[0]), "1");
    assert_eq!(session.format(statements[1]), "plus(2,3)");
}

#[test]
fn configured_budget_stops_evaluation() {
    let config = Config {
        max_steps: 1,
        max_depth: 4,
        ..Config::default()
    };
    let mut session = Session::with_config(config);
    let error = session.eval_statement("1+2").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}

#[test]
fn collect_garbage_accepts_explicit_roots() {
    let mut session = Session::new();
    let one = session.eval_statement("1").unwrap();
    let two = session.eval_statement("2").unwrap();
    session.collect_garbage(&[one, two]);
    assert_eq!(session.format_infix(one), "1");
    assert_eq!(session.format_infix(two), "2");
}

#[test]
fn views_cover_lists_and_calls() {
    let mut session = Session::new();
    let list = session.eval_statement("[1,2]").unwrap();
    match session.view(list) {
        ExprView::Call { head, args } => {
            assert_eq!(args.len(), 2);
            match session.view(head) {
                ExprView::Symbol(name) => assert_eq!(name, "list"),
                _ => panic!("expected a symbol head"),
            }
        }
        _ => panic!("expected a call"),
    }
}

#[test]
fn diagnostic_display_contains_code_and_location() {
    let mut session = Session::new();
    let error = session.eval_statement("1 + 2").unwrap_err();
    let text = format!("{error}");
    assert!(text.contains("TGN-PARSE-0001"));
    assert!(text.contains("1:2"));
}

#[test]
fn error_code_kinds_are_stable() {
    assert_eq!(ErrorCode::ParseWhitespace.kind(), ErrorKind::Parse);
    assert_eq!(ErrorCode::EvalBudget.kind(), ErrorKind::Eval);
    assert_eq!(ErrorCode::NumericFailure.kind(), ErrorKind::Numeric);
    assert_eq!(ErrorCode::InternalInvariant.kind(), ErrorKind::Internal);
    assert_eq!(ErrorCode::ParseWhitespace.as_str(), "TGN-PARSE-0001");
}

#[test]
fn parse_cache_returns_the_same_handle() {
    let mut session = Session::new();
    let first = session.parse_statement("x+y").unwrap();
    let second = session.parse_statement("x+y").unwrap();
    assert_eq!(first, second);
}

#[test]
fn caches_are_cleared_after_collection() {
    let mut session = Session::new();
    let first = session.eval_statement("x+y").unwrap();
    session.collect_garbage(&[first]);
    let second = session.eval_statement("x+y").unwrap();
    assert_eq!(first, second);
}

#[test]
fn parse_statement_rejects_two_statements_in_a_session() {
    let mut session = Session::new();
    let error = session.parse_statement("1\n2\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}
