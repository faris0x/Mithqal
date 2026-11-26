use mithqal::{ErrorCode, Session};

#[test]
fn named_sources_and_lines_appear_in_diagnostics() {
    let source = "module m.\nwe define f(x) = x.\nprint f().\nend module.";
    let mut session = Session::new();
    let error = session.eval_named_source("math.mith", source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
    assert_eq!(error.source.as_deref(), Some("math.mith"));
    let frame = error.trace.first().expect("a trace frame");
    assert_eq!(frame.source.as_deref(), Some("math.mith"));
    assert_eq!(frame.span.expect("a frame span").line, 3);
    assert!(error.to_string().contains("math.mith:3:"));
}

#[test]
fn parse_errors_carry_the_source_name() {
    let mut session = Session::new();
    let error = session
        .eval_named_source("broken.mith", "module m.\nprint 1 +.\nend module.")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.source.as_deref(), Some("broken.mith"));
    assert!(error.to_string().contains("broken.mith:2:"));
}

#[test]
fn nested_blocks_add_outer_trace_frames() {
    let source = "module m.\nwe define f(x) = x.\nif true then:\nprint f().\nend if.\nend module.";
    let mut session = Session::new();
    let error = session
        .eval_named_source("nested.mith", source)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
    let lines: Vec<usize> = error
        .trace
        .iter()
        .filter_map(|frame| frame.span.map(|span| span.line))
        .collect();
    assert!(lines.contains(&4), "inner frame line: {lines:?}");
    assert!(lines.contains(&3), "outer frame line: {lines:?}");
}

#[test]
fn errors_inside_a_definition_point_at_the_definition() {
    let source = "module m.\nwe define plus(x) = x.\nend module.";
    let mut session = Session::new();
    let error = session
        .eval_named_source("definitions.mith", source)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
    assert_eq!(error.source.as_deref(), Some("definitions.mith"));
    assert!(
        error
            .trace
            .iter()
            .any(|frame| frame.span.is_some_and(|span| span.line == 2))
    );
}
