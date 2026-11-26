use mithqal::{ErrorCode, Session};

fn output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    session.output().to_vec()
}

#[test]
fn body_definitions_are_local_and_capture_parameters() {
    let source = r#"
module local.
we define outer(x) = let helper(y) = x + y in helper(2).
print outer(10).
print helper(2).
end module.
"#;
    assert_eq!(output(source), vec!["12", "helper(2)"]);
}

#[test]
fn body_definitions_can_recurse() {
    let source = r#"
module local.
we define fact(n) = let loop(0, acc) = acc, loop(k, acc) = loop(k - 1, acc * k) in loop(n, 1).
print fact(5).
print fact(7).
end module.
"#;
    assert_eq!(output(source), vec!["120", "5040"]);
}

#[test]
fn body_definitions_can_be_mutually_recursive() {
    let source = r#"
module local.
we define parity(n) = let even(0) = true, even(k) = odd(k - 1), odd(0) = false, odd(k) = even(k - 1) in even(n).
print parity(10).
print parity(7).
end module.
"#;
    assert_eq!(output(source), vec!["true", "false"]);
}

#[test]
fn body_definitions_shadow_globals_only_inside_the_body() {
    let source = r#"
module local.
we define f(x) = x + 1.
we define outer(x) = let f(y) = y + 100 in f(x).
print outer(1).
print f(1).
end module.
"#;
    assert_eq!(output(source), vec!["101", "2"]);
}

#[test]
fn local_definitions_in_conditional_blocks_are_scoped() {
    let source = r#"
module local.
let x be 5.
if x > 0 then:
  we define positive(y) = y > 0.
  print positive(x).
end if.
print positive(5).
end module.
"#;
    assert_eq!(output(source), vec!["true", "positive(5)"]);
}

#[test]
fn local_definitions_cannot_shadow_protected_symbols() {
    let source = r#"
module local.
we define outer(x) = let plus(a, b) = a + b in x.
print outer(1).
end module.
"#;
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn local_definitions_report_arity_errors() {
    let mut session = Session::new();
    session
        .eval_source("module local.\nwe define outer(x) = let helper(a, b) = a + b in helper(x).\nend module.")
        .unwrap();
    let error = session.eval_statement("outer(1)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
}

#[test]
fn local_definitions_do_not_leak_between_calls() {
    let mut session = Session::new();
    session
        .eval_source(
            "module local.\nwe define outer(x) = let helper(y) = y + 1 in helper(x).\nend module.",
        )
        .unwrap();
    let value = session.eval_statement("outer(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "2");
    let value = session.eval_statement("helper(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "helper(1)");
}
