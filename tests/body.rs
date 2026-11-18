use mithqal::{ErrorCode, Session};

fn output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    session.output().to_vec()
}

#[test]
fn block_bodies_bind_intermediates() {
    let source = r#"
mod body
def blocksVar(xs)=
let n=length(xs)
let s=fold(fn(a,b)=>a+b,0,xs)
let q=fold(fn(a,b)=>a+b,0,map(xs,fn(x)=>x*x))
(n*q-s*s)/(n*n)
end
prt blocksVar([1,2,3,4])
end
"#;
    assert_eq!(output(source), vec!["5/4"]);
}

#[test]
fn block_bodies_allow_conditionals() {
    let source = r#"
mod body
def classify(x)=
if x<0 then
-1
else
if x==0 then
0
else
1
end
end
end
prt classify(-5)
prt classify(0)
prt classify(5)
end
"#;
    assert_eq!(output(source), vec!["-1", "0", "1"]);
}

#[test]
fn block_bodies_mix_let_and_if() {
    let source = r#"
mod body
def absdiff(x,y)=
let d=x-y
if d<0 then
-d
else
d
end
end
prt absdiff(3,7)
prt absdiff(7,3)
end
"#;
    assert_eq!(output(source), vec!["4", "4"]);
}

#[test]
fn sequence_parameters_work_in_definitions() {
    let source = r#"
mod patterns
def s(x__)=list(x)
prt s(1,2,3)
prt s(1)
end
"#;
    assert_eq!(output(source), vec!["[1,2,3]", "[1]"]);
}

#[test]
fn list_destructuring_parameters_bind_their_names() {
    let source = r#"
mod patterns
let a=99
def head2([a_,b_])=a
prt head2([7,8])
end
"#;
    assert_eq!(output(source), vec!["7"]);
}

#[test]
fn fixed_arity_functions_report_arity_errors() {
    let mut session = Session::new();
    session.eval_source("mod m\ndef f(x)=x\nend\n").unwrap();
    let value = session.eval_statement("f(1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "1");
    for source in ["f()", "f(1,2)"] {
        let mut session = Session::new();
        session.eval_source("mod m\ndef f(x)=x\nend\n").unwrap();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn variadic_functions_keep_unmatched_calls_unevaluated() {
    let mut session = Session::new();
    session
        .eval_source("mod m\ndef s(x__)=list(x)\nend\n")
        .unwrap();
    let value = session.eval_statement("s(1,2)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "[1,2]");
    let value = session.eval_statement("s()").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "s()");
}

#[test]
fn print_statements_are_rejected_in_a_body() {
    let source = "mod m\ndef outer(x)=\nprt x\nx\nend\nend\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn a_body_expression_must_be_last() {
    let source = "mod m\ndef outer(x)=\nx+1\nx+2\nend\nend\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}
