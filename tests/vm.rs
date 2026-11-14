use cas::{Config, ErrorCode, Session};

fn both(source: &str) -> (String, String) {
    let mut session = Session::new();
    let normal = session.eval_statement(source).unwrap();
    let normal_text = session.format_infix(normal);
    let compiled = session.eval_compiled_statement(source).unwrap();
    let compiled_text = session.format_infix(compiled);
    (normal_text, compiled_text)
}

fn same(source: &str) -> String {
    let (normal, compiled) = both(source);
    assert_eq!(normal, compiled, "normal and compiled differ for {source}");
    normal
}

#[test]
fn compiled_arithmetic_matches_the_evaluator() {
    for source in [
        "1+2", "2*3+4", "x+x", "x^2/x", "2/3+1/6", "1/0", "0/0", "i^2", "[1,2]+3", "f(1+2)",
    ] {
        same(source);
    }
}

#[test]
fn compiled_if_control_flow() {
    assert_eq!(same("if(true,1,2)"), "1");
    assert_eq!(same("if(false,1,2)"), "2");
    assert_eq!(same("if(1<2,10,20)"), "10");
    assert_eq!(same("if(1>2,10,20)"), "20");
    assert_eq!(same("if(x,1,2)"), "if(x,1,2)");
    assert_eq!(same("if(undefined,1,2)"), "undefined");
    assert_eq!(same("if(indeterminate,1,2)"), "indeterminate");
    assert_eq!(same("if(failed,1,2)"), "failed");
    assert_eq!(same("if(true,1,1/0)"), "1");
    assert_eq!(same("if(false,1/0,2)"), "2");
    assert_eq!(same("if(true,if(false,1,2),3)"), "2");
}

#[test]
fn compiled_logic_and_control_forms() {
    for source in [
        "true&&false",
        "true||false",
        "!true",
        "and()",
        "or()",
        "let([x=3],x+1)",
    ] {
        same(source);
    }
}

#[test]
fn compiled_definitions_share_session_state() {
    let mut session = Session::new();
    session.eval_statement("f(x_):=x^2").unwrap();
    let value = session.eval_compiled_statement("f(5)").unwrap();
    assert_eq!(session.format_infix(value), "25");
}

#[test]
fn compiled_recursion_matches_the_evaluator() {
    let mut session = Session::new();
    session
        .eval_source("fact(0):=1\nfact(n_):=n*fact(n-1)\n")
        .unwrap();
    let value = session.eval_compiled_statement("fact(6)").unwrap();
    assert_eq!(session.format_infix(value), "720");
}

#[test]
fn compiled_path_can_be_called_with_a_handle() {
    let mut session = Session::new();
    let parsed = session.parse_statement("x+x").unwrap();
    let value = session.eval_compiled(parsed).unwrap();
    assert_eq!(session.format_infix(value), "2*x");
}

#[test]
fn compiled_path_reports_parse_errors() {
    let mut session = Session::new();
    let error = session.eval_compiled_statement("1 + 2").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseWhitespace);
}

#[test]
fn compiled_path_respects_the_budget() {
    let config = Config {
        max_steps: 1,
        max_depth: 8,
        ..Config::default()
    };
    let mut session = Session::with_config(config);
    let error = session.eval_compiled_statement("1+2").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}

#[test]
fn compiled_path_survives_garbage_collection() {
    let mut session = Session::new();
    session.eval_statement("f(x_):=x+1").unwrap();
    session.collect_garbage(&[]);
    let value = session.eval_compiled_statement("f(2)").unwrap();
    assert_eq!(session.format_infix(value), "3");
}

#[test]
fn compiled_and_normal_paths_agree_on_a_broad_corpus() {
    for source in [
        "0",
        "1",
        "1/2",
        "x",
        "x+y",
        "2x",
        "(x+1)(x-1)",
        "x^2y",
        "f(x_)",
        "x_->x^2",
        "x===y",
        "x=/=y",
        "x~y",
        "a&&b||c",
        "if(x===x,1,2)",
        "if(x<y,1,2)",
        "fn(x)=>x+1",
        "let([x=1],x+1)",
        "f(when(x_,x>0))",
        "[[1,2],[3,4]]",
        "[1,2]^2",
        "(1+i)*(1-i)",
        "(2+3*i)/(1-i)",
    ] {
        same(source);
    }
}
