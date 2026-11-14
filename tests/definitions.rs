use mithqal::{ErrorCode, Session};

fn session() -> Session {
    Session::new()
}

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value)
}

#[test]
fn own_values_persist() {
    let mut session = session();
    assert_eq!(eval(&mut session, "x=3"), "3");
    assert_eq!(eval(&mut session, "x"), "3");
    assert_eq!(eval(&mut session, "x+1"), "4");
}

#[test]
fn delayed_own_values_return_null() {
    let mut session = session();
    assert_eq!(eval(&mut session, "x:=3"), "null");
    assert_eq!(eval(&mut session, "x"), "3");
}

#[test]
fn chained_assignment() {
    let mut session = session();
    assert_eq!(eval(&mut session, "a=b=1"), "1");
    assert_eq!(eval(&mut session, "a+b"), "2");
}

#[test]
fn own_values_resolve_lazily() {
    let mut session = session();
    assert_eq!(eval(&mut session, "x=y"), "y");
    assert_eq!(eval(&mut session, "y=3"), "3");
    assert_eq!(eval(&mut session, "x"), "3");
}

#[test]
fn immediate_and_delayed_functions() {
    let mut session = session();
    assert_eq!(eval(&mut session, "f(x_):=x^2"), "null");
    assert_eq!(eval(&mut session, "f(3)"), "9");
    assert_eq!(eval(&mut session, "g(x_)=x^2"), "x^2");
    assert_eq!(eval(&mut session, "g(4)"), "16");
}

#[test]
fn recursion_with_literal_base_case() {
    let mut session = session();
    let source = "fact(0):=1\nfact(n_):=n*fact(n-1)\nfact(5)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value), "120");
}

#[test]
fn literal_rules_win_over_patterns() {
    let mut session = session();
    let source = "f(x_):=x^2\nf(0):=100\nf(0)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value), "100");
    assert_eq!(eval(&mut session, "f(2)"), "4");
}

#[test]
fn equal_specificity_keeps_definition_order() {
    let mut session = session();
    let source = "f(x_):=1\nf(y_):=2\nf(10)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value), "1");
}

#[test]
fn conditions_accept_and_reject() {
    let mut session = session();
    let source = "f(when(x_,x>0)):=x\nf(5)\n";
    let value = session.eval_source(source).unwrap();
    assert_eq!(session.format_infix(value), "5");
    assert_eq!(eval(&mut session, "f(-5)"), "f(-5)");
}

#[test]
fn conditions_try_the_next_rule() {
    let mut session = session();
    let source = "f(when(x_,x<0)):=1\nf(when(x_,x>0)):=2\n";
    session.eval_source(source).unwrap();
    assert_eq!(eval(&mut session, "f(5)"), "2");
    assert_eq!(eval(&mut session, "f(-5)"), "1");
    assert_eq!(eval(&mut session, "f(0)"), "f(0)");
}

#[test]
fn repeated_pattern_variables_must_agree() {
    let mut session = session();
    session.eval_statement("eq2(x_,x_):=true").unwrap();
    assert_eq!(eval(&mut session, "eq2(a,a)"), "true");
    assert_eq!(eval(&mut session, "eq2(a,b)"), "eq2(a,b)");
}

#[test]
fn sequence_patterns_bind_one_or_more() {
    let mut session = session();
    session.eval_statement("s(x__):=list(x)").unwrap();
    assert_eq!(eval(&mut session, "s(1,2,3)"), "[1,2,3]");
    assert_eq!(eval(&mut session, "s(1)"), "[1]");
    assert_eq!(eval(&mut session, "s()"), "s()");
}

#[test]
fn null_sequence_patterns_accept_zero_arguments() {
    let mut session = session();
    session.eval_statement("n(x___):=list(x)").unwrap();
    assert_eq!(eval(&mut session, "n()"), "[]");
    assert_eq!(eval(&mut session, "n(1,2)"), "[1,2]");
}

#[test]
fn sequence_patterns_mix_with_fixed_arguments() {
    let mut session = session();
    session.eval_statement("p(x_,y__):=list(x,y)").unwrap();
    assert_eq!(eval(&mut session, "p(1,2,3)"), "[1,2,3]");
    assert_eq!(eval(&mut session, "p(1)"), "p(1)");
}

