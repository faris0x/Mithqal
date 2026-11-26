use mithqal::{ErrorCode, Session};

fn output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    session.output().to_vec()
}

#[test]
fn block_bodies_bind_intermediates() {
    let source = r#"
module body.
we define blocksVar(xs) = let n = length(xs) in let s = fold(fn(a, b) => a + b, 0, xs) in let q = fold(fn(a, b) => a + b, 0, map(xs, fn(x) => x * x)) in (n * q - s * s) / (n * n).
print blocksVar([1, 2, 3, 4]).
end module.
"#;
    assert_eq!(output(source), vec!["5/4"]);
}

#[test]
fn block_bodies_allow_conditionals() {
    let source = r#"
module body.
we define classify(x) = if(x < 0, -1, if(x == 0, 0, 1)).
print classify(-5).
print classify(0).
print classify(5).
end module.
"#;
    assert_eq!(output(source), vec!["-1", "0", "1"]);
}

#[test]
fn block_bodies_mix_let_and_if() {
    let source = r#"
module body.
we define absdiff(x, y) = let d = x - y in if(d < 0, -d, d).
print absdiff(3, 7).
print absdiff(7, 3).
end module.
"#;
    assert_eq!(output(source), vec!["4", "4"]);
}

#[test]
fn sequence_parameters_work_in_definitions() {
    let source = r#"
module patterns.
we define s(x...) = list(x).
print s(1, 2, 3).
print s(1).
end module.
"#;
    assert_eq!(output(source), vec!["[1,2,3]", "[1]"]);
}

#[test]
fn list_destructuring_parameters_bind_their_names() {
    let source = r#"
module patterns.
let a be 99.
we define head2([a, b]) = a.
print head2([7, 8]).
end module.
"#;
    assert_eq!(output(source), vec!["7"]);
}

#[test]
fn fixed_arity_functions_report_arity_errors() {
    let mut session = Session::new();
    session
        .eval_source("module m.\nwe define f(x) = x.\nend module.")
        .unwrap();
    let value = session.eval_statement("f(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "1");
    for source in ["f()", "f(1,2)"] {
        let mut session = Session::new();
        session
            .eval_source("module m.\nwe define f(x) = x.\nend module.")
            .unwrap();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn variadic_functions_keep_unmatched_calls_unevaluated() {
    let mut session = Session::new();
    session
        .eval_source("module m.\nwe define s(x...) = list(x).\nend module.")
        .unwrap();
    let value = session.eval_statement("s(1,2)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "[1,2]");
    let value = session.eval_statement("s()").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "s()");
}
