use mithqal::{ErrorCode, Session};

fn output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    session.output().to_vec()
}

#[test]
fn body_definitions_are_local_and_capture_parameters() {
    let source = r#"
mod local
def outer(x)=
def helper(y)=x+y
helper(2)
end
prt outer(10)
prt helper(2)
end
"#;
    assert_eq!(output(source), vec!["12", "helper(2)"]);
}

#[test]
fn body_definitions_can_recurse() {
    let source = r#"
mod local
def fact(n)=
def loop(0,acc)=acc
def loop(k,acc)=loop(k-1,acc*k)
loop(n,1)
end
prt fact(5)
prt fact(7)
end
"#;
    assert_eq!(output(source), vec!["120", "5040"]);
}

#[test]
fn body_definitions_can_be_mutually_recursive() {
    let source = r#"
mod local
def parity(n)=
def even(0)=true
def even(k)=odd(k-1)
def odd(0)=false
def odd(k)=even(k-1)
even(n)
end
prt parity(10)
prt parity(7)
end
"#;
    assert_eq!(output(source), vec!["true", "false"]);
}

#[test]
fn body_definitions_shadow_globals_only_inside_the_body() {
    let source = r#"
mod local
def f(x)=x+1
def outer(x)=
def f(y)=y+100
f(x)
end
prt outer(1)
prt f(1)
end
"#;
    assert_eq!(output(source), vec!["101", "2"]);
}

#[test]
fn local_definitions_in_conditional_blocks_are_scoped() {
    let source = r#"
mod local
def outer(x)=
if x>0 then
def positive(y)=y>0
positive(x)
else
0
end
end
prt outer(5)
prt positive(5)
end
"#;
    assert_eq!(output(source), vec!["true", "positive(5)"]);
}

#[test]
fn local_definitions_cannot_shadow_protected_symbols() {
    let source = r#"
mod local
def outer(x)=
def plus(a,b)=a+b
x
end
prt outer(1)
end
"#;
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn local_definitions_report_arity_errors() {
    let mut session = Session::new();
    session
        .eval_source("mod local\ndef outer(x)=\ndef helper(a,b)=a+b\nhelper(x)\nend\nend\n")
        .unwrap();
    let error = session.eval_statement("outer(1)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
}

#[test]
fn local_definitions_do_not_leak_between_calls() {
    let mut session = Session::new();
    session
        .eval_source("mod local\ndef outer(x)=\ndef helper(y)=y+1\nhelper(x)\nend\nend\n")
        .unwrap();
    let value = session.eval_statement("outer(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "2");
    let value = session.eval_statement("helper(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "helper(1)");
}