#[test]
fn anonymous_blanks_match_anything() {
    let mut session = session();
    session.eval_statement("q(_):=7").unwrap();
    assert_eq!(eval(&mut session, "q(a)"), "7");
    assert_eq!(eval(&mut session, "q(1+2)"), "7");
}

#[test]
fn patterns_can_match_inside_orderless_heads() {
    let mut session = session();
    session.eval_statement("firstSum(plus(x_,y_)):=x").unwrap();
    assert_eq!(eval(&mut session, "firstSum(a+b)"), "a");
}

#[test]
fn let_introduces_lexical_bindings() {
    let mut session = session();
    assert_eq!(eval(&mut session, "let([x=3],x+1)"), "4");
    assert_eq!(eval(&mut session, "let([x=1,y=2],x+y)"), "3");
    assert_eq!(eval(&mut session, "let([x=1],let([y=2],x+y))"), "3");
}

#[test]
fn let_shadows_global_definitions() {
    let mut session = session();
    session.eval_statement("x=10").unwrap();
    assert_eq!(eval(&mut session, "let([x=1],x+1)"), "2");
    assert_eq!(eval(&mut session, "x"), "10");
}

#[test]
fn let_bindings_see_the_outer_environment() {
    let mut session = session();
    session.eval_statement("x=5").unwrap();
    assert_eq!(eval(&mut session, "let([y=x+1],y)"), "6");
}

#[test]
fn let_errors() {
    let mut session = session();
    for source in ["let(x,x)", "let([1=2],x)", "let([x],x)", "let([x=1])"] {
        let error = session.eval_statement(source).unwrap_err();
        assert!(
            error.code == ErrorCode::EvalDefinition || error.code == ErrorCode::EvalArity,
            "source {source}: {error:?}"
        );
    }
}

#[test]
fn protected_symbols_reject_definitions() {
    let mut session = session();
    for source in ["inf=3", "emptyset=1", "true=1", "plus(x_):=1"] {
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "source {source}");
    }
}

#[test]
fn invalid_definition_targets_are_reported() {
    let mut session = session();
    let error = session.eval_statement("1=2").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn constants_can_be_shadowed() {
    let mut session = session();
    let value = session.eval_source("i=3\ni^2\n").unwrap();
    assert_eq!(session.format_infix(value), "9");
    let value = session.eval_source("pi=3\npi+1\n").unwrap();
    assert_eq!(session.format_infix(value), "4");
    let value = session.eval_source("e=2\ne^3\n").unwrap();
    assert_eq!(session.format_infix(value), "8");
}

#[test]
fn shadowing_i_disables_gaussian_normalization() {
    let mut session = session();
    let value = session.eval_source("i=3\n(1+i)*(1-i)\n").unwrap();
    assert_eq!(session.format_infix(value), "-8");
}

#[test]
fn result_caches_invalidate_when_definitions_change() {
    let mut session = session();
    assert_eq!(eval(&mut session, "x+1"), "1+x");
    assert_eq!(eval(&mut session, "x+1"), "1+x");
    session.eval_statement("x=1").unwrap();
    assert_eq!(eval(&mut session, "x+1"), "2");
}

#[test]
fn compiled_result_caches_invalidate_when_definitions_change() {
    let mut session = session();
    let first = session.eval_compiled_statement("x+1").unwrap();
    assert_eq!(session.format_infix(first), "1+x");
    session.eval_statement("x=1").unwrap();
    let second = session.eval_compiled_statement("x+1").unwrap();
    assert_eq!(session.format_infix(second), "2");
}

#[test]
fn call_heads_are_evaluated() {
    let mut session = session();
    session.eval_statement("f=plus").unwrap();
    assert_eq!(eval(&mut session, "f(1,2)"), "3");
    session.eval_statement("g=3").unwrap();
    let error = session.eval_statement("g(1)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalNotCallable);
}

#[test]
fn recursive_definitions_respect_the_budget() {
    let config = mithqal::Config {
        max_steps: 50,
        max_depth: 32,
        ..mithqal::Config::default()
    };
    let mut session = Session::with_config(config);
    session.eval_statement("f(x_):=f(x)").unwrap();
    let error = session.eval_statement("f(1)").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}
