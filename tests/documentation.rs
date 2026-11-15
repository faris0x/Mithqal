use mithqal::{Config, ErrorCode, ExprView, Session};

fn text(session: &Session, value: mithqal::ExprId) -> String {
    session.format_infix(value).unwrap()
}

#[test]
fn approximate_boundary_and_constants_are_available() {
    let mut session = Session::new();
    for source in ["approx(pi,30)", "approx(e,30)"] {
        let value = session.eval_statement(source).unwrap();
        match session.view(value).unwrap() {
            ExprView::Real(_) => {}
            other => panic!("{source} produced {other:?}"),
        }
    }
}

#[test]
fn automatic_collection_follows_the_configured_node_threshold() {
    let config = Config {
        gc_threshold_nodes: 32,
        ..Config::default()
    };
    let mut session = Session::with_config(config);
    for value in 0..200 {
        session.eval_statement(&format!("{value}+1")).unwrap();
    }
    assert!(session.live_expressions() < 200);
}

#[test]
fn staged_factorisation_advertised_in_the_readme_works() {
    let mut session = Session::new();
    let source = "(x+a+b+c+d+e)*(x+a^2+b^2+c^2+d^2+e^2)";
    let factored = session
        .eval_statement(&format!("factor({source})"))
        .unwrap();
    let expanded = session
        .eval_statement(&format!("expand({})", text(&session, factored)))
        .unwrap();
    let target = session
        .eval_statement(&format!("expand({source})"))
        .unwrap();
    assert_eq!(expanded, target);
}

#[test]
fn stale_handles_are_reported_instead_of_panicking() {
    let mut session = Session::new();
    let stale = session.eval_statement("987654+321").unwrap();
    session.eval_statement("1+1").unwrap();
    session.collect_garbage(&[]);
    assert!(!session.is_live(stale));
    assert_eq!(
        session.view(stale).unwrap_err().code,
        ErrorCode::EvalNotLive
    );
    assert_eq!(
        session.format(stale).unwrap_err().code,
        ErrorCode::EvalNotLive
    );
    assert_eq!(
        session.format_infix(stale).unwrap_err().code,
        ErrorCode::EvalNotLive
    );
    assert_eq!(
        session.eval(stale).unwrap_err().code,
        ErrorCode::EvalNotLive
    );
}
