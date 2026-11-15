use mithqal::Session;

fn nested_call(depth: usize) -> String {
    let mut source = String::from("1");
    for _ in 0..depth {
        source = format!("f({source})");
    }
    source
}

fn on_large_stack(action: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(action)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deeply_nested_calls_parse_evaluate_and_format() {
    on_large_stack(|| {
        let source = nested_call(1024);
        let mut session = Session::new();
        let value = session.eval_statement(&source).unwrap();
        let text = session.format_infix(value).unwrap();
        assert_eq!(text.matches("f(").count(), 1024);
    });
}

#[test]
fn deeply_nested_negations_parse_and_evaluate() {
    on_large_stack(|| {
        let mut source = String::from("x");
        for _ in 0..512 {
            source = format!("-({source})");
        }
        let mut session = Session::new();
        let value = session.eval_statement(&source).unwrap();
        assert_eq!(session.format_infix(value).unwrap(), "x");
    });
}
