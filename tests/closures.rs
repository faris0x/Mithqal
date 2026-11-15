use mithqal::{ErrorCode, Session};

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn simple_closure_application() {
    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x+1").unwrap();
    assert_eq!(eval(&mut session, "f(2)"), "3");
}

#[test]
fn zero_argument_closure() {
    let mut session = Session::new();
    session.eval_statement("f=fn()=>42").unwrap();
    assert_eq!(eval(&mut session, "f()"), "42");
}

#[test]
fn multiple_parameters() {
    let mut session = Session::new();
    session.eval_statement("f=fn(x,y)=>x+y").unwrap();
    assert_eq!(eval(&mut session, "f(2,3)"), "5");
}

#[test]
fn closure_arity_is_checked() {
    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x").unwrap();
    for source in ["f()", "f(1,2)"] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn duplicate_parameters_are_rejected() {
    let mut session = Session::new();
    let error = session.eval_statement("fn(x,x)=>x").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn closures_capture_lexical_bindings() {
    let mut session = Session::new();
    let source = "let([a=10,f=fn(x)=>x+a],let([a=100],f(5)))";
    assert_eq!(eval(&mut session, source), "15");
}

#[test]
fn parameters_shadow_captured_bindings() {
    let mut session = Session::new();
    let source = "let([x=10,f=fn(x)=>x+1],f(2))";
    assert_eq!(eval(&mut session, source), "3");
}

#[test]
fn globals_resolve_at_call_time() {
    let mut session = Session::new();
    let source = "x=5\nf=fn(y)=>x+y\nx=6\nf(1)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "7");
}

#[test]
fn nested_closures() {
    let mut session = Session::new();
    let source = "f=fn(x)=>fn(y)=>x+y\ng=f(2)\ng(3)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "5");
}

#[test]
fn recursive_closures() {
    let mut session = Session::new();
    let source = "f=fn(n)=>if(n===0,1,n*f(n-1))\nf(5)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "120");
}

#[test]
fn closures_work_through_the_compiled_path() {
    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x^2").unwrap();
    let value = session.eval_compiled_statement("f(4)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "16");
}

#[test]
fn closure_values_can_be_formatted_and_reparsed() {
    let mut session = Session::new();
    let value = session.eval_statement("fn(x)=>x+1").unwrap();
    let text = session.format(value).unwrap();
    let reparsed = session.parse_statement(&text).unwrap();
    let reparsed = session.eval(reparsed).unwrap();
    assert_eq!(session.format(reparsed).unwrap(), text);
}

#[test]
fn closure_is_protected() {
    let mut session = Session::new();
    let error = session.eval_statement("closure(x)=1").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn closures_survive_collection() {
    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x+1").unwrap();
    for value in 0..20 {
        session.eval_statement(&format!("{}+1", value)).unwrap();
    }
    session.collect_garbage(&[]);
    assert_eq!(eval(&mut session, "f(4)"), "5");
}
