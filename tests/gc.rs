use cas::Session;

#[test]
fn collection_reclaims_unrooted_expressions() {
    let mut session = Session::new();
    for value in 0..100 {
        session
            .eval_statement(&format!("{}+{}", value, value + 1))
            .unwrap();
    }
    let before = session.live_expressions();
    session.collect_garbage(&[]);
    let after = session.live_expressions();
    assert!(after < before, "before {before}, after {after}");
    assert!(after > 0);
}

#[test]
fn collection_keeps_the_last_result() {
    let mut session = Session::new();
    let value = session.eval_statement("1+2").unwrap();
    session.collect_garbage(&[]);
    assert_eq!(session.last_result(), Some(value));
    assert_eq!(session.format_infix(value), "3");
}

#[test]
fn collection_keeps_definitions() {
    let mut session = Session::new();
    session.eval_statement("f(x_):=x^2").unwrap();
    for value in 0..20 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
    }
    session.collect_garbage(&[]);
    let result = session.eval_statement("f(4)").unwrap();
    assert_eq!(session.format_infix(result), "16");
}

#[test]
fn collection_keeps_explicit_roots() {
    let mut session = Session::new();
    let first = session.eval_statement("a+b").unwrap();
    for value in 0..50 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
    }
    session.collect_garbage(&[first]);
    assert_eq!(session.format_infix(first), "a+b");
}

#[test]
fn collection_keeps_symbol_nodes() {
    let mut session = Session::new();
    session.eval_statement("x").unwrap();
    session.collect_garbage(&[]);
    let value = session.eval_statement("x").unwrap();
    assert_eq!(session.format_infix(value), "x");
}

#[test]
fn interning_still_works_after_collection() {
    let mut session = Session::new();
    let first = session.eval_statement("x+y").unwrap();
    session.collect_garbage(&[first]);
    let second = session.eval_statement("x+y").unwrap();
    assert_eq!(first, second);
}

#[test]
fn repeated_collection_keeps_memory_bounded() {
    let mut session = Session::new();
    for value in 0..2000 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
        if value % 50 == 0 {
            session.collect_garbage(&[]);
        }
    }
    session.collect_garbage(&[]);
    assert!(
        session.live_expressions() < 100,
        "live expressions {}",
        session.live_expressions()
    );
}

#[test]
fn collection_reuses_freed_slots() {
    let mut session = Session::new();
    for value in 0..200 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
    }
    session.collect_garbage(&[]);
    let low = session.live_expressions();
    for value in 200..400 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
    }
    session.collect_garbage(&[]);
    assert!(
        session.live_expressions() <= low + 20,
        "low {low}, now {}",
        session.live_expressions()
    );
}

#[test]
fn collection_of_empty_session_is_safe() {
    let mut session = Session::new();
    session.collect_garbage(&[]);
    assert_eq!(session.live_expressions(), 0);
}
